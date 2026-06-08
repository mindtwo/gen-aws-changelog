use crate::cli::AddArgs;
use crate::config::{
    paths, ProjectConfig, ProjectRegistry, RegistryEntry, PROJECT_CONFIG_FILENAME,
};
use crate::error::{parse_repo, Result};
use colored::Colorize;
use std::process::Command;

pub async fn run(args: AddArgs) -> Result<()> {
    let cwd = std::env::current_dir()?.canonicalize()?;

    let name = match args.name {
        Some(n) => n,
        None => cwd
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("could not derive project name from {}", cwd.display()))?
            .to_string(),
    };

    let repo = match args.repo {
        Some(r) => {
            parse_repo(&r)?;
            r
        }
        None => detect_origin_repo(&cwd)?,
    };

    let entry = RegistryEntry {
        name: name.clone(),
        path: cwd.clone(),
        repo: repo.clone(),
        config: PROJECT_CONFIG_FILENAME.to_string(),
    };

    let registry_path = ProjectRegistry::save(&entry)?;
    println!(
        "{} registered project '{}' → {}",
        "✓".green().bold(),
        name.bold(),
        registry_path.display()
    );

    // Scaffold .aws-utils.toml if missing so `config edit` / `config push` work.
    let project_cfg_path = cwd.join(PROJECT_CONFIG_FILENAME);
    if !project_cfg_path.exists() {
        let template = ProjectConfig::template(&name);
        template.save(&project_cfg_path)?;
        println!(
            "{} created {} (edit pipeline + stages before running `check`)",
            "✓".green().bold(),
            project_cfg_path.display()
        );
    } else {
        println!(
            "{} {} already exists — leaving it alone",
            "·".dimmed(),
            project_cfg_path.display()
        );
    }

    // Make sure the projects dir exists so subsequent runs don't error.
    paths::ensure_dir(&paths::projects_dir()?)?;
    Ok(())
}

fn detect_origin_repo(cwd: &std::path::Path) -> Result<String> {
    let out = Command::new("git")
        .args(["-C", cwd.to_str().unwrap_or(".")])
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(|e| anyhow::anyhow!("could not run `git remote get-url origin`: {e}"))?;

    if !out.status.success() {
        anyhow::bail!(
            "no `origin` git remote in {} — pass --repo owner/name",
            cwd.display()
        );
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    parse_git_remote(&url)
        .ok_or_else(|| anyhow::anyhow!("could not parse owner/name from git remote: {url}"))
}

/// Accept both SSH (`git@github.com:owner/name.git`) and HTTPS
/// (`https://github.com/owner/name.git`) forms and return `owner/name`.
fn parse_git_remote(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches(".git");
    if let Some(rest) = trimmed.strip_prefix("git@") {
        // git@github.com:owner/name
        let (_, path) = rest.split_once(':')?;
        return validate(path);
    }
    if let Some(rest) = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
    {
        // github.com/owner/name
        let (_, path) = rest.split_once('/')?;
        return validate(path);
    }
    None
}

fn validate(path: &str) -> Option<String> {
    let mut parts = path.split('/');
    let owner = parts.next()?;
    let name = parts.next()?;
    if owner.is_empty() || name.is_empty() {
        return None;
    }
    Some(format!("{owner}/{name}"))
}

#[cfg(test)]
mod tests {
    use super::parse_git_remote;

    #[test]
    fn parses_ssh() {
        assert_eq!(
            parse_git_remote("git@github.com:mindtwo/foo.git").as_deref(),
            Some("mindtwo/foo")
        );
    }

    #[test]
    fn parses_https() {
        assert_eq!(
            parse_git_remote("https://github.com/mindtwo/foo.git").as_deref(),
            Some("mindtwo/foo")
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_git_remote("not-a-url").is_none());
    }
}
