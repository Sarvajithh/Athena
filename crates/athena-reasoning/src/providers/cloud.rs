//! Cloud `LlmProvider` (06_AI_ENGINE.md §9, primary): Anthropic Claude
//! via the `/v1/messages` endpoint. What leaves the device is exactly
//! `PromptRequest`'s fields — the narrow Stage 3 prompt for this one
//! call — "never a database dump, never raw identifiers beyond what
//! phrasing requires" (§9).

use std::time::Duration;

use serde::Deserialize;

use crate::error::ReasoningError;
use crate::provider::{LlmProvider, PromptRequest};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// The API key is read once at construction (from the OS keychain, via
/// whatever `athena-app` already uses for GitHub's token —
/// `keychain.rs` — never from SQLite or a plaintext config file, same
/// rule §4/§6 of `07_INTEGRATIONS.md` already applies to connector
/// credentials) and held only in memory for the process's lifetime.
pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
}

impl AnthropicProvider {
    /// `model` is the caller's choice (`athena-app` wiring), not
    /// hardcoded here, so a model deprecation or version bump is a
    /// one-line config change rather than a code change — the same
    /// "5-year risk" reasoning §9 gives for the trait itself applies one
    /// level down to the model string.
    pub fn new(api_key: String, model: String) -> AnthropicProvider {
        AnthropicProvider {
            api_key,
            model,
            // Stage 4 calls are short (one narrow prompt, one small JSON
            // reply) — a fixed, generous timeout is enough to catch a
            // hung connection without the caller needing to configure
            // one, matching the "one interface, no speculative
            // configuration surface" spirit of this crate's dependency
            // list.
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::new()),
        }
    }

    fn user_content(request: &PromptRequest) -> String {
        let mut sections = vec![
            format!("VERDICT (already decided, do not change it):\n{}", request.verdict_json),
            format!("EVIDENCE (the only IDs you may cite):\n{}", request.evidence_json),
            format!("RESPOND WITH JSON MATCHING THIS SCHEMA, NOTHING ELSE:\n{}", request.output_schema),
        ];
        if let Some(question) = &request.question {
            sections.push(format!(
                "The user asked a follow-up question about this same verdict — answer it using only the \
                 verdict/evidence above, still as the same JSON shape: \"{question}\""
            ));
        }
        sections.join("\n\n")
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessageResponse {
    content: Vec<AnthropicContentBlock>,
}

impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn complete(&self, request: &PromptRequest) -> Result<String, ReasoningError> {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 512,
            "system": request.system,
            "messages": [
                { "role": "user", "content": Self::user_content(request) }
            ],
        });

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| ReasoningError::ProviderUnavailable(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ReasoningError::ProviderUnavailable(format!(
                "Anthropic API returned {}",
                response.status()
            )));
        }

        let parsed: AnthropicMessageResponse = response
            .json()
            .map_err(|e| ReasoningError::ProviderUnavailable(e.to_string()))?;

        parsed
            .content
            .into_iter()
            .find_map(|block| block.text)
            .ok_or_else(|| ReasoningError::ProviderUnavailable("empty response content".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_content_never_omits_the_evidence_or_schema() {
        let request = PromptRequest {
            system: "persona".into(),
            verdict_json: "{\"headline\":\"X\"}".into(),
            evidence_json: "[{\"id\":7}]".into(),
            output_schema: "{\"type\":\"object\"}".into(),
            question: None,
            stricter: false,
        };
        let content = AnthropicProvider::user_content(&request);
        assert!(content.contains("\"id\":7"));
        assert!(content.contains("object"));
    }

    #[test]
    fn reflection_question_is_appended_not_substituted() {
        let request = PromptRequest {
            system: "persona".into(),
            verdict_json: "{}".into(),
            evidence_json: "[]".into(),
            output_schema: "{}".into(),
            question: Some("why not the other one?".into()),
            stricter: false,
        };
        let content = AnthropicProvider::user_content(&request);
        assert!(content.contains("why not the other one?"));
    }
}
