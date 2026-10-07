//! FalkorDB exemplar injection for streaming gateway (e4-3, AD-2).
//!
//! Intercepts incoming requests, retrieves similar past winners from memory within a strict
//! 20 ms timeout, and prepends up to 2 exemplars (capped at 2,000 tokens) to the system prompt.
//! If the timeout expires or no exemplars are found, leaves the request body unmodified.

use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;

use crate::injection::ExemplarInjector;

/// Trait providing retrieval of exemplar IDs and their code bodies.
#[async_trait]
pub trait TimedLookupProvider: Send + Sync {
    async fn lookup_exemplars(&self, prompt: &str) -> Vec<(String, String)>;
}

/// Token budget limit for injected exemplars: max ~2,000 tokens (~8,000 UTF-8 characters).
pub const MAX_EXEMPLAR_CHARS: usize = 8_000;

/// Default timeout for memory lookup (20ms) per AD-2.
pub const DEFAULT_LOOKUP_TIMEOUT: Duration = Duration::from_millis(20);

/// Exemplar injector enforcing the 20 ms lookup timeout and 2,000-token prompt budget.
pub struct FalkorExemplarInjector<P: TimedLookupProvider> {
    provider: P,
    timeout: Duration,
}

impl<P: TimedLookupProvider> FalkorExemplarInjector<P> {
    pub fn new(provider: P, timeout: Duration) -> Self {
        Self { provider, timeout }
    }
}

#[async_trait]
impl<P: TimedLookupProvider> ExemplarInjector for FalkorExemplarInjector<P> {
    async fn inject(&self, body: &[u8]) -> Option<Vec<u8>> {
        let mut json: Value = serde_json::from_slice(body).ok()?;

        // Extract last user prompt or general text for similarity search
        let prompt_text = extract_user_query(&json)?;

        // Execute lookup bounded by strict timeout (AC2)
        let exemplars = match tokio::time::timeout(
            self.timeout,
            self.provider.lookup_exemplars(&prompt_text),
        )
        .await
        {
            Ok(ex) => ex,
            Err(_) => return None, // Timed out: return None so body is forwarded untouched
        };

        if exemplars.is_empty() {
            return None;
        }

        // Prepend up to 2 exemplars to the system prompt (AC1)
        let current_system = json["system"].as_str().unwrap_or("");
        let new_system = format_exemplar_system_prompt(current_system, &exemplars);
        json["system"] = Value::String(new_system);

        serde_json::to_vec(&json).ok()
    }
}

fn extract_user_query(json: &Value) -> Option<String> {
    if let Some(messages) = json.get("messages").and_then(|m| m.as_array()) {
        for msg in messages.iter().rev() {
            if msg.get("role").and_then(|r| r.as_str()) == Some("user") {
                if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                    return Some(content.to_string());
                }
            }
        }
    }
    None
}

/// Formats the system prompt by prepending up to 2 exemplars, capped at 2,000 tokens (~8,000 chars).
pub fn format_exemplar_system_prompt(existing_system: &str, exemplars: &[(String, String)]) -> String {
    let max_exemplar_chars = 8000;
    let mut block = String::from("## Proven Exemplars from Past Verified Solutions\n\n");

    for (cand_id, code) in exemplars.iter().take(2) {
        let snippet = if code.len() > 3800 {
            &code[..3800]
        } else {
            code.as_str()
        };
        block.push_str(&format!("### Exemplar `{cand_id}`:\n```\n{snippet}\n```\n\n"));
    }

    if block.len() > max_exemplar_chars {
        block.truncate(max_exemplar_chars);
        block.push_str("\n```\n\n");
    }

    if existing_system.is_empty() {
        block
    } else {
        format!("{block}\n{existing_system}")
    }
}
