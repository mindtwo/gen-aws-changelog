use crate::config::{
    paths, GlobalConfig, ProjectConfig, ProjectRegistry, RegistryEntry, DEFAULT_REGION,
};
use crate::error::Result;
use std::path::PathBuf;

/// Optional overrides supplied on the CLI. Any `Some` value wins over project
/// + registry + global values.
#[derive(Debug, Default, Clone)]
pub struct Overrides {
    pub project: Option<String>,
    pub region: Option<String>,
    pub from_stage: Option<String>,
    pub to_stage: Option<String>,
}

/// Fully merged config used by all commands.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub entry: RegistryEntry,
    pub project_config_path: PathBuf,
    pub project: ProjectConfig,
    pub pipeline: String,
    pub region: String,
    pub from_stage: String,
    pub to_stage: String,
}

impl Resolved {
    pub fn from_overrides(overrides: &Overrides) -> Result<Self> {
        let global = GlobalConfig::load_or_default()?;

        let entry = match &overrides.project {
            Some(name) => ProjectRegistry::find(name)?,
            None => {
                let cwd = std::env::current_dir()?;
                ProjectRegistry::find_for_cwd(&cwd)?
            }
        };

        let project_config_path = entry.project_config_path();
        if !project_config_path.exists() {
            anyhow::bail!(
                "project config file not found at {} — run `aws-utils config pull` or create it",
                project_config_path.display()
            );
        }
        let project = ProjectConfig::load(&project_config_path)?;

        let region = overrides
            .region
            .clone()
            .or_else(|| project.region.clone())
            .or_else(|| global.defaults.region.clone())
            .unwrap_or_else(|| DEFAULT_REGION.to_string());

        let from_stage = overrides
            .from_stage
            .clone()
            .unwrap_or_else(|| project.from_stage.clone());

        let to_stage = overrides
            .to_stage
            .clone()
            .unwrap_or_else(|| project.to_stage.clone());

        Ok(Self {
            pipeline: project.pipeline.clone(),
            project,
            project_config_path,
            entry,
            region,
            from_stage,
            to_stage,
        })
    }
}

/// Convenience helper for commands that only need a registry entry.
pub fn entry_from_overrides(overrides: &Overrides) -> Result<RegistryEntry> {
    match &overrides.project {
        Some(name) => ProjectRegistry::find(name),
        None => {
            let cwd = std::env::current_dir()?;
            ProjectRegistry::find_for_cwd(&cwd)
        }
    }
}

// Silence unused-warning until commands use it directly.
#[allow(dead_code)]
fn _touch_paths() -> Result<PathBuf> {
    paths::config_dir()
}
