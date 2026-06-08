use crate::cli::ConfigCommand;
use crate::config::{resolve, Overrides, ProjectConfig, Resolved};
use crate::error::Result;
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

async fn push(_project: Option<String>) -> Result<()> {
    anyhow::bail!("`config push` is implemented in Phase 6 (needs git layer)")
}

async fn pull(_project: Option<String>) -> Result<()> {
    anyhow::bail!("`config pull` is implemented in Phase 4 (needs GitHub layer)")
}
