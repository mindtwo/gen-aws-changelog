mod aws;
mod changelog;
mod cli;
mod commands;
mod config;
mod error;
mod git;
mod github;
mod jira;
mod recipe;
mod tui;
mod ui;

use clap::Parser;
use cli::{Cli, Command};
use colored::control;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Load .env if present; ignore errors when no file exists.
    let _ = dotenvy::dotenv();

    let args = Cli::parse();

    if args.no_color {
        control::set_override(false);
    }

    init_tracing(args.verbose, args.quiet);

    if let Err(err) = dispatch(args).await {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn init_tracing(verbose: bool, quiet: bool) {
    let default_level = match (verbose, quiet) {
        (true, _) => "debug",
        (_, true) => "error",
        _ => "info",
    };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("aws_utils={default_level},warn")));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .init();
}

async fn dispatch(args: Cli) -> error::Result<()> {
    match args.command {
        Command::Add(a) => commands::add::run(a).await,
        Command::Config(c) => commands::config::run(c).await,
        Command::Check(a) => commands::check::run(a).await,
        Command::Changelog(a) => commands::changelog::run(a).await,
        Command::Release(a) => commands::release::run(a).await,
        Command::Recipe(c) => commands::recipe::run(c).await,
        Command::S3Check(a) => commands::s3_check::run(a).await,
        Command::Accounts(c) => commands::accounts::run(c).await,
        Command::Assume(a) => commands::assume::run(a).await,
        Command::Session => commands::session::run().await,
        Command::Logout => commands::logout::run().await,
        Command::Init(a) => commands::init::run(a).await,
        Command::Tui => commands::tui::run().await,
    }
}
