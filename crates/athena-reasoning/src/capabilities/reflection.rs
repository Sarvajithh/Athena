//! Reflection Engine (06_AI_ENGINE.md §4.7) — the "why?" follow-up mode.
//!
//! §4.7 is precise about the shape this takes: "re-runs Stage 4 with the
//! *same* Stage 2/3 payload plus the user's question, never a new
//! retrieval or a new inference path." Concretely, that means this
//! function takes the exact `EvidencePayload` an earlier capability call
//! already built (the caller — `athena-app` — holds onto it after
//! calling e.g. `daily_briefing::build_daily_briefing`) and re-runs the
//! identical pipeline with `question` attached, rather than retrieving
//! anything new or accepting a fresh verdict.
//!
//! §4.7's governing constraint: "secondary and optional, never
//! load-bearing... it exists only as a way to ask 'why does the Now
//! verdict say this' and get the same grounded reasoning, elaborated —
//! not as a general-purpose chat window." There is deliberately no
//! conversation history parameter here — every call is independent, re-
//! grounded against the same fixed payload, never accumulating state
//! across turns (§4.7's explicit rejection of "persistent conversational
//! memory... a chat history the system 'remembers' across sessions").
//!
//! **v1 status:** §4.7 marks this Future Feature-deferred ("specified
//! here so a future session builds it correctly when it's prioritized,
//! not so it ships in v1"). This module exists so that future session
//! has a correct, already-wired implementation to build the UI surface
//! against — `athena-app` is not required to expose an IPC command for
//! it yet.

use crate::context::EvidencePayload;
use crate::output::Recommendation;
use crate::pipeline::Synthesizer;

/// Re-runs Stage 4 against the same `payload` an earlier capability call
/// produced, with `question` appended — same grounding rules apply, so
/// an answer that would need evidence outside `payload.evidence` simply
/// fails Stage 5 and falls through to the template restatement, exactly
/// like any other call.
pub fn reflect(synthesizer: &Synthesizer, payload: &EvidencePayload, question: String) -> Recommendation {
    synthesizer.synthesize(payload, Some(question))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::EvidenceItem;

    #[test]
    fn reflection_without_any_provider_still_returns_a_grounded_template_answer() {
        let synth = Synthesizer::new(vec![]);
        let payload = EvidencePayload {
            capability: "daily_briefing",
            verdict_headline: "Work on: X".into(),
            verdict_reasoning: "because Y".into(),
            confidence: "inferred",
            evidence: vec![EvidenceItem {
                id: 1,
                label: "top_priority_deadline".into(),
                value: "X".into(),
            }],
            data_freshness_note: "as of now".into(),
        };
        let rec = reflect(&synth, &payload, "why not the other one?".into());
        assert_eq!(rec.source, "template");
        assert_eq!(rec.grounded_in, vec![1]);
    }
}
