//! Minimal deterministic priority pick over open deadlines.
//!
//! Pure, zero I/O (01_ARCHITECTURE.md §1.3): every input is a value the
//! caller already retrieved from `athena-data`; nothing here touches
//! SQL, the network, or an LLM.

/// One candidate the caller retrieved from `deadlines` (04_DATA_MODEL.md
/// §5). `due_at` is an ISO-8601-ish string; candidates are expected to
/// already be sorted ascending by `due_at` by the caller's query
/// (`athena-data::repositories::deadline::list_open`), so this module
/// only needs to break ties on `leverage_class`.
#[derive(Debug, Clone)]
pub struct DeadlineCandidate {
    pub id: i64,
    pub title: String,
    pub due_at: String,
    pub leverage_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confidence {
    Inferred,
    InsufficientData,
}

/// One runner-up surfaced alongside the primary verdict when the
/// Closeness Threshold (09_DECISION_ENGINE.md §4) determines the top
/// candidates are genuinely, closely tied — never shown otherwise
/// (§3.2.9: "present options only when genuinely, closely ambiguous;
/// otherwise decide and say so").
#[derive(Debug, Clone)]
pub struct RankedCandidate {
    pub id: i64,
    pub headline: String,
    pub reasoning: String,
}

pub fn rank(candidates: &[DeadlineCandidate]) -> Vec<&DeadlineCandidate> {
    let mut sorted: Vec<&DeadlineCandidate> = candidates.iter().collect();

    sorted.sort_by(|a, b| {
        leverage_rank(&a.leverage_class)
            .cmp(&leverage_rank(&b.leverage_class))
            .then_with(|| a.due_at.cmp(&b.due_at))
    });

    sorted
}
#[derive(Debug, Clone)]
pub struct Verdict {
    pub headline: String,
    pub reasoning: String,
    pub confidence: Confidence,
    pub grounded_in_deadline_id: Option<i64>,
    /// Up to 2 additional ranked items (09_DECISION_ENGINE.md §4, §2.1),
    /// only ever non-empty when the Closeness Threshold below actually
    /// trips. Empty in the ordinary case of a single clear answer.
    pub runners_up: Vec<RankedCandidate>,
}

fn leverage_rank(class: &str) -> u8 {
    match class {
        "high" => 0,
        "medium" => 1,
        "low" => 2,
        _ => 3,
    }
}

/// Converts the `YYYY-MM-DD` prefix of a `due_at` string into a day
/// count (Howard Hinnant's `days_from_civil`, pure integer arithmetic).
/// `athena-domain` takes no third-party dependencies (see Cargo.toml),
/// so this is reimplemented locally rather than pulling in a calendar
/// crate for the one thing that needs day-granularity subtraction.
/// Returns `None` if the string isn't at least `YYYY-MM-DD`-shaped.
fn days_from_civil(due_at: &str) -> Option<i64> {
    let y: i64 = due_at.get(0..4)?.parse().ok()?;
    let m: i64 = due_at.get(5..7)?.parse().ok()?;
    let d: i64 = due_at.get(8..10)?.parse().ok()?;
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

/// Whether two same-leverage-tier candidates are close enough in time
/// to count as genuinely ambiguous rather than one clearly outranking
/// the other. This is a deterministic, documented stand-in for the
/// full 2-of-4 Signal Threshold model (recurrence, stakes,
/// reversibility, contradiction) described in 09_DECISION_ENGINE.md §4
/// — that model's own inputs (`drift_signals`, recurrence history) have
/// no persisted source yet anywhere in this codebase, so this function
/// implements only the one signal it can compute honestly today: same
/// leverage tier and due within a day of each other.
fn is_close(a: &DeadlineCandidate, b: &DeadlineCandidate) -> bool {
    if leverage_rank(&a.leverage_class) != leverage_rank(&b.leverage_class) {
        return false;
    }
    match (days_from_civil(&a.due_at), days_from_civil(&b.due_at)) {
        (Some(da), Some(db)) => (da - db).abs() <= 1,
        _ => false,
    }
}

/// Picks the single highest-leverage, soonest-due open deadline as
/// "the one thing right now" (spec framing in 03_ONBOARDING.md §5.2's
/// cross-reference to `Now`). Returns `insufficient_data` when there is
/// nothing open — the correct, expected cold-start state
/// (01_ARCHITECTURE.md §7.1; MASTER_SPECIFICATION.md §4.7).
pub fn resolve_priority(candidates: &[DeadlineCandidate]) -> Verdict {
    let sorted = rank(candidates);

    match sorted.first() {
        None => Verdict {
            headline: "Not enough data yet to name the one thing right now.".to_string(),
            reasoning: "No open deadlines exist yet — complete Semester Setup or add a deadline to get a real verdict."
                .to_string(),
            confidence: Confidence::InsufficientData,
            grounded_in_deadline_id: None,
            runners_up: Vec::new(),
        },
        Some(top) => {
            // Closeness Threshold (§4): only surfaced when a candidate is
            // genuinely tied with the top pick, capped at 2 runners-up so
            // the list never exceeds 3 items total (§4's own cap).
            let runners_up: Vec<RankedCandidate> = sorted
                .iter()
                .skip(1)
                .filter(|candidate| is_close(top, candidate))
                .take(2)
                .map(|candidate| RankedCandidate {
                    id: candidate.id,
                    headline: format!("Also close: {}", candidate.title),
                    reasoning: format!(
                        "Same leverage tier ({}) and due within a day of the top pick ({}).",
                        candidate.leverage_class, candidate.due_at
                    ),
                })
                .collect();

            Verdict {
                headline: format!("Work on: {}", top.title),
                reasoning: format!(
                    "This is your highest-leverage ({}) open item, due {}, among everything currently open.",
                    top.leverage_class, top.due_at
                ),
                confidence: Confidence::Inferred,
                grounded_in_deadline_id: Some(top.id),
                runners_up,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_candidates_yield_insufficient_data() {
        let verdict = resolve_priority(&[]);
        assert_eq!(verdict.confidence, Confidence::InsufficientData);
        assert!(verdict.grounded_in_deadline_id.is_none());
    }

    #[test]
    fn picks_highest_leverage_first_then_soonest_due() {
        let candidates = vec![
            DeadlineCandidate {
                id: 1,
                title: "Low-leverage, sooner".into(),
                due_at: "2026-08-01T00:00:00".into(),
                leverage_class: "low".into(),
            },
            DeadlineCandidate {
                id: 2,
                title: "High-leverage, later".into(),
                due_at: "2026-08-10T00:00:00".into(),
                leverage_class: "high".into(),
            },
        ];
        let verdict = resolve_priority(&candidates);
        assert_eq!(verdict.grounded_in_deadline_id, Some(2));
        assert_eq!(verdict.confidence, Confidence::Inferred);
    }

    #[test]
    fn breaks_ties_on_leverage_by_soonest_due_at() {
        let candidates = vec![
            DeadlineCandidate {
                id: 1,
                title: "High-leverage, later".into(),
                due_at: "2026-08-10T00:00:00".into(),
                leverage_class: "high".into(),
            },
            DeadlineCandidate {
                id: 2,
                title: "High-leverage, sooner".into(),
                due_at: "2026-08-05T00:00:00".into(),
                leverage_class: "high".into(),
            },
        ];
        let verdict = resolve_priority(&candidates);
        assert_eq!(verdict.grounded_in_deadline_id, Some(2));
    }

    #[test]
    fn no_runners_up_when_clearly_ahead() {
        let candidates = vec![
            DeadlineCandidate {
                id: 1,
                title: "High-leverage, due soon".into(),
                due_at: "2026-08-01T00:00:00".into(),
                leverage_class: "high".into(),
            },
            DeadlineCandidate {
                id: 2,
                title: "Low-leverage, due later".into(),
                due_at: "2026-09-01T00:00:00".into(),
                leverage_class: "low".into(),
            },
        ];
        let verdict = resolve_priority(&candidates);
        assert!(verdict.runners_up.is_empty());
    }

    #[test]
    fn surfaces_runners_up_when_genuinely_close() {
        let candidates = vec![
            DeadlineCandidate {
                id: 1,
                title: "High-leverage A".into(),
                due_at: "2026-08-10T09:00:00".into(),
                leverage_class: "high".into(),
            },
            DeadlineCandidate {
                id: 2,
                title: "High-leverage B, same day".into(),
                due_at: "2026-08-10T23:00:00".into(),
                leverage_class: "high".into(),
            },
            DeadlineCandidate {
                id: 3,
                title: "Low-leverage, unrelated".into(),
                due_at: "2026-08-11T00:00:00".into(),
                leverage_class: "low".into(),
            },
        ];
        let verdict = resolve_priority(&candidates);
        assert_eq!(verdict.grounded_in_deadline_id, Some(1));
        assert_eq!(verdict.runners_up.len(), 1);
        assert_eq!(verdict.runners_up[0].id, 2);
    }

    #[test]
    fn caps_runners_up_at_two() {
        let candidates = vec![
            DeadlineCandidate {
                id: 1,
                title: "High-leverage A".into(),
                due_at: "2026-08-10T00:00:00".into(),
                leverage_class: "high".into(),
            },
            DeadlineCandidate {
                id: 2,
                title: "High-leverage B".into(),
                due_at: "2026-08-10T01:00:00".into(),
                leverage_class: "high".into(),
            },
            DeadlineCandidate {
                id: 3,
                title: "High-leverage C".into(),
                due_at: "2026-08-10T02:00:00".into(),
                leverage_class: "high".into(),
            },
            DeadlineCandidate {
                id: 4,
                title: "High-leverage D".into(),
                due_at: "2026-08-10T03:00:00".into(),
                leverage_class: "high".into(),
            },
        ];
        let verdict = resolve_priority(&candidates);
        assert_eq!(verdict.runners_up.len(), 2);
    }
}
