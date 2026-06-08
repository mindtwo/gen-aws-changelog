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
    /// Which preconfigured account name (from global config) to assume into
    /// for each action group. Skipped when `AWS_SESSION_TOKEN` is already set
    /// in the env.
    #[serde(default)]
    pub aws: AwsAccounts,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct JiraConfig {
    /// JIRA project key prefixes to match in commit messages (e.g. ["LEARN"])
    #[serde(default)]
    pub prefixes: Vec<String>,
}

/// Per-action account selection. All fields optional; `release` covers
/// check/changelog/release, `s3` covers s3-check. Falls back to `default`
/// when an action-specific slot is unset.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AwsAccounts {
    pub default: Option<String>,
    pub release: Option<String>,
    pub s3: Option<String>,
}

#[derive(Debug, Copy, Clone)]
pub enum AwsAction {
    Release,
    S3,
}

impl AwsAccounts {
    pub fn account_for(&self, action: AwsAction) -> Option<&str> {
        let primary = match action {
            AwsAction::Release => self.release.as_deref(),
            AwsAction::S3 => self.s3.as_deref(),
        };
        primary.or(self.default.as_deref())
    }
}

impl ProjectConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("read {}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| anyhow::anyhow!("parse {}: {e}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self)?;
        std::fs::write(path, text).map_err(|e| anyhow::anyhow!("write {}: {e}", path.display()))?;
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
            aws: AwsAccounts::default(),
        }
    }
}
