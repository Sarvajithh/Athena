//! Local `LlmProvider` (06_AI_ENGINE.md §9, §10.1): Ollama or an
//! equivalent local server, "first-class fallback, not an afterthought."
//! Called by `pipeline::Synthesizer` only after the cloud provider
//! returns `ProviderUnavailable` — identical `PromptRequest` in, held to
//! the identical Stage 5 grounding bar out (§10.1: "a local model's
//! synthesis is held to the identical grounding bar as the cloud
//! model's").

use std::time::Duration;

use serde::Deserialize;

use crate::error::ReasoningError;
use crate::provider::{LlmProvider, PromptRequest};

/// Ollama's `/api/generate` with `stream: false` returns the full
/// completion in one response body, closer to the single-shot shape
/// `LlmProvider::complete` expects than the chat-completion streaming
/// endpoint would be.
pub struct OllamaProvider {
    base_url: String,
    model: String,
    client: reqwest::blocking::Client,
}

impl OllamaProvider {
    /// `base_url` defaults to `http://localhost:11434` in
    /// `athena-app`'s wiring but is a constructor argument, not a
    /// constant, so a user running Ollama on a different host/port (or
    /// a compatible local server entirely) needs no code change.
    pub fn new(base_url: String, model: String) -> OllamaProvider {
        OllamaProvider {
            base_url,
            model,
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(45))
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::new()),
        }
    }

    fn prompt_text(request: &PromptRequest) -> String {
        let mut sections = vec![
            request.system.clone(),
            format!("VERDICT (already decided, do not change it):\n{}", request.verdict_json),
            format!("EVIDENCE (the only IDs you may cite):\n{}", request.evidence_json),
            format!("RESPOND WITH JSON MATCHING THIS SCHEMA, NOTHING ELSE:\n{}", request.output_schema),
        ];
        if let Some(question) = &request.question {
            sections.push(format!("Follow-up question about this same verdict: \"{question}\""));
        }
        sections.join("\n\n")
    }
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

impl LlmProvider for OllamaProvider {
    fn name(&self) -> &'static str {
        "ollama"
    }

    fn complete(&self, request: &PromptRequest) -> Result<String, ReasoningError> {
        let url = format!("{}/api/generate", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.model,
            "prompt": Self::prompt_text(request),
            "format": "json",
            "stream": false,
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| ReasoningError::ProviderUnavailable(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ReasoningError::ProviderUnavailable(format!(
                "Ollama returned {}",
                response.status()
            )));
        }

        let parsed: OllamaGenerateResponse = response
            .json()
            .map_err(|e| ReasoningError::ProviderUnavailable(e.to_string()))?;

        Ok(parsed.response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_text_includes_persona_verdict_and_evidence() {
        let request = PromptRequest {
            system: "persona text".into(),
            verdict_json: "{\"headline\":\"X\"}".into(),
            evidence_json: "[{\"id\":7}]".into(),
            output_schema: "{}".into(),
            question: None,
            stricter: false,
        };
        let text = OllamaProvider::prompt_text(&request);
        assert!(text.contains("persona text"));
        assert!(text.contains("\"id\":7"));
    }
}
