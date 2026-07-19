//! `schedule_disruptions` repository (`08_ADAPTIVE_PLANNER.md` §5).
//!
//! Read/write, unlike `decisions` (still read-only): the Adaptive
//! Planner is the write path this table exists for — see the V3
//! migration's doc comment for the two documented deviations from §5's
//! literal column list (`linked_opportunity_id`, `recommendation_id_after`).

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::error::DataError;

#[derive(Debug, Clone, Serialize)]
pub struct DisruptionRow {
    pub id: i64,
    pub semester_id: i64,
    pub date: String,
    pub disruption_type: String,
    pub duration_minutes: i64,
    pub affects_deep_work_window: bool,
    pub linked_deadline_id: Option<i64>,
    pub note: Option<String>,
    pub logged_at: String,
    pub recompute_triggered: bool,
    pub recompute_headline: Option<String>,
    pub recompute_reasoning: Option<String>,
}

/// Fields collected by the disruption-logging flow (§4's per-type user
/// actions). `linked_deadline_id` is populated only for the types that
/// reference one (`surprise_workload` §4.2 after its own new-deadline
/// insert; the caller resolves that id before building this).
#[derive(Debug, Clone)]
pub struct NewDisruption {
    pub semester_id: i64,
    pub date: String,
    pub disruption_type: String,
    pub duration_minutes: i64,
    pub affects_deep_work_window: bool,
    pub linked_deadline_id: Option<i64>,
    pub note: Option<String>,
}

fn row_to_disruption(row: &rusqlite::Row<'_>) -> rusqlite::Result<DisruptionRow> {
    Ok(DisruptionRow {
        id: row.get(0)?,
        semester_id: row.get(1)?,
        date: row.get(2)?,
        disruption_type: row.get(3)?,
        duration_minutes: row.get(4)?,
        affects_deep_work_window: row.get::<_, i64>(5)? != 0,
        linked_deadline_id: row.get(6)?,
        note: row.get(7)?,
        logged_at: row.get(8)?,
        recompute_triggered: row.get::<_, i64>(9)? != 0,
        recompute_headline: row.get(10)?,
        recompute_reasoning: row.get(11)?,
    })
}

const SELECT_COLUMNS: &str = "id, semester_id, date, disruption_type, duration_minutes, \
     affects_deep_work_window, linked_deadline_id, note, logged_at, recompute_triggered, \
     recompute_headline, recompute_reasoning";

/// Inserts one disruption row inside an already-open transaction, so it
/// commits atomically alongside the recompute it triggers (mirrors the
/// `commit_semester_setup` pattern of one caller-owned transaction per
/// business event).
pub fn insert_disruption(
    tx: &rusqlite::Transaction<'_>,
    new: &NewDisruption,
) -> Result<i64, DataError> {
    tx.execute(
        "INSERT INTO schedule_disruptions \
         (semester_id, date, disruption_type, duration_minutes, affects_deep_work_window, \
          linked_deadline_id, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            new.semester_id,
            new.date,
            new.disruption_type,
            new.duration_minutes,
            new.affects_deep_work_window as i64,
            new.linked_deadline_id,
            new.note,
        ],
    )?;
    Ok(tx.last_insert_rowid())
}

/// Records the recompute this disruption triggered (§5's causal-chain
/// guarantee, satisfied via the inline columns rather than an FK — see
/// the V3 migration's doc comment).
pub fn record_recompute(
    tx: &rusqlite::Transaction<'_>,
    disruption_id: i64,
    headline: &str,
    reasoning: &str,
) -> Result<(), DataError> {
    tx.execute(
        "UPDATE schedule_disruptions SET recompute_triggered = 1, recompute_headline = ?1, \
         recompute_reasoning = ?2 WHERE id = ?3",
        params![headline, reasoning, disruption_id],
    )?;
    Ok(())
}

/// Every disruption logged for `date` (`YYYY-MM-DD`) — the evidence the
/// planner's `available_minutes_tonight` reduction reads (§3.1, §4).
pub fn list_for_date(conn: &Connection, date: &str) -> Result<Vec<DisruptionRow>, DataError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM schedule_disruptions WHERE date = ?1 ORDER BY logged_at"
    ))?;
    let rows = stmt
        .query_map(params![date], row_to_disruption)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Most recent disruptions across all dates, newest first — the
/// explainability trail rendered alongside `Now`'s verdict.
pub fn list_recent(conn: &Connection, limit: i64) -> Result<Vec<DisruptionRow>, DataError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM schedule_disruptions ORDER BY logged_at DESC LIMIT ?1"
    ))?;
    let rows = stmt
        .query_map(params![limit], row_to_disruption)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<DisruptionRow>, DataError> {
    conn.query_row(
        &format!("SELECT {SELECT_COLUMNS} FROM schedule_disruptions WHERE id = ?1"),
        params![id],
        row_to_disruption,
    )
    .optional()
    .map_err(DataError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::open_and_migrate;
    use crate::repositories::semester;
    use tempfile::NamedTempFile;

    #[test]
    fn insert_and_list_disruptions() {
        let tmp = NamedTempFile::new().unwrap();
        let mut conn = open_and_migrate(tmp.path()).unwrap();
        let tx = conn.transaction().unwrap();
        let semester_id =
            semester::create_semester(&tx, "Monsoon 2026", "2026-07-15", "2026-11-30").unwrap();
        let id = insert_disruption(
            &tx,
            &NewDisruption {
                semester_id,
                date: "2026-07-14".into(),
                disruption_type: "external_interrupt".into(),
                duration_minutes: 60,
                affects_deep_work_window: true,
                linked_deadline_id: None,
                note: Some("Friend visiting unexpectedly".into()),
            },
        )
        .unwrap();
        record_recompute(
            &tx,
            id,
            "Work on: DSA practice",
            "60 fewer minutes tonight...",
        )
        .unwrap();
        tx.commit().unwrap();

        let today = list_for_date(&conn, "2026-07-14").unwrap();
        assert_eq!(today.len(), 1);
        assert!(today[0].recompute_triggered);
        assert_eq!(
            today[0].recompute_headline.as_deref(),
            Some("Work on: DSA practice")
        );

        let recent = list_recent(&conn, 10).unwrap();
        assert_eq!(recent.len(), 1);
    }
}