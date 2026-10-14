use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// Custom deserializer for `ChatMessage.content` that accepts:
/// - Plain string (`"Hello"`)
/// - `null` (common in multi-step `assistant` messages with `tool_calls`)
/// - Multimodal array of content parts (`[{"type": "text", "text": "..."}, {"type": "image_url", ...}]`)
fn deserialize_flexible_content<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let val = Option::<Value>::deserialize(deserializer)?.unwrap_or(Value::Null);
    Ok(extract_text_from_content_value(&val))
}

pub fn extract_text_from_content_value(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Array(parts) => {
            let mut texts = Vec::new();
            for part in parts {
                if let Some(s) = part.as_str() {
                    texts.push(s.to_string());
                } else if let Some(t) = part.get("text").and_then(|v| v.as_str()) {
                    texts.push(t.to_string());
                }
            }
            texts.join("\n")
        }
        other => other.to_string(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(default, deserialize_with = "deserialize_flexible_content")]
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: Option<bool>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub reality_level: Option<u8>,
    /// Preserves all additional OpenAI/OpenRouter fields (`tools`, `tool_choice`, `modalities`, `reasoning`, `response_format`, etc.)
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub description: String,
    pub is_local: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_multimodal_and_multistep_chat_request_preservation() {
        let raw = json!({
            "model": "google/gemini-4-argon",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "Analyze this architecture diagram across 5 steps."},
                        {"type": "image_url", "image_url": {"url": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="}}
                    ]
                },
                {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [
                        {
                            "id": "call_1",
                            "type": "function",
                            "function": {"name": "inspect_layer", "arguments": "{\"layer\":1}"}
                        }
                    ]
                },
                {
                    "role": "tool",
                    "tool_call_id": "call_1",
                    "content": "Layer 1 verified."
                }
            ],
            "tools": [{"type": "function", "function": {"name": "inspect_layer"}}],
            "tool_choice": "auto",
            "modalities": ["text", "image"]
        });

        let req: ChatCompletionRequest = serde_json::from_value(raw).expect("Should deserialize multimodal and multi-step request");
        assert_eq!(req.messages.len(), 3);
        assert_eq!(req.messages[0].content, "Analyze this architecture diagram across 5 steps.");
        assert_eq!(req.messages[1].content, "");
        assert_eq!(req.messages[2].content, "Layer 1 verified.");
        assert!(req.max_tokens.is_none(), "max_tokens should remain None when not specified so long outputs are never capped at 4096");
        assert!(req.extra.contains_key("tools"));
        assert!(req.extra.contains_key("tool_choice"));
        assert!(req.extra.contains_key("modalities"));
    }
}

