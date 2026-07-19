//! Weekly Digest / "Weekly Planning" (06_AI_ENGINE.md §4.2).
//!
//! §4.2 is explicit this is not a mandatory 10-15 minute conversational
//! review — "there is no requirement that the user 'complete' the
//! weekly digest — it is a state of the Trajectory screen, not an event
//! the user must attend." This function phrases a rollup of the week's
//! already-computed Adaptive Planner verdicts
//! (`athena_domain::planner::replan`, one per day) — it introduces no
//! new scoring of its own, per §1's "every 'engine' named in this
//! document... is the same five-stage pipeline pointed at a different
//! retrieval query."

use athena_domain::planner::ReplanResult;

use crate::context;
use crate::output::Recommendation;
use crate::pipeline::Synthesizer;

/// `days` is `(day_label, ReplanResult)` for however many days of the
/// week the caller already has disruption/deadline data for — callers
/// with a partial week (e.g. mid-week) simply pass fewer entries; an
/// empty slice correctly yields `insufficient_data` rather than being a
/// special case here.
pub fn build_weekly_plan(
    synthesizer: &Synthesizer,
    days: &[(String, ReplanResult)],
    data_freshness_note: impl Into<String>,
) -> Recommendation {
    let payload = context::from_week_of_replans(days, data_freshness_note);
    synthesizer.synthesize(&payload, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use athena_domain::priority::{Confidence, Verdict};

    fn day(label: &str, id: i64) -> (String, ReplanResult) {
        (
            label.to_string(),
            ReplanResult {
                verdict: Verdict {
                    headline: format!("Work on: item {id}"),
                    reasoning: "highest leverage".into(),
                    confidence: Confidence::Inferred,
                    grounded_in_deadline_id: Some(id),
                    runners_up: vec![],
                },
                available_minutes_tonight: 120,
                substituted: false,
            },
        )
    }

    #[test]
    fn empty_week_is_honest_about_insufficient_data() {
        let synth = Synthesizer::new(vec![]);
        let rec = build_weekly_plan(&synth, &[], "as of 2026-07-17T09:00:00Z");
        assert_eq!(rec.confidence, "insufficient_data");
    }

    #[test]
    fn weekly_plan_grounds_in_each_days_pick() {
        let synth = Synthesizer::new(vec![]);
        let days = vec![day("monday", 1), day("tuesday", 2)];
        let rec = build_weekly_plan(&synth, &days, "as of 2026-07-17T09:00:00Z");
        assert_eq!(rec.grounded_in, vec![1, 2]);
    }
}
