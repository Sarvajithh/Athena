//! Notion connector (07_INTEGRATIONS.md §1.10, OAuth amendment).
//! Read-only sync of page reference metadata (title, URL, parent
//! database, last-edited time) for pages visible to the authorized
//! connection — never a write, never a content edit. Enforced
//! structurally, not just by scope: this module contains exactly one
//! outbound call, Notion's `search` endpoint, and nothing that could
//! modify content.

use serde::Deserialize;

use crate::error::IngestionError;

pub const AUTHORIZE_URL: &str = "https://api.notion.com/v1/oauth/authorize";
pub const TOKEN_URL: &str = "https://api.notion.com/v1/oauth/token";

/// Notion's own per-request cap for `search`. Version 1 takes a single
/// page of results rather than a recursive full-workspace crawl,
/// matching this document's "narrow payloads only" principle (§6) — a
/// future version can add `next_cursor` pagination if a real need for
/// more than this many pages arises, but that is its own reviewed change.
const PAGE_SIZE: u32 = 100;

#[derive(Debug, Clone, PartialEq)]
pub struct NotionPageSnapshot {
    pub page_id: String,
    pub title: Option<String>,
    pub url: Option<String>,
    pub parent_database_id: Option<String>,
    pub last_edited_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    results: Vec<PageDto>,
}

#[derive(Debug, Deserialize)]
struct PageDto {
    id: String,
    url: Option<String>,
    #[serde(rename = "last_edited_time")]
    last_edited_time: Option<String>,
    parent: Option<ParentDto>,
    properties: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ParentDto {
    database_id: Option<String>,
}

fn build_client(access_token: &str) -> Result<reqwest::Client, IngestionError> {
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

    let mut headers = HeaderMap::new();
    let auth_value = HeaderValue::from_str(&format!("Bearer {access_token}"))
        .map_err(|e| IngestionError::Parse(format!("notion auth header: {e}")))?;
    headers.insert(AUTHORIZATION, auth_value);
    headers.insert("Notion-Version", HeaderValue::from_static("2022-06-28"));

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| IngestionError::Network(format!("notion client build: {e}")))
}

/// Notion's title property is a per-database-schema-named property
/// (not always literally called "title"), identified by
/// `"type": "title"` rather than a fixed key — this walks the
/// properties object looking for it, same defensive shape Notion's own
/// API docs describe.
fn extract_title(properties: &Option<serde_json::Value>) -> Option<String> {
    let props = properties.as_ref()?.as_object()?;
    for value in props.values() {
        if value.get("type").and_then(|t| t.as_str()) == Some("title") {
            let title_arr = value.get("title")?.as_array()?;
            let text: String = title_arr
                .iter()
                .filter_map(|t| t.get("plain_text").and_then(|p| p.as_str()))
                .collect();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

/// Fetches page reference metadata visible to the authorized connection
/// (§1.10: "pages/databases needed by Athena"). Read-only by
/// construction — `POST /v1/search` is the only Notion endpoint this
/// module ever calls.
pub async fn fetch_pages(access_token: &str) -> Result<Vec<NotionPageSnapshot>, IngestionError> {
    let client = build_client(access_token)?;
    let body = serde_json::json!({
        "filter": { "value": "page", "property": "object" },
        "page_size": PAGE_SIZE,
    });

    let resp = client
        .post("https://api.notion.com/v1/search")
        .json(&body)
        .send()
        .await
        .map_err(|e| IngestionError::Network(format!("notion search: {e}")))?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(IngestionError::AuthExpired("notion access token rejected".into()));
    }
    if !resp.status().is_success() {
        return Err(IngestionError::Network(format!("notion search returned {}", resp.status())));
    }

    let parsed: SearchResponse = resp
        .json()
        .await
        .map_err(|e| IngestionError::Parse(format!("notion search payload: {e}")))?;

    Ok(parsed
        .results
        .into_iter()
        .map(|p| NotionPageSnapshot {
            title: extract_title(&p.properties),
            page_id: p.id,
            url: p.url,
            parent_database_id: p.parent.and_then(|pa| pa.database_id),
            last_edited_at: p.last_edited_time,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_title_finds_the_title_typed_property_regardless_of_its_key_name() {
        let properties = serde_json::json!({
            "Task name": {
                "type": "title",
                "title": [{ "plain_text": "Finish " }, { "plain_text": "milestone report" }]
            },
            "Status": { "type": "status" }
        });
        assert_eq!(
            extract_title(&Some(properties)).as_deref(),
            Some("Finish milestone report")
        );
    }

    #[test]
    fn extract_title_is_none_without_a_title_property() {
        let properties = serde_json::json!({ "Status": { "type": "status" } });
        assert_eq!(extract_title(&Some(properties)), None);
    }
}
