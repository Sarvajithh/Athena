//! IPC surface for 06_AI_ENGINE.md's AI layer. Every command here is a
//! thin translation, same precedent as `bootstrap.rs`/`planner.rs`:
//! fetch the same Decision Engine verdict the rest of the app already
//! computes (never a second scoring path — this file calls
//! `athena_domain::priority::resolve_priority` / `athena_domain::planner`
//! directly, the identical functions `bootstrap.rs`/`planner.rs` call),
//! hand it to `athena_reasoning::capabilities::*` to phrase, and return
//! the typed `Recommendation`. `athena_reasoning::Recommendation` already
//! derives `Serialize` (this crate owns its own `serde` dependency,
//! unlike `athena-domain`), so no DTO remapping happens here.
//!
//! Blocking by construction: `Synthesizer::synthesize` calls a blocking
//! HTTP client (see `athena-reasoning/Cargo.toml`'s dependency comment
//! for why). Every command below wraps that call in
//! `tauri::async_runtime::spawn_blocking` so a slow or unreachable LLM
//! provider never stalls the Tauri IPC thread — 07_INTEGRATIONS.md's
//! "never block startup waiting for integrations," extended to "never
//! block the UI thread waiting for an LLM call" (06_AI_ENGINE.md §10's
//! offline-first requirement applies just as much to responsiveness as
//! to availability).
//!
//! Nothing in this file runs on app startup, same as `commands::integrations`
//! — every AI call here is user-triggered (opening a screen that renders
//! a phrased verdict) or scheduler-triggered, never blocking the window
//! from opening.

use std::sync::Mutex;

use athena_data::repositories::{deadline, disruption, profile};
use athena_domain::planner::{self, DisruptionType, ScheduleDisruption};
use athena_domain::priority::{self, DeadlineCandidate};
use athena_reasoning::capabilities::{daily_briefing, weakness_analysis, weekly_planning};
use athena_reasoning::providers::cloud::AnthropicProvider;
use athena_reasoning::providers::hf::HuggingFaceProvider;
use athena_reasoning::providers::local::OllamaProvider;
use athena_reasoning::{LlmProvider, Recommendation, Synthesizer};
use rusqlite::Connection;
use tauri::State;

use crate::keychain;

const DEFAULT_ANTHROPIC_MODEL: &str = "claude-sonnet-4-6";
// Best free-tier model for JSON instruction-following as of 2026-07.
// Swap to "meta-llama/Llama-3.3-70B-Instruct" or
// "mistralai/Mistral-7B-Instruct-v0.3" for a faster/lighter option.
const DEFAULT_HF_MODEL: &str = "Qwen/Qwen2.5-72B-Instruct";
const DEFAULT_OLLAMA_BASE_URL: &str = "http://localhost:11434";
const DEFAULT_OLLAMA_MODEL: &str = "llama3";

// Duplicated in `commands::integrations` as `now_iso8601`/`chrono_like_iso_date`/
// `civil_from_days` — same reasoning that file's own doc comment gives
// for its date helper: one timestamp format doesn't justify a full
// date/time crate dependency for a second call site.
fn now_iso8601() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}Z", chrono_like_iso_date(now.as_secs()), now.subsec_millis())
}
fn chrono_like_iso_date(total_secs: u64) -> String {
    let days = total_secs / 86_400;
    let secs_of_day = total_secs % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}")
}
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

/// Cascade: Anthropic (paid) → Hugging Face (free) → Ollama (local) → template.
/// Each provider is only added if its credential is configured; Ollama is always
/// attempted last (it just returns ProviderUnavailable immediately if the local
/// server isn't running). Built fresh per call so a newly-saved key takes effect
/// without a restart.
fn build_synthesizer() -> Synthesizer {
    let mut providers: Vec<Box<dyn LlmProvider>> = Vec::new();

    // 1. Anthropic Claude — paid, cloud, fastest
    if let Ok(Some(api_key)) = keychain::get_anthropic_api_key() {
        providers.push(Box::new(AnthropicProvider::new(
            api_key,
            DEFAULT_ANTHROPIC_MODEL.to_string(),
        )));
    }

    // 2. Hugging Face — free tier, cloud, no billing required
    if let Ok(Some(token)) = keychain::get_hf_api_token() {
        providers.push(Box::new(HuggingFaceProvider::new(
            token,
            DEFAULT_HF_MODEL.to_string(),
        )));
    }

    // 3. Ollama — local, always in the list; ProviderUnavailable if not running
    providers.push(Box::new(OllamaProvider::new(
        DEFAULT_OLLAMA_BASE_URL.to_string(),
        DEFAULT_OLLAMA_MODEL.to_string(),
    )));

    Synthesizer::new(providers)
}

fn open_candidates(conn: &Connection) -> Result<Vec<DeadlineCandidate>, String> {
    let rows = deadline::list_open(conn).map_err(|e| e.to_string())?;
    Ok(rows
        .iter()
        .map(|d| DeadlineCandidate {
            id: d.id,
            title: d.title.clone(),
            due_at: d.due_at.clone(),
            leverage_class: d.leverage_class.clone(),
        })
        .collect())
}

/// Daily Pass, on demand (§4.1) — phrases the exact same Priority
/// Resolution verdict `get_bootstrap_state` computes for `Now`.
#[tauri::command]
pub async fn get_daily_briefing(db: State<'_, Mutex<Connection>>) -> Result<Recommendation, String> {
    let candidates = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        open_candidates(&conn)?
    };
    let verdict = priority::resolve_priority(&candidates);
    let freshness_note = format!("as of {}", now_iso8601());

    tauri::async_runtime::spawn_blocking(move || {
        let synthesizer = build_synthesizer();
        daily_briefing::build_daily_briefing(&synthesizer, &verdict, freshness_note)
    })
    .await
    .map_err(|e| e.to_string())
}

/// Weekly Digest, on demand (§4.2) — rolls up the same
/// `athena_domain::planner::replan` verdict every disruption-affected
/// day already produced (the identical function `commands::planner::log_disruption`
/// calls), one call per distinct date already logged, most recent 7
/// first. Introduces no new scoring: every per-day verdict here is
/// recomputed by the same deterministic function, never invented.
#[tauri::command]
pub async fn get_weekly_plan(db: State<'_, Mutex<Connection>>) -> Result<Recommendation, String> {
    let (candidates, base_window_minutes, days): (Vec<DeadlineCandidate>, i64, Vec<(String, Vec<ScheduleDisruption>)>) = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let candidates = open_candidates(&conn)?;
        let base_window_minutes = profile::get_current_profile(&conn)
            .map_err(|e| e.to_string())?
            .and_then(|p| planner::base_window_minutes(&p.deep_work_window_start, &p.deep_work_window_end))
            .unwrap_or(240);

        let recent = disruption::list_recent(&conn, 100).map_err(|e| e.to_string())?;
        let mut dates: Vec<String> = recent.iter().map(|r| r.date.clone()).collect();
        dates.sort();
        dates.dedup();
        dates.reverse();
        dates.truncate(7);

        let days = dates
            .into_iter()
            .map(|date| {
                let disruptions: Vec<ScheduleDisruption> = recent
                    .iter()
                    .filter(|r| r.date == date)
                    .filter_map(|r| {
                        DisruptionType::from_str(&r.disruption_type).map(|t| ScheduleDisruption {
                            disruption_type: t,
                            duration_minutes: r.duration_minutes,
                        })
                    })
                    .collect();
                (date, disruptions)
            })
            .collect();

        (candidates, base_window_minutes, days)
    };

    tauri::async_runtime::spawn_blocking(move || {
        let replans: Vec<(String, planner::ReplanResult)> = days
            .into_iter()
            .map(|(date, disruptions)| {
                let result = planner::replan(&candidates, base_window_minutes, &disruptions);
                (date, result)
            })
            .collect();
        let synthesizer = build_synthesizer();
        weekly_planning::build_weekly_plan(&synthesizer, &replans, format!("as of {}", now_iso8601()))
    })
    .await
    .map_err(|e| e.to_string())
}

/// Weakness Analysis, on demand (§4.4). Honest gap, matching
/// `athena_domain::planner`'s own documented precedent: `drift_signals`/
/// `bottlenecks` don't exist in this schema yet
/// (`crates/athena-data/migrations/` stops at `V5`), so there is no
/// repository to call. Calling this today correctly returns
/// `insufficient_data` — never a fabricated pattern — and the one line
/// that changes once those tables land is the empty slice below being
/// replaced with a real repository read.
#[tauri::command]
pub async fn get_weakness_analysis() -> Result<Recommendation, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let synthesizer = build_synthesizer();
        weakness_analysis::build_weakness_analysis(&synthesizer, &[], format!("as of {}", now_iso8601()))
    })
    .await
    .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------
// Anthropic API key (§9's cloud provider config) — same shape as
// `commands::integrations`'s `save_github_token`/`delete_github_token`,
// exposed here rather than in `integrations.rs` since the AI layer is
// not a data-source connector, but the credential handling is identical.
// ---------------------------------------------------------------------

#[tauri::command]
pub fn save_anthropic_api_key(key: String) -> Result<(), String> {
    keychain::save_anthropic_api_key(&key)
}

#[tauri::command]
pub fn delete_anthropic_api_key() -> Result<(), String> {
    keychain::delete_anthropic_api_key()
}

#[tauri::command]
pub fn has_anthropic_api_key() -> Result<bool, String> {
    Ok(keychain::has_anthropic_api_key())
}

// ---------------------------------------------------------------------
// Hugging Face API token management — same shape as the Anthropic key
// commands above. Free-tier token from huggingface.co/settings/tokens
// (role: "Inference"). Absence means HF is skipped in the cascade;
// it is never an error.
// ---------------------------------------------------------------------

#[tauri::command]
pub fn save_hf_api_key(key: String) -> Result<(), String> {
    keychain::save_hf_api_token(&key)
}

#[tauri::command]
pub fn delete_hf_api_key() -> Result<(), String> {
    keychain::delete_hf_api_token()
}

#[tauri::command]
pub fn has_hf_api_key() -> Result<bool, String> {
    Ok(keychain::has_hf_api_token())
}
