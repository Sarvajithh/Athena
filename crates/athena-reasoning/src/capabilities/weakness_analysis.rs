//! Weakness Analysis (06_AI_ENGINE.md §4.4) — "the load-bearing
//! constraint of this entire document": this is a *presentation* of
//! patterns that already cleared the Signal Threshold (§5), never a new
//! inference step, and never an LLM noticing something the user hasn't
//! evidenced in structured data. If `signals` is empty, this function
//! returns `insufficient_data` — it never invents a pattern to have
//! something to say.

use crate::context::{self, WeaknessSignal};
use crate::output::Recommendation;
use crate::pipeline::Synthesizer;

pub fn build_weakness_analysis(
    synthesizer: &Synthesizer,
    signals: &[WeaknessSignal],
    data_freshness_note: impl Into<String>,
) -> Recommendation {
    let payload = context::from_weakness_signals(signals, data_freshness_note);
    synthesizer.synthesize(&payload, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_qualifying_signals_yields_insufficient_data_not_a_guess() {
        let synth = Synthesizer::new(vec![]);
        let rec = build_weakness_analysis(&synth, &[], "as of 2026-07-17T09:00:00Z");
        assert_eq!(rec.confidence, "insufficient_data");
        assert!(rec.grounded_in.is_empty());
    }

    #[test]
    fn qualifying_signals_are_grounded_by_id() {
        let synth = Synthesizer::new(vec![]);
        let signals = vec![WeaknessSignal {
            id: 3,
            kind: "bottleneck",
            description: "Recurring late submissions on Tuesdays".into(),
            occurrences: 4,
        }];
        let rec = build_weakness_analysis(&synth, &signals, "as of 2026-07-17T09:00:00Z");
        assert_eq!(rec.grounded_in, vec![3]);
    }
}
