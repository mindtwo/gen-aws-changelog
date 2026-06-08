use crate::error::Result;
use crate::jira::JiraClient;
use futures::stream::{FuturesUnordered, StreamExt};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Ticket {
    pub key: String,
    pub summary: String,
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct IssueResponse {
    key: String,
    fields: IssueFields,
}

#[derive(Debug, Deserialize)]
struct IssueFields {
    summary: String,
    status: Option<NamedField>,
    assignee: Option<Assignee>,
}

#[derive(Debug, Deserialize)]
struct NamedField {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Assignee {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

/// Fetch each key concurrently. Tickets that JIRA rejects (404, 403, etc.)
/// are skipped with a warning so a single bad key doesn't fail the whole
/// changelog.
pub async fn fetch_tickets(client: &JiraClient, keys: &[String]) -> Result<Vec<Ticket>> {
    let mut tasks = FuturesUnordered::new();
    for key in keys {
        let key = key.clone();
        let url = client.url(&format!("issue/{key}?fields=summary,status,assignee"));
        let http = client.http().clone();
        let browse = client.browse_url(&key);
        tasks.push(async move {
            let resp = http.get(&url).send().await?;
            if !resp.status().is_success() {
                tracing::warn!("skipping {key}: status {}", resp.status());
                return Ok::<Option<Ticket>, anyhow::Error>(None);
            }
            let parsed: IssueResponse = resp.json().await?;
            Ok(Some(Ticket {
                key: parsed.key,
                summary: parsed.fields.summary,
                status: parsed.fields.status.and_then(|s| s.name),
                assignee: parsed.fields.assignee.and_then(|a| a.display_name),
                url: browse,
            }))
        });
    }

    let mut out = Vec::new();
    while let Some(res) = tasks.next().await {
        if let Some(ticket) = res? {
            out.push(ticket);
        }
    }
    out.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(out)
}
