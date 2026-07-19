//! `courses` repository (04_DATA_MODEL.md §2).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::DataError;

/// A single weekly meeting time, stored as JSON on `courses.meeting_pattern`
/// (04_DATA_MODEL.md §2 — a fixed attribute of the course, not a
/// separate time-series table).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingSlot {
    pub day: String,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CourseRow {
    pub id: i64,
    pub semester_id: i64,
    pub code: String,
    pub title: String,
    pub credits: i64,
    pub leverage_class: String,
    pub instructor: Option<String>,
    pub target_grade: Option<String>,
    pub meeting_pattern: Vec<MeetingSlot>,
    pub status: String,
    pub created_at: String,
}

/// Fields collected by Semester Setup Step 1 (03_ONBOARDING.md §3 Step 1).
#[derive(Debug, Clone)]
pub struct NewCourse {
    pub code: String,
    pub title: String,
    pub credits: i64,
    pub leverage_class: String,
    pub instructor: Option<String>,
    pub target_grade: Option<String>,
    pub meeting_pattern: Vec<MeetingSlot>,
}

fn row_to_course(row: &rusqlite::Row<'_>) -> rusqlite::Result<CourseRow> {
    let meeting_pattern_json: Option<String> = row.get(8)?;
    let meeting_pattern = meeting_pattern_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    Ok(CourseRow {
        id: row.get(0)?,
        semester_id: row.get(1)?,
        code: row.get(2)?,
        title: row.get(3)?,
        credits: row.get(4)?,
        leverage_class: row.get(5)?,
        instructor: row.get(6)?,
        target_grade: row.get(7)?,
        meeting_pattern,
        status: row.get(9)?,
        created_at: row.get(10)?,
    })
}

pub fn list_by_semester(conn: &Connection, semester_id: i64) -> Result<Vec<CourseRow>, DataError> {
    let mut stmt = conn.prepare(
        "SELECT id, semester_id, code, title, credits, leverage_class, instructor, target_grade, \
         meeting_pattern, status, created_at FROM courses WHERE semester_id = ?1 ORDER BY id",
    )?;
    let rows = stmt
        .query_map(params![semester_id], row_to_course)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Inserts every course entered in Semester Setup Step 1, inside an
/// already-open transaction (see `semester::create_semester`).
pub fn insert_courses(
    tx: &rusqlite::Transaction<'_>,
    semester_id: i64,
    courses: &[NewCourse],
) -> Result<Vec<i64>, DataError> {
    let mut ids = Vec::with_capacity(courses.len());
    for course in courses {
        let meeting_pattern_json = serde_json::to_string(&course.meeting_pattern)?;

        tx.execute(
            "INSERT INTO courses (semester_id, code, title, credits, leverage_class, instructor, \
             target_grade, meeting_pattern, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active')",
            params![
                semester_id,
                course.code,
                course.title,
                course.credits,
                course.leverage_class,
                course.instructor,
                course.target_grade,
                meeting_pattern_json,
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
    fn insert_and_list_courses() {
        let tmp = NamedTempFile::new().unwrap();
        let mut conn = open_and_migrate(tmp.path()).unwrap();
        let tx = conn.transaction().unwrap();
        let semester_id = semester::create_semester(&tx, "Monsoon 2026", "2026-07-15", "2026-11-30").unwrap();
        insert_courses(
            &tx,
            semester_id,
            &[NewCourse {
                code: "CS5590".into(),
                title: "Statistical Machine Learning".into(),
                credits: 4,
                leverage_class: "high".into(),
                instructor: None,
                target_grade: None,
                meeting_pattern: vec![],
            }],
        )
        .unwrap();
        tx.commit().unwrap();

        let courses = list_by_semester(&conn, semester_id).unwrap();
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].code, "CS5590");
    }
}
