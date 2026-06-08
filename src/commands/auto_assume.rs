//! Auto-assume helper invoked by every AWS-touching command before it
//! creates an SDK client. Decides whether to run the user's
//! `assume-role` script based on:
//!
//! 1. Project config (`[aws]` table)
//! 2. Whether a session is already active (`AWS_SESSION_TOKEN` env var)

use crate::aws::assume;
use crate::config::project::AwsAction;
use crate::config::Resolved;
use crate::error::Result;
use colored::Colorize;

pub fn ensure(resolved: &Resolved, action: AwsAction) -> Result<()> {
    if assume::has_active_session() {
        return Ok(());
    }
    let Some(account) = resolved.project.aws.account_for(action) else {
        return Ok(());
    };

    eprintln!(
        "{} assuming role into {}",
        "·".cyan().bold(),
        account.bold()
    );
    let mfa = assume::prompt_mfa(account)?;
    let vars = assume::run(account, Some(&mfa))?;
    assume::apply_to_env(&vars);
    let _ = assume::write_session_file(&vars);
    eprintln!("{} session active for {}", "✓".green().bold(), account);
    Ok(())
}
