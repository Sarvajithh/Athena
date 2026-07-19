//! GitHub connector (07_INTEGRATIONS.md §1.3). Read-only polling of the
//! public REST API for commit activity, PR/issue counts on repos the
//! user explicitly links — never a full account scan.

use serde::Deserialize;

use crate::error::IngestionError;

#[derive(Debug, Clone, PartialEq)]
pub struct GithubRepoSnapshot {
    pub repo_full_name: String,
    pub commit_count_30d: i64,
    pub open_pr_count: i64,
    pub open_issue_count: i64,
    pub last_commit_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepoResponse {
    open_issues_count: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CommitResponse {
    commit: CommitDetail,
}

#[derive(Debug, Deserialize)]
struct CommitDetail {
    author: Option<CommitAuthor>,
}

#[derive(Debug, Deserialize)]
struct CommitAuthor {
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PullRequestSummary {
    #[allow(dead_code)]
    number: i64,
}

fn build_client(token: Option<&str>) -> Result<reqwest::Client, IngestionError> {
    use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};

    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));
    headers.insert(USER_AGENT, HeaderValue::from_static("athena-app"));
    if let Some(token) = token {
        let value = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|e| IngestionError::Parse(format!("github token header: {e}")))?;
        headers.insert(AUTHORIZATION, value);
    }

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| IngestionError::Network(format!("github client build: {e}")))
}

/// Fetches one linked repo's commit cadence (last 30 days, capped at
/// GitHub's 100-per-page default — a portfolio-strength signal, not an
/// exhaustive audit) plus open PR/issue counts.
///
/// `token` is `None` for a repo the user hasn't provided a personal
/// access token for — GitHub's public API still serves public-repo data
/// unauthenticated, at a lower rate limit (§4: "a read-only-scoped
/// personal access token... never in SQLite" — the token itself is
/// resolved from the OS keychain by the caller in `athena-app`, this
/// connector never touches the keychain directly).
pub async fn fetch_repo_snapshot(
    repo_full_name: &str,
    token: Option<&str>,
) -> Result<GithubRepoSnapshot, IngestionError> {
    if repo_full_name.trim().is_empty() || !repo_full_name.contains('/') {
        return Err(IngestionError::NotConfigured(format!(
            "\"{repo_full_name}\" is not a valid owner/repo name"
        )));
    }

    let client = build_client(token)?;

    let repo_url = format!("https://api.github.com/repos/{repo_full_name}");
    let repo_resp = client
        .get(&repo_url)
        .send()
        .await
        .map_err(|e| IngestionError::Network(format!("github repo lookup: {e}")))?;
    if repo_resp.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(IngestionError::RateLimited(
            "github API rate limit reached".into(),
        ));
    }
    if !repo_resp.status().is_success() {
        return Err(IngestionError::Network(format!(
            "github repo lookup returned {}",
            repo_resp.status()
        )));
    }
    let repo: RepoResponse = repo_resp
        .json()
        .await
        .map_err(|e| IngestionError::Parse(format!("github repo payload: {e}")))?;

    let since = chrono_like_30_days_ago();
    let commits_url = format!("https://api.github.com/repos/{repo_full_name}/commits?since={since}&per_page=100");
    let commits: Vec<CommitResponse> = client
        .get(&commits_url)
        .send()
        .await
        .map_err(|e| IngestionError::Network(format!("github commits: {e}")))?
        .json()
        .await
        .map_err(|e| IngestionError::Parse(format!("github commits payload: {e}")))?;

    let last_commit_at = commits
        .first()
        .and_then(|c| c.commit.author.as_ref())
        .and_then(|a| a.date.clone());

    let prs_url = format!("https://api.github.com/repos/{repo_full_name}/pulls?state=open&per_page=100");
    let prs: Vec<PullRequestSummary> = client
        .get(&prs_url)
        .send()
        .await
        .map_err(|e| IngestionError::Network(format!("github pulls: {e}")))?
        .json()
        .await
        .map_err(|e| IngestionError::Parse(format!("github pulls payload: {e}")))?;

    // GitHub's `open_issues_count` on the repo resource includes open
    // PRs (a documented API quirk) — subtract them so
    // `open_issue_count` means "issues," matching §1.3's field name.
    let open_issue_count = (repo.open_issues_count.unwrap_or(0) - prs.len() as i64).max(0);

    Ok(GithubRepoSnapshot {
        repo_full_name: repo_full_name.to_string(),
        commit_count_30d: commits.len() as i64,
        open_pr_count: prs.len() as i64,
        open_issue_count,
        last_commit_at,
    })
}

/// A minimal ISO-8601 "30 days ago" timestamp for the `since` query
/// param, computed without pulling in a full date/time crate for one
/// call site (`chrono` is not a dependency of this crate — Implementation
/// Plan §4, "no dependency added solely for one narrow use").
fn chrono_like_30_days_ago() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let thirty_days_ago_secs = now.as_secs().saturating_sub(30 * 24 * 60 * 60);
    format_unix_as_iso8601(thirty_days_ago_secs)
}

/// Civil-calendar conversion from a Unix timestamp to `YYYY-MM-DDTHH:MM:SSZ`,
/// good enough for a "since" query filter (GitHub tolerates any valid
/// ISO-8601 instant; sub-day precision here isn't load-bearing).
fn format_unix_as_iso8601(secs: u64) -> String {
    let days_since_epoch = secs / 86_400;
    let secs_of_day = secs % 86_400;
    let (year, month, day) = civil_from_days(days_since_epoch as i64);
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's `civil_from_days` algorithm (public domain), the
/// standard constant-time epoch-days -> (year, month, day) conversion.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invalid_repo_name_is_not_configured_not_a_network_error() {
        let err = fetch_repo_snapshot("not-a-valid-name", None).await.unwrap_err();
        assert!(matches!(err, IngestionError::NotConfigured(_)));
    }

    #[test]
    fn civil_from_days_matches_a_known_epoch_date() {
        // 2026-07-16 is day 20,650 since the Unix epoch (1970-01-01).
        assert_eq!(civil_from_days(20_650), (2026, 7, 16));
    }
}
