//! `event_log` repository — append-only, every event persisted
//! unconditionally (01_ARCHITECTURE.md §6.4, §2.2).

use rusqlite::params;
use serde_json::Value;

use crate::error::DataError;

/// Persists one event. Takes a `rusqlite::Transaction` so it can be
/// written atomically alongside the write it documents (e.g.
/// `ProfileCreated` inside the same transaction as the `user_profile`
/// insert).
pub fn insert_event(tx: &rusqlite::Transaction<'_>, event_type: &str, payload: &Value) -> Result<(), DataError> {
    tx.execute(
        "INSERT INTO event_log (event_type, payload) VALUES (?1, ?2)",
        params![event_type, payload.to_string()],
    )?;
    Ok(())
}
