//! Config layer.
//!
//! Three layers of configuration:
//! - `GlobalConfig` — `~/.config/aws-utils/config.toml` (cross-project defaults)
//! - `RegistryEntry` — `~/.config/aws-utils/projects/<name>.toml` (one per
//!   project registered via `aws-utils add`)
//! - `ProjectConfig` — `.aws-utils.toml` inside the project repo (pipeline,
//!   stages, jira hints — checked into version control)
//!
//! The [`resolve`] module merges them with precedence:
//! CLI > project file > registry entry > global defaults.

pub mod paths;
pub mod project;
pub mod registry;
pub mod resolve;

#[allow(unused_imports)]
pub use project::{AwsAccounts, ProjectConfig};
pub use registry::{ProjectRegistry, RegistryEntry};
pub use resolve::{Overrides, Resolved};

use serde::{Deserialize, Serialize};

pub const DEFAULT_REGION: &str = "eu-central-1";
pub const PROJECT_CONFIG_FILENAME: &str = ".aws-utils.toml";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub github: GithubGlobal,
    #[serde(default)]
    pub jira: JiraGlobal,
    /// Pre-configured AWS account names (passed to `assume-role`).
    #[serde(default, rename = "accounts")]
    pub accounts: Vec<Account>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Defaults {
    pub region: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GithubGlobal {
    pub token: Option<String>,
}

/// JIRA Cloud credentials shared across projects. Hydrated into the env
/// on startup so `JiraClient::from_env()` picks them up transparently
/// (env vars still win, so a `.env` override keeps working).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct JiraGlobal {
    pub base_url: Option<String>,
    pub email: Option<String>,
    pub api_token: Option<String>,
}

impl JiraGlobal {
    /// Copy into process env without overwriting anything already set
    /// (so `.env` and direct exports still take precedence).
    pub fn hydrate_env(&self) {
        if let Some(v) = self.base_url.as_deref() {
            if std::env::var_os("JIRA_BASE_URL").is_none() {
                std::env::set_var("JIRA_BASE_URL", v);
            }
        }
        if let Some(v) = self.email.as_deref() {
            if std::env::var_os("JIRA_EMAIL").is_none() {
                std::env::set_var("JIRA_EMAIL", v);
            }
        }
        if let Some(v) = self.api_token.as_deref() {
            if std::env::var_os("JIRA_API_TOKEN").is_none() {
                std::env::set_var("JIRA_API_TOKEN", v);
            }
        }
    }
}

impl GlobalConfig {
    pub fn load_or_default() -> crate::error::Result<Self> {
        let path = paths::global_config_file()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("read {}: {e}", path.display()))?;
        let cfg: Self =
            toml::from_str(&text).map_err(|e| anyhow::anyhow!("parse {}: {e}", path.display()))?;
        Ok(cfg)
    }

    pub fn save(&self) -> crate::error::Result<()> {
        let path = paths::global_config_file()?;
        paths::ensure_dir(path.parent().unwrap())?;
        let text = toml::to_string_pretty(self)?;
        std::fs::write(&path, &text)
            .map_err(|e| anyhow::anyhow!("write {}: {e}", path.display()))?;
        // The file holds a JIRA API token; tighten perms on unix.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }
}
