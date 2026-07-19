//! IPC commands. `get_app_version` below was S01's one health-check/
//! version round trip (SPRINT1_SPEC.md Objective 3); every command in
//! this crate is named as an imperative verb per Implementation Plan
//! §2.3. `integrations` (07_INTEGRATIONS.md) is the connector surface.
//! `ai` (06_AI_ENGINE.md) is the AI layer's surface — every command
//! there phrases a verdict `athena-domain` already computed, never
//! computes one of its own.
pub mod ai;
pub mod bootstrap;
pub mod integrations;
pub mod onboarding;
pub mod planner;
pub mod routine;
use serde::Serialize;

/// Whether credential storage is currently falling back to the local
/// encrypted file store because the OS keychain backend is unavailable
/// (Task 4, keychain.rs's `fallback` submodule). A minimal, standalone
/// command living here rather than in a new file — `get_app_version`
/// above is this crate's existing precedent for a free command with no
/// dedicated module of its own, and Task 4's scope note restricts that
/// task to `keychain.rs` and `Cargo.toml`; this one extra line here (and
/// its registration in `main.rs`'s `generate_handler!`) is the minimal
/// exception needed so the frontend can actually surface Task 4's
/// required "stored locally, not in your OS keychain" notice — that
/// requirement is otherwise unreachable from a Settings screen that
/// only ever talks to the backend over IPC.
#[tauri::command]
pub fn is_using_keychain_fallback() -> bool {
    crate::keychain::is_using_fallback_storage()
}

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