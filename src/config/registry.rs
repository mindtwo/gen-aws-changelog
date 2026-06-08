#![allow(dead_code)] // remove() consumed by future `remove` command

use crate::config::paths;
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One file per project under `~/.config/aws-utils/projects/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub path: PathBuf,
    pub repo: String,
    #[serde(default = "default_config_filename")]
    pub config: String,
}

fn default_config_filename() -> String {
    super::PROJECT_CONFIG_FILENAME.to_string()
}

impl RegistryEntry {
    pub fn project_config_path(&self) -> PathBuf {
        self.path.join(&self.config)
    }
}

pub struct ProjectRegistry;

impl ProjectRegistry {
    fn entry_path(name: &str) -> Result<PathBuf> {
        Ok(paths::projects_dir()?.join(format!("{name}.toml")))
    }

    pub fn save(entry: &RegistryEntry) -> Result<PathBuf> {
        let dir = paths::projects_dir()?;
        paths::ensure_dir(&dir)?;
        let file = Self::entry_path(&entry.name)?;
        let text = toml::to_string_pretty(entry)?;
        std::fs::write(&file, text)
            .map_err(|e| anyhow::anyhow!("write {}: {e}", file.display()))?;
        Ok(file)
    }

    pub fn find(name: &str) -> Result<RegistryEntry> {
        let file = Self::entry_path(name)?;
        if !file.exists() {
            return Err(AppError::ProjectNotFound(name.to_string()).into());
        }
        let text = std::fs::read_to_string(&file)
            .map_err(|e| anyhow::anyhow!("read {}: {e}", file.display()))?;
        let entry: RegistryEntry = toml::from_str(&text)
            .map_err(|e| anyhow::anyhow!("parse {}: {e}", file.display()))?;
        Ok(entry)
    }

    pub fn list() -> Result<Vec<RegistryEntry>> {
        let dir = paths::projects_dir()?;
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
            match toml::from_str::<RegistryEntry>(&text) {
                Ok(e) => out.push(e),
                Err(err) => tracing::warn!("skipping {}: {err}", path.display()),
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Find the registry entry whose `path` is the closest ancestor of `cwd`.
    pub fn find_for_cwd(cwd: &Path) -> Result<RegistryEntry> {
        let entries = Self::list()?;
        let cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
        let best = entries
            .into_iter()
            .filter(|e| {
                let canon = e.path.canonicalize().unwrap_or_else(|_| e.path.clone());
                cwd.starts_with(&canon)
            })
            .max_by_key(|e| e.path.components().count());
        best.ok_or_else(|| AppError::ProjectNotRegistered(cwd).into())
    }

    pub fn remove(name: &str) -> Result<()> {
        let file = Self::entry_path(name)?;
        if file.exists() {
            std::fs::remove_file(&file)
                .map_err(|e| anyhow::anyhow!("remove {}: {e}", file.display()))?;
        }
        Ok(())
    }
}
