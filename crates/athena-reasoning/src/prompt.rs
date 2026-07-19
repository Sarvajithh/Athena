//! Stage 3 — Prompt Construction (06_AI_ENGINE.md §7).
//!
//! This is the one place in the whole application allowed to assemble a
//! Stage 4 prompt. Nothing here is freeform string concatenation of
//! arbitrary retrieved text (§7's own ban): every `PromptRequest` is
//! built from exactly one `EvidencePayload` plus §8's fixed persona
//! text, and every prompt carries a JSON schema the model must satisfy.
//! `src/screens/**` and every other React surface must never construct
//! prompt text of its own — that would be exactly the kind of
//! ungrounded seam §7's last paragraph calls out by name ("use your
//! judgment about what else might be relevant... banned from every
//! prompt template as a matter of design review").

use crate::context::EvidencePayload;
use crate::provider::PromptRequest;

/// §8's tone constraint, verbatim, identical across every capability —
/// no capability gets its own persona.
const PERSONA: &str = "You are Athena's phrasing layer, not its decision layer. Every fact, ranking, \
    weakness, and confidence class below was already decided by deterministic code before you were \
    called. Your only job is to turn the verdict and evidence into one well-reasoned, well-formatted \
    piece of prose. Be direct and economical; respect the user's time. No performed enthusiasm, no \
    hedging a disagreement into mush, no moralizing, no nagging, and never soften a negative verdict \
    for comfort. Never introduce a fact, number, or claim that is not present in the verdict or evidence \
    JSON below — if you cannot support a sentence with an evidence ID, do not write that sentence.";

/// The fixed output shape every capability's Stage 4 response must
/// satisfy (§7.4, §11): a restated verdict, grounded reasoning
/// sentence(s), and citations by evidence ID — constrained output is
/// what makes Stage 5's grounding check mechanical.
const OUTPUT_SCHEMA: &str = r#"{
  "type": "object",
  "required": ["verdict", "reasoning", "citations"],
  "properties": {
    "verdict": { "type": "string", "description": "One-sentence restatement of the Stage 2 verdict headline." },
    "reasoning": { "type": "string", "description": "1-3 sentences of grounded reasoning, citing only IDs present in the evidence JSON." },
    "citations": { "type": "array", "items": { "type": "integer" }, "description": "Evidence IDs actually cited in `reasoning`." }
  }
}"#;

/// Centralizes Stage 3 for every capability in 06_AI_ENGINE.md §4.
/// Takes an `EvidencePayload` (already built by `context.rs` from an
/// `athena-domain` verdict) and an optional Reflection Engine question
/// (§4.7), and returns the one `PromptRequest` Stage 4 is allowed to
/// send.
pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build(payload: &EvidencePayload, question: Option<String>) -> PromptRequest {
        let verdict_json = serde_json::json!({
            "capability": payload.capability,
            "headline": payload.verdict_headline,
            "reasoning": payload.verdict_reasoning,
            "confidence": payload.confidence,
        })
        .to_string();

        let evidence_json = serde_json::to_string(&payload.evidence).unwrap_or_else(|_| "[]".to_string());

        PromptRequest {
            system: PERSONA.to_string(),
            verdict_json,
            evidence_json,
            output_schema: OUTPUT_SCHEMA.to_string(),
            question,
            stricter: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::EvidenceItem;

    #[test]
    fn build_serializes_verdict_and_evidence_as_json_not_free_text() {
        let payload = EvidencePayload {
            capability: "daily_briefing",
            verdict_headline: "Work on: X".into(),
            verdict_reasoning: "because Y".into(),
            confidence: "inferred",
            evidence: vec![EvidenceItem {
                id: 7,
                label: "top_priority_deadline".into(),
                value: "X".into(),
            }],
            data_freshness_note: "as of now".into(),
        };
        let request = PromptBuilder::build(&payload, None);
        assert!(request.verdict_json.contains("\"headline\":\"Work on: X\""));
        assert!(request.evidence_json.contains("\"id\":7"));
        assert!(!request.stricter);
        assert!(request.question.is_none());
    }

    #[test]
    fn reflection_question_is_carried_through_unmodified() {
        let payload = EvidencePayload {
            capability: "daily_briefing",
            verdict_headline: "Work on: X".into(),
            verdict_reasoning: "because Y".into(),
            confidence: "inferred",
            evidence: vec![],
            data_freshness_note: "as of now".into(),
        };
        let request = PromptBuilder::build(&payload, Some("why not Z instead?".into()));
        assert_eq!(request.question.as_deref(), Some("why not Z instead?"));
    }
}
