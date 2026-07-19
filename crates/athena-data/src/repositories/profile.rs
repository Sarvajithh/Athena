//! `user_profile` / `user_profile_history` repository (04_DATA_MODEL.md
//! §1). `user_profile` is the one table where "current state" is
//! genuinely mutable (01_ARCHITECTURE.md §2.3's named exception);
//! `user_profile_history` is append-only.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::error::DataError;

/// Current-state row, returned to the frontend unmodified in shape
/// (01_ARCHITECTURE.md §2.1).
#[derive(Debug, Clone, Serialize)]
pub struct ProfileRow {
    pub id: i64,
    pub name: String,
    pub institute: String,
    pub program: String,
    pub current_semester_id: Option<i64>,
    pub target_cgpa: f64,
    pub current_cgpa: Option<f64>,
    pub career_target: String,
    pub masters_target: Option<String>,
    pub codeforces_handle: Option<String>,
    pub deep_work_window_start: String,
    pub deep_work_window_end: String,
    pub timezone: String,
    /// `HH:MM`, 24-hour, local time — when the scheduled daily-
    /// questionnaire trigger should fire (V7 migration). Defaults to
    /// `"20:00"` for every row via the column's own `DEFAULT`.
    pub routine_questionnaire_time: String,
    pub created_at: String,
    pub updated_at: String,
}

/// The exact fields collected by Profile Creation, Steps 1-4
/// (03_ONBOARDING.md §2). Nothing here is decorative — every field maps
/// directly to a `user_profile` column.
#[derive(Debug, Clone)]
pub struct NewProfile {
    pub name: String,
    pub institute: String,
    pub program: String,
    pub target_cgpa: f64,
    pub current_cgpa: Option<f64>,
    pub career_target: String,
    pub masters_target: Option<String>,
    pub codeforces_handle: Option<String>,
    pub deep_work_window_start: String,
    pub deep_work_window_end: String,
    pub timezone: String,
}

/// 03_ONBOARDING.md §1: "On app boot, athena-app checks whether a
/// `user_profile` row exists." Since `user_profile` is a single-row
/// table by product rule (not a schema constraint), existence of any row
/// answers the question.
pub fn has_profile(conn: &Connection) -> Result<bool, DataError> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM user_profile", [], |row| row.get(0))?;
    Ok(count > 0)
}

pub fn get_current_profile(conn: &Connection) -> Result<Option<ProfileRow>, DataError> {
    conn.query_row(
        "SELECT id, name, institute, program, current_semester_id, target_cgpa, current_cgpa, \
         career_target, masters_target, codeforces_handle, deep_work_window_start, \
         deep_work_window_end, timezone, routine_questionnaire_time, created_at, updated_at \
         FROM user_profile ORDER BY id LIMIT 1",
        [],
        row_to_profile,
    )
    .optional()
    .map_err(DataError::from)
}

fn row_to_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProfileRow> {
    Ok(ProfileRow {
        id: row.get(0)?,
        name: row.get(1)?,
        institute: row.get(2)?,
        program: row.get(3)?,
        current_semester_id: row.get(4)?,
        target_cgpa: row.get(5)?,
        current_cgpa: row.get(6)?,
        career_target: row.get(7)?,
        masters_target: row.get(8)?,
        codeforces_handle: row.get(9)?,
        deep_work_window_start: row.get(10)?,
        deep_work_window_end: row.get(11)?,
        timezone: row.get(12)?,
        routine_questionnaire_time: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

/// Reads just the configured questionnaire time, without paying for a
/// full `ProfileRow` fetch — `None` means no `user_profile` row exists
/// yet (pre-onboarding); the command layer falls back to `"20:00"` in
/// that case, same as a fresh row's own column default would.
pub fn get_routine_questionnaire_time(conn: &Connection) -> Result<Option<String>, DataError> {
    conn.query_row(
        "SELECT routine_questionnaire_time FROM user_profile ORDER BY id LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(DataError::from)
}

/// Updates the configured questionnaire time on the single
/// `user_profile` row. A no-op (`Ok(())`, zero rows affected) if no
/// profile exists yet — there is nothing to save onto before
/// onboarding, and the Settings screen that calls this is unreachable
/// until onboarding completes anyway (`App.tsx`'s `needsOnboarding`
/// gate), so this is a defensive fallback, not a reachable path today.
pub fn set_routine_questionnaire_time(conn: &Connection, time: &str) -> Result<(), DataError> {
    conn.execute(
        "UPDATE user_profile SET routine_questionnaire_time = ?1, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = (SELECT id FROM user_profile ORDER BY id LIMIT 1)",
        params![time],
    )?;
    Ok(())
}

/// Commits Profile Creation Step 5 (03_ONBOARDING.md §2, Step 5):
/// one `user_profile` row plus one `user_profile_history` row,
/// `reason: "onboarding"`, in the same transaction.
pub fn create_profile_with_history(conn: &mut Connection, input: &NewProfile) -> Result<i64, DataError> {
    let tx = conn.transaction()?;

    tx.execute(
        "INSERT INTO user_profile (\
            name, institute, program, target_cgpa, current_cgpa, career_target, \
            masters_target, codeforces_handle, deep_work_window_start, deep_work_window_end, timezone\
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            input.name,
            input.institute,
            input.program,
            input.target_cgpa,
            input.current_cgpa,
            input.career_target,
            input.masters_target,
            input.codeforces_handle,
            input.deep_work_window_start,
            input.deep_work_window_end,
            input.timezone,
        ],
    )?;
    let profile_id = tx.last_insert_rowid();

    let snapshot = serde_json::json!({
        "target_cgpa": input.target_cgpa,
        "current_cgpa": input.current_cgpa,
        "career_target": input.career_target,
        "masters_target": input.masters_target,
        "deep_work_window_start": input.deep_work_window_start,
        "deep_work_window_end": input.deep_work_window_end,
    })
    .to_string();

    tx.execute(
        "INSERT INTO user_profile_history (user_profile_id, semester_id, reason, snapshot, changed_fields) \
         VALUES (?1, NULL, 'onboarding', ?2, ?3)",
        params![profile_id, snapshot, "[]"],
    )?;

    crate::repositories::event_log::insert_event(
        &tx,
        "ProfileCreated",
        &serde_json::json!({ "user_profile_id": profile_id }),
    )?;

    tx.commit()?;
    Ok(profile_id)
}

/// Called at the end of Semester Setup's own commit (03_ONBOARDING.md
/// §3 Step 5) — Semester Setup always writes its own
/// `user_profile_history` row too, even though it doesn't necessarily
/// change profile fields, "so this preserves a clean 'profile as of the
/// start of every semester' audit trail."
pub fn record_semester_setup_history(
    tx: &rusqlite::Transaction<'_>,
    profile: &ProfileRow,
    semester_id: i64,
    reason: &str,
) -> Result<(), DataError> {
    let snapshot = serde_json::json!({
        "target_cgpa": profile.target_cgpa,
        "current_cgpa": profile.current_cgpa,
        "career_target": profile.career_target,
        "masters_target": profile.masters_target,
        "deep_work_window_start": profile.deep_work_window_start,
        "deep_work_window_end": profile.deep_work_window_end,
    })
    .to_string();

    tx.execute(
        "INSERT INTO user_profile_history (user_profile_id, semester_id, reason, snapshot, changed_fields) \
         VALUES (?1, ?2, ?3, ?4, '[]')",
        params![profile.id, semester_id, reason, snapshot],
    )?;
    Ok(())
}

/// Points the profile at the newly-created current semester
/// (`current_semester_id`), and refreshes `updated_at`.
pub fn set_current_semester(tx: &rusqlite::Transaction<'_>, profile_id: i64, semester_id: i64) -> Result<(), DataError> {
    tx.execute(
        "UPDATE user_profile SET current_semester_id = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?2",
        params![semester_id, profile_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::open_and_migrate;
    use tempfile::NamedTempFile;

    fn new_profile() -> NewProfile {
        NewProfile {
            name: "Test Student".into(),
            institute: "Test Institute".into(),
            program: "Test Program".into(),
            target_cgpa: 8.8,
            current_cgpa: None,
            career_target: "Test career target".into(),
            masters_target: None,
            codeforces_handle: None,
            deep_work_window_start: "20:00".into(),
            deep_work_window_end: "00:00".into(),
            timezone: "Asia/Kolkata".into(),
        }
    }

    #[test]
    fn no_profile_on_fresh_db() {
        let tmp = NamedTempFile::new().unwrap();
        let conn = open_and_migrate(tmp.path()).unwrap();
        assert!(!has_profile(&conn).unwrap());
        assert!(get_current_profile(&conn).unwrap().is_none());
    }

    #[test]
    fn create_profile_writes_row_and_history() {
        let tmp = NamedTempFile::new().unwrap();
        let mut conn = open_and_migrate(tmp.path()).unwrap();
        let id = create_profile_with_history(&mut conn, &new_profile()).unwrap();
        assert!(has_profile(&conn).unwrap());
        let profile = get_current_profile(&conn).unwrap().unwrap();
        assert_eq!(profile.id, id);
        assert_eq!(profile.name, "Test Student");

        let history_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM user_profile_history", [], |r| r.get(0))
            .unwrap();
        assert_eq!(history_count, 1);
    }

    #[test]
    fn routine_questionnaire_time_defaults_and_updates() {
        let tmp = NamedTempFile::new().unwrap();
        let mut conn = open_and_migrate(tmp.path()).unwrap();

        // No profile yet — nothing to read.
        assert!(get_routine_questionnaire_time(&conn).unwrap().is_none());

        create_profile_with_history(&mut conn, &new_profile()).unwrap();

        // Column default applies to the freshly-inserted row.
        assert_eq!(get_routine_questionnaire_time(&conn).unwrap().as_deref(), Some("20:00"));

        set_routine_questionnaire_time(&conn, "07:45").unwrap();
        assert_eq!(get_routine_questionnaire_time(&conn).unwrap().as_deref(), Some("07:45"));

        // Also visible through the full profile row.
        let profile = get_current_profile(&conn).unwrap().unwrap();
        assert_eq!(profile.routine_questionnaire_time, "07:45");
    }
}
