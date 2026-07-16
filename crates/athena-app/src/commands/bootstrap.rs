//! The one read command every screen boots from. Per
//! 01_ARCHITECTURE.md §2.1 (the canonical read path), this is a thin
//! translation of IPC -> repository calls -> typed response.
//!
//! Since `08_ADAPTIVE_PLANNER.md` was integrated, the verdict this
//! command returns is computed by `athena_domain::planner::replan`
//! rather than calling `athena_domain::priority::resolve_priority`
//! directly — `replan` reuses `priority::rank` internally (§2: "every
//! trigger... runs the identical scoring function"), so a normal boot
//! with no disruptions logged today produces byte-identical reasoning
//! to before this change; a boot on a day with disruptions already
//! logged (e.g. the app was restarted mid-evening) now correctly
//! reflects them instead of silently reverting to the undisrupted
//! verdict until the next `log_disruption` call.

use std::sync::Mutex;

use athena_data::repositories::{course, deadline, decision, disruption, profile, semester};
use athena_domain::planner::{self, DisruptionType, ScheduleDisruption};
use athena_domain::priority::{self, DeadlineCandidate};
use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

/// Mirrors `athena_domain::priority::RankedCandidate` (see `VerdictDto`'s
/// own doc comment for why this mapping happens here rather than via
/// `Serialize` on the domain type itself).
#[derive(Debug, Clone, Serialize)]
pub struct RankedCandidateDto {
    pub id: i64,
    pub headline: String,
    pub reasoning: String,
}

/// Mirrors `athena_domain::priority::Verdict`, mapped to owned strings
/// for serialization — `athena-domain` deliberately takes no
/// third-party dependencies (including serde), per its Cargo.toml, so
/// this mapping happens here rather than by deriving `Serialize` on the
/// domain type itself.
#[derive(Debug, Clone, Serialize)]
pub struct VerdictDto {
    pub headline: String,
    pub reasoning: String,
    pub confidence: String,
    pub grounded_in_deadline_id: Option<i64>,
    /// Populated only when the Closeness Threshold trips
    /// (09_DECISION_ENGINE.md §4) — see `Now`'s `VerdictCard`.
    pub runners_up: Vec<RankedCandidateDto>,
}

/// The one `Verdict` -> `VerdictDto` mapping, shared by
/// `get_bootstrap_state` and `commands::planner::log_disruption` so the
/// two commands can never diverge in how they present a verdict to the
/// frontend. `pub(crate)` — internal to `athena-app`, not part of the
/// IPC surface itself.
pub(crate) fn verdict_to_dto(verdict: priority::Verdict) -> VerdictDto {
    VerdictDto {
        headline: verdict.headline,
        reasoning: verdict.reasoning,
        confidence: match verdict.confidence {
            priority::Confidence::Inferred => "inferred".to_string(),
            priority::Confidence::InsufficientData => "insufficient_data".to_string(),
        },
        grounded_in_deadline_id: verdict.grounded_in_deadline_id,
        runners_up: verdict
            .runners_up
            .into_iter()
            .map(|r| RankedCandidateDto {
                id: r.id,
                headline: r.headline,
                reasoning: r.reasoning,
            })
            .collect(),
    }
}

/// Everything the frontend needs to render `Now`, `Trajectory`,
/// `Decision Log`, and to decide whether onboarding is required
/// (03_ONBOARDING.md §1) — one IPC round trip, real persisted data,
/// unmodified in shape.
#[derive(Debug, Clone, Serialize)]
pub struct BootstrapState {
    pub has_profile: bool,
    pub profile: Option<profile::ProfileRow>,
    pub current_semester: Option<semester::SemesterRow>,
    pub courses: Vec<course::CourseRow>,
    pub deadlines: Vec<deadline::DeadlineRow>,
    /// Real read model behind Trajectory's "Career threads" section
    /// (04_DATA_MODEL.md §5.2) — `deadlines WHERE category = 'career'`.
    pub career_deadlines: Vec<deadline::DeadlineRow>,
    pub decisions: Vec<decision::DecisionRow>,
    pub verdict: VerdictDto,
    /// §3.1's `available_minutes_tonight` after today's already-logged
    /// disruptions (empty list, undisrupted window, on a normal day).
    pub available_minutes_tonight: i64,
    pub base_window_minutes: i64,
    /// Disruptions logged for today's date specifically — drives the
    /// "logged today" section of the planner UI. `local_date` is
    /// supplied by the frontend (see `commands::planner`'s doc comment
    /// on why date/time stays a frontend concern in this schema).
    pub today_disruptions: Vec<disruption::DisruptionRow>,
    /// Most recent disruptions overall — the explainability trail
    /// (§5's causal-chain guarantee) shown regardless of date.
    pub recent_disruptions: Vec<disruption::DisruptionRow>,
}

#[tauri::command]
pub fn get_bootstrap_state(
    db: State<'_, Mutex<Connection>>,
    local_date: Option<String>,
) -> Result<BootstrapState, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;

    let has_profile = profile::has_profile(&conn).map_err(|e| e.to_string())?;
    let profile_row = profile::get_current_profile(&conn).map_err(|e| e.to_string())?;
    let current_semester = semester::get_current_semester(&conn).map_err(|e| e.to_string())?;

    let (courses, deadlines) = match &current_semester {
        Some(sem) => (
            course::list_by_semester(&conn, sem.id).map_err(|e| e.to_string())?,
            deadline::list_by_semester(&conn, sem.id).map_err(|e| e.to_string())?,
        ),
        None => (Vec::new(), Vec::new()),
    };

    let career_deadlines = deadline::list_open_career(&conn).map_err(|e| e.to_string())?;
    let decisions = decision::list_recent(&conn, 8).map_err(|e| e.to_string())?;
    let recent_disruptions = disruption::list_recent(&conn, 10).map_err(|e| e.to_string())?;

    let base_window_minutes = profile_row
        .as_ref()
        .and_then(|p| {
            planner::base_window_minutes(&p.deep_work_window_start, &p.deep_work_window_end)
        })
        .unwrap_or(240);

    let today_disruptions = match &local_date {
        Some(date) => disruption::list_for_date(&conn, date).map_err(|e| e.to_string())?,
        None => Vec::new(),
    };
    let planner_disruptions: Vec<ScheduleDisruption> = today_disruptions
        .iter()
        .filter_map(|row| {
            DisruptionType::from_str(&row.disruption_type).map(|t| ScheduleDisruption {
                disruption_type: t,
                duration_minutes: row.duration_minutes,
            })
        })
        .collect();

    let open_deadlines = deadline::list_open(&conn).map_err(|e| e.to_string())?;
    let candidates: Vec<DeadlineCandidate> = open_deadlines
        .iter()
        .map(|d| DeadlineCandidate {
            id: d.id,
            title: d.title.clone(),
            due_at: d.due_at.clone(),
            leverage_class: d.leverage_class.clone(),
        })
        .collect();

    let replan = planner::replan(&candidates, base_window_minutes, &planner_disruptions);
    let verdict_dto = verdict_to_dto(replan.verdict);

    Ok(BootstrapState {
        has_profile,
        profile: profile_row,
        current_semester,
        courses,
        deadlines,
        career_deadlines,
        decisions,
        verdict: verdict_dto,
        available_minutes_tonight: replan.available_minutes_tonight,
        base_window_minutes,
        today_disruptions,
        recent_disruptions,
    })
}