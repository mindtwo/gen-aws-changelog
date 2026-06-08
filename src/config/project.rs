use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Per-project configuration checked into the repo as `.aws-utils.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub pipeline: String,
    pub region: Option<String>,
    pub from_stage: String,
    pub to_stage: String,
    #[serde(default)]
    pub jira: JiraConfig,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct JiraConfig {
    /// JIRA project key prefixes to match in commit messages (e.g. ["LEARN"])
    #[serde(default)]
    pub prefixes: Vec<String>,
}

impl ProjectConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("read {}: {e}", path.display()))?;
        toml::from_str(&text)
            .map_err(|e| anyhow::anyhow!("parse {}: {e}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self)?;
        std::fs::write(path, text)
            .map_err(|e| anyhow::anyhow!("write {}: {e}", path.display()))?;
        Ok(())
    }

    /// Minimal stub for `add` to drop into a fresh project.
    pub fn template(pipeline: &str) -> Self {
        Self {
            pipeline: pipeline.to_string(),
            region: Some(super::DEFAULT_REGION.to_string()),
            from_stage: "DeployPreProd".to_string(),
            to_stage: "DeployProd".to_string(),
            jira: JiraConfig::default(),
        }
    }
}
