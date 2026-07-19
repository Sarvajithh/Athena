//! Codeforces connector (07_INTEGRATIONS.md §1.1). Read-only polling of
//! the public API — no auth, no credential of any kind.

use serde::Deserialize;

use crate::error::IngestionError;

/// What this connector hands back to `athena-app` for it to persist as
/// a `codeforces_snapshots` row. Deliberately not the same shape as the
/// raw API response — only the fields §1.1's feeds (trajectory metric,
/// Divergence Check, Career Analysis) actually need.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeforcesSnapshot {
    pub handle: String,
    pub rating: Option<i64>,
    pub max_rating: Option<i64>,
    pub rank: Option<String>,
    pub solved_count: i64,
}

#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    status: String,
    result: Option<Vec<UserInfoResult>>,
    comment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserInfoResult {
    rating: Option<i64>,
    #[serde(rename = "maxRating")]
    max_rating: Option<i64>,
    rank: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserStatusResponse {
    status: String,
    result: Option<Vec<SubmissionResult>>,
    comment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubmissionResult {
    verdict: Option<String>,
    problem: SubmissionProblem,
}

#[derive(Debug, Deserialize)]
struct SubmissionProblem {
    #[serde(rename = "contestId")]
    contest_id: Option<i64>,
    index: Option<String>,
}

/// Fetches a Codeforces handle's current rating/rank plus a distinct
/// solved-problem count (§1.1: `user.rating`, `user.status`). Two
/// public, keyless endpoints — `user.info` for rating/rank, `user.status`
/// for the submission history a solved count is derived from.
///
/// Never blocks the app: this is a plain async fn the caller (a
/// scheduler tick or a manual "sync now") awaits on its own — nothing
/// about this function's shape forces it onto the startup path.
pub async fn fetch_snapshot(handle: &str) -> Result<CodeforcesSnapshot, IngestionError> {
    if handle.trim().is_empty() {
        return Err(IngestionError::NotConfigured(
            "no Codeforces handle on file".into(),
        ));
    }

    let info_url = format!("https://codeforces.com/api/user.info?handles={handle}");
    let info: UserInfoResponse = reqwest::get(&info_url)
        .await
        .map_err(|e| IngestionError::Network(format!("codeforces user.info: {e}")))?
        .json()
        .await
        .map_err(|e| IngestionError::Parse(format!("codeforces user.info payload: {e}")))?;

    if info.status != "OK" {
        return Err(IngestionError::Parse(format!(
            "codeforces user.info returned status={}: {}",
            info.status,
            info.comment.unwrap_or_default()
        )));
    }
    let user = info
        .result
        .and_then(|mut r| if r.is_empty() { None } else { Some(r.remove(0)) })
        .ok_or_else(|| IngestionError::Parse("codeforces user.info: empty result".into()))?;

    let status_url = format!("https://codeforces.com/api/user.status?handle={handle}&from=1&count=10000");
    let status: UserStatusResponse = reqwest::get(&status_url)
        .await
        .map_err(|e| IngestionError::Network(format!("codeforces user.status: {e}")))?
        .json()
        .await
        .map_err(|e| IngestionError::Parse(format!("codeforces user.status payload: {e}")))?;

    if status.status != "OK" {
        return Err(IngestionError::Parse(format!(
            "codeforces user.status returned status={}: {}",
            status.status,
            status.comment.unwrap_or_default()
        )));
    }

    let mut solved = std::collections::HashSet::new();
    for submission in status.result.unwrap_or_default() {
        if submission.verdict.as_deref() == Some("OK") {
            if let (Some(contest_id), Some(index)) =
                (submission.problem.contest_id, submission.problem.index)
            {
                solved.insert((contest_id, index));
            }
        }
    }

    Ok(CodeforcesSnapshot {
        handle: handle.to_string(),
        rating: user.rating,
        max_rating: user.max_rating,
        rank: user.rank,
        solved_count: solved.len() as i64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_handle_is_not_configured_not_a_network_error() {
        let err = fetch_snapshot("").await.unwrap_err();
        assert!(matches!(err, IngestionError::NotConfigured(_)));
    }
}
