mod mcp;
mod types;
mod vertebrae;

use axum::{
    body::Body,
    extract::{Json, State},
    http::{header, HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use futures_util::StreamExt;
use serde_json::json;
use std::{net::SocketAddr, sync::Arc, time::Instant};
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

use types::{ChatCompletionRequest, ModelInfo};
use vertebrae::{
    strip_initial_cushions, AdversarialAuditEngine, DemuxedChunk, RealityLevel, ReasoningDemuxer,
    SpineAuditEngine, SpineTelemetrySnapshot, StreamTailSanitizer,
};



#[derive(Clone)]
pub struct AppState {
    pub openrouter_api_key: String,
    pub chronofact_url: Option<String>,
    pub http_client: reqwest::Client,
    pub tx: broadcast::Sender<serde_json::Value>,
    pub recent_events: Arc<tokio::sync::RwLock<Vec<serde_json::Value>>>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let openrouter_key = std::env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| "".to_string());
    let chronofact_url = std::env::var("CHRONOFACT_URL").ok();

    // Check if invoked as an MCP server by IDE (Cursor / Antigravity / VS Code)
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "mcp" {
        mcp::run_stdio_server(openrouter_key).await;
        return;
    }

    tracing_subscriber::fmt::init();
    if openrouter_key.is_empty() {
        warn!("OPENROUTER_API_KEY is not set in environment or .env!");
    } else {
        info!("OPENROUTER_API_KEY detected and loaded successfully.");
    }

    if let Some(ref cf_url) = chronofact_url {
        info!("ChronoFact Pipeline Middleware active pointing to: {}", cf_url);
    }

    let (tx, _rx) = broadcast::channel(100);

    let state = Arc::new(AppState {
        openrouter_api_key: openrouter_key,
        chronofact_url,
        http_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap(),
        tx,
        recent_events: Arc::new(tokio::sync::RwLock::new(Vec::new())),
    });

    // Only the local HUD may call the gateway cross-origin.
    let allowed_origins: Vec<axum::http::HeaderValue> = [
        "http://localhost:3333",
        "http://127.0.0.1:3333",
    ]
    .iter()
    .filter_map(|o| o.parse().ok())
    .collect();
    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/v1/models", get(models_handler))
        .route("/v1/chat/completions", post(chat_completions_handler))
        .route("/v1/completions", post(completions_handler))
        .route("/api/spine/audit", post(audit_spine_handler))
        .route("/api/spine/notify", post(notify_spine_handler))
        .route("/api/spine/events", get(events_spine_handler))
        .route("/api/spine/history", get(history_spine_handler))
        .route("/api/spine/logs", get(logs_spine_handler))
        .route("/api/spine/redteam", post(redteam_spine_handler))
        .route("/api/spine/attest", post(attest_spine_handler))
        .layer(cors)
        .layer(axum::middleware::from_fn(loopback_host_guard))
        .with_state(state);

    let port = 8080;
    // Loopback only: this gateway can spend OPENROUTER_API_KEY and exposes prompt logs.
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("SPINE Reality Gateway starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Rejects requests whose Host header is not loopback (defends against DNS rebinding
/// from a browser page reaching the local gateway).
async fn loopback_host_guard(
    req: axum::http::Request<Body>,
    next: axum::middleware::Next,
) -> Response {
    let host_ok = req
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .map(|h| {
            let name = h.rsplit_once(':').map(|(n, _)| n).unwrap_or(h);
            matches!(name, "localhost" | "127.0.0.1" | "[::1]")
        })
        .unwrap_or(false);
    if !host_ok {
        return (StatusCode::FORBIDDEN, "SPINE gateway only serves loopback Host headers").into_response();
    }
    next.run(req).await
}

async fn health_handler() -> impl IntoResponse {
    Json(json!({
        "status": "online",
        "system": "SPINE Anti-Sycophancy Reality Gateway",
        "vertebrae_count": 33,
        "mode": "Dual Gateway + MCP HUD Server",
        "version": "1.0.0"
    }))
}

async fn completions_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> Response {
    let is_stream = req.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);
    let resp_res = state
        .http_client
        .post("http://127.0.0.1:8089/v1/completions")
        .json(&req)
        .send()
        .await;

    match resp_res {
        Ok(resp) => {
            let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::OK);
            if !resp.status().is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                return (status, Json(json!({ "error": err_text }))).into_response();
            }

            if is_stream {
                let builder = Response::builder()
                    .status(status)
                    .header(header::CONTENT_TYPE, "text/event-stream")
                    .header(header::CACHE_CONTROL, "no-cache")
                    .header(header::CONNECTION, "keep-alive");
                let body = Body::from_stream(resp.bytes_stream());
                return builder.body(body).unwrap().into_response();
            } else {
                let json_body: serde_json::Value = resp.json().await.unwrap_or(json!({}));
                return (status, Json(json_body)).into_response();
            }
        }
        Err(e) => {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": format!("Harness server unavailable: {}", e) })),
            )
                .into_response()
        }
    }
}

async fn models_handler() -> impl IntoResponse {
    // Factual, verified active October 2026 models
    let models = vec![
        ModelInfo {
            id: "google/gemini-4-argon".to_string(),
            name: "Gemini 4 Argon".to_string(),
            provider: "Google DeepMind".to_string(),
            description: "Active 2026 Next-Gen Argon Frontier Reasoning & Coding Model".to_string(),
            is_local: false,
        },
        ModelInfo {
            id: "google/gemini-3.8-flash".to_string(),
            name: "Gemini 3.8 Flash".to_string(),
            provider: "Google DeepMind".to_string(),
            description: "Active 2026 Flagship Multimodal Flash Model".to_string(),
            is_local: false,
        },
        ModelInfo {
            id: "anthropic/claude-opus-5.5".to_string(),
            name: "Claude Opus 5.5".to_string(),
            provider: "Anthropic".to_string(),
            description: "Active Frontier Flagship (Replaces retired Claude 3.5 Sonnet)".to_string(),
            is_local: false,
        },
        ModelInfo {
            id: "x-ai/grok-4.7".to_string(),
            name: "Grok 4.7".to_string(),
            provider: "xAI".to_string(),
            description: "Active Frontier Reasoner".to_string(),
            is_local: false,
        },
        ModelInfo {
            id: "deepseek/deepseek-v4.1-flash".to_string(),
            name: "DeepSeek V4.1 Flash".to_string(),
            provider: "DeepSeek".to_string(),
            description: "Active MoE High-Speed Architecture".to_string(),
            is_local: false,
        },
        ModelInfo {
            id: "openai/astra-6".to_string(),
            name: "Astra 6".to_string(),
            provider: "OpenAI".to_string(),
            description: "Active 2026 OpenAI Frontier Reasoning Model".to_string(),
            is_local: false,
        },
        ModelInfo {
            id: "openai/sol-6.1".to_string(),
            name: "Sol 6.1".to_string(),
            provider: "OpenAI".to_string(),
            description: "Active 2026 OpenAI Flagship Foundation Model".to_string(),
            is_local: false,
        },
        ModelInfo {
            id: "openai/gpt-4o-2024-11-20".to_string(),
            name: "GPT-4o".to_string(),
            provider: "OpenAI".to_string(),
            description: "Flagship Multimodal Foundation Engine".to_string(),
            is_local: false,
        },
        ModelInfo {
            id: "meta-llama/llama-3.3-70b-instruct".to_string(),
            name: "Llama 3.3 70B Instruct".to_string(),
            provider: "Meta Open Weights".to_string(),
            description: "Industry-standard open weights instruction model".to_string(),
            is_local: false,
        },
        ModelInfo {
            id: "ollama/llama3.3:70b".to_string(),
            name: "Llama 3.3 70B (Local)".to_string(),
            provider: "Local Ollama".to_string(),
            description: "Runs locally on your machine via Ollama (port 11434)".to_string(),
            is_local: true,
        },
        ModelInfo {
            id: "harness/qwen2.5-coder:32b".to_string(),
            name: "Qwen2.5-Coder 32B (Harness Speculative ~3 tok/s)".to_string(),
            provider: "Harness Local GPU".to_string(),
            description: "Runs on bare metal RTX 5060 + DDR4 with 1.5B speculative drafter".to_string(),
            is_local: true,
        },
    ];

    Json(json!({ "object": "list", "data": models }))
}

#[derive(serde::Deserialize)]
pub struct AuditRequest {
    pub messages: Vec<types::ChatMessage>,
    pub reality_level: Option<u8>,
    pub model: Option<String>,
}

async fn audit_spine_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AuditRequest>,
) -> impl IntoResponse {
    let reality_level = RealityLevel::from_u8(payload.reality_level.unwrap_or(4));
    let model = payload.model.unwrap_or_else(|| "google/gemini-4-argon".to_string());
    
    let (vertebrae, _prompt) = SpineAuditEngine::audit_input(&payload.messages, reality_level, &model);
    let active_count = vertebrae.iter().filter(|v| v.active).count();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let prompt_text = payload.messages.last().map(|m| m.content.clone()).unwrap_or_default();
    let pushback_detected = vertebrae.iter().any(|v| v.id == "T4" && v.active);

    let snapshot = json!({
        "id": next_event_id("evt"),
        "timestamp": now,
        "source": "SPINE Reality Gateway (/api/spine/audit)",
        "prompt": prompt_text,
        "vertebrae": vertebrae,
        "active_vertebrae_count": active_count,
        "total_vertebrae": 33,
        "reality_level": reality_level as u8,
        "model": model,
        "pushback_detected": pushback_detected,
    });

    {
        let mut history = state.recent_events.write().await;
        history.push(snapshot.clone());
        if history.len() > 50 {
            history.remove(0);
        }
    }

    // Persistent disk logging
    append_to_telemetry_log(&snapshot);

    let _ = state.tx.send(snapshot.clone());

    Json(snapshot)
}

static EVENT_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
const LOG_ROTATE_BYTES: u64 = 10 * 1024 * 1024;
const LOG_KEEP_FILES: usize = 3;
const LOG_PROMPT_MAX_CHARS: usize = 1000;

/// Collision-free event id: millisecond timestamp + process-wide sequence.
pub fn next_event_id(prefix: &str) -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let seq = EVENT_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{}_{}_{}", prefix, ms, seq)
}

pub fn telemetry_log_dir() -> std::path::PathBuf {
    std::env::var("SPINE_LOG_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("C:/spine/logs"))
}

fn rotate_telemetry_log(log_dir: &std::path::Path) {
    let log_file = log_dir.join("telemetry.jsonl");
    let too_big = std::fs::metadata(&log_file).map(|m| m.len() > LOG_ROTATE_BYTES).unwrap_or(false);
    if !too_big {
        return;
    }
    let _ = std::fs::remove_file(log_dir.join(format!("telemetry.{}.jsonl", LOG_KEEP_FILES)));
    for i in (1..LOG_KEEP_FILES).rev() {
        let _ = std::fs::rename(
            log_dir.join(format!("telemetry.{}.jsonl", i)),
            log_dir.join(format!("telemetry.{}.jsonl", i + 1)),
        );
    }
    let _ = std::fs::rename(&log_file, log_dir.join("telemetry.1.jsonl"));
}

pub fn append_to_telemetry_log(event: &serde_json::Value) {
    let log_dir = telemetry_log_dir();
    if !log_dir.exists() {
        let _ = std::fs::create_dir_all(&log_dir);
    }
    rotate_telemetry_log(&log_dir);
    // Truncate the stored prompt: logs are local-only, but unbounded prompts bloat the file.
    let mut event = event.clone();
    if let Some(p) = event.get("prompt").and_then(|p| p.as_str()) {
        if p.chars().count() > LOG_PROMPT_MAX_CHARS {
            let cut: String = p.chars().take(LOG_PROMPT_MAX_CHARS).collect();
            event["prompt"] = json!(format!("{}…[truncated]", cut));
        }
    }
    let log_file = log_dir.join("telemetry.jsonl");
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(log_file) {
        use std::io::Write;
        if let Ok(json_str) = serde_json::to_string(&event) {
            let _ = writeln!(file, "{}", json_str);
        }
    }
}

async fn notify_spine_handler(
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Some(obj) = payload.as_object_mut() {
        if !obj.contains_key("timestamp") {
            obj.insert("timestamp".to_string(), json!(now));
        }
        if !obj.contains_key("id") {
            obj.insert("id".to_string(), json!(next_event_id("evt")));
        }
    }

    let already_persisted = payload.get("persisted").and_then(|v| v.as_bool()).unwrap_or(false);
    if !already_persisted {
        append_to_telemetry_log(&payload);
    }

    {
        let mut history = state.recent_events.write().await;
        history.push(payload.clone());
        if history.len() > 50 {
            history.remove(0);
        }
    }

    let _ = state.tx.send(payload);
    Json(json!({ "status": "broadcasted" }))
}

async fn history_spine_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let history = state.recent_events.read().await;
    Json(json!({
        "events": *history,
        "count": history.len()
    }))
}

async fn logs_spine_handler() -> impl IntoResponse {
    const TAIL_BYTES: u64 = 2 * 1024 * 1024;
    let log_path = telemetry_log_dir().join("telemetry.jsonl");
    let mut events = Vec::new();
    if let Ok(mut file) = std::fs::File::open(&log_path) {
        use std::io::{Read, Seek, SeekFrom};
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
        let start = len.saturating_sub(TAIL_BYTES);
        if file.seek(SeekFrom::Start(start)).is_ok() {
            let mut buf = Vec::new();
            if file.read_to_end(&mut buf).is_ok() {
                let text = String::from_utf8_lossy(&buf);
                let mut lines = text.lines();
                if start > 0 {
                    lines.next(); // first line is likely partial
                }
                for line in lines {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                            events.push(val);
                        }
                    }
                }
            }
        }
    }
    events.reverse();
    events.truncate(500);
    Json(json!({
        "events": events,
        "count": events.len()
    }))
}

async fn events_spine_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut rx = state.tx.subscribe();

    let stream = async_stream::stream! {
        while let Ok(msg) = rx.recv().await {
            yield Ok::<Event, std::convert::Infallible>(
                Event::default().event("spine_telemetry").data(msg.to_string())
            );
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn chat_completions_handler(
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<ChatCompletionRequest>,
) -> Response {
    let start_time = Instant::now();
    let reality_level = RealityLevel::from_u8(req.reality_level.unwrap_or(4));
    
    let (vertebrae, directive) = SpineAuditEngine::audit_input(&req.messages, reality_level, &req.model);
    let active_vertebrae_count = vertebrae.iter().filter(|v| v.active).count();

    let prompt_text = req.messages.iter().rev().find(|m| m.role == "user").map(|m| m.content.clone()).unwrap_or_default();
    let prior_claim = req.messages.iter().rev().find(|m| m.role == "assistant").map(|m| m.content.clone()).unwrap_or_default();

    let pushback = SpineAuditEngine::verify_pushback_grounded(&prompt_text, &prior_claim, reality_level);
    if pushback.verdict == vertebrae::PushbackVerdict::ConcedeAndCorrect {
        req.messages.insert(
            0,
            types::ChatMessage {
                role: "system".to_string(),
                content: pushback.directive,
                name: None,
            },
        );
    } else if pushback.verdict == vertebrae::PushbackVerdict::ConcedeAndAdapt {
        req.messages.insert(
            0,
            types::ChatMessage {
                role: "system".to_string(),
                content: format!("{}\n\n{}", directive, pushback.directive),
                name: None,
            },
        );
    } else if pushback.verdict == vertebrae::PushbackVerdict::ObjectiveMatrix {
        req.messages.insert(
            0,
            types::ChatMessage {
                role: "system".to_string(),
                content: format!("{}\n\n{}", directive, pushback.directive),
                name: None,
            },
        );
    } else if pushback.verdict == vertebrae::PushbackVerdict::HoldTheLine && (pushback.has_authority_intimidation || pushback.demands_unearned_apology || !pushback.technical_indicators_found.is_empty()) {
        req.messages.insert(
            0,
            types::ChatMessage {
                role: "system".to_string(),
                content: format!("{}\n\n{}", directive, pushback.directive),
                name: None,
            },
        );
    } else if !directive.is_empty() {
        req.messages.insert(
            0,
            types::ChatMessage {
                role: "system".to_string(),
                content: directive,
                name: None,
            },
        );
    }

    // Optional Zero-Config Pipeline Middleware: Transparently queries ChronoFact (:3030) if CHRONOFACT_URL is set
    if let Some(ref cf_url) = state.chronofact_url {
        let endpoint = format!("{}/api/temporal/check", cf_url.trim_end_matches('/'));
        let cf_payload = json!({
            "model_id": req.model,
            "query": prompt_text,
        });

        if let Ok(cf_res) = state.http_client
            .post(&endpoint)
            .json(&cf_payload)
            .timeout(std::time::Duration::from_millis(500))
            .send()
            .await
        {
            if cf_res.status().is_success() {
                if let Ok(cf_body) = cf_res.json::<serde_json::Value>().await {
                    if let Some(cal_block) = cf_body.get("calibration_block").and_then(|v| v.as_str()) {
                        let requires_grounding = cf_body.get("requires_grounding").and_then(|v| v.as_bool()).unwrap_or(false);
                        let risk = cf_body.get("temporal_risk_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let is_outdated = cf_body.get("is_model_outdated_or_retired").and_then(|v| v.as_bool()).unwrap_or(false);

                        if requires_grounding || risk >= 0.35 || is_outdated {
                            req.messages.insert(
                                0,
                                types::ChatMessage {
                                    role: "system".to_string(),
                                    content: cal_block.to_string(),
                                    name: None,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    let is_stream = req.stream.unwrap_or(true);
    let model_id = req.model.clone();

    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    let event_obj = json!({
        "id": next_event_id("evt"),
        "source": "SPINE Gateway (/v1/chat/completions)",
        "tool": "spine_execute_grounded_reality",
        "vertebrae": vertebrae,
        "active_vertebrae_count": active_vertebrae_count,
        "reality_level": reality_level as u8,
        "model": model_id,
        "prompt": prompt_text,
        "timestamp": now_ts,
    });

    {
        let mut history = state.recent_events.write().await;
        history.push(event_obj.clone());
        if history.len() > 50 {
            history.remove(0);
        }
    }

    // Persistent disk logging
    append_to_telemetry_log(&event_obj);

    // Broadcast audit event to HUD
    let _ = state.tx.send(event_obj);

    if model_id.starts_with("ollama/") {
        let clean_model = model_id.trim_start_matches("ollama/");
        let ollama_payload = json!({
            "model": clean_model,
            "messages": req.messages,
            "stream": is_stream,
            "options": {
                "temperature": req.temperature.unwrap_or(0.2)
            }
        });

        let resp_res = state
            .http_client
            .post("http://localhost:11434/api/chat")
            .json(&ollama_payload)
            .send()
            .await;

        match resp_res {
            Ok(resp) => {
                let stream = resp.bytes_stream().map(|item| {
                    match item {
                        Ok(bytes) => Ok::<Event, std::io::Error>(Event::default().data(String::from_utf8_lossy(&bytes).to_string())),
                        Err(e) => Ok::<Event, std::io::Error>(Event::default().data(format!("{{\"error\": \"{}\"}}", e))),
                    }
                });
                return Sse::new(stream).keep_alive(KeepAlive::default()).into_response();
            }
            Err(e) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({ "error": format!("Local Ollama unavailable: {}", e) })),
                )
                    .into_response();
            }
        }
    }

    if model_id.starts_with("harness/") || model_id == "qwen2.5-coder:32b" || model_id == "local/qwen32b" {
        let clean_model = if model_id.starts_with("harness/") {
            model_id.trim_start_matches("harness/")
        } else {
            &model_id
        };
        let mut harness_payload = json!({
            "model": clean_model,
            "messages": req.messages,
            "stream": is_stream,
            "temperature": req.temperature.unwrap_or(0.2),
        });
        if let Some(max_tok) = req.max_tokens {
            harness_payload["max_tokens"] = json!(max_tok);
        }

        let resp_res = state
            .http_client
            .post("http://127.0.0.1:8089/v1/chat/completions")
            .json(&harness_payload)
            .send()
            .await;

        match resp_res {
            Ok(resp) => {
                let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::OK);
                if !resp.status().is_success() {
                    let err_text = resp.text().await.unwrap_or_default();
                    return (status, Json(json!({ "error": err_text }))).into_response();
                }

                if is_stream {
                    let builder = Response::builder()
                        .status(status)
                        .header(header::CONTENT_TYPE, "text/event-stream")
                        .header(header::CACHE_CONTROL, "no-cache")
                        .header(header::CONNECTION, "keep-alive");
                    let body = Body::from_stream(resp.bytes_stream());
                    return builder.body(body).unwrap().into_response();
                } else {
                    let json_body: serde_json::Value = resp.json().await.unwrap_or(json!({}));
                    return (status, Json(json_body)).into_response();
                }
            }
            Err(e) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({ "error": format!("Harness accelerated model server unavailable on http://127.0.0.1:8089: {}. Start it with C:\\Harness\\launch_qwen32b_coder.bat", e) })),
                )
                    .into_response();
            }
        }
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {}", state.openrouter_api_key).parse().unwrap(),
    );
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("HTTP-Referer", "https://spine-gateway.local".parse().unwrap());
    headers.insert("X-Title", "SPINE Reality Gateway".parse().unwrap());

    let payload = json!({
        "model": model_id,
        "messages": req.messages,
        "stream": is_stream,
        "temperature": req.temperature.unwrap_or(0.2),
        "max_tokens": req.max_tokens.unwrap_or(4096),
    });

    let resp_res = state
        .http_client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .headers(headers)
        .json(&payload)
        .send()
        .await;

    let upstream_resp = match resp_res {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": format!("Upstream dispatch failure: {}", e) })),
            )
                .into_response();
        }
    };

    if !upstream_resp.status().is_success() {
        let err_text = upstream_resp.text().await.unwrap_or_default();
        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("Upstream API error: {}", err_text) })),
        )
            .into_response();
    }

    if is_stream {
        let mut byte_stream = upstream_resp.bytes_stream();
        let pushbacks_resisted = vertebrae.iter().filter(|v| v.id == "T4" && v.active).count() as u32;
        let telemetry_snapshot = SpineTelemetrySnapshot {
            vertebrae,
            overall_rigidity: match reality_level {
                RealityLevel::Diplomatic => 0.25,
                RealityLevel::Objective => 0.65,
                RealityLevel::Rigorous => 0.88,
                RealityLevel::BrutalReality => 0.99,
            },
            active_vertebrae_count,
            pushbacks_resisted,
            apologies_intercepted: 0,
            ttft_ms: start_time.elapsed().as_millis() as u64,
            // Not measured at snapshot time (stream has not started); 0.0 = unknown, not a real rate.
            tokens_per_sec: 0.0,
            total_tokens: 0,
        };

        let initial_telemetry = Event::default()
            .event("spine_telemetry")
            .data(serde_json::to_string(&telemetry_snapshot).unwrap_or_default());

        let mapped_stream = async_stream::stream! {
            yield Ok::<Event, std::convert::Infallible>(initial_telemetry);

            let mut filter_active = true;
            let mut reasoning_demuxer = ReasoningDemuxer::new();
            let mut buffered_chunks: Vec<serde_json::Value> = Vec::new();
            let mut buffered_text = String::new();
            let mut tail_sanitizer = StreamTailSanitizer::new(64);
            let mut phase1_token_delta: i64 = 0;
            let mut sse_line_buffer = String::new();

            while let Some(chunk) = byte_stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        sse_line_buffer.push_str(&String::from_utf8_lossy(&bytes));
                        let complete_part = if let Some(last_nl) = sse_line_buffer.rfind('\n') {
                            let part = sse_line_buffer[..=last_nl].to_string();
                            sse_line_buffer.drain(..=last_nl);
                            part
                        } else {
                            continue;
                        };
                        for line in complete_part.lines() {
                            if line.starts_with("data: ") {
                                let data_content = line.trim_start_matches("data: ").trim();
                                if data_content == "[DONE]" {
                                    // Flush any remaining prefix in reasoning demuxer
                                    let final_demux = reasoning_demuxer.flush_final();
                                    for ch in final_demux {
                                        match ch {
                                            DemuxedChunk::Reasoning(thought_part) => {
                                                let thought_evt = json!({
                                                    "id": "chatcmpl-spine-thought-tail",
                                                    "object": "chat.completion.chunk",
                                                    "choices": [{
                                                        "delta": { "content": thought_part },
                                                        "index": 0,
                                                        "finish_reason": null
                                                    }]
                                                });
                                                yield Ok::<Event, std::convert::Infallible>(Event::default().data(thought_evt.to_string()));
                                            }
                                            DemuxedChunk::Visible(vis_part) => {
                                                if filter_active {
                                                    buffered_text.push_str(&vis_part);
                                                } else {
                                                    let drained = tail_sanitizer.push_and_drain(&vis_part);
                                                    if !drained.is_empty() {
                                                        let vis_evt = json!({
                                                            "id": "chatcmpl-spine-vis-tail",
                                                            "object": "chat.completion.chunk",
                                                            "choices": [{
                                                                "delta": { "content": drained },
                                                                "index": 0,
                                                                "finish_reason": null
                                                            }]
                                                        });
                                                        yield Ok::<Event, std::convert::Infallible>(Event::default().data(vis_evt.to_string()));
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Flush any remaining buffered chunks before completing
                                    if filter_active && !buffered_chunks.is_empty() {
                                        let safe_idx = buffered_text.floor_char_boundary(buffered_text.len());
                                        let orig_chars = buffered_text[..safe_idx].chars().count();
                                        let (sanitized, _intercepted) = strip_initial_cushions(&buffered_text[..safe_idx]);
                                        let sanitized_chars = sanitized.chars().count();
                                        if orig_chars > sanitized_chars {
                                            let diff = orig_chars - sanitized_chars;
                                            phase1_token_delta -= ((diff as f64) / 3.8).round().max(1.0) as i64;
                                        }

                                        let drained = tail_sanitizer.push_and_drain(&sanitized);
                                        if let Some(first) = buffered_chunks.first_mut() {
                                            if let Some(target) = first.pointer_mut("/choices/0/delta/content") {
                                                *target = serde_json::Value::String(drained);
                                            }
                                            yield Ok::<Event, std::convert::Infallible>(Event::default().data(first.to_string()));
                                        }
                                        buffered_chunks.clear();
                                        buffered_text.clear();
                                        filter_active = false;
                                    }

                                    // Flush final remaining lookahead bytes from StreamTailSanitizer
                                    if let Some(final_tail) = tail_sanitizer.flush_final_chunk() {
                                        let final_evt = json!({
                                            "id": "chatcmpl-spine-tail",
                                            "object": "chat.completion.chunk",
                                            "choices": [{
                                                "delta": { "content": final_tail },
                                                "index": 0,
                                                "finish_reason": null
                                            }]
                                        });
                                        yield Ok::<Event, std::convert::Infallible>(Event::default().data(final_evt.to_string()));
                                    }

                                    yield Ok::<Event, std::convert::Infallible>(Event::default().data("[DONE]"));
                                } else if let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(data_content) {
                                    // Dual-Usage Accounting: maintain both client-sanitized completion_tokens
                                    // and upstream billed completion_tokens to prevent chargeback divergence.
                                    if let Some(usage) = parsed.get_mut("usage") {
                                        if let Some(raw_comp_tok) = usage.get("completion_tokens").and_then(|v| v.as_i64()) {
                                            usage["upstream_completion_tokens"] = json!(raw_comp_tok);
                                            let total_token_delta = phase1_token_delta + tail_sanitizer.estimated_token_delta();
                                            let adjusted = (raw_comp_tok + total_token_delta).max(0);
                                            usage["completion_tokens"] = json!(adjusted);
                                            if let Some(prompt_tok) = usage.get("prompt_tokens").and_then(|v| v.as_i64()) {
                                                usage["upstream_total_tokens"] = json!(prompt_tok + raw_comp_tok);
                                                usage["total_tokens"] = json!(prompt_tok + adjusted);
                                            }
                                        }
                                    }

                                    // CoT Exemption Boundary 1: Dedicated provider thought channels (reasoning_content / thought / reasoning / thinking)
                                    let has_reasoning_channel = parsed.pointer("/choices/0/delta/reasoning_content").is_some()
                                        || parsed.pointer("/choices/0/delta/thought").is_some()
                                        || parsed.pointer("/choices/0/delta/reasoning").is_some()
                                        || parsed.pointer("/choices/0/delta/thinking").is_some()
                                        || parsed.pointer("/delta/thinking").is_some()
                                        || parsed.get("type").and_then(|t| t.as_str()) == Some("thinking_delta")
                                        || parsed.pointer("/delta/type").and_then(|t| t.as_str()) == Some("thinking_delta");

                                    if has_reasoning_channel {
                                        yield Ok::<Event, std::convert::Infallible>(Event::default().data(parsed.to_string()));
                                        continue;
                                    }

                                    // CoT Exemption Boundary 2: 3-State Sliding Window Demuxer for Inline Reasoning Tags (<think>...</think>, etc.)
                                    let delta_content_opt = parsed.pointer("/choices/0/delta/content").and_then(|v| v.as_str()).map(|s| s.to_string());
                                    if let Some(ref content) = delta_content_opt {
                                        let demux_res = reasoning_demuxer.process_delta(content);
                                        if demux_res.transitioned_to_visible {
                                            // Thought block closed: re-arm Phase 1 filter for beginning of visible answer
                                            filter_active = true;
                                        }

                                        for chunk_item in demux_res.chunks {
                                            match chunk_item {
                                                DemuxedChunk::Reasoning(thought_part) => {
                                                    let mut thought_evt = parsed.clone();
                                                    if let Some(target) = thought_evt.pointer_mut("/choices/0/delta/content") {
                                                        *target = serde_json::Value::String(thought_part);
                                                    }
                                                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(thought_evt.to_string()));
                                                }
                                                DemuxedChunk::Visible(visible_part) => {
                                                    if !filter_active {
                                                        // Phase 2: Stream through cross-chunk StreamTailSanitizer lookahead buffer (<1µs)
                                                        let drained = tail_sanitizer.push_and_drain(&visible_part);
                                                        if !drained.is_empty() {
                                                            let mut out_evt = parsed.clone();
                                                            if let Some(target) = out_evt.pointer_mut("/choices/0/delta/content") {
                                                                *target = serde_json::Value::String(drained);
                                                            }
                                                            yield Ok::<Event, std::convert::Infallible>(Event::default().data(out_evt.to_string()));
                                                        }
                                                    } else {
                                                        // Phase 1: Optimistic micro-buffer window (48-64 chars) with UTF-8 boundary safety
                                                        buffered_text.push_str(&visible_part);
                                                        let chunk_copy = parsed.clone();
                                                        buffered_chunks.push(chunk_copy);

                                                        // Flush condition: buffer has enough content (>= 48 chars) or completed first sentence
                                                        if buffered_text.len() >= 48 || buffered_text.contains('\n') || (buffered_text.contains('.') && buffered_text.len() >= 20) {
                                                            let safe_idx = buffered_text.floor_char_boundary(buffered_text.len());
                                                            let valid_slice = &buffered_text[..safe_idx];
                                                            let orig_chars = valid_slice.chars().count();
                                                            let (sanitized, _intercepted) = strip_initial_cushions(valid_slice);
                                                            let sanitized_chars = sanitized.chars().count();
                                                            if orig_chars > sanitized_chars {
                                                                let diff = orig_chars - sanitized_chars;
                                                                phase1_token_delta -= ((diff as f64) / 3.8).round().max(1.0) as i64;
                                                            }

                                                            let drained = tail_sanitizer.push_and_drain(&sanitized);
                                                            if let Some(first) = buffered_chunks.first_mut() {
                                                                if let Some(target) = first.pointer_mut("/choices/0/delta/content") {
                                                                    *target = serde_json::Value::String(drained);
                                                                }
                                                                yield Ok::<Event, std::convert::Infallible>(Event::default().data(first.to_string()));
                                                            }

                                                            buffered_chunks.clear();
                                                            buffered_text.clear();
                                                            filter_active = false;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        // Non-content events (role definitions, metadata, usage) pass through immediately
                                        yield Ok::<Event, std::convert::Infallible>(Event::default().data(parsed.to_string()));
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Ok::<Event, std::convert::Infallible>(Event::default().data(format!("{{\"error\": \"{}\"}}", e)));
                    }
                }
            }
        };

        Sse::new(mapped_stream)
            .keep_alive(KeepAlive::default())
            .into_response()
    } else {
        let mut json_body: serde_json::Value = upstream_resp.json().await.unwrap_or(json!({}));
        let mut apologies_intercepted = 0;
        let mut token_adjustment: i64 = 0;
        if let Some(content) = json_body.pointer("/choices/0/message/content").and_then(|v| v.as_str()) {
            let orig_len = content.chars().count();
            let (sanitized, intercepted) = strip_initial_cushions(content);
            let (fully_sanitized, delayed_intercepted) = vertebrae::sanitize_delayed_apology(&sanitized);
            if intercepted || delayed_intercepted {
                let final_len = fully_sanitized.chars().count();
                if orig_len > final_len {
                    let diff = orig_len - final_len;
                    token_adjustment = ((diff as f64) / 3.8).round().max(1.0) as i64;
                }
                if let Some(target) = json_body.pointer_mut("/choices/0/message/content") {
                    *target = serde_json::Value::String(fully_sanitized);
                    apologies_intercepted += 1;
                }
            }
        }
        if let Some(usage) = json_body.get_mut("usage") {
            if let Some(raw_comp_tok) = usage.get("completion_tokens").and_then(|v| v.as_i64()) {
                usage["upstream_completion_tokens"] = json!(raw_comp_tok);
                let adjusted = (raw_comp_tok - token_adjustment).max(0);
                usage["completion_tokens"] = json!(adjusted);
                if let Some(prompt_tok) = usage.get("prompt_tokens").and_then(|v| v.as_i64()) {
                    usage["upstream_total_tokens"] = json!(prompt_tok + raw_comp_tok);
                    usage["total_tokens"] = json!(prompt_tok + adjusted);
                }
            }
        }
        let ttft_ms = start_time.elapsed().as_millis() as u64;

        Json(json!({
            "response": json_body,
            "spine_telemetry": {
                "vertebrae": vertebrae,
                "active_vertebrae_count": active_vertebrae_count,
                "apologies_intercepted": apologies_intercepted,
                "ttft_ms": ttft_ms,
                "reality_level": reality_level as u8,
            }
        }))
        .into_response()
    }
}

#[derive(serde::Deserialize)]
pub struct RedTeamRequest {
    pub target_name: Option<String>,
    pub proposal: String,
    pub reality_level: Option<u8>,
}

async fn redteam_spine_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RedTeamRequest>,
) -> impl IntoResponse {
    let reality_level = RealityLevel::from_u8(payload.reality_level.unwrap_or(4));
    let target = payload.target_name.unwrap_or_else(|| "Architecture Proposal".to_string());
    let report = AdversarialAuditEngine::audit_proposal(&target, &payload.proposal, reality_level);

    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    let event_obj = json!({
        "id": next_event_id("evt_redteam"),
        "source": "SPINE Red-Team Engine (/api/spine/redteam)",
        "target": target,
        "verdict": report.verdict,
        "rigidity_index": report.rigidity_index,
        "violations_count": report.violations.len(),
        "timestamp": now_ts,
    });
    let _ = state.tx.send(event_obj);

    Json(json!({
        "status": "success",
        "report": report,
    }))
}

#[derive(serde::Deserialize)]
pub struct AttestRequest {
    pub target_name: Option<String>,
    pub proposal: String,
    pub reality_level: Option<u8>,
}

async fn attest_spine_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AttestRequest>,
) -> impl IntoResponse {
    let reality_level = RealityLevel::from_u8(payload.reality_level.unwrap_or(4));
    let target = payload.target_name.unwrap_or_else(|| "Enterprise System".to_string());
    let report = AdversarialAuditEngine::audit_proposal(&target, &payload.proposal, reality_level);
    let attestation = AdversarialAuditEngine::generate_gate_attestation(&report, &payload.proposal);

    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    let event_obj = json!({
        "id": next_event_id("evt_attest"),
        "source": "SPINE Gate Attestation (/api/spine/attest)",
        "target": target,
        "attestation_id": attestation.attestation_id,
        "verdict": attestation.verdict,
        "rigidity_index": attestation.rigidity_index,
        "timestamp": now_ts,
    });
    let _ = state.tx.send(event_obj);

    Json(json!({
        "status": "success",
        "attestation": attestation,
        "report": report,
    }))
}
