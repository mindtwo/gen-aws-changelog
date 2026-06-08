use crate::aws::{codepipeline::PipelineClient, load_sdk_config};
use crate::changelog::{render, ChangelogInput};
use crate::cli::ChangelogArgs;
use crate::config::{Overrides, Resolved};
use crate::error::Result;
use crate::github::{compare::compare_commits, GithubClient};
use crate::jira::{extract::extract_keys, fetch::fetch_tickets, JiraClient};
use colored::Colorize;

pub async fn run(args: ChangelogArgs) -> Result<()> {
    let resolved = Resolved::from_overrides(&Overrides {
        project: args.project,
        from_stage: args.from_stage,
        to_stage: args.to_stage,
        ..Default::default()
    })?;

    let sdk = load_sdk_config(&resolved.region).await;
    let pipeline = PipelineClient::new(&sdk, &resolved.pipeline);
    let (from, to) = tokio::try_join!(
        pipeline.stage_revision(&resolved.from_stage),
        pipeline.stage_revision(&resolved.to_stage),
    )?;

    let gh = GithubClient::new(&resolved.entry.repo)?;
    let commits = compare_commits(&gh, &to.revision_id, &from.revision_id).await?;

    let tickets = match JiraClient::from_env() {
        Ok(client) => {
            let keys = extract_keys(
                commits.iter().map(|c| c.commit.message.as_str()),
                &resolved.project.jira.prefixes,
            );
            if keys.is_empty() {
                Vec::new()
            } else {
                fetch_tickets(&client, &keys).await?
            }
        }
        Err(e) => {
            eprintln!(
                "{} JIRA credentials missing — skipping ticket enrichment ({e})",
                "warn:".yellow().bold()
            );
            Vec::new()
        }
    };

    let body = render(&ChangelogInput {
        project: &resolved.entry.name,
        pipeline: &resolved.pipeline,
        from_stage: &resolved.from_stage,
        to_stage: &resolved.to_stage,
        from_sha: &from.revision_id,
        to_sha: &to.revision_id,
        commits: &commits,
        tickets: &tickets,
    });

    if let Some(path) = args.out {
        std::fs::write(&path, &body)
            .map_err(|e| anyhow::anyhow!("write {}: {e}", path.display()))?;
        println!("{} wrote {}", "✓".green().bold(), path.display());
    } else {
        print!("{body}");
    }
    Ok(())
}
