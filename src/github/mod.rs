pub mod compare;
pub mod contents;
pub mod release;

use crate::error::{parse_repo, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use std::time::Duration;

const API_BASE: &str = "https://api.github.com";

pub struct GithubClient {
    http: reqwest::Client,
    repo: String,
}

impl GithubClient {
    pub fn new(repo: impl Into<String>) -> Result<Self> {
        let repo = repo.into();
        parse_repo(&repo)?;
        let token = resolve_token()?;
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("aws-utils"));
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|_| anyhow::anyhow!("invalid GITHUB_TOKEN value"))?,
        );
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self { http, repo })
    }

    pub fn repo(&self) -> &str {
        &self.repo
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn url(&self, path: &str) -> String {
        format!("{API_BASE}/repos/{}/{}", self.repo, path)
    }
}

fn resolve_token() -> Result<String> {
    if let Ok(t) = std::env::var("GITHUB_TOKEN") {
        if !t.trim().is_empty() {
            return Ok(t);
        }
    }
    // Fallback: shell out to `gh auth token`.
    let out = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output();
    match out {
        Ok(out) if out.status.success() => {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if s.is_empty() {
                anyhow::bail!("GITHUB_TOKEN not set and `gh auth token` returned empty output");
            }
            Ok(s)
        }
        _ => anyhow::bail!("no GitHub token available: set GITHUB_TOKEN or run `gh auth login`"),
    }
}
