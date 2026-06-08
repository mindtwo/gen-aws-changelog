pub mod extract;
pub mod fetch;

use crate::error::{AppError, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use std::time::Duration;

pub struct JiraClient {
    http: reqwest::Client,
    base_url: String,
}

impl JiraClient {
    pub fn from_env() -> Result<Self> {
        let base_url =
            std::env::var("JIRA_BASE_URL").map_err(|_| AppError::MissingEnv("JIRA_BASE_URL"))?;
        let email = std::env::var("JIRA_EMAIL").map_err(|_| AppError::MissingEnv("JIRA_EMAIL"))?;
        let token =
            std::env::var("JIRA_API_TOKEN").map_err(|_| AppError::MissingEnv("JIRA_API_TOKEN"))?;

        let auth = format!("{email}:{token}");
        let encoded = basic_b64(auth.as_bytes());
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Basic {encoded}"))
                .map_err(|_| anyhow::anyhow!("invalid JIRA credentials"))?,
        );
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}/rest/api/3/{}", self.base_url, path)
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn browse_url(&self, key: &str) -> String {
        format!("{}/browse/{}", self.base_url, key)
    }
}

fn basic_b64(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 0x3F) as usize] as char);
        out.push(T[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[((n >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(T[(n & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}
