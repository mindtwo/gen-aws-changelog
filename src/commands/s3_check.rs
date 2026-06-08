use crate::cli::S3CheckArgs;
use crate::error::Result;

pub async fn run(_args: S3CheckArgs) -> Result<()> {
    anyhow::bail!("`s3-check` is not implemented yet (Phase 8)")
}
