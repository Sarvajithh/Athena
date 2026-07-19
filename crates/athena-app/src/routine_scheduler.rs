//! Scheduled, time-based trigger for the daily routine questionnaire.
//!
//! This is the first feature in the codebase that pushes anything to
//! the frontend outside of a direct command response — `athena-events`
//! (the crate reserved for a future general-purpose command/event bus,
//! see its own module doc comment) is deliberately left untouched:
//! this module emits exactly one scoped Tauri event,
//! [`DAILY_QUESTIONNAIRE_DUE_EVENT`], directly via `tauri::Emitter`,
//! rather than standing up a dispatcher for a single event type.
//!
//! Structurally this mirrors `scheduler.rs`'s own background-task
//! pattern (`tauri::async_runtime::spawn`, an initial short startup
//! delay, `app_handle.try_state::<Mutex<Connection>>()` per check so a
//! not-yet-managed DB degrades to a skipped tick rather than a panic)
//! but is its own module rather than folded into `scheduler.rs`:
//! `scheduler.rs`'s loop runs on a fixed 30-minute interval forever
//! and polls a fixed set of data sources every tick; this loop instead
//! sleeps until one specific wall-clock time each day and does nothing
//! in between, which is a different enough shape (single-fire-per-day
//! vs. fixed-interval-forever) that merging them would make both
//! harder to read for no shared logic beyond "it's a background task."
//!
//! Reuses the existing routine repository/commands' own "already
//! answered today" check (`routine::has_response_for_date`) rather
//! than duplicating that logic — this module never inserts, reads, or
//! reasons about a response row itself beyond that one existence
//! check.

use std::sync::Mutex;
use std::time::Duration;

use athena_data::repositories::{profile, routine};
use chrono::{Duration as ChronoDuration, Local, NaiveTime};
use rusqlite::Connection;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tracing::{info, warn};

/// The one Tauri event this module ever emits. Kept as a named
/// constant (rather than a bare string literal at each call site) so
/// the frontend listener registered in `bootstrapContext.tsx` and this
/// module can't silently drift apart on the event name.
pub const DAILY_QUESTIONNAIRE_DUE_EVENT: &str = "daily-questionnaire-due";

/// Same default as `commands::routine::DEFAULT_ROUTINE_QUESTIONNAIRE_TIME`
/// and the V7 migration column's own `DEFAULT` — duplicated here (a
/// `const` in each of three places) rather than imported from one,
/// since `athena-data`'s repository layer is the one already-correct
/// source of truth (`get_routine_questionnaire_time` returning
/// `Option<String>`) and this module only ever needs the fallback for
/// the rare case that call returns `Ok(None)` mid-loop.
const DEFAULT_ROUTINE_QUESTIONNAIRE_TIME: &str = "20:00";

/// One short delay before this task's very first check, staggered
/// slightly after `scheduler.rs`'s own 15-second startup delay so a
/// cold launch's DB-open/first-render work doesn't compete with two
/// background tasks waking up at the exact same instant.
const STARTUP_DELAY: Duration = Duration::from_secs(20);

/// How long to wait before retrying if the DB isn't managed yet on a
/// given check (mirrors `scheduler.rs`'s "skip this tick silently and
/// try again" handling, just on a shorter retry cadence appropriate to
/// a once-a-day trigger having to get today's check right).
const DB_NOT_READY_RETRY: Duration = Duration::from_secs(5 * 60);

fn local_date_today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn parse_configured_time(time_str: &str) -> NaiveTime {
    NaiveTime::parse_from_str(time_str, "%H:%M").unwrap_or_else(|_| {
        // A malformed stored value (shouldn't happen — Settings' time
        // input only ever writes `HH:MM`) falls back to the same
        // default the command layer uses, rather than panicking a
        // background task over a bad string.
        NaiveTime::parse_from_str(DEFAULT_ROUTINE_QUESTIONNAIRE_TIME, "%H:%M")
            .expect("DEFAULT_ROUTINE_QUESTIONNAIRE_TIME is a valid HH:MM literal")
    })
}

/// Reads the configured fire time and today's "already answered"
/// status in one short-lived lock, without holding the connection (or
/// the `State` guard) across an `.await` point. Returns `None` only
/// when the DB isn't managed yet.
fn read_config_and_status(app_handle: &AppHandle) -> Option<(NaiveTime, bool)> {
    let db = app_handle.try_state::<Mutex<Connection>>()?;
    let conn = db.lock().ok()?;

    let time_str = profile::get_routine_questionnaire_time(&conn)
        .ok()
        .flatten()
        .unwrap_or_else(|| DEFAULT_ROUTINE_QUESTIONNAIRE_TIME.to_string());
    let answered = routine::has_response_for_date(&conn, &local_date_today())
        // Fail safe: if the check itself errors, treat today as
        // already answered so a DB hiccup can never cause a spurious
        // notification/event — the existing card on Now still shows
        // the questionnaire regardless of what this scheduler does.
        .unwrap_or(true);

    Some((parse_configured_time(&time_str), answered))
}

/// Seconds until the next occurrence of `target` local time — today if
/// it hasn't passed yet, tomorrow otherwise. Always at least 1 second,
/// so a `target` equal to "right now" doesn't produce a zero-length
/// sleep that could tight-loop.
fn seconds_until_next(target: NaiveTime) -> u64 {
    let now = Local::now().naive_local();
    let today_target = now.date().and_time(target);
    let next = if today_target > now {
        today_target
    } else {
        today_target + ChronoDuration::days(1)
    };
    (next - now).num_seconds().max(1) as u64
}

/// Emits [`DAILY_QUESTIONNAIRE_DUE_EVENT`] and shows the system
/// notification. Both are best-effort: a failure in either one is
/// logged and never panics the background task, matching
/// `scheduler.rs`'s "a missing/expired/revoked token degrades... never
/// a panic" precedent for this same kind of best-effort background
/// side effect.
fn fire_due(app_handle: &AppHandle) {
    info!(event = "daily_questionnaire_due_fired");

    if let Err(e) = app_handle.emit(DAILY_QUESTIONNAIRE_DUE_EVENT, ()) {
        warn!(event = "daily_questionnaire_emit_failed", error = %e);
    }

    let notification = app_handle.notification();
    // `request_permission` returns the current state without re-
    // prompting if the user already made a choice — safe to call on
    // every fire rather than needing a separate "check first" branch.
    match notification.request_permission() {
        Ok(tauri_plugin_notification::PermissionState::Granted) => {
            if let Err(e) = notification
                .builder()
                .title("Quick check-in")
                .body("Today's check-in is ready — answer whenever you have a moment.")
                .show()
            {
                warn!(event = "daily_questionnaire_notification_failed", error = %e);
            }
        }
        Ok(state) => {
            info!(event = "daily_questionnaire_notification_skipped", permission = ?state);
        }
        Err(e) => {
            warn!(event = "daily_questionnaire_permission_check_failed", error = %e);
        }
    }
}

/// Spawns the background trigger loop. Fire-and-forget, exactly like
/// `scheduler::spawn` — nothing here is awaited from `main.rs`'s
/// `setup`, so it can never delay the window opening.
pub fn spawn(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(STARTUP_DELAY).await;

        // Edge case: "app closed at the scheduled time" — on this
        // launch, if the configured time has already passed today and
        // today still has no response, fire immediately instead of
        // waiting until tomorrow's target time.
        if let Some((target, answered)) = read_config_and_status(&app_handle) {
            if !answered && Local::now().time() >= target {
                fire_due(&app_handle);
            }
        }

        loop {
            let Some((target, _)) = read_config_and_status(&app_handle) else {
                warn!(event = "routine_scheduler_tick_skipped", reason = "db_not_ready");
                tokio::time::sleep(DB_NOT_READY_RETRY).await;
                continue;
            };

            tokio::time::sleep(Duration::from_secs(seconds_until_next(target))).await;

            // Re-read after waking: the configured time may have
            // changed while asleep (harmless — it just takes effect
            // starting from the *next* wait, same as any config change
            // picked up on the following loop iteration), and — the
            // edge case that matters here — the user may have already
            // answered today's questionnaire (via the Now card or
            // Settings' manual trigger) while this task slept, in
            // which case the scheduled prompt must not fire again.
            match read_config_and_status(&app_handle) {
                Some((_, true)) => {
                    info!(event = "daily_questionnaire_skipped_already_answered");
                }
                Some((_, false)) => fire_due(&app_handle),
                None => {
                    warn!(event = "routine_scheduler_tick_skipped", reason = "db_not_ready");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_and_falls_back_on_invalid_time() {
        assert_eq!(parse_configured_time("20:00"), NaiveTime::from_hms_opt(20, 0, 0).unwrap());
        assert_eq!(parse_configured_time("07:30"), NaiveTime::from_hms_opt(7, 30, 0).unwrap());
        // Malformed input falls back to the documented default rather
        // than panicking.
        assert_eq!(
            parse_configured_time("not-a-time"),
            NaiveTime::from_hms_opt(20, 0, 0).unwrap()
        );
    }

    #[test]
    fn seconds_until_next_is_never_zero() {
        let now = Local::now().time();
        assert!(seconds_until_next(now) >= 1);
    }
}
