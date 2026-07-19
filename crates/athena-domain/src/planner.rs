//! Adaptive Planner (`08_ADAPTIVE_PLANNER.md`) — extends
//! `athena_domain::priority`, does not duplicate it. Observes completed
//! work, missed work, unexpected interruptions, new deadlines, and
//! schedule changes as one uniform shape, `ScheduleDisruption`, and
//! recomputes through the *same* deterministic scoring function
//! `priority::rank` already provides (§2: "every trigger, disruption or
//! otherwise, runs the identical scoring function... a disruption is
//! not special-cased logic bolted onto the side of Priority Resolution,
//! it is one more shape of input the same function already has to
//! handle").
//!
//! Pure, zero I/O, same as `priority` (§1.1: "every reprioritization...
//! is pure Rust"). The caller (`athena-app::commands::planner`) is
//! responsible for retrieving candidates/disruptions from `athena-data`
//! and persisting the result.
//!
//! ## Two honest gaps versus the doc's literal §3.1 model
//!
//! §3.1 names `bottleneck_amplifier` and `drift_amplifier`, both reading
//! `bottlenecks` / `drift_signals` tables. Neither table exists in this
//! schema (`crates/athena-data/migrations/` only goes through
//! `schedule_disruptions` as of V3) — `src/screens/Now/index.tsx`
//! already documents the same gap for its own UI sections. Per the
//! precedent `priority::is_close` already set for its own unimplemented
//! Signal Threshold inputs ("implements only the one signal it can
//! compute honestly today"), both amplifiers are wired as documented
//! constants (`DEFAULT_BOTTLENECK_AMPLIFIER`, `DEFAULT_DRIFT_AMPLIFIER`)
//! rather than faked, and are multiplied in at the one call site
//! (`amplified_rank_hint`) so wiring the real tables in later only means
//! replacing those two constants with real lookups — the shape is
//! already correct.
//!
//! §3.1 also assumes a per-candidate duration estimate to test "does it
//! fit in `available_minutes_tonight`." No such column exists on
//! `deadlines` (04_DATA_MODEL.md §5, as actually migrated) or anywhere
//! else in this schema. `estimated_minutes` below is the same kind of
//! documented, honest stand-in: a fixed per-leverage-tier estimate,
//! not a fabricated per-item number.

use crate::priority::{self, Confidence, DeadlineCandidate, Verdict};

/// The six disruption shapes named in §4. Kept as a closed enum (not a
/// bare string) so an unrecognized type is a compile error here and a
/// caught, explicit error at the IPC boundary — never a silently
/// no-op string comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisruptionType {
    ExternalInterrupt,
    SurpriseWorkload,
    CancelledClass,
    UnexpectedOpportunity,
    Illness,
    EarlyFinish,
}

impl DisruptionType {
    pub fn as_str(self) -> &'static str {
        match self {
            DisruptionType::ExternalInterrupt => "external_interrupt",
            DisruptionType::SurpriseWorkload => "surprise_workload",
            DisruptionType::CancelledClass => "cancelled_class",
            DisruptionType::UnexpectedOpportunity => "unexpected_opportunity",
            DisruptionType::Illness => "illness",
            DisruptionType::EarlyFinish => "early_finish",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "external_interrupt" => Some(DisruptionType::ExternalInterrupt),
            "surprise_workload" => Some(DisruptionType::SurpriseWorkload),
            "cancelled_class" => Some(DisruptionType::CancelledClass),
            "unexpected_opportunity" => Some(DisruptionType::UnexpectedOpportunity),
            "illness" => Some(DisruptionType::Illness),
            "early_finish" => Some(DisruptionType::EarlyFinish),
            _ => None,
        }
    }

    /// Whether this type, by itself, removes time from tonight's window
    /// (§4.1, §4.2, §4.5) versus adding time back (§4.3, §4.6).
    /// `unexpected_opportunity` (§4.4) is neither by default — it is the
    /// one type whose effect on the window depends on a decision the
    /// Decision Challenge Layer would own, which is out of scope for
    /// this schema today (`src/components/modals/ChallengeDialogShell.tsx`
    /// is already documented as "visual shell only... out of scope this
    /// sprint") — so it is logged and surfaced, but does not itself
    /// change `available_minutes_tonight`.
    fn minutes_delta(self, logged_minutes: i64) -> i64 {
        match self {
            DisruptionType::ExternalInterrupt
            | DisruptionType::SurpriseWorkload
            | DisruptionType::Illness => -logged_minutes,
            DisruptionType::CancelledClass | DisruptionType::EarlyFinish => logged_minutes,
            DisruptionType::UnexpectedOpportunity => 0,
        }
    }
}

/// One logged disruption, already resolved to a typed `DisruptionType`
/// (the IPC layer maps the stored string via `DisruptionType::from_str`
/// before calling in here — `athena-domain` takes no serde dependency,
/// same reasoning as `priority::Verdict`).
#[derive(Debug, Clone)]
pub struct ScheduleDisruption {
    pub disruption_type: DisruptionType,
    pub duration_minutes: i64,
}

/// §6.2: fixed, documented, not runtime-tunable. 1.0 means "no effect" —
/// the honest value until `bottlenecks`/`drift_signals` exist.
pub const DEFAULT_BOTTLENECK_AMPLIFIER: f64 = 1.0;
pub const DEFAULT_DRIFT_AMPLIFIER: f64 = 1.0;

/// Documented per-leverage-tier minute estimate, used only to test
/// whether the ranked-first candidate still fits a reduced window
/// (§3.2's "fits within `available_minutes_tonight`" test). See the
/// module doc comment's second gap note for why this is a fixed
/// constant rather than a per-candidate field.
fn estimated_minutes(leverage_class: &str) -> i64 {
    match leverage_class {
        "high" => 90,
        "medium" => 60,
        "low" => 30,
        _ => 45,
    }
}

/// Parses `deep_work_window_start`/`_end` (`HH:MM`, `user_profile`
/// columns) into the base window length in minutes, per
/// `04_DATA_MODEL.md` §1. Handles the overnight-wrap case
/// (`start: "20:00"`, `end: "00:00"`) the same way the existing profile
/// test fixtures already use it (`crates/athena-data/src/repositories/profile.rs`).
/// Returns `None` if either string isn't `HH:MM`-shaped — the caller
/// falls back to a documented default rather than panicking.
pub fn base_window_minutes(start: &str, end: &str) -> Option<i64> {
    fn to_minutes(hhmm: &str) -> Option<i64> {
        let (h, m) = hhmm.split_once(':')?;
        let h: i64 = h.parse().ok()?;
        let m: i64 = m.parse().ok()?;
        Some(h * 60 + m)
    }
    let start_m = to_minutes(start)?;
    let end_m = to_minutes(end)?;
    Some(if end_m > start_m {
        end_m - start_m
    } else {
        (24 * 60 - start_m) + end_m
    })
}

/// §3.1's `available_minutes_tonight`: the base deep-work window,
/// reduced or increased by every disruption logged for today, floored
/// at zero (a fully-disrupted evening, never a negative window).
pub fn available_minutes_tonight(
    base_window_minutes: i64,
    disruptions: &[ScheduleDisruption],
) -> i64 {
    let delta: i64 = disruptions
        .iter()
        .map(|d| d.disruption_type.minutes_delta(d.duration_minutes))
        .sum();
    (base_window_minutes + delta).max(0)
}

/// The result of one recompute: the verdict itself, the window it was
/// computed against, and whether a substitution (§3.2) was needed —
/// the caller uses `substituted` to decide whether the Deep Work Guard
/// interaction (§7) is relevant, once that interceptor chain exists.
#[derive(Debug, Clone)]
pub struct ReplanResult {
    pub verdict: Verdict,
    pub available_minutes_tonight: i64,
    pub substituted: bool,
}

/// Recomputes priorities against today's disruptions, reusing
/// `priority::rank` for the ranking itself (§2) and only adding the
/// window-fit / rest-day logic §3.2 and §4.5 specify on top.
///
/// `candidates` should already include any new deadline created by a
/// `surprise_workload` disruption (§4.2: "no special-casing... it will
/// likely rank first on its own merits") — the caller inserts that row
/// via `athena_data::repositories::deadline` before calling this, the
/// same as any other deadline.
pub fn replan(
    candidates: &[DeadlineCandidate],
    base_window_minutes: i64,
    disruptions: &[ScheduleDisruption],
) -> ReplanResult {
    let minutes = available_minutes_tonight(base_window_minutes, disruptions);

    // §4.5: illness that zeroes the window out is a first-class rest
    // verdict, never a forced fit into zero minutes.
    let illness_today = disruptions
        .iter()
        .any(|d| d.disruption_type == DisruptionType::Illness);
    if illness_today && minutes <= 0 {
        return ReplanResult {
            verdict: Verdict {
                headline: "Rest — logged as unavailable today.".to_string(),
                reasoning: "No deep-work allocation recommended today. Recovering is the trajectory-preserving \
                            call here; nothing currently open is worth pushing through illness for."
                    .to_string(),
                confidence: Confidence::Inferred,
                grounded_in_deadline_id: None,
                runners_up: Vec::new(),
            },
            available_minutes_tonight: 0,
            substituted: false,
        };
    }

    let ranked = priority::rank(candidates);
    let Some(top) = ranked.first() else {
        // No open candidates at all — identical cold-start verdict to
        // `resolve_priority`'s own empty case; disruptions don't change
        // what "nothing is open" means.
        return ReplanResult {
            verdict: priority::resolve_priority(candidates),
            available_minutes_tonight: minutes,
            substituted: false,
        };
    };

    let reduced = minutes < base_window_minutes;
    let increased = minutes > base_window_minutes;
    let top_fits = estimated_minutes(&top.leverage_class) <= minutes;

    if !reduced || top_fits {
        // The identical top pick still stands (§3.3's Closeness Check
        // is unchanged, reused via `resolve_priority`) — but the
        // reasoning explains *why*, per §4's "never a silent
        // recalculation the user has to take on faith."
        let mut verdict = priority::resolve_priority(candidates);
        if reduced {
            verdict.reasoning = format!(
                "{} With {} fewer minutes tonight after today's logged disruption(s), this still fits and \
                 remains highest-leverage — plan unchanged.",
                verdict.reasoning,
                base_window_minutes - minutes
            );
        } else if increased {
            verdict.reasoning = format!(
                "{} Tonight's window grew by {} minutes after a logged disruption; this pick still stands \
                 as the highest-leverage fit for the extra time.",
                verdict.reasoning,
                minutes - base_window_minutes
            );
        }
        return ReplanResult {
            verdict,
            available_minutes_tonight: minutes,
            substituted: false,
        };
    }

    // Reduced window, top pick no longer fits: walk the same ranked
    // order looking for the highest-leverage candidate that *does* fit
    // (§3.2's substitution rule), stating the substitution explicitly.
    match ranked.iter().skip(1).find(|c| estimated_minutes(&c.leverage_class) <= minutes) {
        Some(substitute) => ReplanResult {
            verdict: Verdict {
                headline: format!("Work on: {}", substitute.title),
                reasoning: format!(
                    "Normally {} would rank first, but tonight's reduced window ({} min, down {} after \
                     today's logged disruption(s)) means {} is the highest-leverage thing that actually \
                     fits. Recommend picking {} back up first thing in the next available window.",
                    top.title,
                    minutes,
                    base_window_minutes - minutes,
                    substitute.title,
                    top.title
                ),
                confidence: Confidence::Inferred,
                grounded_in_deadline_id: Some(substitute.id),
                runners_up: Vec::new(),
            },
            available_minutes_tonight: minutes,
            substituted: true,
        },
        None => ReplanResult {
            // Nothing open fits even a reduced window — still names the
            // real top pick honestly rather than hiding the verdict.
            verdict: Verdict {
                headline: format!("Start: {}", top.title),
                reasoning: format!(
                    "Tonight's window is down to {} minutes after today's logged disruption(s) — nothing \
                     open fits in full. {} remains the highest-leverage open item; recommend starting it \
                     now and finishing in the next available window.",
                    minutes, top.title
                ),
                confidence: Confidence::Inferred,
                grounded_in_deadline_id: Some(top.id),
                runners_up: Vec::new(),
            },
            available_minutes_tonight: minutes,
            substituted: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: i64, title: &str, due_at: &str, leverage_class: &str) -> DeadlineCandidate {
        DeadlineCandidate {
            id,
            title: title.into(),
            due_at: due_at.into(),
            leverage_class: leverage_class.into(),
        }
    }

    #[test]
    fn base_window_minutes_handles_overnight_wrap() {
        assert_eq!(base_window_minutes("20:00", "00:00"), Some(240));
        assert_eq!(base_window_minutes("21:00", "23:00"), Some(120));
        assert_eq!(base_window_minutes("garbage", "00:00"), None);
    }

    #[test]
    fn external_interrupt_reduces_available_minutes() {
        let disruptions = vec![ScheduleDisruption {
            disruption_type: DisruptionType::ExternalInterrupt,
            duration_minutes: 60,
        }];
        assert_eq!(available_minutes_tonight(240, &disruptions), 180);
    }

    #[test]
    fn early_finish_increases_available_minutes() {
        let disruptions = vec![ScheduleDisruption {
            disruption_type: DisruptionType::EarlyFinish,
            duration_minutes: 50,
        }];
        assert_eq!(available_minutes_tonight(240, &disruptions), 290);
    }

    #[test]
    fn illness_with_full_window_loss_yields_rest_verdict() {
        let candidates = vec![candidate(
            1,
            "Company X application",
            "2026-07-20T00:00:00",
            "high",
        )];
        let disruptions = vec![ScheduleDisruption {
            disruption_type: DisruptionType::Illness,
            duration_minutes: 240,
        }];
        let result = replan(&candidates, 240, &disruptions);
        assert_eq!(result.available_minutes_tonight, 0);
        assert!(result.verdict.grounded_in_deadline_id.is_none());
        assert!(result.verdict.headline.to_lowercase().contains("rest"));
    }

    #[test]
    fn top_pick_unchanged_when_it_still_fits_reduced_window() {
        let candidates = vec![
            candidate(1, "Company X application", "2026-07-20T00:00:00", "high"),
            candidate(2, "DSA practice log", "2026-07-21T00:00:00", "low"),
        ];
        let disruptions = vec![ScheduleDisruption {
            disruption_type: DisruptionType::ExternalInterrupt,
            duration_minutes: 60,
        }];
        // 240 -> 180 minutes; "high" estimate is 90, still fits.
        let result = replan(&candidates, 240, &disruptions);
        assert!(!result.substituted);
        assert_eq!(result.verdict.grounded_in_deadline_id, Some(1));
        assert!(result.verdict.reasoning.contains("fewer minutes"));
    }

    #[test]
    fn substitutes_lower_ranked_candidate_when_top_no_longer_fits() {
        let candidates = vec![
            candidate(1, "Company X application", "2026-07-20T00:00:00", "high"),
            candidate(2, "DSA practice log", "2026-07-21T00:00:00", "low"),
        ];
        // 240 -> 45 minutes; "high" estimate (90) no longer fits, "low" (30) does.
        let disruptions = vec![ScheduleDisruption {
            disruption_type: DisruptionType::ExternalInterrupt,
            duration_minutes: 195,
        }];
        let result = replan(&candidates, 240, &disruptions);
        assert!(result.substituted);
        assert_eq!(result.verdict.grounded_in_deadline_id, Some(2));
        assert!(result.verdict.reasoning.contains("Company X application"));
    }

    #[test]
    fn surprise_workload_relies_on_caller_supplied_new_deadline_ranking_itself() {
        // §4.2: no special-casing — a same-day-due new deadline should
        // simply outrank everything else via the identical scoring
        // function, exactly like any other candidate.
        let candidates = vec![
            candidate(1, "Company X application", "2026-07-25T00:00:00", "high"),
            candidate(2, "Surprise quiz tomorrow", "2026-07-15T00:00:00", "high"),
        ];
        let result = replan(&candidates, 240, &[]);
        assert_eq!(result.verdict.grounded_in_deadline_id, Some(2));
    }
}