use crate::aws::assume;
use crate::cli::AssumeArgs;
use crate::config::GlobalConfig;
use crate::error::Result;
use crate::ui::prompts;
use std::io::IsTerminal;

pub async fn run(args: AssumeArgs) -> Result<()> {
    let cfg = GlobalConfig::load_or_default()?;
    if cfg.accounts.is_empty() {
        anyhow::bail!("no accounts configured — run `aws-utils accounts add <name>` first");
    }

    let account = match args.account {
        Some(n) => {
            if !cfg.accounts.iter().any(|a| a.name == n) {
                eprintln!("warn: '{n}' is not in the configured accounts list");
            }
            n
        }
        None => {
            let names: Vec<&str> = cfg.accounts.iter().map(|a| a.name.as_str()).collect();
            let idx = prompts::select("Account", &names)?;
            names[idx].to_string()
        }
    };

    let mfa = if args.no_mfa {
        None
    } else {
        Some(assume::prompt_mfa(&account)?)
    };

    let vars = assume::run(&account, mfa.as_deref())?;

    // Stdout: the export lines (for `eval "$(aws-utils assume X)"`).
    print!("{}", assume::render_exports(&vars));

    // Persist to disk too so the TUI wrapper / `aws-utils session` can
    // pick them up later.
    let session_path = assume::write_session_file(&vars);

    // If we're a TTY (no `eval` capturing stdout), the user just sees
    // a wall of `export` lines and nothing happens to their env. Warn
    // them with the right incantation.
    if std::io::stdout().is_terminal() {
        eprintln!();
        eprintln!("note: nothing was exported into your shell because stdout is a TTY.");
        eprintln!("      Run it as one of:");
        eprintln!("        eval \"$(aws-utils assume {account})\"");
        if let Some(p) = &session_path {
            eprintln!("        source {}", p.display());
        }
        eprintln!("      Or install the wrapper: eval \"$(aws-utils init zsh)\"  # bash/zsh");
        eprintln!("      then use:           awsu assume {account}");
    }
    Ok(())
}
