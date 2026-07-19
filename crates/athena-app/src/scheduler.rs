//! Background polling for the connectors with a meaningful refresh
//! cadence — Codeforces, LeetCode, GitHub (07_INTEGRATIONS.md §5: "polls
//! on a scheduled cadence... independent of the app being in the
//! foreground"), plus Gmail, Google Classroom, and Notion as of the
//! 2026-07-17 OAuth amendment (§1.8-§1.10). Calendar/PDF/CSV import and
//! Manual entry have no cadence to schedule — they are one-shot, user-
//! initiated actions (`commands::integrations`'s own module doc).
//!
//! `spawn` is called once, from `main.rs`'s `setup`, **after**
//! `app.manage(...)` for the DB connection has already returned — the
//! interval loop below only ever starts ticking on a background async
//! task (`tauri::async_runtime::spawn`), so it can never delay the
//! window opening (07_INTEGRATIONS.md: "never block startup waiting for
//! integrations"). This holds identically for the three OAuth
//! connectors: a missing/expired/revoked token degrades that one
//! source's tick to an `error` status, never a panic, never a delay to
//! any other source's tick or to startup itself.

use std::sync::Mutex;
use std::time::Duration;

use athena_data::repositories::integrations as integrations_repo;
use rusqlite::Connection;
use tauri::Manager;
use tracing::{info, warn};

/// How often a background tick checks every configured poll source.
/// 30 minutes: frequent enough that a Divergence Check
/// (`06_AI_ENGINE.md` §7.4) run first-thing-in-the-morning is working
/// from same-day data, infrequent enough to stay well inside every
/// provider's public, keyless rate limit (§1.1/§1.2/§1.3's shared
/// "read-only, no destructive scope" framing implies "polite cadence"
/// as its natural companion).
const POLL_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// Spawns the background polling loop. Fire-and-forget: nothing calls
/// `.await` on the returned handle, and nothing in `setup` waits on
/// this before returning `Ok(())`.
pub fn spawn(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        // One immediate short delay so the very first tick doesn't
        // compete with the app's own startup work (DB open, first
        // screen render) for CPU/network on a cold launch.
        tokio::time::sleep(Duration::from_secs(15)).await;

        loop {
            run_one_tick(&app_handle).await;
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    });
}

async fn run_one_tick(app_handle: &tauri::AppHandle) {
    let Some(db) = app_handle.try_state::<Mutex<Connection>>() else {
        // DB isn't managed yet (shouldn't happen given call order in
        // `main.rs`, but this task must never panic the process over
        // it) — skip this tick silently and try again next interval.
        warn!(event = "scheduler_tick_skipped", reason = "db_not_ready");
        return;
    };
    let db: &Mutex<Connection> = &*db;

    for source_key in [
        "codeforces",
        "leetcode",
        "github",
        "gmail",
        "google_classroom",
        "notion",
    ] {
        let configured_handle = {
            let Ok(conn) = db.lock() else { continue };
            let Ok(Some(row)) = integrations_repo::get_data_source(&conn, source_key) else {
                continue;
            };
            if row.status == "disconnected" {
                continue;
            }
            row.config_json
                .as_deref()
                .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
                .and_then(|v| v.get("handle").and_then(|h| h.as_str()).map(str::to_string))
        };

        info!(event = "scheduler_tick", source = source_key);

        let outcome = match source_key {
            "codeforces" => match configured_handle {
                Some(handle) => Some(crate::commands::integrations::run_codeforces_sync(db, &handle).await),
                None => None,
            },
            "leetcode" => match configured_handle {
                Some(handle) => Some(crate::commands::integrations::run_leetcode_sync(db, &handle).await),
                None => None,
            },
            "github" => Some(crate::commands::integrations::run_github_sync(db).await),
            "gmail" => Some(crate::commands::integrations::run_gmail_sync(db).await),
            "google_classroom" => Some(crate::commands::integrations::run_google_classroom_sync(db).await),
            "notion" => Some(crate::commands::integrations::run_notion_sync(db).await),
            _ => None,
        };

        if let Some(Err(e)) = outcome {
            warn!(event = "scheduler_sync_failed", source = source_key, error = %e);
        }
    }
}
