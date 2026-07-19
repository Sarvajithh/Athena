//! §11's mandatory output shape. There is no code path anywhere in this
//! crate that hands a bare `String` to a caller — every capability
//! (`capabilities/*.rs`) returns exactly this struct, whether the
//! sentence inside it came from the cloud provider, the local provider,
//! or the zero-LLM template.

use serde::Serialize;

use crate::context::EvidencePayload;

#[derive(Debug, Clone, Serialize)]
pub struct Recommendation {
    /// Stage 2's typed decision, restated (§11).
    pub verdict: String,
    /// Stage 4's grounded, cited sentence(s) — or, on full fallback, the
    /// Stage 2 verdict rendered directly as a template sentence (§10.2).
    pub reasoning: String,
    /// One of `confirmed` / `inferred` / `insufficient_data` (§6), never
    /// null.
    pub confidence: String,
    /// The evidence row IDs the reasoning actually cites, checkable
    /// against the Stage 1 payload (§11).
    pub grounded_in: Vec<i64>,
    /// §2's freshness stamp, carried through unchanged from the payload.
    pub data_freshness_note: String,
    /// Provenance: the provider name that produced `reasoning`, or
    /// `"template"` when no LLM was involved (§10.2). Never affects
    /// correctness — `verdict`/`grounded_in` are identical either way —
    /// but lets the UI show "phrased by Claude" vs "no AI phrasing
    /// available right now" without a second round trip.
    pub source: String,
}

impl Recommendation {
    /// §10.2's worst-case, always-available fallback: Stage 2's typed
    /// verdict rendered directly as a template sentence, zero LLM
    /// involvement, fully grounded by construction (it cites exactly the
    /// evidence IDs the payload already has).
    pub fn from_template(payload: &EvidencePayload) -> Recommendation {
        Recommendation {
            verdict: payload.verdict_headline.clone(),
            reasoning: payload.verdict_reasoning.clone(),
            confidence: payload.confidence.to_string(),
            grounded_in: payload.evidence.iter().map(|e| e.id).collect(),
            data_freshness_note: payload.data_freshness_note.clone(),
            source: "template".to_string(),
        }
    }

    /// Stage 4 succeeded and Stage 5's grounding check passed: the
    /// model's `reasoning`/`citations` replace the template sentence,
    /// but `verdict`/`confidence`/`data_freshness_note` are still taken
    /// from the payload — the LLM never gets to alter those (§6.1).
    pub fn from_synthesis(
        payload: &EvidencePayload,
        reasoning: String,
        citations: Vec<i64>,
        provider_name: &str,
    ) -> Recommendation {
        Recommendation {
            verdict: payload.verdict_headline.clone(),
            reasoning,
            confidence: payload.confidence.to_string(),
            grounded_in: citations,
            data_freshness_note: payload.data_freshness_note.clone(),
            source: provider_name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::EvidenceItem;

    fn payload() -> EvidencePayload {
        EvidencePayload {
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
        }
    }

    #[test]
    fn template_fallback_is_fully_grounded_by_construction() {
        let rec = Recommendation::from_template(&payload());
        assert_eq!(rec.source, "template");
        assert_eq!(rec.grounded_in, vec![7]);
        assert_eq!(rec.verdict, "Work on: X");
    }

    #[test]
    fn synthesis_keeps_verdict_and_confidence_from_payload_not_the_model() {
        let rec = Recommendation::from_synthesis(&payload(), "Do X because of evidence 7.".into(), vec![7], "claude-sonnet");
        assert_eq!(rec.verdict, "Work on: X");
        assert_eq!(rec.confidence, "inferred");
        assert_eq!(rec.source, "claude-sonnet");
    }
}
