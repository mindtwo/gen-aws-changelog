use crate::cli::ConfigCommand;
use crate::config::{resolve, Overrides, ProjectConfig, Resolved};
use crate::error::Result;
use crate::github::{contents::fetch_file, GithubClient};
use colored::Colorize;

pub async fn run(cmd: ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::Show { project } => show(project).await,
        ConfigCommand::Edit { project } => edit(project).await,
        ConfigCommand::Push { project } => push(project).await,
        ConfigCommand::Pull { project } => pull(project).await,
    }
}

async fn show(project: Option<String>) -> Result<()> {
    let overrides = Overrides {
        project,
        ..Default::default()
    };
    let resolved = Resolved::from_overrides(&overrides)?;
    println!(
        "{} {}\n  {} {}\n  {} {}\n  {} {} → {}\n  {} {}\n  {} {}",
        "project:".bold(),
        resolved.entry.name,
        "repo:".bold(),
        resolved.entry.repo,
        "pipeline:".bold(),
        resolved.pipeline,
        "stages:".bold(),
        resolved.from_stage,
        resolved.to_stage,
        "region:".bold(),
        resolved.region,
        "config file:".bold(),
        resolved.project_config_path.display(),
    );
    if !resolved.project.jira.prefixes.is_empty() {
        println!(
            "  {} {}",
            "jira prefixes:".bold(),
            resolved.project.jira.prefixes.join(", ")
        );
    }
    let aws = &resolved.project.aws;
    if aws.default.is_some() || aws.release.is_some() || aws.s3.is_some() {
        println!("  {}", "aws accounts:".bold());
        if let Some(v) = &aws.default {
            println!("    default = {v}");
        }
        if let Some(v) = &aws.release {
            println!("    release = {v}");
        }
        if let Some(v) = &aws.s3 {
            println!("    s3      = {v}");
        }
    }
    Ok(())
}

async fn edit(project: Option<String>) -> Result<()> {
    let entry = resolve::entry_from_overrides(&Overrides {
        project,
        ..Default::default()
    })?;
    let path = entry.project_config_path();
    if !path.exists() {
        let template = ProjectConfig::template(&entry.name);
        template.save(&path)?;
        println!("{} created {}", "·".dimmed(), path.display());
    }
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .map_err(|e| anyhow::anyhow!("could not launch {editor}: {e}"))?;
    if !status.success() {
        anyhow::bail!("{editor} exited with status {status}");
    }
    // Reparse to surface syntax errors immediately.
    ProjectConfig::load(&path)?;
    println!("{} {} validated", "✓".green().bold(), path.display());
    Ok(())
}

async fn push(project: Option<String>) -> Result<()> {
    let entry = resolve::entry_from_overrides(&Overrides {
        project,
        ..Default::default()
    })?;
    let path = entry.project_config_path();
    if !path.exists() {
        anyhow::bail!(
            "{} does not exist — run `aws-utils config edit` first",
            path.display()
        );
    }
    // Stage + commit using the system git binary (git2's index/commit
    // dance is heavy and rebuilds nothing the user can't already see).
    let cwd = &entry.path;
    let rel = path
        .strip_prefix(cwd)
        .unwrap_or(&path)
        .to_string_lossy()
        .into_owned();
    run_git(cwd, &["add", &rel])?;
    let message = format!("chore: update {}", entry.config);
    let status = std::process::Command::new("git")
        .args(["-C", cwd.to_string_lossy().as_ref()])
        .args(["diff", "--cached", "--quiet"])
        .status()
        .map_err(|e| anyhow::anyhow!("git diff: {e}"))?;
    if status.success() {
        println!("{} no staged changes — nothing to push", "·".dimmed());
        return Ok(());
    }
    run_git(cwd, &["commit", "-m", &message])?;
    run_git(cwd, &["push"])?;
    println!("{} pushed {}", "✓".green().bold(), rel);
    Ok(())
}

fn run_git(cwd: &std::path::Path, args: &[&str]) -> Result<()> {
    let out = std::process::Command::new("git")
        .args(["-C", cwd.to_string_lossy().as_ref()])
        .args(args)
        .output()
        .map_err(|e| anyhow::anyhow!("git: {e}"))?;
    if !out.status.success() {
        anyhow::bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

async fn pull(project: Option<String>) -> Result<()> {
    let entry = resolve::entry_from_overrides(&Overrides {
        project,
        ..Default::default()
    })?;
    let gh = GithubClient::new(&entry.repo)?;
    let bytes = fetch_file(&gh, &entry.config).await?;
    let path = entry.project_config_path();
    std::fs::write(&path, &bytes).map_err(|e| anyhow::anyhow!("write {}: {e}", path.display()))?;
    // Validate the result so a malformed remote doesn't go unnoticed.
    ProjectConfig::load(&path)?;
    println!(
        "{} pulled {} from {}",
        "✓".green().bold(),
        path.display(),
        entry.repo,
    );
    Ok(())
}
