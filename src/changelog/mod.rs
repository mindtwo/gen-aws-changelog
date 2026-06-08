use crate::github::compare::Commit;
use crate::jira::fetch::Ticket;
use chrono::Local;
use once_cell::sync::Lazy;
use regex::Regex;

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

/// Conventional commit groups rendered in this fixed order. Anything that
/// doesn't match a known type ends up under [`MISC`].
const GROUPS: &[(&str, &str)] = &[
    ("feat", "Features"),
    ("fix", "Bug Fixes"),
    ("perf", "Performance"),
    ("refactor", "Refactors"),
    ("docs", "Documentation"),
    ("test", "Tests"),
    ("build", "Build System"),
    ("ci", "CI"),
    ("chore", "Chores"),
    ("revert", "Reverts"),
];
const MISC: &str = "Miscellaneous";

/// `type(scope)!: subject` — captures: 1=type, 2=scope (optional, no parens),
/// 3=`!` marker (optional), 4=subject.
static CONVENTIONAL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?P<type>[a-zA-Z]+)(?:\((?P<scope>[^)]+)\))?(?P<bang>!)?:\s*(?P<subject>.+)$")
        .expect("regex")
});

struct ParsedCommit<'a> {
    short_sha: String,
    group: &'static str, // heading key (e.g. "Features") or MISC
    scope: Option<String>,
    breaking: bool,
    subject: String,
    original: &'a str,
}

fn parse_commit(c: &Commit) -> ParsedCommit<'_> {
    let first = c.first_line();
    let short_sha = c.short_sha();
    if let Some(caps) = CONVENTIONAL.captures(first) {
        let kind = caps["type"].to_lowercase();
        let heading = GROUPS
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, h)| *h)
            .unwrap_or(MISC);
        ParsedCommit {
            short_sha,
            group: heading,
            scope: caps.name("scope").map(|m| m.as_str().to_string()),
            breaking: caps.name("bang").is_some(),
            subject: caps["subject"].to_string(),
            original: first,
        }
    } else {
        ParsedCommit {
            short_sha,
            group: MISC,
            scope: None,
            breaking: false,
            subject: first.to_string(),
            original: first,
        }
    }
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
        return out;
    }

    // Group, preserving input order within each group.
    let parsed: Vec<ParsedCommit> = input.commits.iter().map(parse_commit).collect();
    for (_key, heading) in GROUPS.iter().chain(std::iter::once(&("", MISC))) {
        let group_commits: Vec<&ParsedCommit> =
            parsed.iter().filter(|p| p.group == *heading).collect();
        if group_commits.is_empty() {
            continue;
        }
        out.push_str(&format!("### {heading}\n\n"));
        for p in group_commits {
            out.push_str(&format_line(p));
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

fn format_line(p: &ParsedCommit<'_>) -> String {
    // For misc lines we keep the original first line untouched so the
    // reader sees what was actually in git.
    if p.group == MISC {
        return format!("- `{}` {}", p.short_sha, p.original);
    }
    let mut line = format!("- `{}` ", p.short_sha);
    if p.breaking {
        line.push_str("**BREAKING** ");
    }
    if let Some(scope) = &p.scope {
        line.push_str(&format!("**{scope}**: "));
    }
    line.push_str(&p.subject);
    line
}

fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::compare::{Commit, CommitDetails};

    fn commit(sha: &str, msg: &str) -> Commit {
        Commit {
            sha: sha.to_string(),
            commit: CommitDetails {
                message: msg.to_string(),
                author: None,
            },
            author: None,
            html_url: None,
        }
    }

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

    #[test]
    fn groups_by_conventional_type() {
        let commits = vec![
            commit("aaaaaaa", "feat: add foo"),
            commit("bbbbbbb", "fix(api): handle null"),
            commit("ccccccc", "random commit without prefix"),
            commit("ddddddd", "chore: bump deps"),
            commit("eeeeeee", "feat(tui)!: rewrite event loop"),
            commit("fffffff", "wip thing"),
        ];
        let r = render(&ChangelogInput {
            project: "demo",
            pipeline: "p",
            from_stage: "a",
            to_stage: "b",
            from_sha: "0",
            to_sha: "1",
            commits: &commits,
            tickets: &[],
        });
        // Sections in expected order.
        let feat = r.find("### Features").unwrap();
        let fix = r.find("### Bug Fixes").unwrap();
        let chore = r.find("### Chores").unwrap();
        let misc = r.find("### Miscellaneous").unwrap();
        assert!(feat < fix && fix < chore && chore < misc);

        // Scope rendered.
        assert!(r.contains("**api**: handle null"));
        // Breaking marker.
        assert!(r.contains("**BREAKING** **tui**: rewrite event loop"));
        // Miscellaneous keeps original line.
        assert!(r.contains("random commit without prefix"));
        assert!(r.contains("wip thing"));
    }

    #[test]
    fn unknown_conventional_type_goes_to_misc() {
        // 'noop' is not in the recognized list.
        let commits = vec![commit("aaaaaaa", "noop: did nothing")];
        let r = render(&ChangelogInput {
            project: "demo",
            pipeline: "p",
            from_stage: "a",
            to_stage: "b",
            from_sha: "0",
            to_sha: "1",
            commits: &commits,
            tickets: &[],
        });
        assert!(r.contains("### Miscellaneous"));
        assert!(r.contains("noop: did nothing"));
    }
}
