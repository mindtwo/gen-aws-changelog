#![allow(dead_code)] // populated incrementally over Phases 2–8

use thiserror::Error;

pub type Result<T> = anyhow::Result<T>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("project '{0}' is not registered (run `aws-utils add` inside its directory)")]
    ProjectNotFound(String),

    #[error("no project registered for current directory: {0}")]
    ProjectNotRegistered(std::path::PathBuf),

    #[error("pipeline stage '{0}' not found in pipeline '{1}'")]
    StageNotFound(String, String),

    #[error("no pending manual approval action found in stage '{0}'")]
    NoPendingApproval(String),

    #[error("missing required environment variable: {0}")]
    MissingEnv(&'static str),

    #[error("repo must be in `owner/name` form, got: {0}")]
    InvalidRepoName(String),
}

pub fn parse_repo(slug: &str) -> Result<(String, String)> {
    let mut parts = slug.splitn(2, '/');
    let owner = parts.next().filter(|s| !s.is_empty());
    let name = parts.next().filter(|s| !s.is_empty());
    match (owner, name) {
        (Some(o), Some(n)) if !n.contains('/') => Ok((o.to_string(), n.to_string())),
        _ => Err(AppError::InvalidRepoName(slug.to_string()).into()),
    }
}
