use crate::cli::ChangelogArgs;
use crate::error::Result;

pub async fn run(_args: ChangelogArgs) -> Result<()> {
    anyhow::bail!("`changelog` is not implemented yet (Phase 5)")
}
