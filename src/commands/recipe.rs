use crate::cli::RecipeCommand;
use crate::error::Result;

pub async fn run(_cmd: RecipeCommand) -> Result<()> {
    anyhow::bail!("`recipe` is not implemented yet (Phase 7)")
}
