use crate::aws::{codepipeline::PipelineClient, load_sdk_config};
use crate::cli::CheckArgs;
use crate::config::{Overrides, Resolved};
use crate::error::Result;
use crate::github::{compare::compare_commits, GithubClient};
use crate::ui;
use colored::Colorize;

pub async fn run(args: CheckArgs) -> Result<()> {
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

    ui::tables::print_stage_revisions(&resolved.pipeline, &from, &to);

    if from.revision_id == to.revision_id {
        println!("\nstages are at the same revision — nothing to release");
        return Ok(());
    }

    println!(
        "\n{} {} → {}",
        "commits in".bold(),
        resolved.to_stage,
        resolved.from_stage,
    );
    let gh = GithubClient::new(&resolved.entry.repo)?;
    // base = currently-deployed prod (to.revision_id), head = preprod commit (from.revision_id)
    // so the diff is "what's about to be released".
    let commits = compare_commits(&gh, &to.revision_id, &from.revision_id).await?;
    if commits.is_empty() {
        println!("(no new commits)");
    } else {
        for c in &commits {
            println!("- {} {}", c.short_sha().yellow(), c.first_line());
        }
    }
    Ok(())
}
