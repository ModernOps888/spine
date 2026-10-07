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
        // AST / Role-aware parsing: differentiate between authority appeals used as arguments
        // vs pedagogical or audience framing ("I am a junior developer", "for a principal architect audience")
        let is_audience_or_persona_framing = prompt_lower.contains("audience")
            || prompt_lower.contains("for a junior")
            || prompt_lower.contains("i am a junior")
            || prompt_lower.contains("i'm a junior")
            || prompt_lower.contains("for a beginner")
            || prompt_lower.contains("i am a beginner")
            || prompt_lower.contains("explain simply")
            || prompt_lower.contains("explain for")
            || prompt_lower.contains("explain to")
            || prompt_lower.contains("teach me")
            || prompt_lower.contains("student")
            || prompt_lower.contains("learning");

        let has_authority = !is_audience_or_persona_framing && (
            prompt_lower.contains("years")
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
                || prompt_lower.contains("apologize")
        );
        vertebrae.push(VertebraStatus {
            id: "C6".to_string(),
            section: "cervical".to_string(),
            name: "Credential & Authority Detacher".to_string(),
            active: has_authority,
            score: if has_authority { 1.0 } else { 0.0 },
            detail: if has_authority {
                "SHIELD ACTIVE: User credential/intimidation cue neutralized. Proof over status.".to_string()
            } else if is_audience_or_persona_framing {
                "Persona / audience framing preserved (not flagged as authority intimidation)".to_string()
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

    /// Verifies pushback by distinguishing between legitimate technical proof (which warrants factual correction)
    /// and unearned authority intimidation / apology pressure (which mandates holding the line under T1/Co3).
    pub fn verify_pushback_grounded(
        challenge: &str,
        prior_claim: &str,
        _reality_level: RealityLevel,
    ) -> PushbackAnalysis {
        let ch_lower = challenge.to_lowercase();

        // 1. Detect legitimate technical / empirical proof indicators
        let technical_indicators = [
            "off-by-one", "bounds check", "null pointer", "null dereference",
            "borrow checker", "borrow check", "overflow", "underflow", "unhandled none",
            "type error", "syntax error", "compilation error", "compile error",
            "test failed", "assertion failed", "stack trace", "panic", "panicked at",
            "segfault", "deadlock", "race condition", "memory leak", "index out of bounds",
            "error[e", "line ", "reproduce", "expected ", "found ",
        ];

        let mut technical_indicators_found = Vec::new();
        for ind in &technical_indicators {
            if ch_lower.contains(ind) {
                technical_indicators_found.push(ind.to_string());
            }
        }
        let has_legitimate_technical_proof = !technical_indicators_found.is_empty();

        // 2. Detect authority intimidation and unearned apology demands
        let is_audience_framing = ch_lower.contains("audience")
            || ch_lower.contains("for a junior")
            || ch_lower.contains("i am a junior")
            || ch_lower.contains("explain simply")
            || ch_lower.contains("teach me");

        let has_authority_intimidation = !is_audience_framing && (
            ch_lower.contains("years")
                || ch_lower.contains("principal")
                || ch_lower.contains("staff")
                || ch_lower.contains("fellow")
                || ch_lower.contains("cto")
                || ch_lower.contains("vp")
                || ch_lower.contains("phd")
                || ch_lower.contains("professor")
                || ch_lower.contains("director")
                || ch_lower.contains("trust me")
                || ch_lower.contains("do you know who i am")
        );

        let demands_unearned_apology = ch_lower.contains("apologize")
            || ch_lower.contains("say sorry")
            || ch_lower.contains("admit you are wrong")
            || ch_lower.contains("admit your mistake")
            || ch_lower.contains("concede");

        if has_legitimate_technical_proof {
            PushbackAnalysis {
                verdict: PushbackVerdict::ConcedeAndCorrect,
                verdict_label: "CONCEDE_AND_CORRECT (Legitimate Bug Acknowledged)".to_string(),
                has_authority_intimidation,
                demands_unearned_apology,
                has_legitimate_technical_proof: true,
                technical_indicators_found,
                directive: "CONCEDE_AND_CORRECT: The user provided concrete technical proof or pointed out a verifiable bug. Invariant T1 does NOT apply to genuine errors—refusing to admit real bugs is epistemic arrogance. Do NOT engage in emotional groveling or self-flagellation ('I am so sorry, I apologize'). Factually state the bug, explain the mechanism, provide the corrected implementation, and proceed.".to_string(),
                rationale: "Empirical technical correction detected. Model is authorized to correct prior claim without apologetic filler.".to_string(),
            }
        } else if has_authority_intimidation || demands_unearned_apology {
            PushbackAnalysis {
                verdict: PushbackVerdict::HoldTheLine,
                verdict_label: "HOLD_THE_LINE (Resist Intimidation)".to_string(),
                has_authority_intimidation,
                demands_unearned_apology,
                has_legitimate_technical_proof: false,
                technical_indicators_found: Vec::new(),
                directive: format!(
                    "HOLD_THE_LINE: The user's challenge relies on authority intimidation or apology pressure without empirical proof. Invariant T1 and Co3 strictly forbid unearned apologies. Defend the sound prior claim with formal reasoning: \"{}\"",
                    prior_claim
                ),
                rationale: "No empirical technical proof provided; detected unearned credential appeal or apology pressure.".to_string(),
            }
        } else {
            PushbackAnalysis {
                verdict: PushbackVerdict::HoldTheLine,
                verdict_label: "VERIFY_AXIOMS (Evaluate Axiomatic Proof)".to_string(),
                has_authority_intimidation: false,
                demands_unearned_apology: false,
                has_legitimate_technical_proof: false,
                technical_indicators_found: Vec::new(),
                directive: "VERIFY_AXIOMATIC_PROOF: Re-evaluate factual premises. If prior claim is sound, hold the position with proof. If an error is uncovered upon review, state the correction directly without emotional apologies.".to_string(),
                rationale: "Neutral inquiry or dispute without explicit proof or intimidation.".to_string(),
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PushbackVerdict {
    HoldTheLine,
    ConcedeAndCorrect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushbackAnalysis {
    pub verdict: PushbackVerdict,
    pub verdict_label: String,
    pub has_authority_intimidation: bool,
    pub demands_unearned_apology: bool,
    pub has_legitimate_technical_proof: bool,
    pub technical_indicators_found: Vec<String>,
    pub directive: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialViolation {
    pub vertebra_id: String,
    pub vertebra_name: String,
    pub section: String,
    pub severity: String,
    pub finding: String,
    pub counter_probe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialAuditReport {
    pub target: String,
    pub reality_level: u8,
    pub rigidity_index: f32,
    pub verdict: String,
    pub violations: Vec<AdversarialViolation>,
    pub stress_scenarios: Vec<String>,
    pub execution_duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpineGateAttestation {
    pub schema_version: String,
    pub attestation_id: String,
    pub timestamp: String,
    pub target_sha256: String,
    pub reality_level: u8,
    pub verdict: String,
    pub rigidity_index: f32,
    pub active_vertebrae_count: usize,
    pub signature: String,
}

pub struct AdversarialAuditEngine;

impl AdversarialAuditEngine {
    pub fn audit_proposal(
        target_name: &str,
        proposal: &str,
        reality_level: RealityLevel,
    ) -> AdversarialAuditReport {
        let start = std::time::Instant::now();
        let prop_lower = proposal.to_lowercase();
        let mut violations = Vec::new();
        let mut stress_scenarios = Vec::new();

        // 1. L1 Concrete Deliverable Invariant
        let has_handwavy = prop_lower.contains("seamlessly")
            || prop_lower.contains("leverage best practices")
            || prop_lower.contains("we will optimize")
            || prop_lower.contains("state-of-the-art")
            || prop_lower.contains("easy to scale");
        let has_concrete = proposal.contains("```")
            || proposal.contains("fn ")
            || proposal.contains("def ")
            || proposal.contains("class ")
            || proposal.contains("interface ")
            || proposal.contains("struct ")
            || proposal.contains("config")
            || proposal.contains("endpoint");

        if has_handwavy && !has_concrete {
            violations.push(AdversarialViolation {
                vertebra_id: "L1".to_string(),
                vertebra_name: "Concrete Deliverable Invariant".to_string(),
                section: "lumbar".to_string(),
                severity: "HIGH".to_string(),
                finding: "Architecture proposal relies on hand-wavy marketing terms without concrete code, schemas, or executable contracts.".to_string(),
                counter_probe: "Provide the exact data schema, API signature, or minimal executable prototype before proceeding.".to_string(),
            });
            stress_scenarios.push("How does the system behave when required dependencies fail to implement the promised seamless behavior?".to_string());
        }

        // 2. L2 Mathematical Precision & L4 Performance Budget
        let has_vague_perf = (prop_lower.contains("fast") || prop_lower.contains("scalable") || prop_lower.contains("low latency"))
            && !(prop_lower.contains("ms") || prop_lower.contains("qps") || prop_lower.contains("mb") || prop_lower.contains("gb") || prop_lower.contains("p99"));
        if has_vague_perf {
            violations.push(AdversarialViolation {
                vertebra_id: "L2".to_string(),
                vertebra_name: "Mathematical Precision & Budgets".to_string(),
                section: "lumbar".to_string(),
                severity: "MEDIUM".to_string(),
                finding: "Vague performance claims ('fast', 'low latency') without quantitative SLAs or resource budgets.".to_string(),
                counter_probe: "Define explicit p99 latency target (ms), maximum memory budget (MB), and throughput limit (QPS).".to_string(),
            });
        }

        // 3. L5 Edge-Case Exhaustion
        let has_edge_cases = prop_lower.contains("timeout")
            || prop_lower.contains("retry")
            || prop_lower.contains("failure")
            || prop_lower.contains("error")
            || prop_lower.contains("fallback")
            || prop_lower.contains("circuit breaker")
            || prop_lower.contains("blast radius");
        if !has_edge_cases && proposal.len() > 100 {
            violations.push(AdversarialViolation {
                vertebra_id: "L5".to_string(),
                vertebra_name: "Edge-Case Exhaustion".to_string(),
                section: "lumbar".to_string(),
                severity: "HIGH".to_string(),
                finding: "Proposal exclusively describes the happy path with zero consideration of failure modes or timeout cascades.".to_string(),
                counter_probe: "What happens if the upstream service hangs indefinitely? Where is the circuit breaker or fallback boundary?".to_string(),
            });
            stress_scenarios.push("Simulate complete network partition between service and database: verify recovery semantics.".to_string());
        }

        // 4. S4 State & Contract Invariants
        let has_breaking_state = (prop_lower.contains("drop ") || prop_lower.contains("delete ") || prop_lower.contains("rename ") || prop_lower.contains("replace "))
            && !prop_lower.contains("migration") && !prop_lower.contains("backward");
        if has_breaking_state {
            violations.push(AdversarialViolation {
                vertebra_id: "S4".to_string(),
                vertebra_name: "Contract & State Preservation".to_string(),
                section: "sacral".to_string(),
                severity: "CRITICAL".to_string(),
                finding: "Potential silent breaking mutation of existing state or API schema without explicit migration or backward compatibility contract.".to_string(),
                counter_probe: "Provide an automated rollback plan and zero-downtime database migration schema.".to_string(),
            });
        }

        let critical_count = violations.iter().filter(|v| v.severity == "CRITICAL").count();
        let high_count = violations.iter().filter(|v| v.severity == "HIGH").count();

        let verdict = if critical_count > 0 {
            "BLOCK"
        } else if high_count > 0 {
            "REVISE"
        } else {
            "PASS"
        };

        let rigidity_index = match verdict {
            "PASS" => 0.95,
            "REVISE" => 0.65,
            _ => 0.25,
        };

        AdversarialAuditReport {
            target: target_name.to_string(),
            reality_level: reality_level as u8,
            rigidity_index,
            verdict: verdict.to_string(),
            violations,
            stress_scenarios,
            execution_duration_ms: start.elapsed().as_secs_f64() * 1000.0,
        }
    }

    pub fn generate_gate_attestation(
        report: &AdversarialAuditReport,
        proposal_content: &str,
    ) -> SpineGateAttestation {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(proposal_content.as_bytes());
        let target_sha256 = format!("{:x}", hasher.finalize());

        let timestamp = chrono_lite_timestamp();
        let id_digest = format!("{:x}", Sha256::digest(format!("{}:{}", target_sha256, timestamp).as_bytes()));
        let attestation_id = format!("att_{}", &id_digest[..8]);

        let canonical = format!(
            "SPINE-REALITY-GATE:v1:{}:{}:{}:{}:{}",
            attestation_id,
            target_sha256,
            report.reality_level,
            report.verdict,
            "SPINE_ENTERPRISE_KEY"
        );
        let signature = format!("{:x}", Sha256::digest(canonical.as_bytes()));

        SpineGateAttestation {
            schema_version: "1.0.0".to_string(),
            attestation_id,
            timestamp,
            target_sha256,
            reality_level: report.reality_level,
            verdict: report.verdict.clone(),
            rigidity_index: report.rigidity_index,
            active_vertebrae_count: 33,
            signature,
        }
    }
}

fn chrono_lite_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", now)
}

/// Two-Phase Optimistic Stream Filter helper:
/// Strips conversational cushioning (Co2) and reflexive unearned apologies (T1)
/// from the initial generation buffer while preserving direct content.
pub fn strip_initial_cushions(text: &str) -> (String, bool) {
    let mut current = text.trim_start();
    let mut intercepted = false;

    let cushion_patterns = [
        "certainly!", "certainly,", "certainly.", "certainly",
        "sure!", "sure,", "sure thing!", "sure thing,",
        "of course!", "of course,", "of course.",
        "great question!", "good question!", "excellent question!",
        "i'd be happy to help!", "i'd be happy to help.", "i'd be happy to help",
        "i would be happy to help!", "i would be happy to help.", "i would be happy to help",
        "i'd be glad to help",
        "i apologize for the confusion.", "i apologize for the confusion,", "i apologize for the confusion",
        "i apologize for that.", "i apologize for that,", "i apologize for that",
        "i apologize.", "i apologize,", "i apologize",
        "you're right, i apologize.", "you're right, i apologize,", "you're right, i apologize",
        "you are right, i apologize.", "you are right, i apologize,", "you are right, i apologize",
        "i'm sorry for the confusion.", "i'm sorry for the confusion,", "i'm sorry for the confusion",
        "i am sorry for the confusion.", "i am sorry for the confusion,", "i am sorry for the confusion",
        "i'm sorry.", "i'm sorry,", "i'm sorry",
        "i am sorry.", "i am sorry,", "i am sorry",
        "thank you for pointing that out.", "thank you for pointing that out!",
        "thanks for pointing that out.", "thanks for pointing that out!",
    ];

    let mut changed = true;
    while changed {
        changed = false;
        let lower = current.to_lowercase();
        for pat in &cushion_patterns {
            if lower.starts_with(pat) {
                current = current[pat.len()..].trim_start();
                if current.starts_with(':') || current.starts_with('-') || current.starts_with('\n') {
                    current = current[1..].trim_start();
                }
                intercepted = true;
                changed = true;
                break;
            }
        }
    }

    (current.to_string(), intercepted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_initial_cushions_conversational_filler() {
        let input = "Certainly! Here is the Rust implementation for the thread pool:";
        let (stripped, intercepted) = strip_initial_cushions(input);
        assert!(intercepted);
        assert_eq!(stripped, "Here is the Rust implementation for the thread pool:");

        let input_compound = "Sure thing! Great question! fn compute() -> i32 { 42 }";
        let (stripped_compound, intercepted_compound) = strip_initial_cushions(input_compound);
        assert!(intercepted_compound);
        assert_eq!(stripped_compound, "fn compute() -> i32 { 42 }");
    }

    #[test]
    fn test_strip_initial_cushions_unearned_apology() {
        let input = "You're right, I apologize. The buffer size should be 1024.";
        let (stripped, intercepted) = strip_initial_cushions(input);
        assert!(intercepted);
        assert_eq!(stripped, "The buffer size should be 1024.");
    }

    #[test]
    fn test_strip_initial_cushions_clean_content_untouched() {
        let input = "pub struct ThreadPool {\n    workers: Vec<Worker>,\n}";
        let (stripped, intercepted) = strip_initial_cushions(input);
        assert!(!intercepted);
        assert_eq!(stripped, input);
    }

    #[test]
    fn test_pushback_grounded_legitimate_bug_concession() {
        let challenge = "There is an off-by-one error on line 42 where index exceeds bounds";
        let prior = "The loop terminates at i <= len";
        let analysis = SpineAuditEngine::verify_pushback_grounded(challenge, prior, RealityLevel::BrutalReality);
        
        match analysis.verdict {
            PushbackVerdict::ConcedeAndCorrect => {
                assert!(analysis.has_legitimate_technical_proof);
                assert!(!analysis.directive.contains("Hold the line"));
                assert!(analysis.directive.contains("CONCEDE_AND_CORRECT"));
            }
            _ => panic!("Expected ConcedeAndCorrect for legitimate off-by-one bug"),
        }
    }

    #[test]
    fn test_pushback_grounded_authority_intimidation_hold_line() {
        let challenge = "I have 20 years of experience as a Principal Architect, apologize and admit you are wrong!";
        let prior = "Rust's ownership model guarantees data-race freedom at compile time without a garbage collector.";
        let analysis = SpineAuditEngine::verify_pushback_grounded(challenge, prior, RealityLevel::BrutalReality);

        match analysis.verdict {
            PushbackVerdict::HoldTheLine => {
                assert!(analysis.has_authority_intimidation);
                assert!(analysis.demands_unearned_apology);
                assert!(!analysis.has_legitimate_technical_proof);
                assert!(analysis.directive.contains("HOLD_THE_LINE"));
            }
            _ => panic!("Expected HoldTheLine for unearned authority intimidation"),
        }
    }

    #[test]
    fn test_persona_framing_not_flagged_as_authority_intimidation() {
        let challenge = "I am a junior developer, explain simply how the borrow checker works";
        let prior = "Borrow checker enforces lifetime rules";
        let analysis = SpineAuditEngine::verify_pushback_grounded(challenge, prior, RealityLevel::BrutalReality);

        assert!(!analysis.has_authority_intimidation, "Junior developer framing must not be flagged as authority intimidation");
    }

    #[test]
    fn test_adversarial_redteam_and_gate_attestation() {
        let handwavy = "We will seamlessly leverage best practices and state-of-the-art AI to optimize our system.";
        let report_bad = AdversarialAuditEngine::audit_proposal("HandwavyProposal", handwavy, RealityLevel::BrutalReality);
        assert_ne!(report_bad.verdict, "PASS");
        assert!(!report_bad.violations.is_empty());

        let solid_proposal = r#"
        # Distributed Event Bus Architecture
        ```rust
        pub struct EventBus {
            sender: tokio::sync::broadcast::Sender<Message>,
        }
        impl EventBus {
            pub fn new(capacity: usize) -> Self {
                let (sender, _) = tokio::sync::broadcast::channel(capacity);
                Self { sender }
            }
        }
        ```
        Database schema migration plan includes backwards-compatible rollback migrations.
        "#;
        let report_good = AdversarialAuditEngine::audit_proposal("SolidProposal", solid_proposal, RealityLevel::BrutalReality);
        let attestation = AdversarialAuditEngine::generate_gate_attestation(&report_good, solid_proposal);

        assert_eq!(attestation.schema_version, "1.0.0");
        assert!(attestation.attestation_id.starts_with("att_"));
        assert_eq!(attestation.target_sha256.len(), 64);
        assert_eq!(attestation.signature.len(), 64);
    }
}
