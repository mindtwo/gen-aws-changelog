use crate::cli::AccountsCommand;
use crate::config::{Account, GlobalConfig};
use crate::error::Result;
use colored::Colorize;

pub async fn run(cmd: AccountsCommand) -> Result<()> {
    match cmd {
        AccountsCommand::Add { name, description } => add(name, description).await,
        AccountsCommand::List => list().await,
        AccountsCommand::Remove { name } => remove(name).await,
    }
}

async fn add(name: String, description: Option<String>) -> Result<()> {
    let mut cfg = GlobalConfig::load_or_default()?;
    if cfg.accounts.iter().any(|a| a.name == name) {
        anyhow::bail!("account '{name}' is already configured");
    }
    cfg.accounts.push(Account {
        name: name.clone(),
        description: description.unwrap_or_default(),
    });
    cfg.save()?;
    println!("{} added account '{}'", "✓".green().bold(), name);
    Ok(())
}

async fn list() -> Result<()> {
    let cfg = GlobalConfig::load_or_default()?;
    if cfg.accounts.is_empty() {
        println!("no accounts configured (run `aws-utils accounts add <name>`)");
        return Ok(());
    }
    for a in &cfg.accounts {
        if a.description.is_empty() {
            println!("{}", a.name);
        } else {
            println!("{}  — {}", a.name.bold(), a.description.dimmed());
        }
    }
    Ok(())
}

async fn remove(name: String) -> Result<()> {
    let mut cfg = GlobalConfig::load_or_default()?;
    let before = cfg.accounts.len();
    cfg.accounts.retain(|a| a.name != name);
    if cfg.accounts.len() == before {
        anyhow::bail!("account '{name}' not found");
    }
    cfg.save()?;
    println!("{} removed account '{}'", "✓".green().bold(), name);
    Ok(())
}
