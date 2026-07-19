//! The Adaptive Planner's one write command (`08_ADAPTIVE_PLANNER.md`).
//! Logs a `schedule_disruptions` row and recomputes through
//! `athena_domain::planner::replan`, which itself reuses
//! `athena_domain::priority::rank` — the same Decision Engine
//! `get_bootstrap_state` already calls (§2: "every trigger... runs the
//! identical scoring function"). See `bootstrap.rs`'s `verdict_dto`
//! helper, now shared by both commands so `Now`'s always-on verdict and
//! this on-demand recompute can never diverge in how they map
//! `athena_domain::priority::Verdict` to the wire shape.

use std::sync::Mutex;

use athena_data::repositories::{deadline, disruption, profile, semester};
use athena_domain::planner::{self, DisruptionType, ScheduleDisruption};
use athena_domain::priority::DeadlineCandidate;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::bootstrap::VerdictDto;

/// Mirrors `athena_data::repositories::disruption::DisruptionRow`,
/// re-exported here (rather than deriving `Serialize` on the repository
/// type directly in the response, matching `bootstrap.rs`'s existing
/// DTO-mapping convention) so the frontend has one consistent
/// `ipc/bindings.ts` shape per command family.
#[derive(Debug, Clone, Serialize)]
pub struct DisruptionDto {
    pub id: i64,
    pub date: String,
    pub disruption_type: String,
    pub duration_minutes: i64,
    pub affects_deep_work_window: bool,
    pub linked_deadline_id: Option<i64>,
    pub note: Option<String>,
    pub logged_at: String,
}

/// The exact fields a disruption-logging flow collects, one struct per
/// §4's six types plus the fields every type shares.
#[derive(Debug, Deserialize)]
pub struct LogDisruptionInput {
    /// `YYYY-MM-DD`, computed client-side from the user's local clock
    /// (`user_profile.timezone`) — `athena-data`/`athena-domain` take no
    /// date/time dependency (Implementation Plan §4's "no speculative
    /// generality"), so the caller supplies it explicitly, the same way
    /// `due_at` is supplied explicitly on `deadlines`.
    pub date: String,
    pub disruption_type: String,
    /// Minutes lost (`external_interrupt`, `surprise_workload`,
    /// `illness`) or minutes gained (`cancelled_class`, `early_finish`).
    /// Ignored for `unexpected_opportunity` (§4.4 — no direct minutes
    /// effect; see `DisruptionType::minutes_delta`'s doc comment).
    pub duration_minutes: i64,
    pub affects_deep_work_window: bool,
    pub linked_deadline_id: Option<i64>,
    pub note: Option<String>,
}

/// The recomputed verdict plus the window it was computed against —
/// everything `Now` needs to render the recovery plan explainably
/// (§4's "never a silent recalculation the user has to take on faith").
#[derive(Debug, Clone, Serialize)]
pub struct ReplanResultDto {
    pub disruption: DisruptionDto,
    pub verdict: VerdictDto,
    pub available_minutes_tonight: i64,
    pub base_window_minutes: i64,
    pub substituted: bool,
}

fn disruption_to_dto(row: disruption::DisruptionRow) -> DisruptionDto {
    DisruptionDto {
        id: row.id,
        date: row.date,
        disruption_type: row.disruption_type,
        duration_minutes: row.duration_minutes,
        affects_deep_work_window: row.affects_deep_work_window,
        linked_deadline_id: row.linked_deadline_id,
        note: row.note,
        logged_at: row.logged_at,
    }
}

/// Logs one `schedule_disruptions` row and immediately recomputes
/// tonight's verdict against it and every other disruption already
/// logged for the same date (§3.1's `available_minutes_tonight`
/// accumulates every disruption for the day, not just the latest one).
#[tauri::command]
pub fn log_disruption(
    db: State<'_, Mutex<Connection>>,
    input: LogDisruptionInput,
) -> Result<ReplanResultDto, String> {
    let disruption_type = DisruptionType::from_str(&input.disruption_type)
        .ok_or_else(|| format!("Unrecognized disruption_type: {}", input.disruption_type))?;

    let mut conn = db.lock().map_err(|e| e.to_string())?;

    let current_profile = profile::get_current_profile(&conn)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "A profile must exist before logging a disruption.".to_string())?;
    let current_semester = semester::get_current_semester(&conn)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "An active semester must exist before logging a disruption.".to_string())?;

    let base_window_minutes = planner::base_window_minutes(
        &current_profile.deep_work_window_start,
        &current_profile.deep_work_window_end,
    )
    .unwrap_or(240);

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let disruption_id = disruption::insert_disruption(
        &tx,
        &disruption::NewDisruption {
            semester_id: current_semester.id,
            date: input.date.clone(),
            disruption_type: disruption_type.as_str().to_string(),
            duration_minutes: input.duration_minutes,
            affects_deep_work_window: input.affects_deep_work_window,
            linked_deadline_id: input.linked_deadline_id,
            note: input.note.clone(),
        },
    )
    .map_err(|e| e.to_string())?;

    // Every disruption logged for the same date, so a second interrupt
    // the same evening compounds on the first rather than overwriting it.
    let todays_rows = disruption::list_for_date(&tx, &input.date).map_err(|e| e.to_string())?;
    let todays_disruptions: Vec<ScheduleDisruption> = todays_rows
        .iter()
        .filter_map(|row| {
            DisruptionType::from_str(&row.disruption_type).map(|t| ScheduleDisruption {
                disruption_type: t,
                duration_minutes: row.duration_minutes,
            })
        })
        .collect();

    let open_deadlines = deadline::list_open(&tx).map_err(|e| e.to_string())?;
    let candidates: Vec<DeadlineCandidate> = open_deadlines
        .iter()
        .map(|d| DeadlineCandidate {
            id: d.id,
            title: d.title.clone(),
            due_at: d.due_at.clone(),
            leverage_class: d.leverage_class.clone(),
        })
        .collect();

    let replan = planner::replan(&candidates, base_window_minutes, &todays_disruptions);
    let verdict_dto = super::bootstrap::verdict_to_dto(replan.verdict);

    disruption::record_recompute(
        &tx,
        disruption_id,
        &verdict_dto.headline,
        &verdict_dto.reasoning,
    )
    .map_err(|e| e.to_string())?;

    let disruption_row = disruption::get(&tx, disruption_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Disruption row vanished immediately after insert.".to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(ReplanResultDto {
        disruption: disruption_to_dto(disruption_row),
        verdict: verdict_dto,
        available_minutes_tonight: replan.available_minutes_tonight,
        base_window_minutes,
        substituted: replan.substituted,
    })
}

/// Most recent disruptions across all dates — the explainability trail
/// `Now` renders alongside the verdict (§5's causal-chain guarantee).
#[tauri::command]
pub fn list_recent_disruptions(
    db: State<'_, Mutex<Connection>>,
    limit: i64,
) -> Result<Vec<DisruptionDto>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let rows = disruption::list_recent(&conn, limit).map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(disruption_to_dto).collect())
}