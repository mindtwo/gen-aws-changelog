use crate::aws::assume;
use crate::error::Result;

pub async fn run() -> Result<()> {
    let Some(path) = assume::session_file_path() else {
        anyhow::bail!("could not resolve cache dir");
    };
    if !path.exists() {
        anyhow::bail!(
            "no session file at {} — run `aws-utils assume <account>` first",
            path.display()
        );
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("read {}: {e}", path.display()))?;
    print!("{text}");
    Ok(())
}
