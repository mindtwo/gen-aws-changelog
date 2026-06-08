//! Config layer.
//!
//! Three layers of configuration:
//! - [`GlobalConfig`]: `~/.config/aws-utils/config.toml` (cross-project defaults)
//! - [`RegistryEntry`]: `~/.config/aws-utils/projects/<name>.toml` (one per
//!   project registered via `aws-utils add`)
//! - [`ProjectConfig`]: `.aws-utils.toml` inside the project repo (pipeline,
//!   stages, jira hints — checked into version control)
//!
//! [`resolve`] merges them with this precedence: **CLI > project file >
//! registry entry > global defaults**.

pub mod paths;
pub mod project;
pub mod registry;
pub mod resolve;

pub use project::ProjectConfig;
pub use registry::{RegistryEntry, ProjectRegistry};
pub use resolve::{Resolved, Overrides};

use serde::{Deserialize, Serialize};

pub const DEFAULT_REGION: &str = "eu-central-1";
pub const PROJECT_CONFIG_FILENAME: &str = ".aws-utils.toml";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub github: GithubGlobal,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Defaults {
    pub region: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GithubGlobal {
    pub token: Option<String>,
}

impl GlobalConfig {
    pub fn load_or_default() -> crate::error::Result<Self> {
        let path = paths::global_config_file()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("read {}: {e}", path.display()))?;
        let cfg: Self = toml::from_str(&text)
            .map_err(|e| anyhow::anyhow!("parse {}: {e}", path.display()))?;
        Ok(cfg)
    }
}
