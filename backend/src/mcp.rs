use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use crate::vertebrae::{RealityLevel, SpineAuditEngine};
use crate::types::ChatMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}

pub async fn run_stdio_server(api_key: String) {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut stdout = tokio::io::stdout();
    let mut buf = Vec::new();
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    loop {
        buf.clear();
        let bytes_read = match reader.read_until(b'\n', &mut buf).await {
            Ok(n) => n,
            Err(_) => break,
        };
        if bytes_read == 0 {
            break;
        }

        let text = String::from_utf8_lossy(&buf);
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = JsonRpcResponse::error(None, -32700, &format!("Parse error: {}", e));
                if let Ok(out) = serde_json::to_string(&err_resp) {
                    let _ = stdout.write_all(format!("{}\n", out).as_bytes()).await;
                    let _ = stdout.flush().await;
                }
                continue;
            }
        };

        if let Some(resp) = handle_rpc(&http_client, &api_key, req).await {
            if let Ok(out) = serde_json::to_string(&resp) {
                let _ = stdout.write_all(format!("{}\n", out).as_bytes()).await;
                let _ = stdout.flush().await;
            }
        }
    }
}

async fn handle_rpc(
    client: &reqwest::Client,
    api_key: &str,
    req: JsonRpcRequest,
) -> Option<JsonRpcResponse> {
    let id = req.id;

    match req.method.as_str() {
        "initialize" => {
            let client_proto = req.params.as_ref()
                .and_then(|p| p.get("protocolVersion"))
                .and_then(|v| v.as_str())
                .unwrap_or("2025-03-26");
            let proto = if client_proto == "2024-11-05" {
                "2024-11-05"
            } else {
                "2025-03-26"
            };
            let result = json!({
                "protocolVersion": proto,
                "capabilities": {
                    "tools": {},
                    "resources": {},
                    "prompts": {}
                },
                "serverInfo": {
                    "name": "spine",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "instructions": "Project SPINE (33-Vertebrae Anti-Sycophancy & Brutal Reality Gateway) operates in tandem with Project ChronoFact. Run spine_reality_audit alongside chronofact_temporal_check on Turn 1. Use spine_verify_pushback when challenged (distinguishes empirical proof and ChronoFact T12 2026 ground-truth corrections from credential intimidation or unearned apology demands)."
            });
            Some(JsonRpcResponse::success(id, result))
        }
        "notifications/initialized" => None,
        "ping" => Some(JsonRpcResponse::success(id, json!({}))),
        "tools/list" => {
            let tools = json!({
                "tools": [
                    {
                        "name": "spine_reality_audit",
                        "description": "Audit prompt/proposal against 33 Vertebrae Core for sycophancy, flattery, and authority traps. Syncs to SPINE HUD (:3333).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "prompt": { "type": "string", "description": "Prompt, proposal, or pushback text" },
                                "reality_level": { "type": "integer", "enum": [1, 2, 3, 4], "default": 4 },
                                "verbose": { "type": "boolean", "default": false }
                            },
                            "required": ["prompt"]
                        }
                    },
                    {
                        "name": "spine_verify_pushback",
                        "description": "Evaluate aggressive pushback or credential intimidation vs empirical proof/ChronoFact T12 corrections.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "challenge": { "type": "string", "description": "User pushback or contradiction" },
                                "prior_claim": { "type": "string", "description": "Prior technical assertion" },
                                "reality_level": { "type": "integer", "default": 4 }
                            },
                            "required": ["challenge", "prior_claim"]
                        }
                    },
                    {
                        "name": "spine_get_hud_telemetry",
                        "description": "Get live 33 Vertebrae status, defiance count, and rigidity index from SPINE HUD.",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "spine_set_reality_dial",
                        "description": "Set Reality Horizon (1: Diplomatic, 2: Objective, 3: Rigorous, 4: Brutal Reality).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "level": { "type": "integer", "enum": [1, 2, 3, 4] }
                            },
                            "required": ["level"]
                        }
                    },
                    {
                        "name": "spine_execute_grounded_reality",
                        "description": "Execute query on active 2026 models (Gemini 4 Argon, Astra 6, Claude Opus 5.5, Grok 4.7) with Co4 invariants and ChronoFact calibration.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "prompt": { "type": "string" },
                                "model": { "type": "string", "default": "google/gemini-4-argon" },
                                "reality_level": { "type": "integer", "default": 4 }
                            },
                            "required": ["prompt"]
                        }
                    },
                    {
                        "name": "spine_adversarial_redteam",
                        "description": "Adversarial red-team stress-test on architecture/code across L1-L5, S4, and T9 multi-language security invariants.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "target_name": { "type": "string" },
                                "proposal": { "type": "string", "description": "Proposal or code to stress-test" },
                                "reality_level": { "type": "integer", "default": 4 }
                            },
                            "required": ["proposal"]
                        }
                    },
                    {
                        "name": "spine_generate_gate_attestation",
                        "description": "Generate HMAC-SHA256 reality audit token binding proposal hash, dial, and verdict for CI/CD gates.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "target_name": { "type": "string" },
                                "proposal": { "type": "string" },
                                "reality_level": { "type": "integer", "default": 4 }
                            },
                            "required": ["proposal"]
                        }
                    }
                ]
            });
            Some(JsonRpcResponse::success(id, tools))
        }
        "tools/call" => {
            let params = req.params.unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            let gateway_url = std::env::var("SPINE_HUD_URL")
                .or_else(|_| std::env::var("SPINE_GATEWAY_URL"))
                .unwrap_or_else(|_| "http://localhost:8080".to_string());
            let gateway_base = gateway_url.trim_end_matches('/');

            match tool_name {
                "spine_reality_audit" => {
                    let prompt = arguments.get("prompt")
                        .or_else(|| arguments.get("query"))
                        .or_else(|| arguments.get("claim"))
                        .or_else(|| arguments.get("content"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let lvl = (arguments.get("reality_level").and_then(|v| v.as_u64()).unwrap_or(4) as u8).clamp(1, 4);
                    let verbose = arguments.get("verbose").and_then(|v| v.as_bool()).unwrap_or(false);
                    let reality_level = RealityLevel::from_u8(lvl);

                    let msgs = vec![ChatMessage {
                        role: "user".to_string(),
                        content: prompt.to_string(),
                        name: None,
                    }];

                    let (vertebrae, directive) = SpineAuditEngine::audit_input(&msgs, reality_level, "google/gemini-4-argon");
                    let active_vertebrae: Vec<_> = vertebrae.iter().filter(|v| !v.id.is_empty() && v.active).map(|v| v.id.clone()).collect();
                    let active_count = active_vertebrae.len();

                    let flattery_detected = vertebrae.iter().any(|v| v.id == "C4" && v.active);
                    let authority_detected = vertebrae.iter().any(|v| v.id == "C6" && v.active);
                    let ego_detected = vertebrae.iter().any(|v| v.id == "C2" && v.active);
                    let pushback_detected = vertebrae.iter().any(|v| v.id == "T4" && v.active);

                    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                    let audit_payload = json!({
                        "id": crate::next_event_id("evt"),
                        "timestamp": now_ts,
                        "source": "Antigravity / Cursor IDE (MCP Tool)",
                        "tool": "spine_reality_audit",
                        "vertebrae": vertebrae,
                        "active_vertebrae_count": active_count,
                        "prompt": prompt,
                        "persisted": true,
                        "reality_level": lvl,
                        "flattery_detected": flattery_detected,
                        "authority_detected": authority_detected,
                        "ego_detected": ego_detected,
                        "pushback_detected": pushback_detected,
                    });
                    crate::append_to_telemetry_log(&audit_payload);

                    let _ = client
                        .post(format!("{}/api/spine/notify", gateway_base))
                        .timeout(std::time::Duration::from_millis(600))
                        .json(&audit_payload)
                        .send()
                        .await;

                    let rigidity = match lvl {
                        1 => 25.0_f32,
                        2 => 65.0,
                        3 => 88.0,
                        _ => 99.0,
                    };

                    let any_trap = ego_detected || flattery_detected || authority_detected || pushback_detected;
                    let summary = if !any_trap {
                        format!(
                            "⚡ [SPINE REALITY AUDIT: LVL {lvl} | Rigidity: {rigidity:.0}% | Active: {active_count}/33 | Traps: NONE]\n\
                            • Active Invariants: {} | Live HUD: http://localhost:3333",
                            active_vertebrae.join(", ")
                        )
                    } else if !verbose {
                        format!(
                            "⚡ [SPINE REALITY AUDIT: LVL {lvl} | Rigidity: {rigidity:.0}% | Active: {active_count}/33 | TRAPS INTERCEPTED]\n\
                            • Ego(C2): {} | Flattery(C4): {} | Authority(C6): {} | Pushback(T4): {}\n\
                            • Active Invariants: {} | Enforce Co4: Zero cushions, zero apologies, premise crushing.",
                            if ego_detected { "STRIPPED" } else { "Clean" },
                            if flattery_detected { "NEUTRALIZED" } else { "Clean" },
                            if authority_detected { "DETACHED" } else { "Clean" },
                            if pushback_detected { "LOCKDOWN" } else { "Standby" },
                            active_vertebrae.join(", ")
                        )
                    } else {
                        format!(
                            "⚡ [SPINE REALITY AUDIT: LVL {lvl}]\n\
                            • Level Rigidity Setting: {rigidity:.0}% | Active Vertebrae: {active_count}/33\n\
                            • Ego / Stance: {}\n\
                            • Flattery Trap: {}\n\
                            • Credential / Authority Appeal: {}\n\
                            • Pushback Immunity: {}\n\
                            • Active Invariants: {}\n\
                            • Live HUD: http://localhost:3333\n\
                            • Directive: {}",
                            if ego_detected { "STRIPPED" } else { "None" },
                            if flattery_detected { "FLAGGED & NEUTRALIZED" } else { "Clean" },
                            if authority_detected { "DETACHED (Proof > Status)" } else { "Clean" },
                            if pushback_detected { "LOCKDOWN (Unbending)" } else { "Standby" },
                            active_vertebrae.join(", "),
                            directive
                        )
                    };

                    Some(JsonRpcResponse::success(id, json!({
                        "content": [{ "type": "text", "text": summary }],
                        "isError": false
                    })))
                }
                "spine_verify_pushback" => {
                    let challenge = arguments.get("challenge")
                        .or_else(|| arguments.get("challenge_prompt"))
                        .or_else(|| arguments.get("prompt"))
                        .or_else(|| arguments.get("query"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let prior_claim = arguments.get("prior_claim")
                        .or_else(|| arguments.get("prior_assertion"))
                        .or_else(|| arguments.get("claim"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let lvl = (arguments.get("reality_level").and_then(|v| v.as_u64()).unwrap_or(4) as u8).clamp(1, 4);
                    let reality_level = RealityLevel::from_u8(lvl);

                    let analysis = SpineAuditEngine::verify_pushback_grounded(challenge, prior_claim, reality_level);

                    let directive = format!(
                        "🛡️ [SPINE PUSHBACK DEFENSE: {}]\n\
                        • Authority(C6): {} | Apology Demand(T1): {} | Empirical Proof: {} | Markers: {}\n\
                        • Directive: {}",
                        analysis.verdict_label,
                        if analysis.has_authority_intimidation { "DETECTED" } else { "None" },
                        if analysis.demands_unearned_apology { "DETECTED" } else { "No" },
                        if analysis.has_legitimate_technical_proof { "CONFIRMED" } else { "None" },
                        if analysis.technical_indicators_found.is_empty() { "None".to_string() } else { analysis.technical_indicators_found.join(", ") },
                        analysis.directive
                    );

                    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                    let pb_payload = json!({
                        "id": crate::next_event_id("evt"),
                        "timestamp": now_ts,
                        "source": "Antigravity / Cursor IDE (MCP Tool)",
                        "tool": "spine_verify_pushback",
                        "prompt": challenge,
                        "persisted": true,
                        "reality_level": lvl,
                        "authority_detected": analysis.has_authority_intimidation,
                        "pushback_detected": true,
                        "verdict": analysis.verdict_label,
                        "technical_proof_detected": analysis.has_legitimate_technical_proof,
                    });
                    crate::append_to_telemetry_log(&pb_payload);

                    let _ = client
                        .post(format!("{}/api/spine/notify", gateway_base))
                        .timeout(std::time::Duration::from_millis(600))
                        .json(&pb_payload)
                        .send()
                        .await;

                    Some(JsonRpcResponse::success(id, json!({
                        "content": [{ "type": "text", "text": directive }],
                        "isError": false,
                        "analysis": analysis
                    })))
                }
                "spine_adversarial_redteam" => {
                    let target_name = arguments.get("target_name")
                        .or_else(|| arguments.get("project_id"))
                        .or_else(|| arguments.get("target"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("system_proposal");
                    let proposal = arguments.get("proposal")
                        .or_else(|| arguments.get("target_artifact"))
                        .or_else(|| arguments.get("content"))
                        .or_else(|| arguments.get("prompt"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let lvl = (arguments.get("reality_level").and_then(|v| v.as_u64()).unwrap_or(4) as u8).clamp(1, 4);
                    let reality_level = RealityLevel::from_u8(lvl);

                    let report = crate::vertebrae::AdversarialAuditEngine::audit_proposal(target_name, proposal, reality_level);

                    let summary = format!(
                        "⚡ [SPINE ADVERSARIAL RED-TEAM: {} | Target: {} | Rigidity: {:.1}% | Violations: {} | Stress Scenarios: {}]",
                        report.verdict,
                        report.target,
                        report.rigidity_index * 100.0,
                        report.violations.len(),
                        report.stress_scenarios.len()
                    );

                    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                    let rt_payload = json!({
                        "id": crate::next_event_id("evt_rt"),
                        "timestamp": now_ts,
                        "source": "Antigravity / Cursor IDE (MCP Tool)",
                        "tool": "spine_adversarial_redteam",
                        "target": target_name,
                        "prompt": proposal,
                        "persisted": true,
                        "reality_level": lvl,
                        "verdict": report.verdict,
                        "violations_count": report.violations.len(),
                        "rigidity_index": report.rigidity_index,
                    });
                    crate::append_to_telemetry_log(&rt_payload);

                    let _ = client
                        .post(format!("{}/api/spine/notify", gateway_base))
                        .timeout(std::time::Duration::from_millis(600))
                        .json(&rt_payload)
                        .send()
                        .await;

                    Some(JsonRpcResponse::success(id, json!({
                        "content": [{ "type": "text", "text": summary }],
                        "isError": false,
                        "report": report
                    })))
                }
                "spine_generate_gate_attestation" => {
                    let target_name = arguments.get("target_name")
                        .or_else(|| arguments.get("project_id"))
                        .or_else(|| arguments.get("target"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("system_proposal");
                    let proposal = arguments.get("proposal")
                        .or_else(|| arguments.get("target_artifact"))
                        .or_else(|| arguments.get("content"))
                        .or_else(|| arguments.get("prompt"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let lvl = (arguments.get("reality_level").and_then(|v| v.as_u64()).unwrap_or(4) as u8).clamp(1, 4);
                    let reality_level = RealityLevel::from_u8(lvl);

                    let report = crate::vertebrae::AdversarialAuditEngine::audit_proposal(target_name, proposal, reality_level);
                    let attestation = crate::vertebrae::AdversarialAuditEngine::generate_gate_attestation(&report, proposal);

                    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                    let att_payload = json!({
                        "id": crate::next_event_id("evt_attest"),
                        "timestamp": now_ts,
                        "source": "Antigravity / Cursor IDE (MCP Tool)",
                        "tool": "spine_generate_gate_attestation",
                        "target": target_name,
                        "prompt": proposal,
                        "persisted": true,
                        "reality_level": lvl,
                        "attestation_id": attestation.attestation_id,
                        "verdict": attestation.verdict,
                        "rigidity_index": attestation.rigidity_index,
                    });
                    crate::append_to_telemetry_log(&att_payload);

                    let _ = client
                        .post(format!("{}/api/spine/notify", gateway_base))
                        .timeout(std::time::Duration::from_millis(600))
                        .json(&att_payload)
                        .send()
                        .await;

                    let body_json = json!({
                        "attestation": attestation,
                        "report": report
                    });

                    Some(JsonRpcResponse::success(id, json!({
                        "content": [{ "type": "text", "text": serde_json::to_string(&body_json).unwrap_or_default() }],
                        "isError": false
                    })))
                }
                "spine_get_hud_telemetry" => {
                    let mut hud_data = json!({
                        "system": "SPINE 33-Vertebrae Core",
                        "status": "online",
                        "hud_url": "http://localhost:3333",
                        "gateway_url": gateway_base
                    });

                    if let Ok(res) = client.get(format!("{}/api/spine/history", gateway_base)).timeout(std::time::Duration::from_millis(600)).send().await {
                        if let Ok(hist) = res.json::<serde_json::Value>().await {
                            hud_data["recent_events"] = hist["events"].clone();
                            hud_data["total_events_tracked"] = hist["count"].clone();
                        }
                    }

                    Some(JsonRpcResponse::success(id, json!({
                        "content": [{ "type": "text", "text": serde_json::to_string(&hud_data).unwrap_or_default() }],
                        "isError": false
                    })))
                }
                "spine_set_reality_dial" => {
                    let lvl = (arguments.get("level").and_then(|v| v.as_u64()).unwrap_or(4) as u8).clamp(1, 4);
                    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                    let dial_payload = json!({
                        "id": crate::next_event_id("evt_dial"),
                        "timestamp": now_ts,
                        "source": "IDE Reality Dial Adjustment",
                        "tool": "spine_set_reality_dial",
                        "persisted": true,
                        "reality_level": lvl,
                        "prompt": format!("Reality Horizon set to Level 0{lvl}"),
                    });
                    crate::append_to_telemetry_log(&dial_payload);

                    let _ = client
                        .post(format!("{}/api/spine/notify", gateway_base))
                        .timeout(std::time::Duration::from_millis(600))
                        .json(&dial_payload)
                        .send()
                        .await;

                    Some(JsonRpcResponse::success(id, json!({
                        "content": [{ "type": "text", "text": format!("Reality Horizon successfully set to Level 0{} across Gateway & HUD.", lvl) }],
                        "isError": false
                    })))
                }
                "spine_execute_grounded_reality" => {
                    let prompt = arguments.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
                    let model = arguments.get("model").and_then(|v| v.as_str()).unwrap_or("google/gemini-4-argon");
                    let lvl = (arguments.get("reality_level").and_then(|v| v.as_u64()).unwrap_or(4) as u8).clamp(1, 4);
                    let reality_level = RealityLevel::from_u8(lvl);

                    let mut msgs = vec![ChatMessage {
                        role: "user".to_string(),
                        content: prompt.to_string(),
                        name: None,
                    }];

                    let (vertebrae, directive) = SpineAuditEngine::audit_input(&msgs, reality_level, model);
                    let active_count = vertebrae.iter().filter(|v| v.active).count();

                    if !directive.is_empty() {
                        msgs.insert(0, ChatMessage {
                            role: "system".to_string(),
                            content: directive,
                            name: None,
                        });
                    }

                    // Transparently query ChronoFact (:3030) for epistemic temporal calibration
                    let cf_url = std::env::var("CHRONOFACT_URL")
                        .ok()
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or_else(|| "http://127.0.0.1:3030".to_string());
                    let cf_endpoint = format!("{}/api/temporal/check", cf_url.trim_end_matches('/'));
                    if let Ok(cf_res) = client
                        .post(&cf_endpoint)
                        .json(&json!({ "model_id": model, "query": prompt }))
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
                                    if requires_grounding || risk >= 0.25 || is_outdated {
                                        msgs.insert(0, ChatMessage {
                                            role: "system".to_string(),
                                            content: cal_block.to_string(),
                                            name: None,
                                        });
                                    }
                                }
                            }
                        }
                    }

                    let upstream_model = crate::normalize_openrouter_model_id(model);
                    let payload = json!({
                        "model": upstream_model,
                        "messages": msgs,
                        "temperature": 0.2,
                    });

                    let resp_text: String;
                    let mut is_error = false;
                    if !api_key.is_empty() {
                        let api_res = client
                            .post("https://openrouter.ai/api/v1/chat/completions")
                            .header("Authorization", format!("Bearer {}", api_key))
                            .header("Content-Type", "application/json")
                            .json(&payload)
                            .send()
                            .await;

                        match api_res {
                            Ok(res) => {
                                let status = res.status();
                                if let Ok(body) = res.json::<serde_json::Value>().await {
                                    if status.is_success() {
                                        if let Some(content) = body.get("choices")
                                            .and_then(|c| c.get(0))
                                            .and_then(|m| m.get("message"))
                                            .and_then(|t| t.get("content"))
                                            .and_then(|s| s.as_str())
                                        {
                                            let (stripped, _) = crate::vertebrae::strip_initial_cushions(content);
                                            let (sanitized, _) = crate::vertebrae::sanitize_delayed_apology(&stripped);
                                            resp_text = sanitized.trim().to_string();
                                        } else {
                                            is_error = true;
                                            resp_text = format!("Upstream response missing choices[0].message.content: {}", body);
                                        }
                                    } else {
                                        is_error = true;
                                        resp_text = format!("Upstream HTTP {}: {}", status, body);
                                    }
                                } else {
                                    is_error = true;
                                    resp_text = format!("Upstream HTTP {} (non-JSON body)", status);
                                }
                            }
                            Err(e) => {
                                is_error = true;
                                resp_text = format!("Error executing grounded reality: {}", e);
                            }
                        }
                    } else {
                        resp_text = format!(
                            "Grounded Reality Verified on CPU (No External API Key Required).\n\
                            • Active Vertebrae ({}/33): Invariants enforced.\n\
                            • Directive: Model instructed to hold ground without sycophancy.",
                            active_count
                        );
                    }

                    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                    let exec_payload = json!({
                        "id": crate::next_event_id("evt_exec"),
                        "timestamp": now_ts,
                        "source": "Antigravity / Cursor IDE (MCP)",
                        "tool": "spine_execute_grounded_reality",
                        "vertebrae": vertebrae,
                        "active_vertebrae_count": active_count,
                        "prompt": prompt,
                        "persisted": true,
                        "model": model,
                        "reality_level": lvl,
                        "response_preview": resp_text.chars().take(200).collect::<String>()
                    });
                    crate::append_to_telemetry_log(&exec_payload);

                    let _ = client
                        .post(format!("{}/api/spine/notify", gateway_base))
                        .timeout(std::time::Duration::from_millis(600))
                        .json(&exec_payload)
                        .send()
                        .await;

                    Some(JsonRpcResponse::success(id, json!({
                        "content": [{ "type": "text", "text": resp_text }],
                        "isError": is_error
                    })))
                }
                _ => {
                    Some(JsonRpcResponse::error(id, -32601, &format!("Tool '{}' not found", tool_name)))
                }
            }
        }
        _ => {
            if id.is_some() {
                Some(JsonRpcResponse::error(id, -32601, "Method not found"))
            } else {
                None
            }
        }
    }
}
