//! `deadlines` repository (04_DATA_MODEL.md §5).

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::error::DataError;

#[derive(Debug, Clone, Serialize)]
pub struct DeadlineRow {
    pub id: i64,
    pub semester_id: i64,
    pub course_id: Option<i64>,
    pub title: String,
    pub category: String,
    pub due_at: String,
    pub leverage_class: String,
    pub status: String,
    pub created_at: String,
    pub notes: Option<String>,
}

/// Fields collected by Semester Setup Step 2 (03_ONBOARDING.md §3 Step 2).
#[derive(Debug, Clone)]
pub struct NewDeadline {
    pub course_id: Option<i64>,
    pub title: String,
    pub category: String,
    pub due_at: String,
    pub leverage_class: String,
    pub notes: Option<String>,
}

fn row_to_deadline(row: &rusqlite::Row<'_>) -> rusqlite::Result<DeadlineRow> {
    Ok(DeadlineRow {
        id: row.get(0)?,
        semester_id: row.get(1)?,
        course_id: row.get(2)?,
        title: row.get(3)?,
        category: row.get(4)?,
        due_at: row.get(5)?,
        leverage_class: row.get(6)?,
        status: row.get(7)?,
        created_at: row.get(8)?,
        notes: row.get(9)?,
    })
}

pub fn list_by_semester(conn: &Connection, semester_id: i64) -> Result<Vec<DeadlineRow>, DataError> {
    let mut stmt = conn.prepare(
        "SELECT id, semester_id, course_id, title, category, due_at, leverage_class, status, \
         created_at, notes FROM deadlines WHERE semester_id = ?1 ORDER BY due_at",
    )?;
    let rows = stmt
        .query_map(params![semester_id], row_to_deadline)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Every open deadline across all semesters, most urgent first — the
/// evidence Priority Resolution reads (01_ARCHITECTURE.md §3.2 Stage 1).
pub fn list_open(conn: &Connection) -> Result<Vec<DeadlineRow>, DataError> {
    let mut stmt = conn.prepare(
        "SELECT id, semester_id, course_id, title, category, due_at, leverage_class, status, \
         created_at, notes FROM deadlines WHERE status = 'open' ORDER BY due_at",
    )?;
    let rows = stmt.query_map([], row_to_deadline)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Open, `category = 'career'` deadlines — the real read model behind
/// Trajectory's "Career threads" section (04_DATA_MODEL.md §5.2).
pub fn list_open_career(conn: &Connection) -> Result<Vec<DeadlineRow>, DataError> {
    let mut stmt = conn.prepare(
        "SELECT id, semester_id, course_id, title, category, due_at, leverage_class, status, \
         created_at, notes FROM deadlines WHERE status = 'open' AND category = 'career' ORDER BY due_at",
    )?;
    let rows = stmt.query_map([], row_to_deadline)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn insert_deadlines(
    tx: &rusqlite::Transaction<'_>,
    semester_id: i64,
    deadlines: &[NewDeadline],
) -> Result<Vec<i64>, DataError> {
    let mut ids = Vec::with_capacity(deadlines.len());
    for deadline in deadlines {
        tx.execute(
            "INSERT INTO deadlines (semester_id, course_id, title, category, due_at, leverage_class, status, notes) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'open', ?7)",
            params![
                semester_id,
                deadline.course_id,
                deadline.title,
                deadline.category,
                deadline.due_at,
                deadline.leverage_class,
                deadline.notes,
            ],
        )?;
        ids.push(tx.last_insert_rowid());
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::open_and_migrate;
    use crate::repositories::semester;
    use tempfile::NamedTempFile;

    #[test]
    fn insert_and_list_deadlines() {
        let tmp = NamedTempFile::new().unwrap();
        let mut conn = open_and_migrate(tmp.path()).unwrap();
        let tx = conn.transaction().unwrap();
        let semester_id = semester::create_semester(&tx, "Monsoon 2026", "2026-07-15", "2026-11-30").unwrap();
        insert_deadlines(
            &tx,
            semester_id,
            &[NewDeadline {
                course_id: None,
                title: "Test deadline".into(),
                category: "career".into(),
                due_at: "2026-08-10T23:59:00".into(),
                leverage_class: "high".into(),
                notes: None,
            }],
        )
        .unwrap();
        tx.commit().unwrap();

        let deadlines = list_by_semester(&conn, semester_id).unwrap();
        assert_eq!(deadlines.len(), 1);
        let open = list_open(&conn).unwrap();
        assert_eq!(open.len(), 1);
        let career = list_open_career(&conn).unwrap();
        assert_eq!(career.len(), 1);
    }
}
