use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "aws-utils",
    version,
    about = "AWS pipeline management utility for mindtwo",
    propagate_version = true
)]
pub struct Cli {
    /// Override AWS region (defaults to project config or eu-central-1)
    #[arg(long, global = true)]
    pub region: Option<String>,

    /// Increase log verbosity
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Only show errors
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Disable ANSI colors in output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Register the current directory as a project
    Add(AddArgs),

    /// Manage configuration (per-project and global)
    #[command(subcommand)]
    Config(ConfigCommand),

    /// Compare deployed commits between two pipeline stages
    Check(CheckArgs),

    /// Generate a markdown changelog between two pipeline stages
    Changelog(ChangelogArgs),

    /// Approve the pending release in preprod and tag the commit
    Release(ReleaseArgs),

    /// Manage and run release recipes (multi-project releases)
    #[command(subcommand)]
    Recipe(RecipeCommand),

    /// Check S3 object existence for a list of keys
    #[command(name = "s3-check")]
    S3Check(S3CheckArgs),

    /// Manage the list of pre-configured AWS account names
    #[command(subcommand)]
    Accounts(AccountsCommand),

    /// Run the assume-role script and emit shell `export` statements
    Assume(AssumeArgs),

    /// Launch the interactive TUI
    Tui,
}

#[derive(Debug, Subcommand)]
pub enum AccountsCommand {
    /// Add an account to the global config
    Add {
        name: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// List all pre-configured accounts
    List,
    /// Remove an account from the global config
    Remove { name: String },
}

#[derive(Debug, Args)]
pub struct AssumeArgs {
    /// Account name (one of the pre-configured accounts). Interactive
    /// picker if omitted.
    pub account: Option<String>,
    /// Skip the MFA prompt — useful with YubiKey integration where
    /// the script obtains the token itself.
    #[arg(long)]
    pub no_mfa: bool,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// Override the project name (defaults to the directory name)
    #[arg(long)]
    pub name: Option<String>,

    /// Repo slug `owner/name` (defaults to git remote `origin`)
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the effective config resolved for the current project
    Show {
        /// Project name (defaults to the project registered for cwd)
        #[arg(long)]
        project: Option<String>,
    },
    /// Open the project config file in $EDITOR
    Edit {
        #[arg(long)]
        project: Option<String>,
    },
    /// Commit and push the project config file to the repository
    Push {
        #[arg(long)]
        project: Option<String>,
    },
    /// Pull the project config file from the repository (overwrites local)
    Pull {
        #[arg(long)]
        project: Option<String>,
    },
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long = "from")]
    pub from_stage: Option<String>,
    #[arg(long = "to")]
    pub to_stage: Option<String>,
}

#[derive(Debug, Args)]
pub struct ChangelogArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long = "from")]
    pub from_stage: Option<String>,
    #[arg(long = "to")]
    pub to_stage: Option<String>,
    /// Write the changelog to this file instead of stdout
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ReleaseArgs {
    #[arg(long)]
    pub project: Option<String>,
    /// Skip git tag creation
    #[arg(long)]
    pub no_tag: bool,
    /// Skip changelog generation
    #[arg(long)]
    pub no_changelog: bool,
    /// Approval summary text sent to AWS (defaults to "Released by aws-utils")
    #[arg(long)]
    pub summary: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum RecipeCommand {
    /// Create a new recipe interactively
    Create { name: String },
    /// List all recipes in the global config dir
    List,
    /// Run a recipe by name
    Run { name: String },
}

#[derive(Debug, Args)]
pub struct S3CheckArgs {
    /// Path to a text file containing one S3 key per line
    pub file: PathBuf,
    /// S3 bucket name (interactive picker if omitted)
    #[arg(long)]
    pub bucket: Option<String>,
    /// Number of concurrent HEAD requests
    #[arg(long, default_value_t = 10)]
    pub concurrency: usize,
    /// Look up delete markers and report deletion timestamps for missing keys
    #[arg(long)]
    pub show_deleted: bool,
    /// Project name to use for auto-assume (defaults to project for cwd
    /// if any; omit to skip assume-role)
    #[arg(long)]
    pub project: Option<String>,
}
