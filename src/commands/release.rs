use crate::cli::ReleaseArgs;
use crate::error::Result;

pub async fn run(_args: ReleaseArgs) -> Result<()> {
    anyhow::bail!("`release` is not implemented yet (Phase 6)")
}
