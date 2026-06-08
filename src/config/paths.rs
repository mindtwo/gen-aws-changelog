#![allow(dead_code)] // recipes_dir used in Phase 7

use crate::error::Result;
use std::path::PathBuf;

const APP_DIR: &str = "aws-utils";

pub fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("could not resolve OS config dir"))?;
    Ok(base.join(APP_DIR))
}

pub fn global_config_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

pub fn projects_dir() -> Result<PathBuf> {
    Ok(config_dir()?.join("projects"))
}

pub fn recipes_dir() -> Result<PathBuf> {
    Ok(config_dir()?.join("recipes"))
}

pub fn ensure_dir(path: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(path)
        .map_err(|e| anyhow::anyhow!("create {}: {e}", path.display()))?;
    Ok(())
}
