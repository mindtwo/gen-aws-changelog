use crate::aws::assume;
use crate::cli::AssumeArgs;
use crate::config::GlobalConfig;
use crate::error::Result;
use crate::ui::prompts;

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

    // Emit eval-style output so callers can do `eval "$(aws-utils assume X)"`.
    let mut keys: Vec<&String> = vars.keys().collect();
    keys.sort();
    for k in keys {
        println!(r#"export {}="{}";"#, k, escape(&vars[k]));
    }
    Ok(())
}

fn escape(s: &str) -> String {
    s.replace('\\', r"\\").replace('"', r#"\""#)
}
