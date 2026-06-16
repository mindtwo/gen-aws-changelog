use crate::error::Result;
use crate::jira::JiraClient;
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

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    issues: Vec<IssueResponse>,
}

/// Fetch tickets that live in the **active sprint** of any of the given
/// `projects`, filtered to tickets whose `status` is in `statuses` (when
/// non-empty). Replaces the old commit-message scraping: it surfaces work
/// the team is actually shipping this sprint, regardless of whether
/// someone remembered to reference the ticket key in their commit.
pub async fn fetch_active_sprint_tickets(
    client: &JiraClient,
    projects: &[String],
    statuses: &[String],
) -> Result<Vec<Ticket>> {
    if projects.is_empty() {
        return Ok(Vec::new());
    }

    let mut clauses = vec![format!("project in ({})", join_quoted(projects))];
    clauses.push("sprint in openSprints()".to_string());
    if !statuses.is_empty() {
        clauses.push(format!("status in ({})", join_quoted(statuses)));
    }
    let jql = clauses.join(" AND ");

    let url = client.url("search");
    let resp = client
        .http()
        .get(&url)
        .query(&[
            ("jql", jql.as_str()),
            ("fields", "summary,status,assignee"),
            ("maxResults", "100"),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("JIRA search failed ({status}): {body}");
    }
    let parsed: SearchResponse = resp.json().await?;
    let mut out: Vec<Ticket> = parsed
        .issues
        .into_iter()
        .map(|i| Ticket {
            url: client.browse_url(&i.key),
            key: i.key,
            summary: i.fields.summary,
            status: i.fields.status.and_then(|s| s.name),
            assignee: i.fields.assignee.and_then(|a| a.display_name),
        })
        .collect();
    out.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(out)
}

fn join_quoted(values: &[String]) -> String {
    values
        .iter()
        .map(|v| format!("\"{}\"", v.replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(", ")
}
