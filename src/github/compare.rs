#![allow(dead_code)] // some Commit fields surfaced by future TUI / templates

use crate::error::Result;
use crate::github::GithubClient;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Commit {
    pub sha: String,
    #[serde(default)]
    pub commit: CommitDetails,
    #[serde(default)]
    pub author: Option<CommitAuthor>,
    #[serde(default)]
    pub html_url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CommitDetails {
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub author: Option<CommitAuthorDetails>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitAuthorDetails {
    pub name: Option<String>,
    pub email: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitAuthor {
    pub login: Option<String>,
}

impl Commit {
    pub fn short_sha(&self) -> String {
        self.sha.chars().take(7).collect()
    }

    pub fn first_line(&self) -> &str {
        self.commit.message.lines().next().unwrap_or("")
    }
}

#[derive(Debug, Deserialize)]
struct CompareResponse {
    commits: Vec<Commit>,
}

/// Returns commits **between** `base` and `head`. The GitHub API's
/// "compare" endpoint lists commits in `head` not in `base`, so call with
/// `base = old_revision` and `head = new_revision`.
pub async fn compare_commits(
    client: &GithubClient,
    base: &str,
    head: &str,
) -> Result<Vec<Commit>> {
    let url = client.url(&format!("compare/{base}...{head}"));
    let resp = client.http().get(&url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("GitHub compare failed ({status}): {body}");
    }
    let parsed: CompareResponse = resp.json().await?;
    Ok(parsed.commits)
}
