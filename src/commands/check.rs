use crate::cli::CheckArgs;
use crate::error::Result;

pub async fn run(_args: CheckArgs) -> Result<()> {
    anyhow::bail!("`check` is not implemented yet (Phase 3)")
}
