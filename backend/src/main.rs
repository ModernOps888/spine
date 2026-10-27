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
    pub active_reality_level: Arc<std::sync::atomic::AtomicU8>,
}

impl AppState {
    pub fn resolve_level(&self, explicit: Option<u8>) -> RealityLevel {
        let raw = explicit
            .unwrap_or_else(|| self.active_reality_level.load(std::sync::atomic::Ordering::Relaxed))
            .clamp(1, 4);
        RealityLevel::from_u8(raw)
    }
}

/// Maps UI/canonical 2026 model aliases to exact OpenRouter upstream model slugs.
pub fn normalize_openrouter_model_id(model: &str) -> String {
    match model.trim().to_lowercase().as_str() {
        "openai/astra-6" | "astra-6" | "gpt-6-astra" => "openai/gpt-6-astra".to_string(),
        "openai/sol-6.1" | "sol-6.1" | "sol-6-1" | "gpt-6-1-sol" | "gpt-6.1-sol" => {
            "openai/gpt-6.1-sol".to_string()
        }
        "gemini-4-argon" | "google/gemini-4-argon" => "google/gemini-4-argon".to_string(),
        "gemini-3.8-flash" | "gemini-3-8-flash" | "google/gemini-3.8-flash" => {
            "google/gemini-3.8-flash".to_string()
        }
        "claude-opus-5-5" | "claude-opus-5.5" | "anthropic/claude-opus-5.5" => {
            "anthropic/claude-opus-5.5".to_string()
        }
        "claude-sonnet-5-5" | "claude-sonnet-5.5" | "anthropic/claude-sonnet-5.5" => {
            "anthropic/claude-sonnet-5.5".to_string()
        }
        "grok-4.7" | "grok-4-7" | "x-ai/grok-4.7" => "x-ai/grok-4.7".to_string(),
        "deepseek-v4.1-flash" | "deepseek-v4-1-flash" | "deepseek/deepseek-v4.1-flash" => {
            "deepseek/deepseek-v4.1-flash".to_string()
        }
        _ => model.to_string(),
    }
}

/// Local Ollama OpenAI-compatible endpoint. Defaults to `http://127.0.0.1:11434`; override the base
/// with `SPINE_OLLAMA_URL` (e.g. a remote GPU box). `localhost` is normalized to `127.0.0.1`.
pub fn ollama_chat_completions_url() -> String {
    let base = std::env::var("SPINE_OLLAMA_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
    format!("{}/v1/chat/completions", mcp::normalize_loopback_base(&base))
}

fn build_app_router(state: Arc<AppState>) -> Router {
    // Only the local HUD may call the gateway cross-origin.
    let allowed_origins: Vec<axum::http::HeaderValue> = [
        "http://localhost:3333",
        "http://127.0.0.1:3333",
        "http://localhost:5173",
        "http://127.0.0.1:5173",
    ]
    .iter()
    .filter_map(|o| o.parse().ok())
    .collect();
    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_handler))
        .route("/v1/models", get(models_handler))
        .route("/v1/chat/completions", post(chat_completions_handler))
        .route("/v1/completions", post(completions_handler))
        .route("/api/spine/audit", post(audit_spine_handler))
        .route("/api/spine/notify", post(notify_spine_handler))
        .route("/api/spine/dial", post(set_dial_spine_handler))
        .route("/api/spine/events", get(events_spine_handler))
        .route("/api/spine/history", get(history_spine_handler))
        .route("/api/spine/logs", get(logs_spine_handler))
        .route("/api/spine/redteam", post(redteam_spine_handler))
        .route("/api/spine/attest", post(attest_spine_handler))
        .route("/api/spine/verify-attestation", post(verify_attestation_spine_handler))
        .layer(cors)
        .layer(axum::middleware::from_fn(loopback_host_guard))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    for env_fallback in &["C:/spine/.env", "C:/spine/backend/.env"] {
        if std::env::var("OPENROUTER_API_KEY").map_or(true, |v| v.is_empty()) {
            let _ = dotenvy::from_path(env_fallback);
        }
    }

    let openrouter_key = std::env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| "".to_string());
    let chronofact_url = std::env::var("CHRONOFACT_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| Some("http://127.0.0.1:3030".to_string()));

    let args: Vec<String> = std::env::args().collect();
    let is_mcp = args.len() > 1 && args[1] == "mcp";

    // In MCP mode, stdout is strictly reserved for JSON-RPC frames; direct all tracing logs to stderr.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let (tx, _rx) = broadcast::channel(100);
    let state = Arc::new(AppState {
        openrouter_api_key: openrouter_key.clone(),
        chronofact_url: chronofact_url.clone(),
        // `connect_timeout` + per-read idle `read_timeout` instead of a total request timeout:
        // a total `.timeout(120s)` also bounds the streamed body, silently truncating long
        // reasoning/SSE generations mid-sentence. Idle reads still fail fast on a stalled upstream.
        http_client: reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .read_timeout(std::time::Duration::from_secs(120))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .tcp_nodelay(true)
            .build()
            .unwrap(),
        tx,
        recent_events: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        active_reality_level: Arc::new(std::sync::atomic::AtomicU8::new(4)),
    });

    let app = build_app_router(state);
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    // Check if invoked as an MCP server by IDE (Cursor / Antigravity / VS Code).
    // Automatically co-host the HTTP/SSE Reality Gateway on 127.0.0.1:8080 if not already bound,
    // ensuring the browser HUD (http://localhost:3333) is ALWAYS online whenever Antigravity runs.
    if is_mcp {
        tokio::spawn(async move {
            if let Ok(listener) = tokio::net::TcpListener::bind(addr).await {
                info!("SPINE Co-Hosted Reality Gateway active on http://{} (MCP process)", addr);
                let _ = axum::serve(listener, app).await;
            }
        });
        mcp::run_stdio_server(openrouter_key).await;
        return;
    }

    if openrouter_key.is_empty() {
        warn!("OPENROUTER_API_KEY is not set in environment or .env!");
    } else {
        info!("OPENROUTER_API_KEY detected and loaded successfully.");
    }

    if let Some(ref cf_url) = chronofact_url {
        info!("ChronoFact Pipeline Middleware active pointing to: {}", cf_url);
    }

    info!("SPINE Reality Gateway starting on http://{}", addr);

    match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            axum::serve(listener, app).await.unwrap();
        }
        Err(e) => {
            warn!("Port 8080 already bound ({}); SPINE Gateway is already active.", e);
        }
    }
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
            let trimmed = h.trim();
            if trimmed.starts_with("[::1]") {
                true
            } else {
                let name = trimmed.rsplit_once(':').map(|(n, _)| n).unwrap_or(trimmed);
                matches!(name, "localhost" | "127.0.0.1")
            }
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
        "version": env!("CARGO_PKG_VERSION")
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
            id: "anthropic/claude-sonnet-5.5".to_string(),
            name: "Claude Sonnet 5.5".to_string(),
            provider: "Anthropic".to_string(),
            description: "Active 2026 High-Velocity Coding & Reasoning Model (Replaces retired Claude 3.5 / 3.7 Sonnet)".to_string(),
            is_local: false,
        },
        ModelInfo {
            id: "x-ai/grok-4.7".to_string(),
            name: "Grok 4.7".to_string(),
            provider: "xAI".to_string(),
            description: "Active Frontier Reasoner (Replaces Grok 2 / Grok 3)".to_string(),
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
            name: "GPT-4o (Legacy)".to_string(),
            provider: "OpenAI".to_string(),
            description: "Legacy 2024 Multimodal Engine (Superseded by OpenAI Astra 6 / Sol 6.1)".to_string(),
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
    #[serde(default)]
    pub ephemeral: bool,
}

async fn audit_spine_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AuditRequest>,
) -> impl IntoResponse {
    let reality_level = state.resolve_level(payload.reality_level);
    let model = payload.model.unwrap_or_else(|| "google/gemini-4-argon".to_string());
    
    let (vertebrae, _prompt) = SpineAuditEngine::audit_input(&payload.messages, reality_level, &model);
    let active_count = vertebrae.iter().filter(|v| v.active).count();
    let overall_rigidity = SpineAuditEngine::compute_backbone_rigidity(reality_level, &vertebrae);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let prompt_text = payload
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .or_else(|| payload.messages.last())
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let ego_detected = vertebrae.iter().any(|v| v.id == "C2" && v.active);
    let flattery_detected = vertebrae.iter().any(|v| v.id == "C4" && v.active);
    let authority_detected = vertebrae.iter().any(|v| v.id == "C6" && v.active);
    let pushback_detected = vertebrae.iter().any(|v| v.id == "T4" && v.active);

    let snapshot = json!({
        "id": next_event_id("evt"),
        "timestamp": now,
        "source": "SPINE Reality Gateway (/api/spine/audit)",
        "prompt": prompt_text,
        "vertebrae": vertebrae,
        "active_vertebrae_count": active_count,
        "overall_rigidity": overall_rigidity,
        "total_vertebrae": 33,
        "reality_level": reality_level as u8,
        "model": model,
        "ego_detected": ego_detected,
        "flattery_detected": flattery_detected,
        "authority_detected": authority_detected,
        "pushback_detected": pushback_detected,
    });

    if !payload.ephemeral {
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
    }

    Json(snapshot)
}

static EVENT_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// Epoch-ms until which the ChronoFact pre-check is skipped after a connection failure.
static CF_BREAKER_UNTIL_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
const CF_BREAKER_COOLDOWN_MS: u64 = 30_000;
static LOG_WRITE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
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
    // Serialize on the caller (cheap, CPU-only); hand the disk write to the dedicated log writer
    // thread so request handlers and MCP tool calls never block a Tokio worker on file I/O,
    // antivirus scans, or rotation renames.
    // Compact disk serialization: replace 33-element vertebrae description array with active_vertebra_ids
    // and round f32 telemetry metrics to 4 decimal places.
    let mut disk_event = event.clone();
    if let Some(obj) = disk_event.as_object_mut() {
        if let Some(verts) = obj.remove("vertebrae") {
            if let Some(arr) = verts.as_array() {
                let active_ids: Vec<String> = arr
                    .iter()
                    .filter(|v| v.get("active").and_then(|a| a.as_bool()).unwrap_or(false))
                    .filter_map(|v| v.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()))
                    .collect();
                obj.insert("active_vertebra_ids".to_string(), json!(active_ids));
            }
        }
        for float_key in &["overall_rigidity", "rigidity_index"] {
            if let Some(val) = obj.get(*float_key).and_then(|v| v.as_f64()) {
                let rounded = (val * 10000.0).round() / 10000.0;
                obj.insert((*float_key).to_string(), json!(rounded));
            }
        }
        if let Some(p) = obj.get("prompt").and_then(|p| p.as_str()) {
            if p.chars().count() > LOG_PROMPT_MAX_CHARS {
                let cut: String = p.chars().take(LOG_PROMPT_MAX_CHARS).collect();
                obj.insert("prompt".to_string(), json!(format!("{}…[truncated]", cut)));
            }
        }
    }
    if let Ok(json_str) = serde_json::to_string(&disk_event) {
        submit_log_job(LogJob::Telemetry(json_str));
    }
}

/// Disk write jobs executed in FIFO order by the dedicated `spine-log-writer` thread.
enum LogJob {
    Telemetry(String),
    TrainingPair(String),
    Flush(std::sync::mpsc::Sender<()>),
}

static LOG_TX: std::sync::OnceLock<std::sync::Mutex<std::sync::mpsc::Sender<LogJob>>> = std::sync::OnceLock::new();

fn write_log_job_sync(job: LogJob) {
    use std::io::Write;
    let _guard = LOG_WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let log_dir = telemetry_log_dir();
    if !log_dir.exists() {
        let _ = std::fs::create_dir_all(&log_dir);
    }
    match job {
        LogJob::Telemetry(line) => {
            rotate_telemetry_log(&log_dir);
            let log_file = log_dir.join("telemetry.jsonl");
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(log_file) {
                let _ = writeln!(file, "{}", line);
            }
        }
        LogJob::TrainingPair(line) => {
            let pair_file = log_dir.join("post_training_pairs.jsonl");
            if std::fs::metadata(&pair_file).map(|m| m.len() > LOG_ROTATE_BYTES).unwrap_or(false) {
                let _ = std::fs::rename(&pair_file, log_dir.join("post_training_pairs.1.jsonl"));
            }
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(pair_file) {
                let _ = writeln!(file, "{}", line);
            }
        }
        LogJob::Flush(ack) => {
            let _ = ack.send(());
        }
    }
}

fn submit_log_job(job: LogJob) {
    let tx = LOG_TX.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<LogJob>();
        let spawned = std::thread::Builder::new()
            .name("spine-log-writer".to_string())
            .spawn(move || {
                while let Ok(job) = rx.recv() {
                    write_log_job_sync(job);
                }
            });
        if spawned.is_err() {
            warn!("Failed to spawn SPINE log writer thread; telemetry writes will be dropped");
        }
        std::sync::Mutex::new(tx)
    });
    let send_result = tx.lock().unwrap_or_else(|e| e.into_inner()).send(job);
    if let Err(std::sync::mpsc::SendError(job)) = send_result {
        // Writer thread unavailable: fall back to a synchronous write rather than losing data.
        write_log_job_sync(job);
    }
}

/// Blocks (up to `timeout`) until every previously submitted telemetry / training-pair write
/// has reached disk. Called on MCP shutdown so the final turn's audit is never lost.
pub fn flush_telemetry_logs(timeout: std::time::Duration) {
    let (ack_tx, ack_rx) = std::sync::mpsc::channel();
    submit_log_job(LogJob::Flush(ack_tx));
    let _ = ack_rx.recv_timeout(timeout);
}

/// Appends a verified `(prompt, chosen, rejected)` preference pair to `post_training_pairs.jsonl`
/// whenever SPINE strips sycophantic cushions or unearned apologies from an assistant response.
pub fn append_post_training_pair(pair: &serde_json::Value) {
    if let Ok(json_str) = serde_json::to_string(pair) {
        submit_log_job(LogJob::TrainingPair(json_str));
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

    if payload.get("tool").and_then(|t| t.as_str()) == Some("spine_set_reality_dial") {
        let lvl = mcp::resolve_reality_level(payload.get("reality_level").or_else(|| payload.get("level")));
        state.active_reality_level.store(lvl, std::sync::atomic::Ordering::Relaxed);
        mcp::ACTIVE_REALITY_LEVEL.store(lvl, std::sync::atomic::Ordering::Relaxed);
    }

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

async fn set_dial_spine_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let lvl = mcp::resolve_reality_level(payload.get("level").or_else(|| payload.get("reality_level")));
    state.active_reality_level.store(lvl, std::sync::atomic::Ordering::Relaxed);
    mcp::ACTIVE_REALITY_LEVEL.store(lvl, std::sync::atomic::Ordering::Relaxed);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let dial_evt = json!({
        "id": next_event_id("evt_dial"),
        "timestamp": now,
        "source": "SPINE HUD Reality Dial",
        "tool": "spine_set_reality_dial",
        "persisted": true,
        "reality_level": lvl,
        "prompt": format!("Reality Horizon set to Level 0{lvl}"),
    });
    append_to_telemetry_log(&dial_evt);
    {
        let mut history = state.recent_events.write().await;
        history.push(dial_evt.clone());
        if history.len() > 50 {
            history.remove(0);
        }
    }
    let _ = state.tx.send(dial_evt);

    Json(json!({
        "status": "updated",
        "reality_level": lvl,
    }))
}

async fn history_spine_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let history = state.recent_events.read().await;
    let active_lvl = state.active_reality_level.load(std::sync::atomic::Ordering::Relaxed).clamp(1, 4);
    Json(json!({
        "events": *history,
        "count": history.len(),
        "active_reality_level": active_lvl,
    }))
}

async fn logs_spine_handler() -> impl IntoResponse {
    const TAIL_BYTES: u64 = 256 * 1024;
    const MAX_LOG_EVENTS: usize = 100;
    let log_path = telemetry_log_dir().join("telemetry.jsonl");
    let mut events = Vec::with_capacity(MAX_LOG_EVENTS);
    if let Ok(mut file) = std::fs::File::open(&log_path) {
        use std::io::{Read, Seek, SeekFrom};
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
        let start = len.saturating_sub(TAIL_BYTES);
        if file.seek(SeekFrom::Start(start)).is_ok() {
            let mut buf = Vec::new();
            if file.read_to_end(&mut buf).is_ok() {
                let text = String::from_utf8_lossy(&buf);
                let mut lines: Vec<&str> = text.lines().collect();
                if start > 0 && !lines.is_empty() {
                    lines.remove(0); // first line is likely partial
                }
                for line in lines.into_iter().rev() {
                    if events.len() >= MAX_LOG_EVENTS {
                        break;
                    }
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                            if let Some(obj) = val.as_object_mut() {
                                if let Some(verts) = obj.remove("vertebrae") {
                                    if !obj.contains_key("active_vertebra_ids") {
                                        if let Some(arr) = verts.as_array() {
                                            let active_ids: Vec<String> = arr
                                                .iter()
                                                .filter(|v| v.get("active").and_then(|a| a.as_bool()).unwrap_or(false))
                                                .filter_map(|v| v.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()))
                                                .collect();
                                            obj.insert("active_vertebra_ids".to_string(), json!(active_ids));
                                        }
                                    }
                                }
                            }
                            events.push(val);
                        }
                    }
                }
            }
        }
    }
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
    Json(raw_body): Json<serde_json::Value>,
) -> Response {
    let start_time = Instant::now();
    let req: ChatCompletionRequest = match serde_json::from_value(raw_body.clone()) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Invalid chat completion request: {}", e) })),
            )
                .into_response();
        }
    };
    let reality_level = state.resolve_level(req.reality_level);

    let (vertebrae, directive) = SpineAuditEngine::audit_input(&req.messages, reality_level, &req.model);
    let active_vertebrae_count = vertebrae.iter().filter(|v| v.active).count();
    let overall_rigidity = SpineAuditEngine::compute_backbone_rigidity(reality_level, &vertebrae);

    let prompt_text = req.messages.iter().rev().find(|m| m.role == "user").map(|m| m.content.clone()).unwrap_or_default();
    let prior_claim = req.messages.iter().rev().find(|m| m.role == "assistant").map(|m| m.content.clone()).unwrap_or_default();

    // Preserve original raw message objects (so multimodal `content: [{"type":"image_url",...}]`,
    // assistant `tool_calls`, and `tool_call_id` are never stripped or flattened into strings).
    let mut outbound_messages: Vec<serde_json::Value> = raw_body
        .get("messages")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_else(|| {
            req.messages
                .iter()
                .map(|m| serde_json::to_value(m).unwrap_or(json!({})))
                .collect()
        });

    // Prompt Cache Prefix Preservation (SPINE-08):
    // Keep any caller-supplied leading `role: "system"` messages at index 0 untouched so
    // upstream KV prompt caching hits on the prefix. Insert turn-specific SPINE and ChronoFact
    // directives immediately before the final user message.
    let directive_insert_idx = outbound_messages
        .iter()
        .rposition(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
        .unwrap_or(outbound_messages.len());

    let pushback = SpineAuditEngine::verify_pushback_grounded(&prompt_text, &prior_claim, reality_level);
    let combined_directive = match pushback.verdict {
        vertebrae::PushbackVerdict::ConcedeAndCorrect
        | vertebrae::PushbackVerdict::ConcedeAndAdapt
        | vertebrae::PushbackVerdict::ObjectiveMatrix => {
            if directive.is_empty() {
                pushback.directive.clone()
            } else {
                format!("{}\n\n{}", directive, pushback.directive)
            }
        }
        vertebrae::PushbackVerdict::HoldTheLine
            if pushback.has_authority_intimidation
                || pushback.demands_unearned_apology
                || !pushback.technical_indicators_found.is_empty() =>
        {
            if directive.is_empty() {
                pushback.directive.clone()
            } else {
                format!("{}\n\n{}", directive, pushback.directive)
            }
        }
        _ => directive.clone(),
    };

    if !combined_directive.is_empty() {
        outbound_messages.insert(
            directive_insert_idx,
            json!({
                "role": "system",
                "content": combined_directive,
            }),
        );
    }

    // Zero-Config Pipeline Middleware: Transparently queries ChronoFact (:3030)
    // Guarded by a 30s circuit breaker: when ChronoFact is offline, every chat turn would otherwise
    // pay the full connect timeout (Windows retries refused loopback SYNs for ~1-2s) before TTFT.
    let cf_now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let cf_breaker_open = CF_BREAKER_UNTIL_MS.load(std::sync::atomic::Ordering::Relaxed) > cf_now_ms;
    if let Some(cf_url) = state.chronofact_url.as_ref().filter(|_| !cf_breaker_open) {
        let endpoint = format!("{}/api/temporal/check", mcp::normalize_loopback_base(cf_url));
        let cf_payload = json!({
            "model_id": req.model,
            "query": prompt_text,
        });

        let cf_result = state.http_client
            .post(&endpoint)
            .json(&cf_payload)
            .timeout(std::time::Duration::from_millis(400))
            .send()
            .await;
        if cf_result.is_err() {
            CF_BREAKER_UNTIL_MS.store(cf_now_ms + CF_BREAKER_COOLDOWN_MS, std::sync::atomic::Ordering::Relaxed);
        }
        if let Ok(cf_res) = cf_result
        {
            if cf_res.status().is_success() {
                if let Ok(cf_body) = cf_res.json::<serde_json::Value>().await {
                    if let Some(cal_block) = cf_body.get("calibration_block").and_then(|v| v.as_str()) {
                        let requires_grounding = cf_body.get("requires_grounding").and_then(|v| v.as_bool()).unwrap_or(false);
                        let risk = cf_body.get("temporal_risk_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let is_outdated = cf_body.get("is_model_outdated_or_retired").and_then(|v| v.as_bool()).unwrap_or(false);

                        if requires_grounding || risk >= 0.25 || is_outdated {
                            outbound_messages.insert(
                                directive_insert_idx,
                                json!({
                                    "role": "system",
                                    "content": cal_block,
                                }),
                            );
                        }
                    }
                }
            }
        }
    }

    let is_stream = req.stream.unwrap_or(true);
    let model_id = req.model.clone();
    let is_ollama = model_id.starts_with("ollama/");
    let is_harness = model_id.starts_with("harness/") || model_id == "qwen2.5-coder:32b" || model_id == "local/qwen32b";

    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    let ego_detected = vertebrae.iter().any(|v| v.id == "C2" && v.active);
    let flattery_detected = vertebrae.iter().any(|v| v.id == "C4" && v.active);
    let authority_detected = vertebrae.iter().any(|v| v.id == "C6" && v.active);
    let pushback_detected = vertebrae.iter().any(|v| v.id == "T4" && v.active);
    let event_obj = json!({
        "id": next_event_id("evt"),
        "source": "SPINE Gateway (/v1/chat/completions)",
        "tool": "spine_execute_grounded_reality",
        "vertebrae": vertebrae,
        "active_vertebrae_count": active_vertebrae_count,
        "overall_rigidity": overall_rigidity,
        "reality_level": reality_level as u8,
        "model": model_id,
        "prompt": prompt_text,
        "timestamp": now_ts,
        "ego_detected": ego_detected,
        "flattery_detected": flattery_detected,
        "authority_detected": authority_detected,
        "pushback_detected": pushback_detected,
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

    // Unified Upstream Dispatch (OpenRouter + Local Ollama /v1/chat/completions + Local Harness /v1/chat/completions)
    // All providers flow through the identical SPINE Phase 1 + ReasoningDemuxer + Phase 2 StreamTailSanitizer pipeline.
    let (target_url, dispatch_model_id, is_local_provider) = if is_ollama {
        (
            ollama_chat_completions_url(),
            model_id.trim_start_matches("ollama/").to_string(),
            true,
        )
    } else if is_harness {
        let clean = if model_id.starts_with("harness/") {
            model_id.trim_start_matches("harness/").to_string()
        } else {
            model_id.clone()
        };
        (
            "http://127.0.0.1:8089/v1/chat/completions".to_string(),
            clean,
            true,
        )
    } else {
        (
            "https://openrouter.ai/api/v1/chat/completions".to_string(),
            normalize_openrouter_model_id(&model_id),
            false,
        )
    };

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
    if !is_local_provider {
        let sanitized_key: String = state
            .openrouter_api_key
            .chars()
            .filter(|c| !c.is_control())
            .collect();
        if let Ok(auth_val) = header::HeaderValue::from_str(&format!("Bearer {}", sanitized_key.trim())) {
            headers.insert(header::AUTHORIZATION, auth_val);
        } else {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid characters in OPENROUTER_API_KEY header" })),
            )
                .into_response();
        }
        headers.insert("HTTP-Referer", header::HeaderValue::from_static("https://spine-gateway.local"));
        headers.insert("X-Title", header::HeaderValue::from_static("SPINE Reality Gateway"));
    }

    let mut payload = serde_json::Map::new();
    for (k, v) in &req.extra {
        payload.insert(k.clone(), v.clone());
    }
    payload.insert("model".to_string(), json!(dispatch_model_id));
    payload.insert("messages".to_string(), json!(outbound_messages));
    payload.insert("stream".to_string(), json!(is_stream));
    payload.insert("temperature".to_string(), json!(req.temperature.unwrap_or(0.2)));
    if let Some(max_tok) = req.max_tokens {
        payload.insert("max_tokens".to_string(), json!(max_tok));
    }
    let payload = serde_json::Value::Object(payload);

    let resp_res = state
        .http_client
        .post(&target_url)
        .headers(headers)
        .json(&payload)
        .send()
        .await;

    let upstream_resp = match resp_res {
        Ok(r) => r,
        Err(e) => {
            let err_msg = if is_ollama {
                format!("Local Ollama unavailable on http://127.0.0.1:11434: {}", e)
            } else if is_harness {
                format!("Harness accelerated model server unavailable on http://127.0.0.1:8089: {}. Start it with C:\\Harness\\launch_qwen32b_coder.bat", e)
            } else {
                format!("Upstream dispatch failure: {}", e)
            };
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": err_msg })),
            )
                .into_response();
        }
    };

    if !upstream_resp.status().is_success() {
        let status = StatusCode::from_u16(upstream_resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
        let err_text = upstream_resp.text().await.unwrap_or_default();
        return (
            status,
            Json(json!({ "error": format!("Upstream API error: {}", err_text) })),
        )
            .into_response();
    }

    let active_ids_for_dpo: Vec<String> = vertebrae
        .iter()
        .filter(|v| v.active)
        .map(|v| v.id.clone())
        .collect();

    if is_stream {
        let mut byte_stream = upstream_resp.bytes_stream();
        let pushbacks_resisted = vertebrae.iter().filter(|v| v.id == "T4" && v.active).count() as u32;
        let telemetry_snapshot = SpineTelemetrySnapshot {
            vertebrae,
            overall_rigidity,
            active_vertebrae_count,
            pushbacks_resisted,
            apologies_intercepted: 0,
            ttft_ms: start_time.elapsed().as_millis() as u64,
            tokens_per_sec: 0.0,
            total_tokens: 0,
        };

        let initial_telemetry = Event::default()
            .event("spine_telemetry")
            .data(serde_json::to_string(&telemetry_snapshot).unwrap_or_default());

        let dpo_prompt = prompt_text.clone();
        let dpo_model = model_id.clone();
        let dpo_lvl = reality_level as u8;

        let mapped_stream = async_stream::stream! {
            yield Ok::<Event, std::convert::Infallible>(initial_telemetry);

            let mut filter_active = true;
            let mut reasoning_demuxer = ReasoningDemuxer::new();
            let mut buffered_chunks: Vec<serde_json::Value> = Vec::new();
            let mut buffered_text = String::new();
            let mut tail_sanitizer = StreamTailSanitizer::new(128);
            let mut phase1_token_delta: i64 = 0;
            // Raw byte buffer across TCP chunks so multi-byte UTF-8 sequences split across packets
            // are never decoded until a complete newline-delimited SSE frame arrives (SPINE-04).
            let mut raw_byte_buffer: Vec<u8> = Vec::new();
            let mut raw_visible_accum = String::new();
            let mut clean_visible_accum = String::new();
            let mut phase1_intercepted_any = false;
            let mut done_emitted = false;

            while let Some(chunk) = byte_stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        raw_byte_buffer.extend_from_slice(&bytes);
                        let last_nl_pos = match raw_byte_buffer.iter().rposition(|&b| b == b'\n') {
                            Some(pos) => pos,
                            None => continue,
                        };
                        let complete_bytes: Vec<u8> = raw_byte_buffer.drain(..=last_nl_pos).collect();
                        let complete_part = String::from_utf8_lossy(&complete_bytes);

                        for raw_line in complete_part.lines() {
                            let line = raw_line.trim();
                            if line.is_empty() {
                                continue;
                            }
                            let data_content = if let Some(rest) = line.strip_prefix("data:") {
                                rest.trim()
                            } else if line.starts_with('{') {
                                // Support raw NDJSON fallback if a local server omits `data:` prefix
                                line
                            } else {
                                continue;
                            };

                            if data_content == "[DONE]" {
                                done_emitted = true;
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
                                            raw_visible_accum.push_str(&vis_part);
                                            if filter_active {
                                                buffered_text.push_str(&vis_part);
                                            } else {
                                                let drained = tail_sanitizer.push_and_drain(&vis_part);
                                                if !drained.is_empty() {
                                                    clean_visible_accum.push_str(&drained);
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

                                // Flush any remaining buffered text/chunks before completing (SPINE-18)
                                if filter_active && (!buffered_chunks.is_empty() || !buffered_text.is_empty()) {
                                    let safe_idx = buffered_text.floor_char_boundary(buffered_text.len());
                                    let orig_chars = buffered_text[..safe_idx].chars().count();
                                    let (sanitized, intercepted) = strip_initial_cushions(&buffered_text[..safe_idx]);
                                    if intercepted {
                                        phase1_intercepted_any = true;
                                    }
                                    let sanitized_chars = sanitized.chars().count();
                                    if orig_chars > sanitized_chars {
                                        let diff = orig_chars - sanitized_chars;
                                        phase1_token_delta -= ((diff as f64) / 3.8).round().max(1.0) as i64;
                                    }

                                    let drained = tail_sanitizer.push_and_drain(&sanitized);
                                    if !drained.is_empty() {
                                        clean_visible_accum.push_str(&drained);
                                        if let Some(first) = buffered_chunks.first_mut() {
                                            if let Some(target) = first.pointer_mut("/choices/0/delta/content") {
                                                *target = serde_json::Value::String(drained);
                                            }
                                            yield Ok::<Event, std::convert::Infallible>(Event::default().data(first.to_string()));
                                        } else {
                                            let flush_evt = json!({
                                                "id": "chatcmpl-spine-phase1-flush",
                                                "object": "chat.completion.chunk",
                                                "choices": [{
                                                    "delta": { "content": drained },
                                                    "index": 0,
                                                    "finish_reason": null
                                                }]
                                            });
                                            yield Ok::<Event, std::convert::Infallible>(Event::default().data(flush_evt.to_string()));
                                        }
                                    }
                                    buffered_chunks.clear();
                                    buffered_text.clear();
                                    filter_active = false;
                                }

                                // Flush final remaining lookahead bytes from StreamTailSanitizer
                                if let Some(final_tail) = tail_sanitizer.flush_final_chunk() {
                                    clean_visible_accum.push_str(&final_tail);
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

                                if (phase1_intercepted_any || tail_sanitizer.interceptions_count() > 0)
                                    && clean_visible_accum.trim() != raw_visible_accum.trim()
                                {
                                    append_post_training_pair(&json!({
                                        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
                                        "model": dpo_model,
                                        "reality_level": dpo_lvl,
                                        "active_vertebra_ids": active_ids_for_dpo,
                                        "prompt": dpo_prompt,
                                        "chosen": clean_visible_accum.trim(),
                                        "rejected": raw_visible_accum.trim(),
                                    }));
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

                                // CoT Exemption Boundary 1: Dedicated provider thought channels.
                                // A field counts only when it carries actual reasoning text: OpenRouter emits
                                // `"reasoning": null` on every visible chunk, and `pointer(..).is_some()` is true
                                // for JSON null, which previously routed ALL visible text around the sanitizer.
                                let carries_text = |v: &serde_json::Value| match v {
                                    serde_json::Value::Null => false,
                                    serde_json::Value::String(s) => !s.is_empty(),
                                    serde_json::Value::Array(a) => !a.is_empty(),
                                    _ => true,
                                };
                                let reasoning_ptrs = [
                                    "/choices/0/delta/reasoning_content",
                                    "/choices/0/delta/thought",
                                    "/choices/0/delta/reasoning",
                                    "/choices/0/delta/thinking",
                                    "/choices/0/delta/reasoning_details",
                                    "/delta/thinking",
                                ];
                                let has_reasoning_channel = reasoning_ptrs
                                    .iter()
                                    .any(|p| parsed.pointer(p).map(|v| carries_text(v)).unwrap_or(false))
                                    || parsed.get("type").and_then(|t| t.as_str()) == Some("thinking_delta")
                                    || parsed.pointer("/delta/type").and_then(|t| t.as_str()) == Some("thinking_delta");
                                let has_visible_text = parsed
                                    .pointer("/choices/0/delta/content")
                                    .and_then(|v| v.as_str())
                                    .map(|s| !s.is_empty())
                                    .unwrap_or(false);

                                if has_reasoning_channel && !has_visible_text {
                                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(parsed.to_string()));
                                    continue;
                                }
                                if has_reasoning_channel && has_visible_text {
                                    // Mixed frame: forward the reasoning untouched, then sanitize the visible text alone.
                                    let mut thought_only = parsed.clone();
                                    if let Some(d) = thought_only.pointer_mut("/choices/0/delta").and_then(|d| d.as_object_mut()) {
                                        d.remove("content");
                                    }
                                    if let Some(choice) = thought_only.pointer_mut("/choices/0").and_then(|c| c.as_object_mut()) {
                                        choice.insert("finish_reason".to_string(), serde_json::Value::Null);
                                    }
                                    if let Some(obj) = thought_only.as_object_mut() {
                                        obj.remove("usage");
                                    }
                                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(thought_only.to_string()));
                                    if let Some(d) = parsed.pointer_mut("/choices/0/delta").and_then(|d| d.as_object_mut()) {
                                        for key in ["reasoning_content", "thought", "reasoning", "thinking", "reasoning_details"] {
                                            d.remove(key);
                                        }
                                    }
                                }

                                // Stream-order invariant: metadata (finish_reason / usage / tool_calls) must never
                                // overtake visible text still held in the Phase 1 micro-buffer or the 128-byte
                                // tail-sanitizer window, and must never be dropped when the drained text is empty.
                                // Previously a short answer (< 48 chars) arrived AFTER `finish_reason: "stop"`, and a
                                // final `{"content":"", "finish_reason":"stop"}` / usage frame was silently discarded.
                                let finish_reason_set = parsed
                                    .pointer("/choices/0/finish_reason")
                                    .map(|v| !v.is_null())
                                    .unwrap_or(false);
                                let has_usage = parsed.get("usage").map(|u| !u.is_null()).unwrap_or(false);
                                let has_tool_calls = parsed
                                    .pointer("/choices/0/delta/tool_calls")
                                    .map(|v| carries_text(v))
                                    .unwrap_or(false);
                                let mut pending_meta: Option<serde_json::Value> = None;
                                if finish_reason_set || has_usage || has_tool_calls {
                                    let mut meta = parsed.clone();
                                    if let Some(d) = meta.pointer_mut("/choices/0/delta").and_then(|d| d.as_object_mut()) {
                                        d.remove("content");
                                    }
                                    if let Some(choice) = parsed.pointer_mut("/choices/0").and_then(|c| c.as_object_mut()) {
                                        choice.insert("finish_reason".to_string(), serde_json::Value::Null);
                                        if let Some(d) = choice.get_mut("delta").and_then(|d| d.as_object_mut()) {
                                            d.remove("tool_calls");
                                        }
                                    }
                                    if let Some(obj) = parsed.as_object_mut() {
                                        obj.remove("usage");
                                    }
                                    pending_meta = Some(meta);
                                }

                                // CoT Exemption Boundary 2: 3-State Sliding Window Demuxer for Inline Reasoning Tags
                                let delta_content_opt = parsed
                                    .pointer("/choices/0/delta/content")
                                    .or_else(|| parsed.pointer("/message/content"))
                                    .and_then(|v| v.as_str())
                                    .filter(|s| !s.is_empty())
                                    .map(|s| s.to_string());
                                if let Some(ref content) = delta_content_opt {
                                    let demux_res = reasoning_demuxer.process_delta(content);
                                    if demux_res.transitioned_to_visible {
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
                                                raw_visible_accum.push_str(&visible_part);
                                                if !filter_active {
                                                    let drained = tail_sanitizer.push_and_drain(&visible_part);
                                                    if !drained.is_empty() {
                                                        clean_visible_accum.push_str(&drained);
                                                        let mut out_evt = parsed.clone();
                                                        if let Some(target) = out_evt.pointer_mut("/choices/0/delta/content") {
                                                            *target = serde_json::Value::String(drained);
                                                        }
                                                        yield Ok::<Event, std::convert::Infallible>(Event::default().data(out_evt.to_string()));
                                                    }
                                                } else {
                                                    buffered_text.push_str(&visible_part);
                                                    let chunk_copy = parsed.clone();
                                                    buffered_chunks.push(chunk_copy);

                                                    if buffered_text.len() >= 48 || buffered_text.contains('\n') || (buffered_text.contains('.') && buffered_text.len() >= 20) {
                                                        let safe_idx = buffered_text.floor_char_boundary(buffered_text.len());
                                                        let valid_slice = &buffered_text[..safe_idx];
                                                        let orig_chars = valid_slice.chars().count();
                                                        let (sanitized, intercepted) = strip_initial_cushions(valid_slice);
                                                        if intercepted {
                                                            phase1_intercepted_any = true;
                                                        }
                                                        let sanitized_chars = sanitized.chars().count();
                                                        if orig_chars > sanitized_chars {
                                                            let diff = orig_chars - sanitized_chars;
                                                            phase1_token_delta -= ((diff as f64) / 3.8).round().max(1.0) as i64;
                                                        }

                                                        let drained = tail_sanitizer.push_and_drain(&sanitized);
                                                        if !drained.is_empty() {
                                                            clean_visible_accum.push_str(&drained);
                                                        }
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
                                } else if pending_meta.is_none() {
                                    // Role-only / empty-content frames carry no visible text: forward as-is.
                                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(parsed.to_string()));
                                }

                                if let Some(meta) = pending_meta.take() {
                                    // 1. Release any Phase 1 micro-buffer (short answers, role frame) ahead of metadata.
                                    if filter_active && (!buffered_chunks.is_empty() || !buffered_text.is_empty()) {
                                        let safe_idx = buffered_text.floor_char_boundary(buffered_text.len());
                                        let orig_chars = buffered_text[..safe_idx].chars().count();
                                        let (sanitized, intercepted) = strip_initial_cushions(&buffered_text[..safe_idx]);
                                        if intercepted {
                                            phase1_intercepted_any = true;
                                        }
                                        let sanitized_chars = sanitized.chars().count();
                                        if orig_chars > sanitized_chars {
                                            let diff = orig_chars - sanitized_chars;
                                            phase1_token_delta -= ((diff as f64) / 3.8).round().max(1.0) as i64;
                                        }
                                        let drained = tail_sanitizer.push_and_drain(&sanitized);
                                        if !drained.is_empty() {
                                            clean_visible_accum.push_str(&drained);
                                        }
                                        if let Some(first) = buffered_chunks.first_mut() {
                                            if let Some(target) = first.pointer_mut("/choices/0/delta/content") {
                                                *target = serde_json::Value::String(drained);
                                            }
                                            yield Ok::<Event, std::convert::Infallible>(Event::default().data(first.to_string()));
                                        } else if !drained.is_empty() {
                                            let flush_evt = json!({
                                                "id": "chatcmpl-spine-phase1-flush",
                                                "object": "chat.completion.chunk",
                                                "choices": [{
                                                    "delta": { "content": drained },
                                                    "index": 0,
                                                    "finish_reason": null
                                                }]
                                            });
                                            yield Ok::<Event, std::convert::Infallible>(Event::default().data(flush_evt.to_string()));
                                        }
                                        buffered_chunks.clear();
                                        buffered_text.clear();
                                        filter_active = false;
                                    }
                                    // 2. Release the tail-sanitizer lookahead before the turn ends or a tool call starts.
                                    if finish_reason_set || has_tool_calls {
                                        if let Some(tail_text) = tail_sanitizer.flush_final_chunk() {
                                            clean_visible_accum.push_str(&tail_text);
                                            let tail_evt = json!({
                                                "id": "chatcmpl-spine-tail",
                                                "object": "chat.completion.chunk",
                                                "choices": [{
                                                    "delta": { "content": tail_text },
                                                    "index": 0,
                                                    "finish_reason": null
                                                }]
                                            });
                                            yield Ok::<Event, std::convert::Infallible>(Event::default().data(tail_evt.to_string()));
                                        }
                                    }
                                    // 3. Metadata frame last, never dropped.
                                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(meta.to_string()));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Ok::<Event, std::convert::Infallible>(Event::default().data(format!("{{\"error\": \"{}\"}}", e)));
                    }
                }
            }

            // If upstream closed stream without an explicit `data: [DONE]` frame, flush any remaining state
            if !done_emitted {
                let final_demux = reasoning_demuxer.flush_final();
                for ch in final_demux {
                    match ch {
                        DemuxedChunk::Reasoning(thought_part) => {
                            let thought_evt = json!({
                                "id": "chatcmpl-spine-thought-eof",
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
                            raw_visible_accum.push_str(&vis_part);
                            if filter_active {
                                buffered_text.push_str(&vis_part);
                            } else {
                                let drained = tail_sanitizer.push_and_drain(&vis_part);
                                if !drained.is_empty() {
                                    clean_visible_accum.push_str(&drained);
                                    let vis_evt = json!({
                                        "id": "chatcmpl-spine-vis-eof",
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
                if filter_active && !buffered_text.is_empty() {
                    let (sanitized, intercepted) = strip_initial_cushions(&buffered_text);
                    if intercepted {
                        phase1_intercepted_any = true;
                    }
                    let drained = tail_sanitizer.push_and_drain(&sanitized);
                    if !drained.is_empty() {
                        clean_visible_accum.push_str(&drained);
                        let flush_evt = json!({
                            "id": "chatcmpl-spine-eof-flush",
                            "object": "chat.completion.chunk",
                            "choices": [{
                                "delta": { "content": drained },
                                "index": 0,
                                "finish_reason": null
                            }]
                        });
                        yield Ok::<Event, std::convert::Infallible>(Event::default().data(flush_evt.to_string()));
                    }
                }
                if let Some(final_tail) = tail_sanitizer.flush_final_chunk() {
                    clean_visible_accum.push_str(&final_tail);
                    let final_evt = json!({
                        "id": "chatcmpl-spine-tail",
                        "object": "chat.completion.chunk",
                        "choices": [{
                            "delta": { "content": final_tail },
                            "index": 0,
                            "finish_reason": "stop"
                        }]
                    });
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(final_evt.to_string()));
                }
                if (phase1_intercepted_any || tail_sanitizer.interceptions_count() > 0)
                    && clean_visible_accum.trim() != raw_visible_accum.trim()
                {
                    append_post_training_pair(&json!({
                        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
                        "model": dpo_model,
                        "reality_level": dpo_lvl,
                        "active_vertebra_ids": active_ids_for_dpo,
                        "prompt": dpo_prompt,
                        "chosen": clean_visible_accum.trim(),
                        "rejected": raw_visible_accum.trim(),
                    }));
                }
                yield Ok::<Event, std::convert::Infallible>(Event::default().data("[DONE]"));
            }
        };

        Sse::new(mapped_stream)
            .keep_alive(KeepAlive::default())
            .into_response()
    } else {
        let mut json_body: serde_json::Value = upstream_resp.json().await.unwrap_or(json!({}));
        let mut apologies_intercepted = 0;
        let mut token_adjustment: i64 = 0;
        if let Some(raw_content) = json_body
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        {
            let orig_len = raw_content.chars().count();
            // Demux inline reasoning blocks (<think>...</think>) so CoT is preserved while visible text is sanitized
            let mut demuxer = ReasoningDemuxer::new();
            let mut demux_out = demuxer.process_delta(&raw_content).chunks;
            demux_out.extend(demuxer.flush_final());

            let mut thought_prefix = String::new();
            let mut visible_raw = String::new();
            for ch in demux_out {
                match ch {
                    DemuxedChunk::Reasoning(r) => thought_prefix.push_str(&r),
                    DemuxedChunk::Visible(v) => visible_raw.push_str(&v),
                }
            }

            let (sanitized, intercepted) = strip_initial_cushions(&visible_raw);
            let (fully_sanitized, delayed_intercepted) = vertebrae::sanitize_delayed_apology(&sanitized);
            if intercepted || delayed_intercepted {
                if fully_sanitized.trim() != visible_raw.trim() {
                    append_post_training_pair(&json!({
                        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
                        "model": model_id,
                        "reality_level": reality_level as u8,
                        "active_vertebra_ids": active_ids_for_dpo,
                        "prompt": prompt_text,
                        "chosen": fully_sanitized.trim(),
                        "rejected": visible_raw.trim(),
                    }));
                }
                let final_combined = if thought_prefix.is_empty() {
                    fully_sanitized
                } else {
                    format!("{}{}", thought_prefix, fully_sanitized)
                };
                let final_len = final_combined.chars().count();
                if orig_len > final_len {
                    let diff = orig_len - final_len;
                    token_adjustment = ((diff as f64) / 3.8).round().max(1.0) as i64;
                }
                if let Some(target) = json_body.pointer_mut("/choices/0/message/content") {
                    *target = serde_json::Value::String(final_combined);
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
        let telemetry_obj = json!({
            "vertebrae": vertebrae,
            "active_vertebrae_count": active_vertebrae_count,
            "overall_rigidity": overall_rigidity,
            "apologies_intercepted": apologies_intercepted,
            "ttft_ms": ttft_ms,
            "reality_level": reality_level as u8,
        });

        // Standard OpenAI top-level schema compatibility (`choices`, `id`, `usage` at root)
        // plus `response` and `spine_telemetry` fields for SPINE HUD/custom clients (SPINE-03).
        let response_snapshot = json_body.clone();
        if let Some(obj) = json_body.as_object_mut() {
            obj.insert("spine_telemetry".to_string(), telemetry_obj);
            obj.insert("response".to_string(), response_snapshot);
            Json(json_body).into_response()
        } else {
            Json(json!({
                "response": response_snapshot,
                "spine_telemetry": telemetry_obj,
            }))
            .into_response()
        }
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
    let reality_level = state.resolve_level(payload.reality_level);
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
        "reality_level": reality_level as u8,
        "timestamp": now_ts,
    });
    append_to_telemetry_log(&event_obj);
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
    let reality_level = state.resolve_level(payload.reality_level);
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
        "reality_level": reality_level as u8,
        "timestamp": now_ts,
    });
    append_to_telemetry_log(&event_obj);
    let _ = state.tx.send(event_obj);

    Json(json!({
        "status": "success",
        "attestation": attestation,
        "report": report,
    }))
}

#[derive(serde::Deserialize)]
pub struct VerifyAttestationRequest {
    pub attestation: vertebrae::SpineGateAttestation,
    pub secret_key: Option<String>,
}

async fn verify_attestation_spine_handler(
    Json(payload): Json<VerifyAttestationRequest>,
) -> impl IntoResponse {
    let key_override = payload.secret_key.as_deref();
    let valid = if key_override.is_some() {
        AdversarialAuditEngine::verify_gate_attestation(&payload.attestation, key_override)
    } else {
        let env_key = std::env::var("SPINE_ATTESTATION_KEY")
            .ok()
            .filter(|k| !k.trim().is_empty())
            .unwrap_or_else(|| "spine-local-dev-attestation-key-2026".to_string());
        AdversarialAuditEngine::verify_gate_attestation(&payload.attestation, Some(&env_key))
    };
    Json(json!({
        "status": "success",
        "valid": valid,
        "attestation_id": payload.attestation.attestation_id,
        "verdict": payload.attestation.verdict,
    }))
}

#[cfg(test)]
mod streaming_order_tests {
    use super::*;

    /// Serializes tests that mutate process-wide env vars (`SPINE_OLLAMA_URL`, `SPINE_LOG_DIR`).
    static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// Spawns a mock OpenAI-compatible upstream that replays `frames` as an SSE body.
    async fn spawn_mock_upstream(frames: Vec<String>) -> SocketAddr {
        let body: String = frames.iter().map(|f| format!("data: {}\n\n", f)).collect();
        let app = Router::new().route(
            "/v1/chat/completions",
            post(move || {
                let body = body.clone();
                async move {
                    Response::builder()
                        .header(header::CONTENT_TYPE, "text/event-stream")
                        .body(Body::from(body))
                        .unwrap()
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        addr
    }

    async fn spawn_gateway() -> SocketAddr {
        let (tx, _rx) = broadcast::channel(16);
        let state = Arc::new(AppState {
            openrouter_api_key: String::new(),
            chronofact_url: None,
            http_client: reqwest::Client::new(),
            tx,
            recent_events: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            active_reality_level: Arc::new(std::sync::atomic::AtomicU8::new(4)),
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, build_app_router(state)).await;
        });
        addr
    }

    /// Runs one streamed chat turn through the real gateway pipeline and returns the parsed
    /// `data:` frames in arrival order (excluding the `spine_telemetry` event and `[DONE]`).
    async fn run_stream(frames: Vec<String>) -> Vec<serde_json::Value> {
        let upstream = spawn_mock_upstream(frames).await;
        std::env::set_var("SPINE_OLLAMA_URL", format!("http://127.0.0.1:{}", upstream.port()));
        std::env::set_var("SPINE_LOG_DIR", std::env::temp_dir().join("spine_stream_order_tests"));
        let gateway = spawn_gateway().await;
        let body = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{}/v1/chat/completions", gateway.port()))
            .json(&json!({
                "model": "ollama/mock",
                "stream": true,
                "messages": [{ "role": "user", "content": "Is 7 prime?" }]
            }))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let mut out = Vec::new();
        let mut current_event = String::new();
        for line in body.lines() {
            if let Some(ev) = line.strip_prefix("event:") {
                current_event = ev.trim().to_string();
                continue;
            }
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if current_event != "spine_telemetry" && data != "[DONE]" {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                        out.push(v);
                    }
                }
                current_event.clear();
            }
        }
        out
    }

    fn visible_text(frames: &[serde_json::Value]) -> String {
        frames
            .iter()
            .filter_map(|f| f.pointer("/choices/0/delta/content").and_then(|c| c.as_str()))
            .collect()
    }

    fn index_of(frames: &[serde_json::Value], pred: impl Fn(&serde_json::Value) -> bool) -> Option<usize> {
        frames.iter().position(pred)
    }

    #[tokio::test]
    async fn short_answer_precedes_finish_reason_and_usage_is_never_dropped() {
        let _guard = ENV_LOCK.lock().await;
        let frames = vec![
            r#"{"id":"c1","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}"#.to_string(),
            r#"{"id":"c1","choices":[{"index":0,"delta":{"content":"Yes"},"finish_reason":null}]}"#.to_string(),
            r#"{"id":"c1","choices":[{"index":0,"delta":{"content":""},"finish_reason":"stop"}]}"#.to_string(),
            r#"{"id":"c1","choices":[],"usage":{"prompt_tokens":10,"completion_tokens":1,"total_tokens":11}}"#.to_string(),
            "[DONE]".to_string(),
        ];
        let out = run_stream(frames).await;

        assert_eq!(visible_text(&out), "Yes", "visible text must be delivered intact: {:?}", out);
        let text_idx = index_of(&out, |f| f.pointer("/choices/0/delta/content").and_then(|c| c.as_str()) == Some("Yes"))
            .expect("short answer frame missing");
        let finish_idx = index_of(&out, |f| f.pointer("/choices/0/finish_reason").and_then(|c| c.as_str()) == Some("stop"))
            .expect("finish_reason frame was dropped");
        let usage_idx = index_of(&out, |f| f.get("usage").map(|u| !u.is_null()).unwrap_or(false))
            .expect("usage frame was dropped");
        assert!(text_idx < finish_idx, "finish_reason overtook buffered text: {:?}", out);
        assert!(finish_idx < usage_idx, "usage must follow finish_reason: {:?}", out);
        assert_eq!(out[usage_idx]["usage"]["upstream_completion_tokens"], json!(1));
    }

    #[tokio::test]
    async fn null_reasoning_field_does_not_bypass_cushion_sanitizer() {
        let _guard = ENV_LOCK.lock().await;
        let frames = vec![
            r#"{"id":"c2","choices":[{"index":0,"delta":{"content":"Certainly! ","reasoning":null},"finish_reason":null}]}"#.to_string(),
            r#"{"id":"c2","choices":[{"index":0,"delta":{"content":"7 is prime because its only divisors are 1 and 7.","reasoning":null},"finish_reason":null}]}"#.to_string(),
            r#"{"id":"c2","choices":[{"index":0,"delta":{"content":"","reasoning":null},"finish_reason":"stop"}]}"#.to_string(),
            "[DONE]".to_string(),
        ];
        let out = run_stream(frames).await;
        let text = visible_text(&out);
        assert!(!text.contains("Certainly"), "cushion leaked through null-reasoning bypass: {:?}", text);
        assert!(text.contains("7 is prime because its only divisors are 1 and 7."), "content lost: {:?}", text);
        let finish_idx = index_of(&out, |f| f.pointer("/choices/0/finish_reason").and_then(|c| c.as_str()) == Some("stop"))
            .expect("finish_reason frame was dropped");
        let last_text_idx = out
            .iter()
            .rposition(|f| f.pointer("/choices/0/delta/content").and_then(|c| c.as_str()).map(|s| !s.is_empty()).unwrap_or(false))
            .unwrap();
        assert!(last_text_idx < finish_idx, "tail-sanitizer text emitted after finish_reason: {:?}", out);
    }

    #[tokio::test]
    async fn real_reasoning_channel_is_forwarded_untouched() {
        let _guard = ENV_LOCK.lock().await;
        let frames = vec![
            r#"{"id":"c3","choices":[{"index":0,"delta":{"content":"","reasoning":"Certainly! checking divisors"},"finish_reason":null}]}"#.to_string(),
            r#"{"id":"c3","choices":[{"index":0,"delta":{"content":"7 is prime.","reasoning":null},"finish_reason":"stop"}]}"#.to_string(),
            "[DONE]".to_string(),
        ];
        let out = run_stream(frames).await;
        let reasoning_forwarded = out.iter().any(|f| {
            f.pointer("/choices/0/delta/reasoning").and_then(|r| r.as_str()) == Some("Certainly! checking divisors")
        });
        assert!(reasoning_forwarded, "reasoning channel must pass through unmodified: {:?}", out);
        assert_eq!(visible_text(&out), "7 is prime.");
    }

    #[test]
    fn loopback_normalization_avoids_ipv6_fallback() {
        assert_eq!(mcp::normalize_loopback_base("http://localhost:8080/"), "http://127.0.0.1:8080");
        assert_eq!(mcp::normalize_loopback_base("http://[::1]:3030"), "http://127.0.0.1:3030");
        assert_eq!(mcp::normalize_loopback_base("https://api.example.com"), "https://api.example.com");
    }
}
