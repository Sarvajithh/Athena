//! Generic OAuth 2.0 Authorization Code (+ optional PKCE) helper, shared
//! by every OAuth-based connector (07_INTEGRATIONS.md §1.8-§1.10: Gmail,
//! Google Classroom, Notion). Provider-agnostic — every endpoint URL and
//! credential is a caller-supplied argument — so no connector module
//! needs its own copy of the token-exchange/refresh plumbing.
//!
//! This is a connector-agnostic *utility*, not a connector itself, so it
//! does not violate `connectors/mod.rs`'s "none imports another" rule at
//! the connector level: `gmail.rs`/`google_classroom.rs`/`notion.rs`
//! still never import each other, they each import this shared, stateless
//! helper the same way every connector already imports `crate::error`.

use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::IngestionError;

/// A generated PKCE verifier/challenge pair (RFC 7636, `S256` method).
pub struct PkcePair {
    pub verifier: String,
    pub challenge: String,
}

/// Generates a fresh PKCE pair using a CSPRNG (`rand::thread_rng`) — a
/// predictable verifier defeats the entire point of PKCE, so this is one
/// of the two places (with `generate_state`) this crate uses a real
/// dependency instead of hand-rolling (see `Cargo.toml`'s comment).
pub fn generate_pkce_pair() -> PkcePair {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let verifier = base64_url_encode(&bytes);
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = base64_url_encode(&digest);
    PkcePair { verifier, challenge }
}

/// A random `state` param — checked on the redirect back so a stray or
/// forged callback can never be mistaken for the flow this app started
/// (CSRF protection, standard OAuth practice).
pub fn generate_state() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64_url_encode(&bytes)
}

/// Base64url, no padding (RFC 4648 §5) — hand-rolled rather than adding
/// a `base64` crate dependency for this one encoding, matching this
/// crate's existing precedent (`github.rs`'s hand-rolled ISO-8601
/// formatting, `commands::integrations`'s hand-rolled base64 decoder).
fn base64_url_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((bytes.len() * 4).div_ceil(3));
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[(triple & 0x3F) as usize] as char);
        }
    }
    out
}

/// Percent-encoding for authorize-URL query params (RFC 3986 unreserved
/// set kept literal, everything else escaped) — hand-rolled for the same
/// "no dependency for one call site" reason as `base64_url_encode`.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Builds `base_url?k1=v1&k2=v2...` with every value percent-encoded —
/// used for the browser-facing authorize URL (never for the token
/// endpoint, which is a POST body).
pub fn build_authorize_url(base_url: &str, params: &[(&str, &str)]) -> String {
    let mut url = format!("{base_url}?");
    for (i, (k, v)) in params.iter().enumerate() {
        if i > 0 {
            url.push('&');
        }
        url.push_str(&percent_encode(k));
        url.push('=');
        url.push_str(&percent_encode(v));
    }
    url
}

/// How the client authenticates itself at the token endpoint — providers
/// disagree on this, so it's a caller-supplied choice rather than an
/// assumption baked into this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientAuthStyle {
    /// `client_id`/`client_secret` (if any) travel as ordinary body
    /// params — Google's token endpoint (§1.8/§1.9). PKCE covers the
    /// "no secret needed" case; a secret is included only if the caller
    /// supplies one.
    BodyParams,
    /// `client_id:client_secret` travels as an HTTP Basic
    /// `Authorization` header, body carries only the grant params —
    /// Notion's documented token-exchange contract (§1.10).
    BasicHeader,
}

/// How the token-endpoint request body is encoded — also
/// provider-specific (Google wants form-encoded, Notion wants JSON).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyEncoding {
    Form,
    Json,
}

#[derive(Debug, Clone)]
pub struct OAuthTokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in_secs: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

pub struct AuthCodeExchangeRequest<'a> {
    pub token_url: &'a str,
    pub client_id: &'a str,
    pub client_secret: Option<&'a str>,
    pub code: &'a str,
    pub redirect_uri: &'a str,
    /// `None` for providers without PKCE support (Notion, §1.10).
    pub code_verifier: Option<&'a str>,
    pub auth_style: ClientAuthStyle,
    pub body_encoding: BodyEncoding,
}

/// Exchanges an authorization code for an access (+ optional refresh)
/// token. A non-2xx response is reported as `AuthExpired` rather than
/// `Network` — at this stage in the flow, a rejection almost always
/// means the code/verifier/secret was wrong or already used, which is
/// functionally the same "needs a fresh authorization" outcome as an
/// expired token later.
pub async fn exchange_code_for_tokens(
    req: AuthCodeExchangeRequest<'_>,
) -> Result<OAuthTokenSet, IngestionError> {
    let mut params: Vec<(&str, &str)> = vec![
        ("grant_type", "authorization_code"),
        ("code", req.code),
        ("redirect_uri", req.redirect_uri),
    ];
    if req.auth_style == ClientAuthStyle::BodyParams {
        params.push(("client_id", req.client_id));
        if let Some(secret) = req.client_secret {
            params.push(("client_secret", secret));
        }
    }
    if let Some(verifier) = req.code_verifier {
        params.push(("code_verifier", verifier));
    }

    send_token_request(req.token_url, req.client_id, req.client_secret, req.auth_style, req.body_encoding, params).await
}

pub struct RefreshRequest<'a> {
    pub token_url: &'a str,
    pub client_id: &'a str,
    pub client_secret: Option<&'a str>,
    pub refresh_token: &'a str,
    pub auth_style: ClientAuthStyle,
    pub body_encoding: BodyEncoding,
}

/// Exchanges a refresh token for a new access token. Some providers omit
/// `refresh_token` from the response (it stays valid); the caller is
/// responsible for keeping the previous one if this returns `None`.
pub async fn refresh_access_token(req: RefreshRequest<'_>) -> Result<OAuthTokenSet, IngestionError> {
    let mut params: Vec<(&str, &str)> = vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", req.refresh_token),
    ];
    if req.auth_style == ClientAuthStyle::BodyParams {
        params.push(("client_id", req.client_id));
        if let Some(secret) = req.client_secret {
            params.push(("client_secret", secret));
        }
    }

    send_token_request(req.token_url, req.client_id, req.client_secret, req.auth_style, req.body_encoding, params).await
}

async fn send_token_request(
    token_url: &str,
    client_id: &str,
    client_secret: Option<&str>,
    auth_style: ClientAuthStyle,
    body_encoding: BodyEncoding,
    params: Vec<(&str, &str)>,
) -> Result<OAuthTokenSet, IngestionError> {
    let client = reqwest::Client::new();
    let mut builder = client.post(token_url);

    builder = match body_encoding {
        BodyEncoding::Form => builder.form(&params),
        BodyEncoding::Json => {
            let map: std::collections::HashMap<&str, &str> = params.into_iter().collect();
            builder.json(&map)
        }
    };
    if auth_style == ClientAuthStyle::BasicHeader {
        builder = builder.basic_auth(client_id, client_secret);
    }

    let resp = builder
        .send()
        .await
        .map_err(|e| IngestionError::Network(format!("oauth token request: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(IngestionError::AuthExpired(format!(
            "oauth token request rejected ({status}): {body}"
        )));
    }

    let parsed: TokenResponse = resp
        .json()
        .await
        .map_err(|e| IngestionError::Parse(format!("oauth token payload: {e}")))?;

    Ok(OAuthTokenSet {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        expires_in_secs: parsed.expires_in,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_pair_verifier_and_challenge_are_url_safe_and_distinct() {
        let pair = generate_pkce_pair();
        assert!(!pair.verifier.is_empty());
        assert!(!pair.challenge.is_empty());
        assert_ne!(pair.verifier, pair.challenge);
        assert!(pair.verifier.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn generate_state_is_nonempty_and_url_safe() {
        let state = generate_state();
        assert!(!state.is_empty());
        assert!(state.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn build_authorize_url_percent_encodes_values() {
        let url = build_authorize_url("https://example.com/auth", &[("scope", "a b"), ("state", "x")]);
        assert_eq!(url, "https://example.com/auth?scope=a%20b&state=x");
    }
}
