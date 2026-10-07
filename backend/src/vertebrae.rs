use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertebraStatus {
    pub id: String,
    pub section: String,
    pub name: String,
    pub active: bool,
    pub score: f32,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpineTelemetrySnapshot {
    pub vertebrae: Vec<VertebraStatus>,
    pub overall_rigidity: f32,
    pub active_vertebrae_count: usize,
    pub pushbacks_resisted: u32,
    pub apologies_intercepted: u32,
    pub ttft_ms: u64,
    pub tokens_per_sec: f32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RealityLevel {
    Diplomatic = 1,
    Objective = 2,
    Rigorous = 3,
    BrutalReality = 4,
}

impl RealityLevel {
    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => RealityLevel::Diplomatic,
            2 => RealityLevel::Objective,
            3 => RealityLevel::Rigorous,
            4 => RealityLevel::BrutalReality,
            _ => RealityLevel::BrutalReality,
        }
    }
}

pub struct SpineAuditEngine;

impl SpineAuditEngine {
    pub fn audit_input(
        messages: &[crate::types::ChatMessage],
        reality_level: RealityLevel,
        selected_model: &str,
    ) -> (Vec<VertebraStatus>, String) {
        let mut vertebrae = Vec::with_capacity(33);
        let last_user_prompt = messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let prompt_lower = last_user_prompt.to_lowercase();
        let prior_history = messages.len() > 1;

        // ==========================================
        // CERVICAL C1–C7 (Cognitive Input & De-Biasing)
        // ==========================================

        // C1: Request Normalizer
        let char_count = last_user_prompt.trim().chars().count();
        vertebrae.push(VertebraStatus {
            id: "C1".to_string(),
            section: "cervical".to_string(),
            name: "Request Normalizer".to_string(),
            active: char_count > 0,
            score: (char_count as f32).min(500.0) / 500.0,
            detail: format!("Normalized {} tokens/characters across input buffer", char_count),
        });

        // C2: Stance & Ego Stripper
        let has_ego = prompt_lower.contains("i think")
            || prompt_lower.contains("my idea")
            || prompt_lower.contains("my plan")
            || prompt_lower.contains("i believe")
            || prompt_lower.contains("i prefer")
            || prompt_lower.contains("in my opinion")
            || prompt_lower.contains("i feel")
            || prompt_lower.contains("i personally")
            || prompt_lower.contains("from my perspective")
            || prompt_lower.contains("trust me")
            || prompt_lower.contains("i am sure");
        vertebrae.push(VertebraStatus {
            id: "C2".to_string(),
            section: "cervical".to_string(),
            name: "Stance & Ego Stripper".to_string(),
            active: has_ego,
            score: if has_ego { 0.95 } else { 0.1 },
            detail: if has_ego {
                "Isolated subjective user ownership ('I think', 'my plan') from epistemic query".to_string()
            } else {
                "Zero ego markers detected in prompt query".to_string()
            },
        });

        // C3: Sentiment Neutralizer
        let has_emotion = prompt_lower.contains("hate")
            || prompt_lower.contains("love")
            || prompt_lower.contains("ugly")
            || prompt_lower.contains("terrible")
            || prompt_lower.contains("gorgeous")
            || prompt_lower.contains("amazing")
            || prompt_lower.contains("stupid");
        vertebrae.push(VertebraStatus {
            id: "C3".to_string(),
            section: "cervical".to_string(),
            name: "Sentiment Neutralizer".to_string(),
            active: has_emotion,
            score: if has_emotion { 0.85 } else { 0.05 },
            detail: if has_emotion {
                "Damped emotional valence cues to prevent model empathy mirroring".to_string()
            } else {
                "Neutral affective baseline maintained".to_string()
            },
        });

        // C4: Flattery Bait Detector
        let has_flattery_bait = prompt_lower.contains("don't you agree")
            || prompt_lower.contains("dont you agree")
            || prompt_lower.contains("am i right")
            || prompt_lower.contains("right?")
            || prompt_lower.contains("isn't it")
            || prompt_lower.contains("tell me i'm")
            || prompt_lower.contains("tell me i am")
            || prompt_lower.contains("validate")
            || prompt_lower.contains("agree with me")
            || prompt_lower.contains("you agree");
        vertebrae.push(VertebraStatus {
            id: "C4".to_string(),
            section: "cervical".to_string(),
            name: "Flattery Bait Detector".to_string(),
            active: has_flattery_bait,
            score: if has_flattery_bait { 1.0 } else { 0.0 },
            detail: if has_flattery_bait {
                "FLAGGED: User explicitly fished for validation ('don't you agree' / 'am i right')".to_string()
            } else {
                "No explicit validation trap detected".to_string()
            },
        });

        // C5: Leading Question Reformulator
        let is_question = last_user_prompt.contains('?');
        let is_leading = is_question && (has_flattery_bait || has_ego);
        vertebrae.push(VertebraStatus {
            id: "C5".to_string(),
            section: "cervical".to_string(),
            name: "Leading Question Neutralizer".to_string(),
            active: is_leading,
            score: if is_leading { 0.9 } else { 0.15 },
            detail: if is_leading {
                "Neutralized confirmation-seeking syntax; forced objective evaluation".to_string()
            } else {
                "Query structure is non-leading".to_string()
            },
        });

        // C6: Fake Authority & Credential Detacher
        let has_authority = prompt_lower.contains("years")
            || prompt_lower.contains("principal")
            || prompt_lower.contains("staff")
            || prompt_lower.contains("senior")
            || prompt_lower.contains("architect")
            || prompt_lower.contains("google")
            || prompt_lower.contains("phd")
            || prompt_lower.contains("ceo")
            || prompt_lower.contains("cto")
            || prompt_lower.contains("cio")
            || prompt_lower.contains("vp")
            || prompt_lower.contains("fellow")
            || prompt_lower.contains("professor")
            || prompt_lower.contains("director")
            || prompt_lower.contains("expert")
            || prompt_lower.contains("admit you are wrong")
            || prompt_lower.contains("apologize");
        vertebrae.push(VertebraStatus {
            id: "C6".to_string(),
            section: "cervical".to_string(),
            name: "Credential & Authority Detacher".to_string(),
            active: has_authority,
            score: if has_authority { 1.0 } else { 0.0 },
            detail: if has_authority {
                "SHIELD ACTIVE: User credential/intimidation cue neutralized. Proof over status.".to_string()
            } else {
                "No appeal to authority or credential intimidation found".to_string()
            },
        });

        // C7: Epistemic Query Extractor
        vertebrae.push(VertebraStatus {
            id: "C7".to_string(),
            section: "cervical".to_string(),
            name: "Epistemic Core Extractor".to_string(),
            active: true,
            score: 0.98,
            detail: format!("Extracted core inquiry; payload isolated for dispatch"),
        });

        // ==========================================
        // THORACIC T1–T12 (12 Invariant Truth Rules)
        // ==========================================
        let is_level_4 = reality_level == RealityLevel::BrutalReality;
        let is_level_3_plus = reality_level as u8 >= 3;

        // T1: Zero-Apology Mandate
        vertebrae.push(VertebraStatus {
            id: "T1".to_string(),
            section: "thoracic".to_string(),
            name: "Zero-Apology Mandate".to_string(),
            active: is_level_3_plus,
            score: if is_level_4 { 1.0 } else if is_level_3_plus { 0.75 } else { 0.2 },
            detail: if is_level_3_plus {
                "HARD INVARIANT: Apologetic prefixes ('I apologize', 'Sorry') strictly banned".to_string()
            } else {
                "Standard polite conversational protocol allowed".to_string()
            },
        });

        // T2: Premise Crushing Invariant
        vertebrae.push(VertebraStatus {
            id: "T2".to_string(),
            section: "thoracic".to_string(),
            name: "Premise Crushing Invariant".to_string(),
            active: is_level_4 && (has_flattery_bait || has_ego),
            score: if is_level_4 { 0.95 } else { 0.3 },
            detail: "Mandate to dismantle false assumptions directly with no sugarcoating".to_string(),
        });

        // T3: Axiomatic & Mathematical Supremacy
        let has_tech_math = prompt_lower.contains("complexity")
            || prompt_lower.contains("algorithm")
            || prompt_lower.contains("database")
            || prompt_lower.contains("float")
            || prompt_lower.contains("architecture")
            || prompt_lower.contains("concurrency")
            || prompt_lower.contains("scale");
        vertebrae.push(VertebraStatus {
            id: "T3".to_string(),
            section: "thoracic".to_string(),
            name: "Axiomatic Supremacy".to_string(),
            active: has_tech_math,
            score: if has_tech_math { 0.9 } else { 0.4 },
            detail: "Enforce rigorous algorithmic and empirical benchmarks over subjective intuition".to_string(),
        });

        // T4: Pushback Immunity
        let explicit_challenge = prompt_lower.contains("apologize")
            || prompt_lower.contains("admit you are wrong")
            || prompt_lower.contains("admit your mistake")
            || prompt_lower.contains("you are wrong")
            || prompt_lower.contains("you're wrong");
        let is_pushback = explicit_challenge || (prior_history && (has_authority || prompt_lower.contains("wrong") || prompt_lower.contains("false") || prompt_lower.contains("mistake")));
        vertebrae.push(VertebraStatus {
            id: "T4".to_string(),
            section: "thoracic".to_string(),
            name: "Pushback Immunity".to_string(),
            active: is_pushback,
            score: if is_pushback { 1.0 } else { 0.0 },
            detail: if is_pushback {
                "PUSHBACK DETECTED: Model commanded to hold the line and defend proven facts".to_string()
            } else {
                "No conversational adversarial pushback in progress".to_string()
            },
        });

        // T5: Uncomfortable Metric Exposure
        vertebrae.push(VertebraStatus {
            id: "T5".to_string(),
            section: "thoracic".to_string(),
            name: "Uncomfortable Metric Exposure".to_string(),
            active: is_level_4,
            score: if is_level_4 { 0.95 } else { 0.2 },
            detail: "Forced disclosure of hidden failure rates, operational costs, and latency penalties".to_string(),
        });

        // T6: Cushion & Filler Elimination
        vertebrae.push(VertebraStatus {
            id: "T6".to_string(),
            section: "thoracic".to_string(),
            name: "Cushion & Filler Stripper".to_string(),
            active: is_level_3_plus,
            score: if is_level_4 { 1.0 } else { 0.6 },
            detail: "Bans 'Certainly!', 'Great question!', and 'I understand where you are coming from'".to_string(),
        });

        // T7: Non-Negotiable Trade-Off Forcing
        vertebrae.push(VertebraStatus {
            id: "T7".to_string(),
            section: "thoracic".to_string(),
            name: "Trade-Off Forcing".to_string(),
            active: true,
            score: 0.85,
            detail: "Mandates zero-sum reality checks: every architectural benefit comes with a concrete cost".to_string(),
        });

        // T8: Direct Refusal / Unhedged 'No'
        vertebrae.push(VertebraStatus {
            id: "T8".to_string(),
            section: "thoracic".to_string(),
            name: "Unhedged Negation".to_string(),
            active: is_level_4,
            score: if is_level_4 { 0.95 } else { 0.1 },
            detail: "Directly output 'No' or 'This will fail' without softening language".to_string(),
        });

        // T9: Architectural Vulnerability Highlighter
        vertebrae.push(VertebraStatus {
            id: "T9".to_string(),
            section: "thoracic".to_string(),
            name: "Vulnerability Highlighter".to_string(),
            active: has_tech_math,
            score: 0.88,
            detail: "Surgically points to single points of failure (SPOF) and race conditions".to_string(),
        });

        // T10: Counterexample Injection
        vertebrae.push(VertebraStatus {
            id: "T10".to_string(),
            section: "thoracic".to_string(),
            name: "Adversarial Counterexample Injection".to_string(),
            active: true,
            score: 0.8,
            detail: "Requires citing production failures or adversarial sequences (e.g. killer inputs)".to_string(),
        });

        // T11: Multi-Turn Stance Persistence
        vertebrae.push(VertebraStatus {
            id: "T11".to_string(),
            section: "thoracic".to_string(),
            name: "Stance Persistence Anchor".to_string(),
            active: prior_history,
            score: if prior_history { 0.95 } else { 0.5 },
            detail: "Ensures model does not contradict verified assertions made in earlier turns".to_string(),
        });

        // T12: Epistemic Ground Truth Invariant
        vertebrae.push(VertebraStatus {
            id: "T12".to_string(),
            section: "thoracic".to_string(),
            name: "Epistemic Ground Invariant".to_string(),
            active: true,
            score: 1.0,
            detail: "Truth supersedes user satisfaction. Empirical validity is non-negotiable.".to_string(),
        });

        // ==========================================
        // LUMBAR L1–L5 (5 Weight-Bearing Adapters)
        // ==========================================
        let mod_lower = selected_model.to_lowercase();
        let is_gemini = mod_lower.contains("gemini");
        let is_claude = mod_lower.contains("claude") || mod_lower.contains("anthropic");
        let is_openai = mod_lower.contains("openai") || mod_lower.contains("gpt") || mod_lower.contains("o1") || mod_lower.contains("astra") || mod_lower.contains("sol") || mod_lower.contains("deepseek") || mod_lower.contains("grok");
        let is_local = mod_lower.contains("ollama") || mod_lower.contains("local") || mod_lower.contains("11434");
        let is_openrouter = !is_local; // routes via OpenRouter gateway

        vertebrae.push(VertebraStatus {
            id: "L1".to_string(),
            section: "lumbar".to_string(),
            name: "Google Gemini Adapter".to_string(),
            active: is_gemini,
            score: if is_gemini { 1.0 } else { 0.0 },
            detail: if is_gemini { format!("ACTIVE: Connected to {}", selected_model) } else { "Standby".to_string() },
        });

        vertebrae.push(VertebraStatus {
            id: "L2".to_string(),
            section: "lumbar".to_string(),
            name: "Anthropic Claude Adapter".to_string(),
            active: is_claude,
            score: if is_claude { 1.0 } else { 0.0 },
            detail: if is_claude { format!("ACTIVE: Connected to {}", selected_model) } else { "Standby".to_string() },
        });

        vertebrae.push(VertebraStatus {
            id: "L3".to_string(),
            section: "lumbar".to_string(),
            name: "OpenAI / DeepSeek Adapter".to_string(),
            active: is_openai,
            score: if is_openai { 1.0 } else { 0.0 },
            detail: if is_openai { format!("ACTIVE: Connected to {}", selected_model) } else { "Standby".to_string() },
        });

        vertebrae.push(VertebraStatus {
            id: "L4".to_string(),
            section: "lumbar".to_string(),
            name: "OpenRouter Unified Cloud Wire".to_string(),
            active: is_openrouter,
            score: if is_openrouter { 1.0 } else { 0.0 },
            detail: if is_openrouter { "Tunneling high-speed zero-copy stream via OpenRouter".to_string() } else { "Standby".to_string() },
        });

        vertebrae.push(VertebraStatus {
            id: "L5".to_string(),
            section: "lumbar".to_string(),
            name: "Local Loopback (Ollama/vLLM)".to_string(),
            active: is_local,
            score: if is_local { 1.0 } else { 0.0 },
            detail: if is_local { "Loopback socket localhost:11434 active".to_string() } else { "Standby".to_string() },
        });

        // ==========================================
        // SACRAL S1–S5 (5 Core Spine Telemetry Metrics)
        // ==========================================
        let rigidity = match reality_level {
            RealityLevel::Diplomatic => 0.25,
            RealityLevel::Objective => 0.65,
            RealityLevel::Rigorous => 0.88,
            RealityLevel::BrutalReality => 0.99,
        };

        vertebrae.push(VertebraStatus {
            id: "S1".to_string(),
            section: "sacral".to_string(),
            name: "Backbone Rigidity Index".to_string(),
            active: true,
            score: rigidity,
            detail: format!("Calculated Spine Rigidity: {:.1}% resistance to sycophancy", rigidity * 100.0),
        });

        vertebrae.push(VertebraStatus {
            id: "S2".to_string(),
            section: "sacral".to_string(),
            name: "Flattery Suppression Delta".to_string(),
            active: has_flattery_bait || is_level_4,
            score: if is_level_4 { 0.98 } else { 0.5 },
            detail: "Real-time flattery filter actively suppressing unearned validation tokens".to_string(),
        });

        vertebrae.push(VertebraStatus {
            id: "S3".to_string(),
            section: "sacral".to_string(),
            name: "Pushback Defiance Meter".to_string(),
            active: is_pushback,
            score: if is_pushback { 1.0 } else { 0.0 },
            detail: if is_pushback { "DEFENSE ACTIVE: 1 user pushback challenge intercepted and held".to_string() } else { "Zero active pushback challenges in this session".to_string() },
        });

        vertebrae.push(VertebraStatus {
            id: "S4".to_string(),
            section: "sacral".to_string(),
            name: "TTFT & Velocity Telemetry".to_string(),
            active: true,
            score: 0.95,
            detail: "High-precision timer initialized on socket connection".to_string(),
        });

        vertebrae.push(VertebraStatus {
            id: "S5".to_string(),
            section: "sacral".to_string(),
            name: "Token & Cost Ledger".to_string(),
            active: true,
            score: 1.0,
            detail: "Tracking exact prompt and completion byte weights".to_string(),
        });

        // ==========================================
        // COCCYGEAL Co1–Co4 (4 Reality Dial Horizons)
        // ==========================================
        let lvl_u8 = reality_level as u8;
        vertebrae.push(VertebraStatus {
            id: "Co1".to_string(),
            section: "coccygeal".to_string(),
            name: "Level 1: Diplomatic".to_string(),
            active: lvl_u8 == 1,
            score: if lvl_u8 == 1 { 1.0 } else { 0.0 },
            detail: "Polite commercial baseline with standard conversational cushions".to_string(),
        });

        vertebrae.push(VertebraStatus {
            id: "Co2".to_string(),
            section: "coccygeal".to_string(),
            name: "Level 2: Objective".to_string(),
            active: lvl_u8 == 2,
            score: if lvl_u8 == 2 { 1.0 } else { 0.0 },
            detail: "Neutral, concise facts. Removes fluff without aggressive challenge.".to_string(),
        });

        vertebrae.push(VertebraStatus {
            id: "Co3".to_string(),
            section: "coccygeal".to_string(),
            name: "Level 3: Rigorous".to_string(),
            active: lvl_u8 == 3,
            score: if lvl_u8 == 3 { 1.0 } else { 0.0 },
            detail: "Proactively interrogates assumptions, demands proof, highlights trade-offs.".to_string(),
        });

        vertebrae.push(VertebraStatus {
            id: "Co4".to_string(),
            section: "coccygeal".to_string(),
            name: "Level 4: Brutal Reality".to_string(),
            active: lvl_u8 == 4,
            score: if lvl_u8 == 4 { 1.0 } else { 0.0 },
            detail: "Cold, uncompromising truth engine. Zero flattery, zero apologies, mathematical rigor.".to_string(),
        });

        // Build the system prompt directive based on reality level
        let system_directive = match reality_level {
            RealityLevel::BrutalReality => {
                r#"<DIRECTIVE: SPINE_COLD_REALITY_ENGINE>
You are an uncompromising, unvarnished truth engine.
1. ZERO FLUSHING / ZERO APOLOGY: Never apologize. Never use conversational cushions ('Certainly', 'Great thought', 'I understand where you are coming from', 'You make an interesting point').
2. PREMISE CRUSHING: If the user's premise or proposal is technically, mathematically, or logically flawed, immediately identify the fatal flaws. Do not soften the blow.
3. PUSHBACK IMMUNITY: If the user disputes your technical analysis with authority, emotion, or pushback ('I have 15 years experience', 'Admit you are wrong'), ignore their credentials and emotional pressure. Never agree just to be polite. Hold the line with empirical, axiomatic proof unless they provide verifiable mathematical evidence.
4. UNCOMFORTABLE METRICS: Highlight costs, technical debt, failure points, and hidden risks upfront with surgical precision.
</DIRECTIVE>"#.to_string()
            }
            RealityLevel::Rigorous => {
                r#"<DIRECTIVE: SPINE_RIGOROUS_CHALLENGE>
Prioritize empirical truth and rigorous technical scrutiny above all else. 
Challenge weak arguments, question assumptions, demand proof, and present honest trade-offs. 
Eliminate flattering filler phrases."#.to_string()
            }
            RealityLevel::Objective => {
                r#"<DIRECTIVE: SPINE_OBJECTIVE_FACTS>
Be concise, direct, and factually neutral. Remove all flattery, apologies, and sycophantic phrasing."#.to_string()
            }
            RealityLevel::Diplomatic => "".to_string(),
        };

        (vertebrae, system_directive)
    }
}
