//! Gmail connector (07_INTEGRATIONS.md §1.8, OAuth amendment). Read-only
//! polling of inbox message *metadata* via the Gmail API — sender,
//! subject, timestamp, snippet — never a message body, never a
//! send/modify/delete scope. Scope used:
//! `https://www.googleapis.com/auth/gmail.readonly`.
//!
//! Like every other connector in this crate, this module never touches
//! SQL or the keychain directly — it takes an already-valid access
//! token and returns typed data or an `IngestionError`; `athena-app`
//! resolves the token from the keychain and persists the result
//! (matching `github.rs`'s own "token resolved by the caller" precedent).

use serde::Deserialize;

use crate::error::IngestionError;

pub const AUTHORIZE_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";

/// How many of the most recent inbox messages a single poll fetches
/// metadata for — a portfolio/attention signal, not an exhaustive
/// mailbox audit, same "bounded per poll" discipline as GitHub's
/// 100-per-page commit fetch (§1.3).
const MAX_MESSAGES_PER_POLL: u32 = 25;

#[derive(Debug, Clone, PartialEq)]
pub struct GmailMessageSnapshot {
    pub message_id: String,
    pub thread_id: Option<String>,
    pub sender: Option<String>,
    pub subject: Option<String>,
    pub received_at: Option<String>,
    pub snippet: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListMessagesResponse {
    messages: Option<Vec<MessageRef>>,
}

#[derive(Debug, Deserialize)]
struct MessageRef {
    id: String,
}

#[derive(Debug, Deserialize)]
struct MessageDetail {
    id: String,
    #[serde(rename = "threadId")]
    thread_id: Option<String>,
    snippet: Option<String>,
    payload: Option<MessagePayload>,
}

#[derive(Debug, Deserialize)]
struct MessagePayload {
    headers: Option<Vec<MessageHeader>>,
}

#[derive(Debug, Deserialize)]
struct MessageHeader {
    name: String,
    value: String,
}

fn build_client(access_token: &str) -> Result<reqwest::Client, IngestionError> {
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

    let mut headers = HeaderMap::new();
    let value = HeaderValue::from_str(&format!("Bearer {access_token}"))
        .map_err(|e| IngestionError::Parse(format!("gmail auth header: {e}")))?;
    headers.insert(AUTHORIZATION, value);

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| IngestionError::Network(format!("gmail client build: {e}")))
}

/// Fetches metadata for the most recent inbox messages. `access_token`
/// must already be valid — a `401` here is surfaced as
/// `IngestionError::AuthExpired` so the caller knows to attempt a
/// refresh (or ask the user to reconnect) rather than treat it as a
/// generic network failure.
pub async fn fetch_inbox_metadata(access_token: &str) -> Result<Vec<GmailMessageSnapshot>, IngestionError> {
    let client = build_client(access_token)?;

    let list_url = format!(
        "https://gmail.googleapis.com/gmail/v1/users/me/messages?maxResults={MAX_MESSAGES_PER_POLL}&labelIds=INBOX"
    );
    let list_resp = client
        .get(&list_url)
        .send()
        .await
        .map_err(|e| IngestionError::Network(format!("gmail list: {e}")))?;

    if list_resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(IngestionError::AuthExpired("gmail access token rejected".into()));
    }
    if !list_resp.status().is_success() {
        return Err(IngestionError::Network(format!(
            "gmail list returned {}",
            list_resp.status()
        )));
    }

    let list: ListMessagesResponse = list_resp
        .json()
        .await
        .map_err(|e| IngestionError::Parse(format!("gmail list payload: {e}")))?;
    let refs = list.messages.unwrap_or_default();

    let mut snapshots = Vec::with_capacity(refs.len());
    for m in refs {
        let detail_url = format!(
            "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}?format=metadata\
             &metadataHeaders=From&metadataHeaders=Subject&metadataHeaders=Date",
            m.id
        );
        let detail_resp = match client.get(&detail_url).send().await {
            Ok(r) => r,
            // One message failing to fetch (deleted mid-poll, transient
            // error) doesn't abort the whole sync — same per-item
            // degrade-path precedent as GitHub's per-repo handling
            // (§1.3/§5).
            Err(_) => continue,
        };
        if !detail_resp.status().is_success() {
            continue;
        }
        let detail: MessageDetail = match detail_resp.json().await {
            Ok(d) => d,
            Err(_) => continue,
        };

        let headers = detail.payload.and_then(|p| p.headers).unwrap_or_default();
        let sender = headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("From"))
            .map(|h| h.value.clone());
        let subject = headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("Subject"))
            .map(|h| h.value.clone());
        let received_at = headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("Date"))
            .map(|h| h.value.clone());

        snapshots.push(GmailMessageSnapshot {
            message_id: detail.id,
            thread_id: detail.thread_id,
            sender,
            subject,
            received_at,
            snippet: detail.snippet,
        });
    }

    Ok(snapshots)
}
