//! `decisions` repository (04_DATA_MODEL.md §9).
//!
//! Read-only this sprint: the Decision Challenge Layer that would write
//! rows here does not exist yet (01_ARCHITECTURE.md §1.1 — a future
//! sprint). This repository exists now so Decision Log queries a real,
//! honestly-empty table instead of a mock fixture (objective 6).

use rusqlite::Connection;
use serde::Serialize;

use crate::error::DataError;

#[derive(Debug, Clone, Serialize)]
pub struct DecisionRow {
    pub id: i64,
    pub semester_id: i64,
    pub decision_type: String,
    pub description: String,
    pub challenge_fired: bool,
    pub challenge_reasoning: Option<String>,
    pub final_outcome: Option<String>,
    pub decided_at: String,
}

pub fn list_recent(conn: &Connection, limit: i64) -> Result<Vec<DecisionRow>, DataError> {
    let mut stmt = conn.prepare(
        "SELECT id, semester_id, decision_type, description, challenge_fired, challenge_reasoning, \
         final_outcome, decided_at FROM decisions ORDER BY decided_at DESC LIMIT ?1",
    )?;
    let rows = stmt
        .query_map([limit], |row| {
            Ok(DecisionRow {
                id: row.get(0)?,
                semester_id: row.get(1)?,
                decision_type: row.get(2)?,
                description: row.get(3)?,
                challenge_fired: row.get::<_, i64>(4)? != 0,
                challenge_reasoning: row.get(5)?,
                final_outcome: row.get(6)?,
                decided_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
