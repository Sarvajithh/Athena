//! Opens the single SQLite file, enables WAL mode, and runs pending
//! migrations. Per Implementation Plan §9 ("migration runner is part of
//! `athena-app`'s startup sequence, runs automatically... and is
//! idempotent"), this module is the mechanism `athena-app::main` calls at
//! boot — it contains no CLI, no manual migration step.

use std::path::Path;

use rusqlite::Connection;

use crate::error::DataError;

/// Opens (creating if necessary) the SQLite database file at `path`,
/// enables WAL mode (Implementation Plan §1/§9 — "WAL mode enabled for
/// concurrent read/write from the app process"), and applies any pending
/// migrations via the embedded `refinery` runner.
///
/// Safe to call on every launch: `refinery` tracks applied migrations in
/// its own bookkeeping table and this function is a no-op on migrations
/// beyond re-verifying nothing is pending (SPRINT1_SPEC.md Acceptance
/// Criteria #7, #8).
pub fn open_and_migrate(path: &Path) -> Result<Connection, DataError> {
    let mut conn = Connection::open(path)?;

    // WAL mode, per Implementation Plan §9. `PRAGMA journal_mode` returns
    // the resulting mode as a row; we assert it took effect.
    let mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
    debug_assert_eq!(mode.to_lowercase(), "wal");

    crate::migrations::runner().run(&mut conn)?;
    // (`crate::migrations` is re-exported from the `embedded` module in
    // lib.rs, which is where `refinery::embed_migrations!` generates it.)

    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn opens_enables_wal_and_migrates_idempotently() {
        let tmp = NamedTempFile::new().expect("create temp file");
        let path = tmp.path();

        // First launch: fresh DB, applies V1__baseline.
        let conn = open_and_migrate(path).expect("first open+migrate should succeed");
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read journal_mode");
        assert_eq!(mode.to_lowercase(), "wal");
        drop(conn);

        // Second launch: same file, already migrated — must not error or
        // re-apply (SPRINT1_SPEC.md Acceptance Criterion #8).
        let conn2 = open_and_migrate(path).expect("second open+migrate should be idempotent");

        let applied: i64 = conn2
            .query_row(
                "SELECT COUNT(*) FROM refinery_schema_history",
                [],
                |row| row.get(0),
            )
            .expect("query refinery_schema_history");
        assert_eq!(
            applied,
            7,
            "exactly 7 migrations should be recorded as applied"
        );

        // MT-6 / Acceptance Criterion #4 (Objective 4, extended by V4's
        // integrations schema): no unexpected domain table exists —
        // only the migration bookkeeping table and the tables V2-V4
        // actually create (7 + 1 + 5 = 13).
        let domain_tables: i64 = conn2
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master \
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
                 AND name != 'refinery_schema_history'",
                [],
                |row| row.get(0),
            )
            .expect("query sqlite_master");
        assert_eq!(
            domain_tables,
            20,
            "all expected domain tables should exist after migrations"
        );
    }
}
