//! The one IPC command this sprint proves end-to-end (SPRINT1_SPEC.md
//! Objective 3). Named as an imperative verb per Implementation Plan
//! §2.3 ("Commands are always named as imperative verbs").
//!
//! This is deliberately the only command registered in S01 — a
//! health-check/version round trip, nothing domain-related yet.
pub mod bootstrap;
pub mod onboarding;
pub mod planner;
use serde::Serialize;

/// The shape returned to the frontend. Kept as an explicit typed struct
/// (never a bare string) so the TypeScript binding in
/// `src/ipc/bindings.ts` has a real contract to check against, per the
/// IPC contract-check tier established this sprint (SPRINT1_SPEC.md §7).
#[derive(Debug, Clone, Serialize)]
pub struct AppVersionInfo {
    pub version: String,
}

/// Returns the running app's version. This is the one proof-of-life IPC
/// round trip for S01 (SPRINT1_SPEC.md Objective 3) — no domain logic,
/// no persistence read, just proof that Rust -> IPC -> TypeScript works.
#[tauri::command]
pub fn get_app_version() -> AppVersionInfo {
    AppVersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_app_version_returns_a_nonempty_version_string() {
        let info = get_app_version();
        assert!(!info.version.is_empty());
    }
}