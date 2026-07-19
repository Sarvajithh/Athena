//! Context Engine (06_AI_ENGINE.md §2) — Stage 1 of the pipeline.
//!
//! This module does not query anything itself — `athena-data` remains
//! "the only crate allowed to write SQL," and every retrieval already
//! happens in a typed repository function the caller (`athena-app`) runs
//! before it gets here. What lives here is the *shaping* step: turning
//! whatever `athena-domain` verdict the caller already computed (Stage
//! 2's job, not this crate's) into the fixed-shape, freshness-stamped,
//! narrow `EvidencePayload` that Stage 3 is allowed to build a prompt
//! from (§2: "narrow by construction... a smaller payload means every
//! claim in the output is checkable against a small, enumerable set of
//! IDs").
//!
//! Every `EvidencePayload` below is built from a value `athena-domain`
//! already computed — `athena_domain::priority::Verdict`,
//! `athena_domain::planner::ReplanResult` — never from a raw repository
//! row. This is the concrete mechanism behind "the AI layer must consume
//! the Decision Engine instead of replacing it": `athena-reasoning` has
//! no scoring logic of its own anywhere in this crate.

use athena_domain::planner::ReplanResult;
use athena_domain::priority::{Confidence as DomainConfidence, Verdict};
use serde::Serialize;

/// One evidence row surfaced to Stage 3/4, carrying only what phrasing
/// requires (06_AI_ENGINE.md §9: "never raw identifiers beyond what
/// phrasing requires"). `id` is the stable ID Stage 5 checks citations
/// against; `label`/`value` are the narrow, already-decided facts the
/// LLM is allowed to reference.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EvidenceItem {
    pub id: i64,
    pub label: String,
    pub value: String,
}

/// The one fixed shape every Stage 3 prompt is built from (§2, §7). Never
/// constructed from an ORM-style "everything about the user" query — one
/// `EvidencePayload` corresponds to exactly one already-decided verdict.
#[derive(Debug, Clone, Serialize)]
pub struct EvidencePayload {
    /// Which capability this payload is for (`daily_briefing`,
    /// `weekly_planning`, `weakness_analysis`, `reflection`) — carried
    /// through so Stage 3's persona/schema selection and Stage 5's
    /// template fallback both know which of the four fixed prompt shapes
    /// to use, without re-deriving it from the payload's contents.
    pub capability: &'static str,
    /// Stage 2's typed verdict headline, restated verbatim — Stage 4
    /// phrases it, never replaces it (§6.1).
    pub verdict_headline: String,
    /// Stage 2's typed reasoning, restated verbatim, given to Stage 4 as
    /// structured input rather than left for the model to reconstruct.
    pub verdict_reasoning: String,
    /// One of `confirmed` / `inferred` / `insufficient_data` (§6) — never
    /// absent; a capability that can't compute this hasn't finished
    /// Stage 2 and this payload should not exist yet.
    pub confidence: &'static str,
    /// The narrow, enumerable evidence set Stage 5 checks citations
    /// against (§2).
    pub evidence: Vec<EvidenceItem>,
    /// §2's freshness stamp, carried as prose so Stage 4 can honestly
    /// say "as of X" — never implying live data the payload doesn't
    /// actually have.
    pub data_freshness_note: String,
}

fn confidence_label(confidence: &DomainConfidence) -> &'static str {
    match confidence {
        DomainConfidence::Inferred => "inferred",
        DomainConfidence::InsufficientData => "insufficient_data",
    }
}

/// Builds the Daily Briefing's payload (06_AI_ENGINE.md §4.1) directly
/// from `athena_domain::priority::resolve_priority`'s output — the Daily
/// Pass re-runs Priority Resolution; this only shapes what it already
/// decided, it never re-derives the ranking.
pub fn from_priority_verdict(verdict: &Verdict, data_freshness_note: impl Into<String>) -> EvidencePayload {
    let mut evidence = Vec::new();
    if let Some(id) = verdict.grounded_in_deadline_id {
        evidence.push(EvidenceItem {
            id,
            label: "top_priority_deadline".to_string(),
            value: verdict.headline.clone(),
        });
    }
    for runner_up in &verdict.runners_up {
        evidence.push(EvidenceItem {
            id: runner_up.id,
            label: "runner_up_deadline".to_string(),
            value: runner_up.headline.clone(),
        });
    }

    EvidencePayload {
        capability: "daily_briefing",
        verdict_headline: verdict.headline.clone(),
        verdict_reasoning: verdict.reasoning.clone(),
        confidence: confidence_label(&verdict.confidence),
        evidence,
        data_freshness_note: data_freshness_note.into(),
    }
}

/// Builds the Weekly Digest's payload (06_AI_ENGINE.md §4.2) from the
/// week's `ReplanResult`s — one per day the Adaptive Planner recomputed
/// against (`athena_domain::planner::replan`), already labeled with the
/// day it belongs to by the caller. Weekly Planning is explicitly the
/// Adaptive Planner's own weekly view, not a second scoring pass: every
/// number here is a verdict `athena-domain` already produced.
pub fn from_week_of_replans(
    days: &[(String, ReplanResult)],
    data_freshness_note: impl Into<String>,
) -> EvidencePayload {
    let mut evidence = Vec::new();
    let mut substitution_days = 0usize;
    for (day_label, result) in days {
        if let Some(id) = result.verdict.grounded_in_deadline_id {
            evidence.push(EvidenceItem {
                id,
                label: format!("{day_label}_pick"),
                value: result.verdict.headline.clone(),
            });
        }
        if result.substituted {
            substitution_days += 1;
        }
    }

    let overall_confidence = if days.is_empty() {
        DomainConfidence::InsufficientData
    } else {
        DomainConfidence::Inferred
    };

    let headline = if days.is_empty() {
        "Not enough data yet for a weekly plan.".to_string()
    } else {
        format!(
            "{} day(s) planned this week, {} replanned after a logged disruption.",
            days.len(),
            substitution_days
        )
    };

    let reasoning = if days.is_empty() {
        "No days in this window had an open deadline to plan against yet.".to_string()
    } else {
        format!(
            "Each day's pick below is the same Priority Resolution / Adaptive Planner verdict already \
             computed for that day (09_DECISION_ENGINE.md, 08_ADAPTIVE_PLANNER.md) — this is a weekly \
             rollup of {} already-decided verdicts, not a new ranking.",
            days.len()
        )
    };

    EvidencePayload {
        capability: "weekly_planning",
        verdict_headline: headline,
        verdict_reasoning: reasoning,
        confidence: confidence_label(&overall_confidence),
        evidence,
        data_freshness_note: data_freshness_note.into(),
    }
}

/// A single already-graduated weakness pattern — i.e. a row that has
/// already cleared the Signal Threshold (06_AI_ENGINE.md §5) in
/// `drift_signals`/`bottlenecks`. **Honest gap, same precedent
/// `athena_domain::planner`'s own module doc already documents for the
/// identical tables:** neither table exists in this schema yet
/// (`crates/athena-data/migrations/` stops at `V5__oauth_connectors.sql`),
/// so there is no repository to call here. This struct is the typed
/// shape Weakness Analysis is specified to consume (§4.4: "a
/// *presentation* of already-computed signals, not a new inference
/// step") — wiring it to a real `athena_data::repositories::drift`/
/// `bottleneck` call is a schema/repository change (Immutable Rule #7),
/// out of scope here, and the call site (`athena-app`) is expected to
/// pass an empty slice until that lands, which correctly yields
/// `insufficient_data` below rather than fabricating a pattern.
#[derive(Debug, Clone)]
pub struct WeaknessSignal {
    pub id: i64,
    /// `drift` or `bottleneck` — mirrors the two source tables §5 names.
    pub kind: &'static str,
    /// The already-decided, factual description of the pattern —
    /// Stage 4 phrases this, it does not invent it (§4.4's "explicitly
    /// not... an LLM noticing a psychological pattern").
    pub description: String,
    pub occurrences: i64,
}

/// Builds the Weakness Analysis payload (§4.4) purely as a presentation
/// of signals that already cleared the Signal Threshold — see
/// `WeaknessSignal`'s doc comment for why the input is a caller-supplied
/// slice rather than a repository call.
pub fn from_weakness_signals(
    signals: &[WeaknessSignal],
    data_freshness_note: impl Into<String>,
) -> EvidencePayload {
    let evidence: Vec<EvidenceItem> = signals
        .iter()
        .map(|s| EvidenceItem {
            id: s.id,
            label: s.kind.to_string(),
            value: format!("{} (seen {}x)", s.description, s.occurrences),
        })
        .collect();

    let (headline, reasoning, confidence) = if signals.is_empty() {
        (
            "No recurring pattern has cleared the signal threshold yet.".to_string(),
            "Weakness Analysis only names a pattern once it has already been evidenced as recurring \
             (06_AI_ENGINE.md §5) — nothing has qualified yet, which is expected early in a semester."
                .to_string(),
            DomainConfidence::InsufficientData,
        )
    } else {
        (
            format!("{} recurring pattern(s) worth reviewing.", signals.len()),
            "Every pattern below already cleared the Signal Threshold before this pass ran — this is a \
             factual presentation of already-computed signals, not a new judgment."
                .to_string(),
            DomainConfidence::Inferred,
        )
    };

    EvidencePayload {
        capability: "weakness_analysis",
        verdict_headline: headline,
        verdict_reasoning: reasoning,
        confidence: confidence_label(&confidence),
        evidence,
        data_freshness_note: data_freshness_note.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use athena_domain::priority::{RankedCandidate, Verdict};

    #[test]
    fn priority_verdict_carries_grounded_id_into_evidence() {
        let verdict = Verdict {
            headline: "Work on: X".into(),
            reasoning: "because Y".into(),
            confidence: DomainConfidence::Inferred,
            grounded_in_deadline_id: Some(7),
            runners_up: vec![RankedCandidate {
                id: 8,
                headline: "Also close: Z".into(),
                reasoning: "tied".into(),
            }],
        };
        let payload = from_priority_verdict(&verdict, "as of 2026-07-17T09:00:00Z");
        assert_eq!(payload.confidence, "inferred");
        assert_eq!(payload.evidence.len(), 2);
        assert!(payload.evidence.iter().any(|e| e.id == 7));
        assert!(payload.evidence.iter().any(|e| e.id == 8));
    }

    #[test]
    fn empty_week_yields_insufficient_data() {
        let payload = from_week_of_replans(&[], "as of 2026-07-17T09:00:00Z");
        assert_eq!(payload.confidence, "insufficient_data");
        assert!(payload.evidence.is_empty());
    }

    #[test]
    fn empty_weakness_signals_yield_insufficient_data_not_a_fabricated_pattern() {
        let payload = from_weakness_signals(&[], "as of 2026-07-17T09:00:00Z");
        assert_eq!(payload.confidence, "insufficient_data");
        assert!(payload.evidence.is_empty());
    }

    #[test]
    fn weakness_signals_carry_through_as_evidence() {
        let signals = vec![WeaknessSignal {
            id: 1,
            kind: "drift",
            description: "DSA practice sessions shortening".into(),
            occurrences: 3,
        }];
        let payload = from_weakness_signals(&signals, "as of 2026-07-17T09:00:00Z");
        assert_eq!(payload.confidence, "inferred");
        assert_eq!(payload.evidence.len(), 1);
        assert_eq!(payload.evidence[0].id, 1);
    }
}
