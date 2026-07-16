// Prevents an additional console window from appearing on Windows release
// builds (standard Tauri boilerplate).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::path::PathBuf;

use tauri::Manager;
use tracing::{error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Where the app keeps its state on disk, resolved once at startup.
///
/// Per Master Spec §7 / Non-Negotiable §8 (sole ownership — data lives on
/// disk under the user's control), everything lives under the OS's
/// standard per-user app-data directory, never a system/shared location.
struct AppPaths {
    db_path: PathBuf,
    log_dir: PathBuf,
}

fn resolve_app_paths(app_handle: &tauri::AppHandle) -> anyhow::Result<AppPaths> {
    // `anyhow` is permitted only in athena-app's glue (Implementation
    // Plan §6) — this bootstrap function is exactly that glue.
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| anyhow::anyhow!("could not resolve app data dir: {e}"))?;
    std::fs::create_dir_all(&data_dir)?;

    Ok(AppPaths {
        db_path: data_dir.join("athena.sqlite"),
        log_dir: data_dir.join("logs"),
    })
}

/// Initializes structured, rotating, local-only JSON-lines logging
/// (Implementation Plan §7). Returns the guard that must stay alive for
/// the duration of the program so buffered log lines are flushed.
fn init_logging(log_dir: &std::path::Path) -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
    std::fs::create_dir_all(log_dir)?;

    let file_appender = tracing_appender::rolling::daily(log_dir, "athena.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json().with_writer(non_blocking))
        .init();

    Ok(guard)
}

fn main() {
    let builder = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_app_version,
            commands::bootstrap::get_bootstrap_state,
            commands::onboarding::create_profile,
            commands::onboarding::commit_semester_setup,
            commands::planner::log_disruption,
            commands::planner::list_recent_disruptions,
        ])
        .setup(|app| {
            let paths = resolve_app_paths(app.handle())?;

            // Logging guard is leaked intentionally: it must live for the
            // whole process lifetime, and `setup` has no natural owner to
            // hand it back to outside of app-managed state.
            let guard = init_logging(&paths.log_dir)?;
            app.manage(guard);

            info!(event = "startup", "athena-app starting up");

            match athena_data::connection::open_and_migrate(&paths.db_path) {
                Ok(conn) => {
                    info!(event = "migration_complete", "database ready, WAL mode active");
                    app.manage(std::sync::Mutex::new(conn));
                }
                Err(e) => {
                    // Per Implementation Plan §6 ("fail loud, not
                    // silent"): a migration failure is an ERROR-level,
                    // actionable log line, and the app must not silently
                    // continue with a broken database.
                    error!(event = "migration_failed", error = %e, "database migration failed");
                    return Err(Box::new(e));
                }
            }

            Ok(())
        });

    builder
        .run(tauri::generate_context!())
        .expect("error while running athena-app");
}