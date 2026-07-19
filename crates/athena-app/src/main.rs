// Prevents an additional console window from appearing on Windows release
// builds (standard Tauri boilerplate).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod keychain;
mod oauth_loopback;
mod routine_scheduler;
mod scheduler;

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
        // Task: scheduled daily-questionnaire trigger — the system
        // notification shown when it fires. Registered once here, at
        // the same point every other cross-cutting builder call lives.
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_app_version,
            commands::is_using_keychain_fallback,
            commands::bootstrap::get_bootstrap_state,
            commands::onboarding::create_profile,
            commands::onboarding::commit_semester_setup,
            commands::planner::log_disruption,
            commands::planner::list_recent_disruptions,
            commands::routine::submit_daily_routine_response,
            commands::routine::has_daily_routine_response,
            commands::routine::list_recent_daily_routine_responses,
            commands::routine::submit_weekly_routine_response,
            commands::routine::has_weekly_routine_response,
            commands::routine::list_recent_weekly_routine_responses,
            commands::routine::save_routine_questionnaire_time,
            commands::routine::get_routine_questionnaire_time,
            commands::integrations::list_data_sources,
            commands::integrations::sync_codeforces,
            commands::integrations::get_latest_codeforces_snapshot,
            commands::integrations::sync_leetcode,
            commands::integrations::get_latest_leetcode_snapshot,
            commands::integrations::save_github_token,
            commands::integrations::delete_github_token,
            commands::integrations::link_github_repo,
            commands::integrations::unlink_github_repo,
            commands::integrations::list_linked_github_repos,
            commands::integrations::sync_github,
            commands::integrations::list_project_status_snapshots,
            commands::integrations::import_calendar_ics,
            commands::integrations::preview_csv_import,
            commands::integrations::preview_pdf_import,
            commands::integrations::start_gmail_oauth,
            commands::integrations::disconnect_gmail,
            commands::integrations::list_gmail_messages,
            commands::integrations::start_google_classroom_oauth,
            commands::integrations::disconnect_google_classroom,
            commands::integrations::list_classroom_courses,
            commands::integrations::list_classroom_coursework,
            commands::integrations::list_classroom_announcements,
            commands::integrations::start_notion_oauth,
            commands::integrations::disconnect_notion,
            commands::integrations::list_notion_pages,
            commands::ai::get_daily_briefing,
            commands::ai::get_weekly_plan,
            commands::ai::get_weakness_analysis,
            commands::ai::save_anthropic_api_key,
            commands::ai::delete_anthropic_api_key,
            commands::ai::has_anthropic_api_key,
            commands::ai::save_hf_api_key,
            commands::ai::delete_hf_api_key,
            commands::ai::has_hf_api_key,
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

            // 07_INTEGRATIONS.md: "never block startup waiting for
            // integrations." `setup` returns `Ok(())` immediately after
            // this call — `scheduler::spawn` only schedules a background
            // async task, it does not await one. The window opens
            // whether or not Codeforces/LeetCode/GitHub are reachable
            // right now, or ever.
            scheduler::spawn(app.handle().clone());

            // Same "never block startup" contract as the line above:
            // this only schedules a background async task and returns
            // immediately — the window opens whether or not today's
            // questionnaire time has been reached yet.
            routine_scheduler::spawn(app.handle().clone());

            Ok(())
        });

    builder
        .run(tauri::generate_context!())
        .expect("error while running athena-app");
}