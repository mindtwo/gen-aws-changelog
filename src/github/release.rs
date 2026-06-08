#![allow(dead_code)] // consumed by `release` command in Phase 6

use crate::error::Result;
use crate::github::GithubClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct CreateReleaseBody<'a> {
    tag_name: &'a str,
    name: &'a str,
    body: &'a str,
    draft: bool,
    prerelease: bool,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseResponse {
    pub html_url: String,
    pub tag_name: String,
}

pub async fn create_release(
    client: &GithubClient,
    tag: &str,
    title: &str,
    body: &str,
) -> Result<ReleaseResponse> {
    let url = client.url("releases");
    let resp = client
        .http()
        .post(&url)
        .json(&CreateReleaseBody {
            tag_name: tag,
            name: title,
            body,
            draft: false,
            prerelease: false,
        })
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("GitHub release create failed ({status}): {body}");
    }
    Ok(resp.json().await?)
}
