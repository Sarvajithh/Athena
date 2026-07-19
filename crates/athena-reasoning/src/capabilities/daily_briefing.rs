//! Daily Pass / "Daily Briefing" (06_AI_ENGINE.md §4.1).
//!
//! §4.1 is explicit that this is *not* a "morning briefing" the user
//! must open and read, and never a chat exchange — "there is no text
//! generated that says 'Good morning' — there is a verdict, updated or
//! not." What this function returns is exactly that: the Now screen's
//! verdict, phrased. The caller (`athena-app`'s daily scheduler tick,
//! `scheduler.rs`) decides whether anything changed enough to write a
//! new `recommendations` row or push a notification — this function
//! only phrases whatever verdict it's given.

use athena_domain::priority::Verdict;

use crate::context;
use crate::output::Recommendation;
use crate::pipeline::Synthesizer;

/// Re-runs no scoring of its own — `verdict` is the output of
/// `athena_domain::priority::resolve_priority` (or
/// `athena_domain::planner::replan`'s `.verdict`), computed by the
/// caller exactly as it already does for the Now screen. This function
/// exists only to turn that verdict into a grounded, phrased
/// `Recommendation` via the Stage 3–5 pipeline.
pub fn build_daily_briefing(
    synthesizer: &Synthesizer,
    verdict: &Verdict,
    data_freshness_note: impl Into<String>,
) -> Recommendation {
    let payload = context::from_priority_verdict(verdict, data_freshness_note);
    synthesizer.synthesize(&payload, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use athena_domain::priority::Confidence;

    #[test]
    fn daily_briefing_is_always_produced_even_with_no_providers_configured() {
        let synth = Synthesizer::new(vec![]);
        let verdict = Verdict {
            headline: "Work on: X".into(),
            reasoning: "highest leverage".into(),
            confidence: Confidence::Inferred,
            grounded_in_deadline_id: Some(1),
            runners_up: vec![],
        };
        let rec = build_daily_briefing(&synth, &verdict, "as of 2026-07-17T09:00:00Z");
        assert_eq!(rec.verdict, "Work on: X");
        assert_eq!(rec.grounded_in, vec![1]);
    }
}
