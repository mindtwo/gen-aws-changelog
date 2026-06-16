//! `aws-utils logout` — clear an assumed session.
//!
//! Two things happen:
//! 1. Print `unset KEY;` statements for every credential variable
//!    currently in the session file (or a known fallback list). The
//!    caller is expected to `eval` the output so its shell environment
//!    clears.
//! 2. Delete `~/.cache/aws-utils/session.sh` so the on-disk credentials
//!    don't linger.

use crate::aws::assume;
use crate::error::Result;
use std::io::IsTerminal;

pub async fn run() -> Result<()> {
    let path = assume::session_file_path();
    let mut keys: Vec<String> = Vec::new();

    if let Some(p) = path.as_ref().filter(|p| p.exists()) {
        if let Ok(text) = std::fs::read_to_string(p) {
            keys.extend(assume::parse_output(&text).exports.into_keys());
        }
    }
    if keys.is_empty() {
        // Fallback when no session file: clear the standard set the
        // assume-role script ever exports.
        keys = STANDARD_AWS_VARS.iter().map(|s| (*s).to_string()).collect();
    }
    keys.sort();
    keys.dedup();

    let mut out = String::new();
    for k in &keys {
        out.push_str(&format!("unset {k};\n"));
    }
    print!("{out}");

    let removed = match path.as_ref() {
        Some(p) if p.exists() => std::fs::remove_file(p).is_ok(),
        _ => false,
    };

    if std::io::stdout().is_terminal() {
        eprintln!();
        eprintln!("note: nothing was unset in your shell because stdout is a TTY.");
        eprintln!("      Run it as one of:");
        eprintln!("        eval \"$(aws-utils logout)\"");
        eprintln!("        awsu logout                  # if you installed the wrapper");
    }
    if removed {
        eprintln!("session file removed");
    }
    Ok(())
}

const STANDARD_AWS_VARS: &[&str] = &[
    "AWS_ACCESS_KEY_ID",
    "AWS_ACCOUNT_ID",
    "AWS_ACCOUNT_NAME",
    "AWS_ACCOUNT_ROLE",
    "AWS_DEFAULT_REGION",
    "AWS_PROFILE_ASSUME_ROLE",
    "AWS_REGION",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SECURITY_TOKEN",
    "AWS_SESSION_ACCESS_KEY_ID",
    "AWS_SESSION_SECRET_ACCESS_KEY",
    "AWS_SESSION_SECURITY_TOKEN",
    "AWS_SESSION_SESSION_TOKEN",
    "AWS_SESSION_START",
    "AWS_SESSION_TOKEN",
    "AWS_STS_ROLE_ARN",
    "GEO_ENV",
    "KUBECONFIG",
];
