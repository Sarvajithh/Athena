//! `data_sources` + every connector snapshot table repository
//! (07_INTEGRATIONS.md; V4/V5 migrations).
//!
//! One file covering `data_sources` plus every connector's snapshot
//! table (see `repositories/mod.rs`'s doc comment: "those seven tables
//! are one aggregate in practice — sync status + the typed data it
//! produces"). Per PROJECT_RULES.md ("athena-data is the only crate
//! allowed to write SQL"), this module never depends on `athena_data`
//! itself, `athena_ingestion`, `tauri`, or anything from `athena-app`
//! (`keychain`, `oauth_loopback`) — it only reads/writes SQLite through
//! plain `rusqlite::Connection`/`Transaction` values and returns typed
//! rows/`DataError`. `athena-app`'s `commands::integrations` module is
//! the caller that wires this repository to Tauri IPC and to
//! `athena_ingestion`'s connectors.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::error::DataError;

// ---------------------------------------------------------------------
// data_sources
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct DataSourceRow {
    pub source_key: String,
    pub kind: String,
    pub status: String,
    pub last_synced_at: Option<String>,
    pub last_error: Option<String>,
    pub config_json: Option<String>,
    pub updated_at: String,
}

fn row_to_data_source(row: &rusqlite::Row<'_>) -> rusqlite::Result<DataSourceRow> {
    Ok(DataSourceRow {
        source_key: row.get(0)?,
        kind: row.get(1)?,
        status: row.get(2)?,
        last_synced_at: row.get(3)?,
        last_error: row.get(4)?,
        config_json: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

const DATA_SOURCE_COLUMNS: &str =
    "source_key, kind, status, last_synced_at, last_error, config_json, updated_at";

/// Every connector's current status, in `source_key` order — the one
/// read the Connectors step boots from.
pub fn list_data_sources(conn: &Connection) -> Result<Vec<DataSourceRow>, DataError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {DATA_SOURCE_COLUMNS} FROM data_sources ORDER BY source_key"
    ))?;
    let rows = stmt
        .query_map([], row_to_data_source)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get_data_source(conn: &Connection, source_key: &str) -> Result<Option<DataSourceRow>, DataError> {
    conn.query_row(
        &format!("SELECT {DATA_SOURCE_COLUMNS} FROM data_sources WHERE source_key = ?1"),
        params![source_key],
        row_to_data_source,
    )
    .optional()
    .map_err(DataError::from)
}

/// Saves per-connector configuration (handle/username, linked repo
/// list, etc. — never a credential itself, see §4/`keychain.rs`) as the
/// JSON text `data_sources.config_json` column.
pub fn set_data_source_config(conn: &Connection, source_key: &str, config_json: &str) -> Result<(), DataError> {
    conn.execute(
        "UPDATE data_sources SET config_json = ?1, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE source_key = ?2",
        params![config_json, source_key],
    )?;
    Ok(())
}

pub fn mark_syncing(conn: &Connection, source_key: &str) -> Result<(), DataError> {
    conn.execute(
        "UPDATE data_sources SET status = 'syncing', \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE source_key = ?1",
        params![source_key],
    )?;
    Ok(())
}

pub fn mark_synced_ok(conn: &Connection, source_key: &str, synced_at: &str) -> Result<(), DataError> {
    conn.execute(
        "UPDATE data_sources SET status = 'ok', last_synced_at = ?1, last_error = NULL, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE source_key = ?2",
        params![synced_at, source_key],
    )?;
    Ok(())
}

pub fn mark_synced_error(conn: &Connection, source_key: &str, error: &str) -> Result<(), DataError> {
    conn.execute(
        "UPDATE data_sources SET status = 'error', last_error = ?1, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE source_key = ?2",
        params![error, source_key],
    )?;
    Ok(())
}

/// Clears a connector's status back to `disconnected` (the OAuth
/// connectors' disconnect action) — leaves `config_json`/history rows
/// untouched, only sync state resets.
pub fn mark_disconnected(conn: &Connection, source_key: &str) -> Result<(), DataError> {
    conn.execute(
        "UPDATE data_sources SET status = 'disconnected', last_error = NULL, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE source_key = ?1",
        params![source_key],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------
// codeforces_snapshots (§1.1)
// ---------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NewCodeforcesSnapshot {
    pub handle: String,
    pub rating: Option<i64>,
    pub max_rating: Option<i64>,
    pub rank: Option<String>,
    pub solved_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodeforcesSnapshotRow {
    pub handle: String,
    pub rating: Option<i64>,
    pub max_rating: Option<i64>,
    pub rank: Option<String>,
    pub solved_count: i64,
    pub fetched_at: String,
}

fn row_to_codeforces_snapshot(row: &rusqlite::Row<'_>) -> rusqlite::Result<CodeforcesSnapshotRow> {
    Ok(CodeforcesSnapshotRow {
        handle: row.get(0)?,
        rating: row.get(1)?,
        max_rating: row.get(2)?,
        rank: row.get(3)?,
        solved_count: row.get(4)?,
        fetched_at: row.get(5)?,
    })
}

const CODEFORCES_COLUMNS: &str = "handle, rating, max_rating, rank, solved_count, fetched_at";

pub fn insert_codeforces_snapshot(conn: &Connection, new: &NewCodeforcesSnapshot) -> Result<i64, DataError> {
    conn.execute(
        "INSERT INTO codeforces_snapshots (handle, rating, max_rating, rank, solved_count) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![new.handle, new.rating, new.max_rating, new.rank, new.solved_count],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn latest_codeforces_snapshot(conn: &Connection) -> Result<Option<CodeforcesSnapshotRow>, DataError> {
    conn.query_row(
        &format!(
            "SELECT {CODEFORCES_COLUMNS} FROM codeforces_snapshots ORDER BY fetched_at DESC LIMIT 1"
        ),
        [],
        row_to_codeforces_snapshot,
    )
    .optional()
    .map_err(DataError::from)
}

// ---------------------------------------------------------------------
// dsa_practice_log (§1.2)
// ---------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NewDsaPracticeLog {
    pub source: String,
    pub handle: String,
    pub total_solved: i64,
    pub easy_solved: i64,
    pub medium_solved: i64,
    pub hard_solved: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DsaPracticeLogRow {
    pub handle: String,
    pub total_solved: i64,
    pub easy_solved: i64,
    pub medium_solved: i64,
    pub hard_solved: i64,
    pub fetched_at: String,
}

fn row_to_dsa_practice_log(row: &rusqlite::Row<'_>) -> rusqlite::Result<DsaPracticeLogRow> {
    Ok(DsaPracticeLogRow {
        handle: row.get(0)?,
        total_solved: row.get(1)?,
        easy_solved: row.get(2)?,
        medium_solved: row.get(3)?,
        hard_solved: row.get(4)?,
        fetched_at: row.get(5)?,
    })
}

const DSA_PRACTICE_LOG_COLUMNS: &str = "handle, total_solved, easy_solved, medium_solved, hard_solved, fetched_at";

pub fn insert_dsa_practice_log(conn: &Connection, new: &NewDsaPracticeLog) -> Result<i64, DataError> {
    conn.execute(
        "INSERT INTO dsa_practice_log (source, handle, total_solved, easy_solved, medium_solved, hard_solved) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            new.source,
            new.handle,
            new.total_solved,
            new.easy_solved,
            new.medium_solved,
            new.hard_solved,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn latest_dsa_practice_log(conn: &Connection, source: &str) -> Result<Option<DsaPracticeLogRow>, DataError> {
    conn.query_row(
        &format!(
            "SELECT {DSA_PRACTICE_LOG_COLUMNS} FROM dsa_practice_log \
             WHERE source = ?1 ORDER BY fetched_at DESC LIMIT 1"
        ),
        params![source],
        row_to_dsa_practice_log,
    )
    .optional()
    .map_err(DataError::from)
}

// ---------------------------------------------------------------------
// linked_github_repos + project_status_snapshots (§1.3)
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct LinkedGithubRepoRow {
    pub repo_full_name: String,
    pub added_at: String,
}

fn row_to_linked_github_repo(row: &rusqlite::Row<'_>) -> rusqlite::Result<LinkedGithubRepoRow> {
    Ok(LinkedGithubRepoRow {
        repo_full_name: row.get(0)?,
        added_at: row.get(1)?,
    })
}

/// Links a repo the user explicitly named (never a full account scan,
/// §1.3). Idempotent: re-linking an already-linked repo is a no-op
/// rather than a uniqueness error, since the Connectors step's "Add"
/// action has no reason to distinguish "already linked" from "just
/// linked" for the caller.
pub fn link_github_repo(conn: &Connection, repo_full_name: &str) -> Result<(), DataError> {
    conn.execute(
        "INSERT OR IGNORE INTO linked_github_repos (repo_full_name) VALUES (?1)",
        params![repo_full_name],
    )?;
    Ok(())
}

pub fn unlink_github_repo(conn: &Connection, repo_full_name: &str) -> Result<(), DataError> {
    conn.execute(
        "DELETE FROM linked_github_repos WHERE repo_full_name = ?1",
        params![repo_full_name],
    )?;
    Ok(())
}

pub fn list_linked_github_repos(conn: &Connection) -> Result<Vec<LinkedGithubRepoRow>, DataError> {
    let mut stmt =
        conn.prepare("SELECT repo_full_name, added_at FROM linked_github_repos ORDER BY added_at")?;
    let rows = stmt
        .query_map([], row_to_linked_github_repo)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[derive(Debug, Clone)]
pub struct NewProjectStatusSnapshot {
    pub repo_full_name: String,
    pub commit_count_30d: i64,
    pub open_pr_count: i64,
    pub open_issue_count: i64,
    pub last_commit_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectStatusSnapshotRow {
    pub repo_full_name: String,
    pub commit_count_30d: i64,
    pub open_pr_count: i64,
    pub open_issue_count: i64,
    pub last_commit_at: Option<String>,
    pub fetched_at: String,
}

fn row_to_project_status_snapshot(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectStatusSnapshotRow> {
    Ok(ProjectStatusSnapshotRow {
        repo_full_name: row.get(0)?,
        commit_count_30d: row.get(1)?,
        open_pr_count: row.get(2)?,
        open_issue_count: row.get(3)?,
        last_commit_at: row.get(4)?,
        fetched_at: row.get(5)?,
    })
}

const PROJECT_STATUS_COLUMNS: &str =
    "repo_full_name, commit_count_30d, open_pr_count, open_issue_count, last_commit_at, fetched_at";

pub fn insert_project_status_snapshot(conn: &Connection, new: &NewProjectStatusSnapshot) -> Result<i64, DataError> {
    conn.execute(
        "INSERT INTO project_status_snapshots \
         (repo_full_name, commit_count_30d, open_pr_count, open_issue_count, last_commit_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            new.repo_full_name,
            new.commit_count_30d,
            new.open_pr_count,
            new.open_issue_count,
            new.last_commit_at,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// The most recent snapshot per linked repo — one row per
/// `repo_full_name`, newest `fetched_at` first, matching
/// `list_linked_github_repos`' own granularity (a status card per repo,
/// not a full history).
pub fn latest_project_status_snapshots(conn: &Connection) -> Result<Vec<ProjectStatusSnapshotRow>, DataError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {PROJECT_STATUS_COLUMNS} FROM project_status_snapshots p1 \
         WHERE fetched_at = (SELECT MAX(fetched_at) FROM project_status_snapshots p2 \
         WHERE p2.repo_full_name = p1.repo_full_name) \
         ORDER BY repo_full_name"
    ))?;
    let rows = stmt
        .query_map([], row_to_project_status_snapshot)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

// ---------------------------------------------------------------------
// gmail_message_snapshots (§1.8)
// ---------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NewGmailMessageSnapshot {
    pub message_id: String,
    pub thread_id: Option<String>,
    pub sender: Option<String>,
    pub subject: Option<String>,
    pub received_at: Option<String>,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GmailMessageSnapshotRow {
    pub message_id: String,
    pub thread_id: Option<String>,
    pub sender: Option<String>,
    pub subject: Option<String>,
    pub received_at: Option<String>,
    pub snippet: Option<String>,
    pub fetched_at: String,
}

fn row_to_gmail_message_snapshot(row: &rusqlite::Row<'_>) -> rusqlite::Result<GmailMessageSnapshotRow> {
    Ok(GmailMessageSnapshotRow {
        message_id: row.get(0)?,
        thread_id: row.get(1)?,
        sender: row.get(2)?,
        subject: row.get(3)?,
        received_at: row.get(4)?,
        snippet: row.get(5)?,
        fetched_at: row.get(6)?,
    })
}

const GMAIL_MESSAGE_COLUMNS: &str = "message_id, thread_id, sender, subject, received_at, snippet, fetched_at";

/// `message_id` is globally unique per Gmail's own API contract, so
/// re-polling upserts the same logical message in place (V5 migration's
/// doc comment) rather than growing an unbounded duplicate history.
pub fn upsert_gmail_message_snapshot(conn: &Connection, new: &NewGmailMessageSnapshot) -> Result<(), DataError> {
    conn.execute(
        "INSERT INTO gmail_message_snapshots \
         (message_id, thread_id, sender, subject, received_at, snippet, fetched_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) \
         ON CONFLICT(message_id) DO UPDATE SET \
         thread_id = excluded.thread_id, sender = excluded.sender, subject = excluded.subject, \
         received_at = excluded.received_at, snippet = excluded.snippet, \
         fetched_at = excluded.fetched_at",
        params![new.message_id, new.thread_id, new.sender, new.subject, new.received_at, new.snippet],
    )?;
    Ok(())
}

pub fn list_gmail_message_snapshots(conn: &Connection) -> Result<Vec<GmailMessageSnapshotRow>, DataError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {GMAIL_MESSAGE_COLUMNS} FROM gmail_message_snapshots ORDER BY received_at DESC"
    ))?;
    let rows = stmt
        .query_map([], row_to_gmail_message_snapshot)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

// ---------------------------------------------------------------------
// classroom_courses / classroom_coursework / classroom_announcements (§1.9)
// ---------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NewClassroomCourse {
    pub course_id: String,
    pub name: String,
    pub section: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassroomCourseRow {
    pub course_id: String,
    pub name: String,
    pub section: Option<String>,
    pub fetched_at: String,
}

fn row_to_classroom_course(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClassroomCourseRow> {
    Ok(ClassroomCourseRow {
        course_id: row.get(0)?,
        name: row.get(1)?,
        section: row.get(2)?,
        fetched_at: row.get(3)?,
    })
}

pub fn upsert_classroom_course(conn: &Connection, new: &NewClassroomCourse) -> Result<(), DataError> {
    conn.execute(
        "INSERT INTO classroom_courses (course_id, name, section, fetched_at) \
         VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) \
         ON CONFLICT(course_id) DO UPDATE SET \
         name = excluded.name, section = excluded.section, fetched_at = excluded.fetched_at",
        params![new.course_id, new.name, new.section],
    )?;
    Ok(())
}

pub fn list_classroom_courses(conn: &Connection) -> Result<Vec<ClassroomCourseRow>, DataError> {
    let mut stmt = conn.prepare(
        "SELECT course_id, name, section, fetched_at FROM classroom_courses ORDER BY name",
    )?;
    let rows = stmt
        .query_map([], row_to_classroom_course)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[derive(Debug, Clone)]
pub struct NewClassroomCoursework {
    pub course_id: String,
    pub coursework_id: String,
    pub title: String,
    pub due_at: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassroomCourseworkRow {
    pub coursework_id: String,
    pub course_id: String,
    pub title: String,
    pub due_at: Option<String>,
    pub state: Option<String>,
    pub fetched_at: String,
}

fn row_to_classroom_coursework(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClassroomCourseworkRow> {
    Ok(ClassroomCourseworkRow {
        coursework_id: row.get(0)?,
        course_id: row.get(1)?,
        title: row.get(2)?,
        due_at: row.get(3)?,
        state: row.get(4)?,
        fetched_at: row.get(5)?,
    })
}

pub fn upsert_classroom_coursework(conn: &Connection, new: &NewClassroomCoursework) -> Result<(), DataError> {
    conn.execute(
        "INSERT INTO classroom_coursework (coursework_id, course_id, title, due_at, state, fetched_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) \
         ON CONFLICT(coursework_id) DO UPDATE SET \
         course_id = excluded.course_id, title = excluded.title, due_at = excluded.due_at, \
         state = excluded.state, fetched_at = excluded.fetched_at",
        params![new.coursework_id, new.course_id, new.title, new.due_at, new.state],
    )?;
    Ok(())
}

pub fn list_classroom_coursework(conn: &Connection) -> Result<Vec<ClassroomCourseworkRow>, DataError> {
    let mut stmt = conn.prepare(
        "SELECT coursework_id, course_id, title, due_at, state, fetched_at \
         FROM classroom_coursework ORDER BY due_at",
    )?;
    let rows = stmt
        .query_map([], row_to_classroom_coursework)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[derive(Debug, Clone)]
pub struct NewClassroomAnnouncement {
    pub course_id: String,
    pub announcement_id: String,
    pub text: Option<String>,
    pub posted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassroomAnnouncementRow {
    pub announcement_id: String,
    pub course_id: String,
    pub text: Option<String>,
    pub posted_at: Option<String>,
    pub fetched_at: String,
}

fn row_to_classroom_announcement(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClassroomAnnouncementRow> {
    Ok(ClassroomAnnouncementRow {
        announcement_id: row.get(0)?,
        course_id: row.get(1)?,
        text: row.get(2)?,
        posted_at: row.get(3)?,
        fetched_at: row.get(4)?,
    })
}

pub fn upsert_classroom_announcement(conn: &Connection, new: &NewClassroomAnnouncement) -> Result<(), DataError> {
    conn.execute(
        "INSERT INTO classroom_announcements (announcement_id, course_id, text, posted_at, fetched_at) \
         VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) \
         ON CONFLICT(announcement_id) DO UPDATE SET \
         course_id = excluded.course_id, text = excluded.text, posted_at = excluded.posted_at, \
         fetched_at = excluded.fetched_at",
        params![new.announcement_id, new.course_id, new.text, new.posted_at],
    )?;
    Ok(())
}

pub fn list_classroom_announcements(conn: &Connection) -> Result<Vec<ClassroomAnnouncementRow>, DataError> {
    let mut stmt = conn.prepare(
        "SELECT announcement_id, course_id, text, posted_at, fetched_at \
         FROM classroom_announcements ORDER BY posted_at DESC",
    )?;
    let rows = stmt
        .query_map([], row_to_classroom_announcement)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

// ---------------------------------------------------------------------
// notion_pages (§1.10)
// ---------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NewNotionPage {
    pub page_id: String,
    pub title: Option<String>,
    pub url: Option<String>,
    pub parent_database_id: Option<String>,
    pub last_edited_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotionPageRow {
    pub page_id: String,
    pub title: Option<String>,
    pub url: Option<String>,
    pub parent_database_id: Option<String>,
    pub last_edited_at: Option<String>,
    pub fetched_at: String,
}

fn row_to_notion_page(row: &rusqlite::Row<'_>) -> rusqlite::Result<NotionPageRow> {
    Ok(NotionPageRow {
        page_id: row.get(0)?,
        title: row.get(1)?,
        url: row.get(2)?,
        parent_database_id: row.get(3)?,
        last_edited_at: row.get(4)?,
        fetched_at: row.get(5)?,
    })
}

pub fn upsert_notion_page(conn: &Connection, new: &NewNotionPage) -> Result<(), DataError> {
    conn.execute(
        "INSERT INTO notion_pages (page_id, title, url, parent_database_id, last_edited_at, fetched_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) \
         ON CONFLICT(page_id) DO UPDATE SET \
         title = excluded.title, url = excluded.url, parent_database_id = excluded.parent_database_id, \
         last_edited_at = excluded.last_edited_at, fetched_at = excluded.fetched_at",
        params![new.page_id, new.title, new.url, new.parent_database_id, new.last_edited_at],
    )?;
    Ok(())
}

pub fn list_notion_pages(conn: &Connection) -> Result<Vec<NotionPageRow>, DataError> {
    let mut stmt = conn.prepare(
        "SELECT page_id, title, url, parent_database_id, last_edited_at, fetched_at \
         FROM notion_pages ORDER BY last_edited_at DESC",
    )?;
    let rows = stmt
        .query_map([], row_to_notion_page)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::open_and_migrate;
    use tempfile::NamedTempFile;

    fn open_db() -> Connection {
        let tmp = NamedTempFile::new().unwrap();
        open_and_migrate(tmp.path()).unwrap()
    }

    #[test]
    fn data_sources_are_seeded_and_status_transitions_persist() {
        let conn = open_db();
        let sources = list_data_sources(&conn).unwrap();
        assert!(sources.iter().any(|s| s.source_key == "codeforces" && s.status == "disconnected"));

        mark_syncing(&conn, "codeforces").unwrap();
        assert_eq!(get_data_source(&conn, "codeforces").unwrap().unwrap().status, "syncing");

        mark_synced_ok(&conn, "codeforces", "2026-07-17T00:00:00.000Z").unwrap();
        let row = get_data_source(&conn, "codeforces").unwrap().unwrap();
        assert_eq!(row.status, "ok");
        assert_eq!(row.last_synced_at.as_deref(), Some("2026-07-17T00:00:00.000Z"));

        mark_synced_error(&conn, "codeforces", "network down").unwrap();
        let row = get_data_source(&conn, "codeforces").unwrap().unwrap();
        assert_eq!(row.status, "error");
        assert_eq!(row.last_error.as_deref(), Some("network down"));
    }

    #[test]
    fn codeforces_snapshots_insert_and_read_back_latest() {
        let conn = open_db();
        insert_codeforces_snapshot(
            &conn,
            &NewCodeforcesSnapshot {
                handle: "tourist".into(),
                rating: Some(3800),
                max_rating: Some(4000),
                rank: Some("legendary grandmaster".into()),
                solved_count: 2500,
            },
        )
        .unwrap();
        let latest = latest_codeforces_snapshot(&conn).unwrap().unwrap();
        assert_eq!(latest.handle, "tourist");
        assert_eq!(latest.solved_count, 2500);
    }

    #[test]
    fn linked_github_repos_link_list_and_unlink() {
        let conn = open_db();
        link_github_repo(&conn, "octocat/Hello-World").unwrap();
        link_github_repo(&conn, "octocat/Hello-World").unwrap(); // idempotent
        assert_eq!(list_linked_github_repos(&conn).unwrap().len(), 1);

        unlink_github_repo(&conn, "octocat/Hello-World").unwrap();
        assert!(list_linked_github_repos(&conn).unwrap().is_empty());
    }

    #[test]
    fn gmail_message_snapshots_upsert_by_message_id() {
        let conn = open_db();
        let new = NewGmailMessageSnapshot {
            message_id: "msg-1".into(),
            thread_id: Some("thread-1".into()),
            sender: Some("a@example.com".into()),
            subject: Some("Hello".into()),
            received_at: Some("2026-07-17T00:00:00.000Z".into()),
            snippet: Some("hi".into()),
        };
        upsert_gmail_message_snapshot(&conn, &new).unwrap();
        upsert_gmail_message_snapshot(&conn, &new).unwrap();
        assert_eq!(list_gmail_message_snapshots(&conn).unwrap().len(), 1);
    }

    #[test]
    fn notion_pages_upsert_by_page_id() {
        let conn = open_db();
        let new = NewNotionPage {
            page_id: "page-1".into(),
            title: Some("Notes".into()),
            url: Some("https://notion.so/page-1".into()),
            parent_database_id: None,
            last_edited_at: Some("2026-07-17T00:00:00.000Z".into()),
        };
        upsert_notion_page(&conn, &new).unwrap();
        assert_eq!(list_notion_pages(&conn).unwrap().len(), 1);
    }
}
