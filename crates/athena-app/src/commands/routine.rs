//! Daily and weekly routine-questionnaire commands. Additive on top of
//! V6's `daily_routine_responses` / `weekly_routine_responses` tables —
//! mirrors `commands::planner`'s style (typed input struct in, typed
//! DTO out, `Mutex<Connection>` state), but these two commands are
//! simple submit/fetch pairs with no `athena_domain` recompute of their
//! own; the daily questionnaire's answers are read by
//! `athena_domain::planner::replan` callers as a second, user-reported
//! input, not recomputed here.

use std::sync::Mutex;

use athena_data::repositories::routine;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct DailyRoutineResponseDto {
    pub id: i64,
    pub date: String,
    pub energy_level: i64,
    pub hours_available_tonight: f64,
    pub had_disruption_today: bool,
    pub disruption_note: Option<String>,
    pub focus_rating: i64,
    pub submitted_at: String,
}

fn daily_to_dto(row: routine::DailyRoutineResponseRow) -> DailyRoutineResponseDto {
    DailyRoutineResponseDto {
        id: row.id,
        date: row.date,
        energy_level: row.energy_level,
        hours_available_tonight: row.hours_available_tonight,
        had_disruption_today: row.had_disruption_today,
        disruption_note: row.disruption_note,
        focus_rating: row.focus_rating,
        submitted_at: row.submitted_at,
    }
}

#[derive(Debug, Deserialize)]
pub struct SubmitDailyRoutineInput {
    /// `YYYY-MM-DD`, computed client-side (same convention as
    /// `commands::planner::LogDisruptionInput::date`).
    pub date: String,
    /// 1-5 self-rated energy for the day.
    pub energy_level: i64,
    /// Hours the user expects to have free tonight — a direct,
    /// user-reported alternative/supplement to
    /// `athena_domain::planner::available_minutes_tonight`'s
    /// disruption-derived estimate.
    pub hours_available_tonight: f64,
    pub had_disruption_today: bool,
    pub disruption_note: Option<String>,
    /// 1-5 retrospective focus rating.
    pub focus_rating: i64,
}

#[tauri::command]
pub fn submit_daily_routine_response(
    db: State<'_, Mutex<Connection>>,
    input: SubmitDailyRoutineInput,
) -> Result<DailyRoutineResponseDto, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let id = routine::insert_daily_response(
        &conn,
        &routine::NewDailyRoutineResponse {
            date: input.date,
            energy_level: input.energy_level,
            hours_available_tonight: input.hours_available_tonight,
            had_disruption_today: input.had_disruption_today,
            disruption_note: input.disruption_note,
            focus_rating: input.focus_rating,
        },
    )
    .map_err(|e| e.to_string())?;

    let recent = routine::list_recent_daily(&conn, 1).map_err(|e| e.to_string())?;
    recent
        .into_iter()
        .find(|r| r.id == id)
        .map(daily_to_dto)
        .ok_or_else(|| "Daily routine response vanished immediately after insert.".to_string())
}

/// Whether `date` already has a submitted daily response — the
/// frontend's "already answered today" check (don't nag).
#[tauri::command]
pub fn has_daily_routine_response(
    db: State<'_, Mutex<Connection>>,
    date: String,
) -> Result<bool, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    routine::has_response_for_date(&conn, &date).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_recent_daily_routine_responses(
    db: State<'_, Mutex<Connection>>,
    limit: i64,
) -> Result<Vec<DailyRoutineResponseDto>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let rows = routine::list_recent_daily(&conn, limit).map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(daily_to_dto).collect())
}

#[derive(Debug, Clone, Serialize)]
pub struct WeeklyRoutineResponseDto {
    pub id: i64,
    pub week_starting: String,
    pub overall_energy_trend: i64,
    pub satisfaction_with_progress: i64,
    pub hardest_course_id: Option<i64>,
    pub biggest_blocker: Option<String>,
    pub hours_studied_estimate: Option<f64>,
    pub wants_deep_work_adjustment: bool,
    pub notes: Option<String>,
    pub submitted_at: String,
}

fn weekly_to_dto(row: routine::WeeklyRoutineResponseRow) -> WeeklyRoutineResponseDto {
    WeeklyRoutineResponseDto {
        id: row.id,
        week_starting: row.week_starting,
        overall_energy_trend: row.overall_energy_trend,
        satisfaction_with_progress: row.satisfaction_with_progress,
        hardest_course_id: row.hardest_course_id,
        biggest_blocker: row.biggest_blocker,
        hours_studied_estimate: row.hours_studied_estimate,
        wants_deep_work_adjustment: row.wants_deep_work_adjustment,
        notes: row.notes,
        submitted_at: row.submitted_at,
    }
}

#[derive(Debug, Deserialize)]
pub struct SubmitWeeklyRoutineInput {
    /// `YYYY-MM-DD`, the Monday of the week being reported on —
    /// computed client-side, same convention as the daily `date` field.
    pub week_starting: String,
    pub overall_energy_trend: i64,
    pub satisfaction_with_progress: i64,
    pub hardest_course_id: Option<i64>,
    pub biggest_blocker: Option<String>,
    pub hours_studied_estimate: Option<f64>,
    pub wants_deep_work_adjustment: bool,
    pub notes: Option<String>,
}

#[tauri::command]
pub fn submit_weekly_routine_response(
    db: State<'_, Mutex<Connection>>,
    input: SubmitWeeklyRoutineInput,
) -> Result<WeeklyRoutineResponseDto, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let id = routine::insert_weekly_response(
        &conn,
        &routine::NewWeeklyRoutineResponse {
            week_starting: input.week_starting,
            overall_energy_trend: input.overall_energy_trend,
            satisfaction_with_progress: input.satisfaction_with_progress,
            hardest_course_id: input.hardest_course_id,
            biggest_blocker: input.biggest_blocker,
            hours_studied_estimate: input.hours_studied_estimate,
            wants_deep_work_adjustment: input.wants_deep_work_adjustment,
            notes: input.notes,
        },
    )
    .map_err(|e| e.to_string())?;

    let recent = routine::list_recent_weekly(&conn, 1).map_err(|e| e.to_string())?;
    recent
        .into_iter()
        .find(|r| r.id == id)
        .map(weekly_to_dto)
        .ok_or_else(|| "Weekly routine response vanished immediately after insert.".to_string())
}

/// Whether `week_starting` already has a submitted weekly response.
#[tauri::command]
pub fn has_weekly_routine_response(
    db: State<'_, Mutex<Connection>>,
    week_starting: String,
) -> Result<bool, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    routine::has_response_for_week(&conn, &week_starting).map_err(|e| e.to_string())
}

/// The scheduled daily-questionnaire trigger's fallback when no
/// `user_profile` row has a value yet — mirrors the V7 migration
/// column's own `DEFAULT '20:00'`, so a fresh profile and a pre-
/// onboarding read agree on the same default without either one having
/// to special-case the other.
const DEFAULT_ROUTINE_QUESTIONNAIRE_TIME: &str = "20:00";

/// Saves the local `HH:MM` time of day the scheduled daily-
/// questionnaire trigger (`routine_scheduler::spawn`) should fire.
/// Stored on `user_profile` (V7 migration) — see that migration's own
/// doc comment for why it lives there rather than a new table.
#[tauri::command]
pub fn save_routine_questionnaire_time(
    db: State<'_, Mutex<Connection>>,
    time: String,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    athena_data::repositories::profile::set_routine_questionnaire_time(&conn, &time)
        .map_err(|e| e.to_string())
}

/// Reads the configured time back, falling back to
/// `DEFAULT_ROUTINE_QUESTIONNAIRE_TIME` both pre-onboarding (no
/// `user_profile` row at all — the repository call returns `Ok(None)`)
/// and on any read error, so Settings' time input always has something
/// valid to render rather than an empty/error state.
#[tauri::command]
pub fn get_routine_questionnaire_time(db: State<'_, Mutex<Connection>>) -> Result<String, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    Ok(
        athena_data::repositories::profile::get_routine_questionnaire_time(&conn)
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| DEFAULT_ROUTINE_QUESTIONNAIRE_TIME.to_string()),
    )
}

#[tauri::command]
pub fn list_recent_weekly_routine_responses(
    db: State<'_, Mutex<Connection>>,
    limit: i64,
) -> Result<Vec<WeeklyRoutineResponseDto>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let rows = routine::list_recent_weekly(&conn, limit).map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(weekly_to_dto).collect())
}
