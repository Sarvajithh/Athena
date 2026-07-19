//! Stage 4's provider trait boundary (06_AI_ENGINE.md §9).
//!
//! `LlmProvider` is deliberately the only thing Stage 4/5 code
//! (`pipeline.rs`) knows about — never a concrete client. §9 spells out
//! why: "a single-vendor, unabstracted client hard-coded into the
//! synthesis module is a real 5-year risk... the trait costs one
//! interface definition now and is materially more expensive to
//! retrofit once multiple call sites have coupled to a concrete
//! client." `providers::cloud::AnthropicProvider` and
//! `providers::local::OllamaProvider` are the two real implementations.
//! §10.2's zero-LLM template fallback is deliberately *not* a third
//! `LlmProvider` impl — it never makes a call, never fails, and always
//! has an answer, so it lives directly in
//! `output::Recommendation::from_template` and is
//! `pipeline::Synthesizer`'s built-in last step rather than something
//! dispatched through this trait.

use serde::Serialize;

use crate::error::ReasoningError;

/// The one fixed shape every Stage 3 prompt is assembled into (§7):
/// system/persona block, the Stage 2 verdict as structured JSON, the
/// evidence block, and the output schema the model must satisfy. No
/// field here is ever freeform string concatenation of arbitrary
/// retrieved text — `prompt::PromptBuilder` is the only place that
/// constructs one of these, from an `EvidencePayload` alone.
#[derive(Debug, Clone, Serialize)]
pub struct PromptRequest {
    /// §8's tone constraint, identical across every capability.
    pub system: String,
    /// The Stage 2 verdict, serialized as JSON (§7.2).
    pub verdict_json: String,
    /// The Stage 1 evidence set, serialized as JSON (§7.3) — the only
    /// IDs the model is allowed to cite.
    pub evidence_json: String,
    /// The JSON schema the response must satisfy (§7.4).
    pub output_schema: String,
    /// Set only by the Reflection Engine (§4.7): the user's "why?"
    /// question, appended to the *same* Stage 2/3 payload — never a new
    /// retrieval, never a new inference path.
    pub question: Option<String>,
    /// Set only on Stage 5's single retry after a grounding-check
    /// failure — a stricter restatement of the citation requirement,
    /// never a relaxation of it (§3: "retry once with a stricter
    /// prompt").
    pub stricter: bool,
}

impl PromptRequest {
    /// Stage 5's single retry: same payload, an explicit reminder that
    /// every claim must cite an ID present in `evidence_json` and
    /// nothing else. Never loosens the schema — a second failure falls
    /// through to the template, per §10.2, rather than trying a third
    /// time with a laxer prompt.
    pub fn stricter_retry(&self) -> PromptRequest {
        PromptRequest {
            system: format!(
                "{} STRICT MODE: your previous response cited something outside the evidence block. \
                 Every `citations` value must be an `id` that appears in the evidence JSON below, and \
                 every sentence in `reasoning` must be traceable to one of those IDs. If you cannot \
                 support a claim this way, omit it.",
                self.system
            ),
            verdict_json: self.verdict_json.clone(),
            evidence_json: self.evidence_json.clone(),
            output_schema: self.output_schema.clone(),
            question: self.question.clone(),
            stricter: true,
        }
    }
}

/// Stage 4's trait boundary. Implementations are synchronous/blocking
/// (see `Cargo.toml`'s dependency comment for why) and return a raw JSON
/// string matching `PromptRequest::output_schema` — `pipeline.rs` is
/// responsible for parsing and grounding-checking it, not the provider.
pub trait LlmProvider {
    /// A short, stable identifier carried into `Recommendation::source`
    /// (e.g. `"claude-sonnet"`, `"ollama:llama3"`) so a rendered
    /// verdict's provenance is always inspectable.
    fn name(&self) -> &'static str;

    /// Performs one Stage 4 call. Returns the raw JSON response text on
    /// success; any transport, auth, or timeout failure is
    /// `ReasoningError::ProviderUnavailable` so `pipeline::Synthesizer`
    /// can move on to the next provider without treating it as fatal.
    fn complete(&self, request: &PromptRequest) -> Result<String, ReasoningError>;
}
