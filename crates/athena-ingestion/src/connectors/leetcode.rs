//! LeetCode connector (07_INTEGRATIONS.md §1.2). Identical shape to
//! Codeforces — read-only polling of the public profile/submission-
//! stats surface, no account linking beyond the username the user
//! supplies. LeetCode has no versioned public REST API for this data;
//! its own website reads difficulty-bucketed solved counts from the
//! `https://leetcode.com/graphql` endpoint, which is public and keyless
//! for a username's aggregate stats — the same "public, unauthenticated
//! surface" shape §1.2 requires, just a GraphQL query instead of a REST
//! path. If LeetCode ever ships a stable public REST endpoint for this
//! data, only this module's request-building changes; `LeetCodeSnapshot`
//! and every caller of `fetch_snapshot` stay the same.

use serde::Deserialize;
use serde_json::json;

use crate::error::IngestionError;

#[derive(Debug, Clone, PartialEq)]
pub struct LeetCodeSnapshot {
    pub handle: String,
    pub total_solved: i64,
    pub easy_solved: i64,
    pub medium_solved: i64,
    pub hard_solved: i64,
}

#[derive(Debug, Deserialize)]
struct GraphQlResponse {
    data: Option<GraphQlData>,
    errors: Option<Vec<GraphQlError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct GraphQlData {
    #[serde(rename = "matchedUser")]
    matched_user: Option<MatchedUser>,
}

#[derive(Debug, Deserialize)]
struct MatchedUser {
    #[serde(rename = "submitStats")]
    submit_stats: SubmitStats,
}

#[derive(Debug, Deserialize)]
struct SubmitStats {
    #[serde(rename = "acSubmissionNum")]
    ac_submission_num: Vec<AcSubmission>,
}

#[derive(Debug, Deserialize)]
struct AcSubmission {
    difficulty: String,
    count: i64,
}

const QUERY: &str = "query userProblemsSolved($username: String!) { \
    matchedUser(username: $username) { \
        submitStats { acSubmissionNum { difficulty count } } \
    } \
}";

/// Fetches a LeetCode username's solved-problem counts, bucketed by
/// difficulty (the `dsa_practice_log` shape — V4 migration, cited per
/// §1.2's schema-change requirement).
pub async fn fetch_snapshot(username: &str) -> Result<LeetCodeSnapshot, IngestionError> {
    if username.trim().is_empty() {
        return Err(IngestionError::NotConfigured(
            "no LeetCode username on file".into(),
        ));
    }

    let client = reqwest::Client::new();
    let body = json!({
        "query": QUERY,
        "variables": { "username": username },
    });

    let response: GraphQlResponse = client
        .post("https://leetcode.com/graphql")
        .header("Content-Type", "application/json")
        .header("Referer", "https://leetcode.com")
        .json(&body)
        .send()
        .await
        .map_err(|e| IngestionError::Network(format!("leetcode graphql: {e}")))?
        .json()
        .await
        .map_err(|e| IngestionError::Parse(format!("leetcode graphql payload: {e}")))?;

    if let Some(errors) = response.errors {
        let combined = errors.into_iter().map(|e| e.message).collect::<Vec<_>>().join("; ");
        return Err(IngestionError::Parse(format!("leetcode graphql errors: {combined}")));
    }

    let matched = response
        .data
        .and_then(|d| d.matched_user)
        .ok_or_else(|| IngestionError::Parse(format!("leetcode: no such user \"{username}\"")))?;

    let mut total = 0i64;
    let mut easy = 0i64;
    let mut medium = 0i64;
    let mut hard = 0i64;
    for bucket in matched.submit_stats.ac_submission_num {
        match bucket.difficulty.as_str() {
            "All" => total = bucket.count,
            "Easy" => easy = bucket.count,
            "Medium" => medium = bucket.count,
            "Hard" => hard = bucket.count,
            _ => {}
        }
    }

    Ok(LeetCodeSnapshot {
        handle: username.to_string(),
        total_solved: total,
        easy_solved: easy,
        medium_solved: medium,
        hard_solved: hard,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_username_is_not_configured_not_a_network_error() {
        let err = fetch_snapshot("").await.unwrap_err();
        assert!(matches!(err, IngestionError::NotConfigured(_)));
    }
}
