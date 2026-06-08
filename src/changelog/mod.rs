use crate::github::compare::Commit;
use crate::jira::fetch::Ticket;
use chrono::Local;

pub struct ChangelogInput<'a> {
    pub project: &'a str,
    pub pipeline: &'a str,
    pub from_stage: &'a str,
    pub to_stage: &'a str,
    pub from_sha: &'a str,
    pub to_sha: &'a str,
    pub commits: &'a [Commit],
    pub tickets: &'a [Ticket],
}

pub fn render(input: &ChangelogInput<'_>) -> String {
    let date = Local::now().format("%Y-%m-%d");
    let mut out = String::new();
    out.push_str(&format!("# Release {date} — {}\n\n", input.project));
    out.push_str(&format!(
        "Pipeline: `{}`  \nStages: `{}` → `{}`  \nCommits: `{}` → `{}`\n\n",
        input.pipeline,
        input.from_stage,
        input.to_stage,
        short(input.to_sha),
        short(input.from_sha),
    ));

    out.push_str("## Tickets\n\n");
    if input.tickets.is_empty() {
        out.push_str("_No JIRA tickets referenced in commit messages._\n\n");
    } else {
        for t in input.tickets {
            let status = t.status.as_deref().unwrap_or("?");
            let assignee = t.assignee.as_deref().unwrap_or("unassigned");
            out.push_str(&format!(
                "- [{}]({}) — {} _(status: {}, assignee: {})_\n",
                t.key, t.url, t.summary, status, assignee
            ));
        }
        out.push('\n');
    }

    out.push_str("## Commits\n\n");
    if input.commits.is_empty() {
        out.push_str("_No commits between stages._\n");
    } else {
        for c in input.commits {
            out.push_str(&format!("- `{}` {}\n", c.short_sha(), c.first_line()));
        }
    }
    out
}

fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_empty_input() {
        let r = render(&ChangelogInput {
            project: "demo",
            pipeline: "demo-pipeline",
            from_stage: "Preprod",
            to_stage: "Prod",
            from_sha: "aaaaaaa1234",
            to_sha: "bbbbbbb5678",
            commits: &[],
            tickets: &[],
        });
        assert!(r.contains("# Release"));
        assert!(r.contains("No JIRA tickets"));
        assert!(r.contains("No commits"));
    }
}
