use crate::cli::JiraCommand;
use crate::config::GlobalConfig;
use crate::error::Result;
use crate::jira::JiraClient;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Input, Password};
use serde::Deserialize;

pub async fn run(cmd: JiraCommand) -> Result<()> {
    match cmd {
        JiraCommand::Configure => configure().await,
        JiraCommand::Test => test().await,
        JiraCommand::Show => show().await,
    }
}

async fn configure() -> Result<()> {
    let mut cfg = GlobalConfig::load_or_default()?;
    let theme = ColorfulTheme::default();

    let base_url: String = Input::with_theme(&theme)
        .with_prompt("JIRA base URL (e.g. https://your-org.atlassian.net)")
        .with_initial_text(cfg.jira.base_url.clone().unwrap_or_default())
        .validate_with(|s: &String| -> std::result::Result<(), &'static str> {
            if s.starts_with("http://") || s.starts_with("https://") {
                Ok(())
            } else {
                Err("must start with http:// or https://")
            }
        })
        .interact_text()?;

    let email: String = Input::with_theme(&theme)
        .with_prompt("JIRA account email")
        .with_initial_text(cfg.jira.email.clone().unwrap_or_default())
        .interact_text()?;

    let api_token: String = Password::with_theme(&theme)
        .with_prompt("JIRA API token (https://id.atlassian.com/manage-profile/security/api-tokens)")
        .allow_empty_password(false)
        .interact()?;

    cfg.jira.base_url = Some(base_url.trim_end_matches('/').to_string());
    cfg.jira.email = Some(email);
    cfg.jira.api_token = Some(api_token);
    cfg.save()?;

    println!(
        "{} saved JIRA credentials to global config",
        "✓".green().bold()
    );
    println!("  Run `aws-utils jira test` to verify.");
    Ok(())
}

#[derive(Debug, Deserialize)]
struct Myself {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "emailAddress")]
    email_address: Option<String>,
    #[serde(rename = "accountId")]
    account_id: Option<String>,
}

async fn test() -> Result<()> {
    // Re-hydrate in case the saved config was just updated this session.
    let cfg = GlobalConfig::load_or_default()?;
    cfg.jira.hydrate_env();

    let client = JiraClient::from_env().map_err(|e| {
        anyhow::anyhow!(
            "JIRA credentials missing ({e}). Run `aws-utils jira configure` first."
        )
    })?;

    let url = client.url("myself");
    let resp = client.http().get(&url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("JIRA returned {status}: {body}");
    }
    let me: Myself = resp.json().await?;
    println!("{} JIRA connection OK", "✓".green().bold());
    if let Some(name) = me.display_name {
        println!("  user:      {name}");
    }
    if let Some(email) = me.email_address {
        println!("  email:     {email}");
    }
    if let Some(id) = me.account_id {
        println!("  accountId: {id}");
    }
    Ok(())
}

async fn show() -> Result<()> {
    let cfg = GlobalConfig::load_or_default()?;
    cfg.jira.hydrate_env();
    let base_url = std::env::var("JIRA_BASE_URL").ok();
    let email = std::env::var("JIRA_EMAIL").ok();
    let token = std::env::var("JIRA_API_TOKEN").ok();

    println!(
        "base_url:  {}",
        base_url.as_deref().unwrap_or("(not set)").bold()
    );
    println!(
        "email:     {}",
        email.as_deref().unwrap_or("(not set)").bold()
    );
    println!(
        "api_token: {}",
        token
            .as_deref()
            .map(mask)
            .unwrap_or_else(|| "(not set)".to_string())
            .bold()
    );
    Ok(())
}

fn mask(token: &str) -> String {
    let n = token.chars().count();
    if n <= 8 {
        return "*".repeat(n);
    }
    let tail: String = token.chars().skip(n - 4).collect();
    format!("{}…{}", "*".repeat(4), tail)
}
