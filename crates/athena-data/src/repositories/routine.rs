//! `daily_routine_responses` / `weekly_routine_responses` repository
//! (V6 migration). Both questionnaires are append-only — see the V6
//! migration's doc comment for why a resubmission is a new row rather
//! than an upsert.

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::error::DataError;

#[derive(Debug, Clone, Serialize)]
pub struct DailyRoutineResponseRow {
    pub id: i64,
    pub date: String,
    pub energy_level: i64,
    pub hours_available_tonight: f64,
    pub had_disruption_today: bool,
    pub disruption_note: Option<String>,
    pub focus_rating: i64,
    pub submitted_at: String,
}

/// Fields collected by the daily questionnaire.
#[derive(Debug, Clone)]
pub struct NewDailyRoutineResponse {
    pub date: String,
    pub energy_level: i64,
    pub hours_available_tonight: f64,
    pub had_disruption_today: bool,
    pub disruption_note: Option<String>,
    pub focus_rating: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeeklyRoutineResponseRow {
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

/// Fields collected by the weekly questionnaire.
#[derive(Debug, Clone)]
pub struct NewWeeklyRoutineResponse {
    pub week_starting: String,
    pub overall_energy_trend: i64,
    pub satisfaction_with_progress: i64,
    pub hardest_course_id: Option<i64>,
    pub biggest_blocker: Option<String>,
    pub hours_studied_estimate: Option<f64>,
    pub wants_deep_work_adjustment: bool,
    pub notes: Option<String>,
}

const DAILY_SELECT_COLUMNS: &str = "id, date, energy_level, hours_available_tonight, \
    had_disruption_today, disruption_note, focus_rating, submitted_at";

const WEEKLY_SELECT_COLUMNS: &str = "id, week_starting, overall_energy_trend, \
    satisfaction_with_progress, hardest_course_id, biggest_blocker, hours_studied_estimate, \
    wants_deep_work_adjustment, notes, submitted_at";

fn row_to_daily(row: &rusqlite::Row<'_>) -> rusqlite::Result<DailyRoutineResponseRow> {
    Ok(DailyRoutineResponseRow {
        id: row.get(0)?,
        date: row.get(1)?,
        energy_level: row.get(2)?,
        hours_available_tonight: row.get(3)?,
        had_disruption_today: row.get::<_, i64>(4)? != 0,
        disruption_note: row.get(5)?,
        focus_rating: row.get(6)?,
        submitted_at: row.get(7)?,
    })
}

fn row_to_weekly(row: &rusqlite::Row<'_>) -> rusqlite::Result<WeeklyRoutineResponseRow> {
    Ok(WeeklyRoutineResponseRow {
        id: row.get(0)?,
        week_starting: row.get(1)?,
        overall_energy_trend: row.get(2)?,
        satisfaction_with_progress: row.get(3)?,
        hardest_course_id: row.get(4)?,
        biggest_blocker: row.get(5)?,
        hours_studied_estimate: row.get(6)?,
        wants_deep_work_adjustment: row.get::<_, i64>(7)? != 0,
        notes: row.get(8)?,
        submitted_at: row.get(9)?,
    })
}

/// Inserts one daily questionnaire response.
pub fn insert_daily_response(
    conn: &Connection,
    new: &NewDailyRoutineResponse,
) -> Result<i64, DataError> {
    conn.execute(
        "INSERT INTO daily_routine_responses \
         (date, energy_level, hours_available_tonight, had_disruption_today, disruption_note, focus_rating) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            new.date,
            new.energy_level,
            new.hours_available_tonight,
            new.had_disruption_today as i64,
            new.disruption_note,
            new.focus_rating,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Whether a daily response already exists for `date` — used by the
/// frontend's "already answered today" check so the prompt doesn't nag.
pub fn has_response_for_date(conn: &Connection, date: &str) -> Result<bool, DataError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM daily_routine_responses WHERE date = ?1",
        params![date],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Most recent daily responses, newest first.
pub fn list_recent_daily(
    conn: &Connection,
    limit: i64,
) -> Result<Vec<DailyRoutineResponseRow>, DataError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {DAILY_SELECT_COLUMNS} FROM daily_routine_responses ORDER BY submitted_at DESC LIMIT ?1"
    ))?;
    let rows = stmt
        .query_map(params![limit], row_to_daily)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Inserts one weekly questionnaire response.
pub fn insert_weekly_response(
    conn: &Connection,
    new: &NewWeeklyRoutineResponse,
) -> Result<i64, DataError> {
    conn.execute(
        "INSERT INTO weekly_routine_responses \
         (week_starting, overall_energy_trend, satisfaction_with_progress, hardest_course_id, \
          biggest_blocker, hours_studied_estimate, wants_deep_work_adjustment, notes) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            new.week_starting,
            new.overall_energy_trend,
            new.satisfaction_with_progress,
            new.hardest_course_id,
            new.biggest_blocker,
            new.hours_studied_estimate,
            new.wants_deep_work_adjustment as i64,
            new.notes,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Whether a weekly response already exists for `week_starting` (the
/// `YYYY-MM-DD` Monday of that week) — used the same way
/// `has_response_for_date` is, for a once-a-week prompt.
pub fn has_response_for_week(conn: &Connection, week_starting: &str) -> Result<bool, DataError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM weekly_routine_responses WHERE week_starting = ?1",
        params![week_starting],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Most recent weekly responses, newest first.
pub fn list_recent_weekly(
    conn: &Connection,
    limit: i64,
) -> Result<Vec<WeeklyRoutineResponseRow>, DataError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {WEEKLY_SELECT_COLUMNS} FROM weekly_routine_responses ORDER BY submitted_at DESC LIMIT ?1"
    ))?;
    let rows = stmt
        .query_map(params![limit], row_to_weekly)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::open_and_migrate;
    use tempfile::NamedTempFile;

    #[test]
    fn insert_and_list_daily_responses() {
        let tmp = NamedTempFile::new().unwrap();
        let conn = open_and_migrate(tmp.path()).unwrap();

        assert!(!has_response_for_date(&conn, "2026-07-18").unwrap());

        insert_daily_response(
            &conn,
            &NewDailyRoutineResponse {
                date: "2026-07-18".into(),
                energy_level: 4,
                hours_available_tonight: 2.5,
                had_disruption_today: false,
                disruption_note: None,
                focus_rating: 3,
            },
        )
        .unwrap();

        assert!(has_response_for_date(&conn, "2026-07-18").unwrap());
        let recent = list_recent_daily(&conn, 10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].energy_level, 4);
    }

    #[test]
    fn insert_and_list_weekly_responses() {
        let tmp = NamedTempFile::new().unwrap();
        let conn = open_and_migrate(tmp.path()).unwrap();

        assert!(!has_response_for_week(&conn, "2026-07-13").unwrap());

        insert_weekly_response(
            &conn,
            &NewWeeklyRoutineResponse {
                week_starting: "2026-07-13".into(),
                overall_energy_trend: 3,
                satisfaction_with_progress: 4,
                hardest_course_id: None,
                biggest_blocker: Some("Too many overlapping deadlines".into()),
                hours_studied_estimate: Some(18.0),
                wants_deep_work_adjustment: true,
                notes: None,
            },
        )
        .unwrap();

        assert!(has_response_for_week(&conn, "2026-07-13").unwrap());
        let recent = list_recent_weekly(&conn, 10).unwrap();
        assert_eq!(recent.len(), 1);
        assert!(recent[0].wants_deep_work_adjustment);
    }
}
