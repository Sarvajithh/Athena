//! Hugging Face Inference API provider (free tier).
//!
//! Uses HF's `/v1/chat/completions` endpoint — the OpenAI-compatible
//! surface every modern HF Inference model exposes — so the request
//! shape is identical to what you'd send to OpenAI, just pointed at
//! `https://router.huggingface.co/v1/chat/completions` with a
//! `Bearer hf_…` token.
//!
//! **Free-tier model choices** (good JSON instruction-following, no
//! billing required as of 2026):
//!   - `"Qwen/Qwen2.5-72B-Instruct"`      — best reasoning on free tier
//!   - `"meta-llama/Llama-3.3-70B-Instruct"` — strong, widely tested
//!   - `"mistralai/Mistral-7B-Instruct-v0.3"` — fast, lighter
//!   - `"HuggingFaceH4/zephyr-7b-beta"`   — good at JSON-only output
//!
//! Slot in the cascade in `commands/ai.rs`: after Anthropic (paid,
//! cloud), before Ollama (local). A missing or invalid HF token falls
//! through to Ollama / template the same way any unavailable provider
//! does — `ProviderUnavailable` is never fatal.
//!
//! **Getting a free token:**
//!   1. <https://huggingface.co/join>
//!   2. Settings → Access Tokens → New token → role: "Inference"
//!      (read-only is enough; no billing required for the free tier)
//!   3. `await window.__TAURI__.invoke('save_hf_api_key', { key: 'hf_…' })`

use std::time::Duration;

use serde::Deserialize;

use crate::error::ReasoningError;
use crate::provider::{LlmProvider, PromptRequest};

const HF_CHAT_URL: &str = "https://router.huggingface.co/v1/chat/completions";

pub struct HuggingFaceProvider {
    api_token: String,
    model: String,
    client: reqwest::blocking::Client,
}

impl HuggingFaceProvider {
    /// `model` is a fully-qualified HF model ID, e.g.
    /// `"Qwen/Qwen2.5-72B-Instruct"`. The default wired in
    /// `commands/ai.rs` is `Qwen/Qwen2.5-72B-Instruct` — best
    /// JSON-instruction following on the free tier. Change it by
    /// saving a different model string to the keychain (or just swap
    /// the constant in `ai.rs`) — no code change needed here.
    pub fn new(api_token: String, model: String) -> HuggingFaceProvider {
        HuggingFaceProvider {
            api_token,
            model,
            // HF free tier can be slow under load; 60 s is generous but
            // still finite — a hung connection falls through to Ollama /
            // template rather than blocking the UI thread indefinitely.
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::new()),
        }
    }

    fn messages(request: &PromptRequest) -> serde_json::Value {
        let user_content = format!(
            "VERDICT (already decided — do not change it):\n{verdict}\n\n\
             EVIDENCE (the only IDs you may cite):\n{evidence}\n\n\
             RESPOND WITH JSON MATCHING THIS SCHEMA, NOTHING ELSE — no markdown fences, \
             no preamble, no explanation outside the JSON object:\n{schema}{question}",
            verdict  = request.verdict_json,
            evidence = request.evidence_json,
            schema   = request.output_schema,
            question = request.question.as_deref()
                .map(|q| format!("\n\nFollow-up question about this same verdict: \"{q}\""))
                .unwrap_or_default(),
        );

        serde_json::json!([
            { "role": "system",  "content": request.system },
            { "role": "user",    "content": user_content   }
        ])
    }
}

// HF's OpenAI-compatible response shape (only the fields we need).
#[derive(Debug, Deserialize)]
struct HfChoice {
    message: HfMessage,
}
#[derive(Debug, Deserialize)]
struct HfMessage {
    content: String,
}
#[derive(Debug, Deserialize)]
struct HfChatResponse {
    choices: Vec<HfChoice>,
}

impl LlmProvider for HuggingFaceProvider {
    fn name(&self) -> &'static str {
        "huggingface"
    }

    fn complete(&self, request: &PromptRequest) -> Result<String, ReasoningError> {
        let body = serde_json::json!({
            "model":       self.model,
            "messages":    Self::messages(request),
            // Cap tokens: the output schema is tiny; 512 is more than
            // enough for verdict + reasoning + citations array.
            "max_tokens":  512,
            // temp=0 for deterministic, schema-faithful JSON output.
            "temperature": 0.0,
        });

        let response = self
            .client
            .post(HF_CHAT_URL)
            .bearer_auth(&self.api_token)
            .json(&body)
            .send()
            .map_err(|e| ReasoningError::ProviderUnavailable(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            // 429 = free-tier rate limit hit; 503 = model loading (cold
            // start). Both are transient — fall through to next provider.
            return Err(ReasoningError::ProviderUnavailable(format!(
                "HF Inference API returned {status}"
            )));
        }

        let parsed: HfChatResponse = response
            .json()
            .map_err(|e| ReasoningError::ProviderUnavailable(e.to_string()))?;

        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| {
                // Some models wrap the JSON in markdown fences even when
                // asked not to. Strip them so `pipeline::Synthesizer`'s
                // grounding check gets clean JSON.
                let raw = c.message.content.trim().to_string();
                if raw.starts_with("```") {
                    raw.lines()
                        .skip(1) // ```json or ```
                        .take_while(|l| !l.starts_with("```"))
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    raw
                }
            })
            .ok_or_else(|| ReasoningError::ProviderUnavailable("empty choices array".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_includes_verdict_evidence_and_schema() {
        let req = PromptRequest {
            system:        "persona".into(),
            verdict_json:  r#"{"headline":"X"}"#.into(),
            evidence_json: r#"[{"id":7}]"#.into(),
            output_schema: r#"{"type":"object"}"#.into(),
            question:      None,
            stricter:      false,
        };
        let msgs = HuggingFaceProvider::messages(&req).to_string();
        assert!(msgs.contains(r#"\"id\":7"#));
        assert!(msgs.contains("object"));
        assert!(msgs.contains("persona"));
    }

    #[test]
    fn reflection_question_is_appended() {
        let req = PromptRequest {
            system:        "p".into(),
            verdict_json:  "{}".into(),
            evidence_json: "[]".into(),
            output_schema: "{}".into(),
            question:      Some("why not Z?".into()),
            stricter:      false,
        };
        let msgs = HuggingFaceProvider::messages(&req).to_string();
        assert!(msgs.contains("why not Z?"));
    }
}
