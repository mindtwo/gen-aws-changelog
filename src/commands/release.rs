use crate::aws::{codepipeline::PipelineClient, load_sdk_config};
use crate::changelog::{render, ChangelogInput};
use crate::cli::ReleaseArgs;
use crate::commands::auto_assume;
use crate::config::project::AwsAction;
use crate::config::{Overrides, Resolved};
use crate::error::Result;
use crate::git;
use crate::github::{compare::compare_commits, GithubClient};
use crate::jira::{extract::extract_keys, fetch::fetch_tickets, JiraClient};
use crate::ui::prompts;
use chrono::Local;
use colored::Colorize;

pub async fn run(args: ReleaseArgs) -> Result<()> {
    let resolved = Resolved::from_overrides(&Overrides {
        project: args.project,
        ..Default::default()
    })?;

    auto_assume::ensure(&resolved, AwsAction::Release)?;
    let sdk = load_sdk_config(&resolved.region).await;
    let pipeline = PipelineClient::new(&sdk, &resolved.pipeline);

    let (from, to) = tokio::try_join!(
        pipeline.stage_revision(&resolved.from_stage),
        pipeline.stage_revision(&resolved.to_stage),
    )?;

    if from.revision_id == to.revision_id {
        println!("{}", "stages are at the same revision — nothing to release".dimmed());
        return Ok(());
    }

    println!(
        "{} {} ({}) → {} ({})",
        "release plan:".bold(),
        resolved.to_stage,
        short(&to.revision_id),
        resolved.from_stage,
        short(&from.revision_id),
    );

    // 1. Render changelog
    let changelog = if args.no_changelog {
        String::new()
    } else {
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
                    "{} JIRA missing ({e}); changelog will omit ticket details",
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
        println!("\n{}", body);
        body
    };

    // 2. Confirm
    if !prompts::confirm("Approve the pending release in preprod?", false)? {
        println!("{}", "aborted".dimmed());
        return Ok(());
    }

    // 3. Approve
    let approval = pipeline.pending_approval(&resolved.from_stage).await?;
    let summary = args
        .summary
        .clone()
        .unwrap_or_else(|| "Released by aws-utils".to_string());
    pipeline
        .approve(&approval.stage, &approval.action, &approval.token, &summary)
        .await?;
    println!(
        "{} approved {}/{}",
        "✓".green().bold(),
        approval.stage,
        approval.action
    );

    // 4. Tag
    if args.no_tag {
        return Ok(());
    }
    let tag_name = format!("release-{}", Local::now().format("%d-%m-%Y"));
    let repo = git::open(&resolved.entry.path)?;
    let tag_message = if changelog.is_empty() {
        format!("Release {tag_name}")
    } else {
        changelog
    };
    git::tag_release(&repo, &from.revision_id, &tag_name, &tag_message)?;
    println!("{} tagged {} → {}", "✓".green().bold(), tag_name, short(&from.revision_id));

    if prompts::confirm("Push tag to origin?", true)? {
        git::push_tag(&repo, "origin", &tag_name)?;
        println!("{} pushed {} to origin", "✓".green().bold(), tag_name);
    }

    Ok(())
}

fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}
