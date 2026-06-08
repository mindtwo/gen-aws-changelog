pub mod runner;

use crate::config::paths;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub steps: Vec<RecipeStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStep {
    /// Name of a registered project (see `aws-utils add`).
    pub project: String,
}

impl Recipe {
    pub fn file_path(name: &str) -> Result<PathBuf> {
        Ok(paths::recipes_dir()?.join(format!("{name}.toml")))
    }

    pub fn save(&self) -> Result<PathBuf> {
        let dir = paths::recipes_dir()?;
        paths::ensure_dir(&dir)?;
        let path = Self::file_path(&self.name)?;
        std::fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(path)
    }

    pub fn load(name: &str) -> Result<Self> {
        let path = Self::file_path(name)?;
        let text = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("read {}: {e}", path.display()))?;
        toml::from_str(&text)
            .map_err(|e| anyhow::anyhow!("parse {}: {e}", path.display()))
    }

    pub fn list() -> Result<Vec<Recipe>> {
        let dir = paths::recipes_dir()?;
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let text = std::fs::read_to_string(&path)?;
            match toml::from_str::<Recipe>(&text) {
                Ok(r) => out.push(r),
                Err(err) => tracing::warn!("skipping {}: {err}", path.display()),
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }
}
