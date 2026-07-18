//! Credential storage for this app: the GitHub personal access token
//! (07_INTEGRATIONS.md §4) plus, as of the 2026-07-17 OAuth amendment
//! (§1.8-§1.10), one OAuth access+refresh token set per OAuth connector
//! (Gmail, Google Classroom, Notion). Every other Version 1 connector
//! needs no credential at all (Codeforces, LeetCode — public APIs) or is
//! a local file parse (Calendar, PDF, CSV — nothing to authenticate to).
//!
//! `keyring::Entry` talks to the OS-native store directly — Keychain on
//! macOS, Credential Manager on Windows, Secret Service on Linux — the
//! same three backends §4 names. Nothing here ever touches SQLite or a
//! config file; a token never becomes part of `athena.sqlite` or its
//! WAL/journal files.
//!
//! ## Graceful fallback when the OS keychain backend is unavailable
//!
//! `keyring`'s Linux backend talks to a running D-Bus Secret Service
//! (gnome-keyring / kwallet). WSL and many minimal Linux environments
//! don't run one, which previously surfaced as a raw, uncaught error —
//! `"Platform secure storage failure: zbus error:
//! org.freedesktop.DBus.Error.ServiceUnknown: The name
//! org.freedesktop.secrets was not provided by any .service files"` —
//! all the way up to the user.
//!
//! `keyring` 2.x (this crate's pinned major version, see
//! `athena-app/Cargo.toml`) does not ship a built-in file-based fallback
//! backend — its default ("v1") feature gives exactly the three native
//! backends §4 names and nothing else (verified against the crate's own
//! published docs: linking with the default feature "gives your
//! application the ability to set, get, and delete... secrets in the
//! native secure stores on Mac, Windows, and *nix operating systems," no
//! alternate store option). Task 4's option (a) therefore does not
//! apply here, and every credential accessor below goes through
//! `store_secret`/`load_secret`/`delete_secret` instead of calling
//! `keyring::Entry` directly, so that option (b) — a local
//! encrypted-file fallback under the app's own data directory — applies
//! uniformly to every credential this module manages (GitHub token,
//! three OAuth token sets, both AI-provider keys), not just one.
//!
//! The fallback activates only when the *backend itself* is
//! unavailable — `keyring::Error::PlatformFailure` or
//! `::NoStorageAccess` (the shapes a missing D-Bus Secret Service or a
//! missing/locked/inaccessible native store actually produce, including
//! an unsupported OS, which also surfaces as `PlatformFailure`) —
//! never for `NoEntry` (a normal "nothing saved yet" result) or any
//! other error, which are returned to the caller unchanged so a
//! genuine problem (e.g. a corrupted keychain entry) is never silently
//! masked.
//!
//! Fallback storage is a single JSON file,
//! `<app-data-dir>/keychain-fallback/fallback-secrets.json`, holding
//! `{ username: base64(nonce || AES-256-GCM ciphertext) }`. The AES key
//! is 32 random bytes generated on first fallback use and written to a
//! sibling file, `fallback.key`, in the same directory (owner-only
//! permissions on Unix) — encrypted-at-rest, not plaintext-on-disk, per
//! Task 4's requirement. `aes-gcm` is the one new dependency this adds
//! (flagged in `MANIFEST.txt`): a well-known, audited RustCrypto crate,
//! already a transitive presence in this workspace's dependency tree
//! for other reasons per `Cargo.lock`, chosen over rolling any
//! hand-written cipher.
//!
//! Every fallback activation logs a single `tracing::warn!` the first
//! time it happens per process (`FALLBACK_LOGGED`), reading "using
//! local fallback storage, OS keychain unavailable" — via the same
//! `tracing` setup `main.rs::init_logging` already installs, per Task
//! 4's requirement, without spamming one line per credential access.

use std::io::Write;
use std::path::PathBuf;
use std::sync::{Once, OnceLock};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "athena-app";
const GITHUB_TOKEN_USERNAME: &str = "github-personal-access-token";
const OAUTH_USERNAME_PREFIX: &str = "oauth-tokens-";
const ANTHROPIC_API_KEY_USERNAME: &str = "anthropic-api-key";
const HF_API_TOKEN_USERNAME: &str = "huggingface-api-token";

static FALLBACK_LOGGED: Once = Once::new();

/// Whether a `keyring::Error` indicates the backend itself is
/// unavailable (missing D-Bus Secret Service or a locked/inaccessible
/// native store) rather than a normal per-entry outcome. Only these two
/// variants trigger the file fallback — see the module doc comment.
/// (`keyring::Error` is `#[non_exhaustive]`; no third
/// "unsupported platform" variant exists in this crate's actual API —
/// an unsupported OS surfaces as `PlatformFailure` instead.)
fn is_backend_unavailable(err: &keyring::Error) -> bool {
    matches!(
        err,
        keyring::Error::PlatformFailure(_) | keyring::Error::NoStorageAccess(_)
    )
}

fn log_fallback_once() {
    FALLBACK_LOGGED.call_once(|| {
        tracing::warn!(
            event = "keychain_fallback_active",
            "using local fallback storage, OS keychain unavailable"
        );
    });
}

/// Stores `value` under `username` in the OS keychain, falling back to
/// the local encrypted file store if the backend is unavailable.
fn store_secret(username: &str, value: &str) -> Result<(), String> {
    match keyring::Entry::new(SERVICE, username) {
        Ok(entry) => match entry.set_password(value) {
            Ok(()) => Ok(()),
            Err(e) if is_backend_unavailable(&e) => {
                log_fallback_once();
                fallback::set(username, value)
            }
            Err(e) => Err(e.to_string()),
        },
        Err(e) if is_backend_unavailable(&e) => {
            log_fallback_once();
            fallback::set(username, value)
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Loads the value stored under `username`, or `None` if nothing has
/// been saved yet — falling back to the local encrypted file store if
/// the backend is unavailable. Whichever backend a value was written
/// to is also the one it is read back from: this app never runs both
/// backends for the same install (the OS backend's own availability
/// doesn't change between calls within one run), so there is no
/// merge/precedence question between them.
fn load_secret(username: &str) -> Result<Option<String>, String> {
    match keyring::Entry::new(SERVICE, username) {
        Ok(entry) => match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => {
                // The OS backend is reachable but has nothing under this
                // username. Still consult the fallback file in case a
                // previous run wrote there while the backend was down
                // (e.g. WSL without a keyring daemon this session, but
                // `keyring::Entry::new` itself didn't fail this time).
                fallback::get(username)
            }
            Err(e) if is_backend_unavailable(&e) => {
                log_fallback_once();
                fallback::get(username)
            }
            Err(e) => Err(e.to_string()),
        },
        Err(e) if is_backend_unavailable(&e) => {
            log_fallback_once();
            fallback::get(username)
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Removes the value stored under `username`, from whichever backend
/// currently holds it. Idempotent — "nothing to remove" is success.
fn delete_secret(username: &str) -> Result<(), String> {
    let keyring_result = match keyring::Entry::new(SERVICE, username) {
        Ok(entry) => entry.delete_password(),
        Err(e) => Err(e),
    };
    match keyring_result {
        Ok(()) | Err(keyring::Error::NoEntry) => {}
        Err(e) if is_backend_unavailable(&e) => log_fallback_once(),
        Err(e) => return Err(e.to_string()),
    }
    // Always also clear the fallback file entry, if any — a value may
    // have been written there during a prior run.
    fallback::delete(username)
}

/// Saves the token the user pastes in during the Connectors step. §4:
/// "a narrow, user-generated, revocable token the user creates
/// themselves on the provider's site and pastes in once."
pub fn save_github_token(token: &str) -> Result<(), String> {
    store_secret(GITHUB_TOKEN_USERNAME, token)
}

/// Returns the stored token, or `None` if none has been saved yet — not
/// an error; "no token" is this connector's normal disconnected state.
pub fn get_github_token() -> Result<Option<String>, String> {
    load_secret(GITHUB_TOKEN_USERNAME)
}

/// Removes the stored token (disconnecting GitHub). Treats "nothing to
/// remove" as success, not an error — disconnecting an
/// already-disconnected source is idempotent, matching
/// `athena_data::repositories::integrations::link_github_repo`'s own
/// idempotency precedent.
pub fn delete_github_token() -> Result<(), String> {
    delete_secret(GITHUB_TOKEN_USERNAME)
}

/// Whether a token is currently stored — used to render the Connectors
/// step's "connected" state without ever handing the token itself back
/// to the frontend (§4/§6: the token never leaves the keychain, not
/// even over the app's own IPC boundary).
pub fn has_github_token() -> bool {
    matches!(get_github_token(), Ok(Some(_)))
}

// ---------------------------------------------------------------------
// OAuth token storage (§1.8-§1.10, 2026-07-17 amendment): Gmail,
// Google Classroom, Notion. One keychain entry per provider key
// ("gmail" / "google_classroom" / "notion"), so disconnecting one never
// touches another's stored tokens.
// ---------------------------------------------------------------------

/// An OAuth access (+ optional refresh) token set, exactly as stored in
/// the keychain — never in SQLite (§4's non-negotiable, extended to
/// these three connectors by the amendment).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredOAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// ISO-8601 instant the access token expires, if the provider told
    /// us. `None` doesn't mean "never expires" (Notion genuinely
    /// doesn't; Google always tells us) — it means "unknown," so the
    /// caller relies on a real 401 to decide a refresh is needed rather
    /// than assuming indefinite validity.
    pub expires_at: Option<String>,
}

fn oauth_username(provider: &str) -> String {
    format!("{OAUTH_USERNAME_PREFIX}{provider}")
}

/// Saves (or overwrites) the token set for one OAuth provider key.
pub fn save_oauth_tokens(provider: &str, tokens: &StoredOAuthTokens) -> Result<(), String> {
    let json = serde_json::to_string(tokens).map_err(|e| e.to_string())?;
    store_secret(&oauth_username(provider), &json)
}

/// Returns the stored token set for one OAuth provider key, or `None` if
/// never connected / already disconnected — same "absence is a normal
/// state, not an error" precedent as `get_github_token`.
pub fn get_oauth_tokens(provider: &str) -> Result<Option<StoredOAuthTokens>, String> {
    match load_secret(&oauth_username(provider))? {
        Some(json) => serde_json::from_str(&json).map(Some).map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

/// Removes the stored token set for one OAuth provider key
/// (disconnecting it). Idempotent, same precedent as `delete_github_token`.
pub fn delete_oauth_tokens(provider: &str) -> Result<(), String> {
    delete_secret(&oauth_username(provider))
}

/// Whether an OAuth provider currently has a stored token set — the
/// OAuth-connector equivalent of `has_github_token`, used the same way
/// (rendering "connected" state without ever handing the token itself
/// back over IPC).
pub fn has_oauth_tokens(provider: &str) -> bool {
    matches!(get_oauth_tokens(provider), Ok(Some(_)))
}

// ---------------------------------------------------------------------
// Anthropic API key (06_AI_ENGINE.md §9's cloud provider). Same
// "never in SQLite, never in a plaintext config file, never logged"
// rule §4 of 07_INTEGRATIONS.md already applies to every other
// connector credential — the AI layer's cloud provider is not a special
// case. Absence is the normal, fully-supported "no cloud provider
// configured" state (§10), not an error.
// ---------------------------------------------------------------------

pub fn save_anthropic_api_key(key: &str) -> Result<(), String> {
    store_secret(ANTHROPIC_API_KEY_USERNAME, key)
}

pub fn get_anthropic_api_key() -> Result<Option<String>, String> {
    load_secret(ANTHROPIC_API_KEY_USERNAME)
}

pub fn delete_anthropic_api_key() -> Result<(), String> {
    delete_secret(ANTHROPIC_API_KEY_USERNAME)
}

pub fn has_anthropic_api_key() -> bool {
    matches!(get_anthropic_api_key(), Ok(Some(_)))
}

// ---------------------------------------------------------------------
// Hugging Face API token (providers/hf.rs). Free-tier token from
// https://huggingface.co/settings/tokens — role "Inference" is enough.
// Same "never in SQLite, never logged" rule as every other credential
// here. Absence means "HF provider not configured" — the cascade falls
// through to Ollama / template, never an error.
// ---------------------------------------------------------------------

pub fn save_hf_api_token(token: &str) -> Result<(), String> {
    store_secret(HF_API_TOKEN_USERNAME, token)
}

pub fn get_hf_api_token() -> Result<Option<String>, String> {
    load_secret(HF_API_TOKEN_USERNAME)
}

pub fn delete_hf_api_token() -> Result<(), String> {
    delete_secret(HF_API_TOKEN_USERNAME)
}

pub fn has_hf_api_token() -> bool {
    matches!(get_hf_api_token(), Ok(Some(_)))
}

/// Whether fallback storage is currently active for this process — the
/// frontend's Settings screen reads this (via a command wrapping this
/// function) to show a clear, non-scary "stored locally, not in your OS
/// keychain" notice per Task 4's frontend requirement.
pub fn is_using_fallback_storage() -> bool {
    FALLBACK_LOGGED.is_completed()
}

// ---------------------------------------------------------------------
// Local encrypted-file fallback (Task 4 option (b)). Activated only
// when the OS keychain backend itself is unavailable — see the module
// doc comment. AES-256-GCM via the `aes-gcm` crate (this module's one
// new dependency), key generated once per install and stored alongside
// the ciphertext file, both under the app's own data directory.
// ---------------------------------------------------------------------

mod fallback {
    use super::*;

    #[derive(Debug, Default, Serialize, Deserialize)]
    struct SecretsFile {
        /// username -> base64(nonce || ciphertext)
        entries: std::collections::BTreeMap<String, String>,
    }

    static DIR: OnceLock<PathBuf> = OnceLock::new();

    /// `<data-dir>/athena-app/keychain-fallback`. Resolved independently
    /// of Tauri's own `app_data_dir` (this module takes no `AppHandle`
    /// dependency — see the module doc comment for why) via the
    /// well-known `dirs` crate, which reads the same OS-standard
    /// per-user data-directory conventions Tauri itself uses
    /// (`XDG_DATA_HOME`/`~/.local/share` on Linux, `%APPDATA%` on
    /// Windows, `~/Library/Application Support` on macOS).
    fn fallback_dir() -> Result<PathBuf, String> {
        if let Some(dir) = DIR.get() {
            return Ok(dir.clone());
        }
        let base = dirs::data_dir().ok_or_else(|| "could not resolve a user data directory".to_string())?;
        let dir = base.join("athena-app").join("keychain-fallback");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let _ = DIR.set(dir.clone());
        Ok(dir)
    }

    fn key_path(dir: &std::path::Path) -> PathBuf {
        dir.join("fallback.key")
    }

    fn secrets_path(dir: &std::path::Path) -> PathBuf {
        dir.join("fallback-secrets.json")
    }

    /// Loads the AES-256 key bytes, generating and persisting a new
    /// random 32-byte key on first use. Restricted to owner-only
    /// permissions on Unix — there is no exact Windows equivalent via
    /// `std::fs` alone, so on Windows the file relies on the
    /// user-profile directory's own ACLs (the same trust boundary
    /// `%APPDATA%` already provides for every other per-user app file).
    fn load_or_create_key(dir: &std::path::Path) -> Result<[u8; 32], String> {
        let path = key_path(dir);
        if let Ok(bytes) = std::fs::read(&path) {
            if bytes.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                return Ok(key);
            }
        }
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);

        let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
        file.write_all(&key_bytes).map_err(|e| e.to_string())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = file.set_permissions(std::fs::Permissions::from_mode(0o600));
        }

        Ok(key_bytes)
    }

    fn read_secrets_file(dir: &std::path::Path) -> Result<SecretsFile, String> {
        let path = secrets_path(dir);
        match std::fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SecretsFile::default()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn write_secrets_file(dir: &std::path::Path, file: &SecretsFile) -> Result<(), String> {
        let json = serde_json::to_string(file).map_err(|e| e.to_string())?;
        std::fs::write(secrets_path(dir), json).map_err(|e| e.to_string())
    }

    pub(super) fn set(username: &str, value: &str) -> Result<(), String> {
        let dir = fallback_dir()?;
        let key_bytes = load_or_create_key(&dir)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, value.as_bytes())
            .map_err(|e| e.to_string())?;

        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);
        let encoded = base64::engine::general_purpose::STANDARD.encode(combined);

        let mut file = read_secrets_file(&dir)?;
        file.entries.insert(username.to_string(), encoded);
        write_secrets_file(&dir, &file)
    }

    pub(super) fn get(username: &str) -> Result<Option<String>, String> {
        let dir = fallback_dir()?;
        let file = read_secrets_file(&dir)?;
        let Some(encoded) = file.entries.get(username) else {
            return Ok(None);
        };

        let combined = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| e.to_string())?;
        if combined.len() < 12 {
            return Err("corrupt fallback secret entry".to_string());
        }
        let (nonce_bytes, ciphertext) = combined.split_at(12);

        let key_bytes = load_or_create_key(&dir)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| e.to_string())?;
        String::from_utf8(plaintext)
            .map(Some)
            .map_err(|e| e.to_string())
    }

    pub(super) fn delete(username: &str) -> Result<(), String> {
        let dir = fallback_dir()?;
        let mut file = read_secrets_file(&dir)?;
        if file.entries.remove(username).is_some() {
            write_secrets_file(&dir, &file)?;
        }
        Ok(())
    }
}
