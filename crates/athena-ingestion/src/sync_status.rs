//! The one sync-status vocabulary every Version 1 connector shares
//! (07_INTEGRATIONS.md §5: "staleness is a first-class, visible state").
//! Mirrors `data_sources.status`'s `CHECK` constraint (V4 migration)
//! exactly, so a round trip through SQLite can never produce a status
//! this enum can't represent.

use serde::Serialize;

/// A connector's current synchronization state, independent of which
/// provider it is. `athena-app`'s Tauri commands are the only thing
/// that maps this to/from the `data_sources.status` TEXT column
/// (`as_str` / `from_str` below) — `athena-ingestion` itself never
/// touches SQL (PROJECT_RULES.md: "athena-data is the only crate
/// allowed to write SQL").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    /// Never configured, no credential/handle on file, or a token was
    /// removed — not an error, just "not connected."
    Disconnected,
    /// Configured but has never successfully synced yet (e.g. an
    /// import-based source before its first file is imported).
    Idle,
    /// A sync is in flight right now.
    Syncing,
    /// Last sync succeeded; `last_synced_at` is current.
    Ok,
    /// Last sync attempt failed; `last_synced_at` (if any) is from a
    /// previous success and must be shown as stale, not current (§0).
    Error,
}

impl SyncStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SyncStatus::Disconnected => "disconnected",
            SyncStatus::Idle => "idle",
            SyncStatus::Syncing => "syncing",
            SyncStatus::Ok => "ok",
            SyncStatus::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "disconnected" => Some(SyncStatus::Disconnected),
            "idle" => Some(SyncStatus::Idle),
            "syncing" => Some(SyncStatus::Syncing),
            "ok" => Some(SyncStatus::Ok),
            "error" => Some(SyncStatus::Error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_every_variant_through_its_string_form() {
        for status in [
            SyncStatus::Disconnected,
            SyncStatus::Idle,
            SyncStatus::Syncing,
            SyncStatus::Ok,
            SyncStatus::Error,
        ] {
            assert_eq!(SyncStatus::from_str(status.as_str()), Some(status));
        }
    }
}
