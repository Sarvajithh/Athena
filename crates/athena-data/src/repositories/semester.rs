//! `semesters` repository (04_DATA_MODEL.md §3).

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::error::DataError;

#[derive(Debug, Clone, Serialize)]
pub struct SemesterRow {
    pub id: i64,
    pub label: String,
    pub starts_on: String,
    pub ends_on: String,
    pub is_current: bool,
    pub created_at: String,
}

fn row_to_semester(row: &rusqlite::Row<'_>) -> rusqlite::Result<SemesterRow> {
    Ok(SemesterRow {
        id: row.get(0)?,
        label: row.get(1)?,
        starts_on: row.get(2)?,
        ends_on: row.get(3)?,
        is_current: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
    })
}

pub fn get_current_semester(conn: &Connection) -> Result<Option<SemesterRow>, DataError> {
    conn.query_row(
        "SELECT id, label, starts_on, ends_on, is_current, created_at \
         FROM semesters WHERE is_current = 1 ORDER BY id DESC LIMIT 1",
        [],
        row_to_semester,
    )
    .optional()
    .map_err(DataError::from)
}

/// Creates a new semester within an already-open transaction (used by
/// `commit_semester_setup` so the semester + courses + deadlines +
/// profile-history row commit atomically, 03_ONBOARDING.md §3 Step 5:
/// "One 'Start Semester' button commits everything as a single
/// transaction.")
///
/// Per 01_ARCHITECTURE.md §7.1's rollover rule, any previously-current
/// semester is flipped to `is_current = 0` in the same transaction —
/// harmless no-op on first launch, when no semester exists yet.
pub fn create_semester(
    tx: &rusqlite::Transaction<'_>,
    label: &str,
    starts_on: &str,
    ends_on: &str,
) -> Result<i64, DataError> {
    tx.execute("UPDATE semesters SET is_current = 0 WHERE is_current = 1", [])?;
    tx.execute(
        "INSERT INTO semesters (label, starts_on, ends_on, is_current) VALUES (?1, ?2, ?3, 1)",
        params![label, starts_on, ends_on],
    )?;
    Ok(tx.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::open_and_migrate;
    use tempfile::NamedTempFile;

    #[test]
    fn no_semester_on_fresh_db() {
        let tmp = NamedTempFile::new().unwrap();
        let conn = open_and_migrate(tmp.path()).unwrap();
        assert!(get_current_semester(&conn).unwrap().is_none());
    }

    #[test]
    fn create_semester_marks_current() {
        let tmp = NamedTempFile::new().unwrap();
        let mut conn = open_and_migrate(tmp.path()).unwrap();
        let tx = conn.transaction().unwrap();
        let id = create_semester(&tx, "Monsoon 2026", "2026-07-15", "2026-11-30").unwrap();
        tx.commit().unwrap();

        let current = get_current_semester(&conn).unwrap().unwrap();
        assert_eq!(current.id, id);
        assert!(current.is_current);
    }
}
