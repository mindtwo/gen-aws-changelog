//! Wrapper around the user's `assume-role` script.
//!
//! The script supports `OUTPUT_TO_EVAL=true` which:
//! - suppresses interactive prompts (account/role/region)
//! - emits `export KEY="VALUE";` lines on stdout
//! - expects the MFA token as the third positional arg (no prompt)
//!
//! We collect the MFA token ourselves (so the script's stdout stays clean
//! for parsing), then spawn the script and parse the exports.

use crate::error::Result;
use dialoguer::{theme::ColorfulTheme, Password};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const DEFAULT_BINARY: &str = "/usr/local/bin/assume-role";

/// AWS env vars that, when set, indicate an active assumed-role session.
/// We skip auto-assume when these are present so users who already ran
/// `assume-role` in their shell don't get re-prompted.
pub fn has_active_session() -> bool {
    std::env::var("AWS_SESSION_TOKEN")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some()
}

pub fn binary_path() -> PathBuf {
    std::env::var("AWS_UTILS_ASSUME_ROLE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_BINARY))
}

/// Capture an MFA token from the user. Goes to the terminal (dialoguer
/// reads /dev/tty), so it doesn't pollute stdout when we're emitting
/// shell `export` lines.
pub fn prompt_mfa(account: &str) -> Result<String> {
    Password::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("MFA token for {account}"))
        .interact()
        .map_err(|e| anyhow::anyhow!("mfa prompt: {e}"))
}

/// Run the assume-role script and return the captured `export` map.
/// `mfa_token` may be `None` if the user has a YubiKey configured or the
/// script can find one elsewhere — we let the script decide.
pub fn run(account: &str, mfa_token: Option<&str>) -> Result<HashMap<String, String>> {
    let binary = binary_path();
    if !binary.exists() {
        anyhow::bail!(
            "assume-role binary not found at {} (set AWS_UTILS_ASSUME_ROLE to override)",
            binary.display()
        );
    }

    let mut cmd = Command::new(&binary);
    cmd.env("OUTPUT_TO_EVAL", "true")
        .arg(account)
        // role is auto-derived from ~/.aws/config when OUTPUT_TO_EVAL is set
        .arg("")
        .arg(mfa_token.unwrap_or(""))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let output = cmd
        .output()
        .map_err(|e| anyhow::anyhow!("spawn {}: {e}", binary.display()))?;
    if !output.status.success() {
        anyhow::bail!(
            "assume-role exited with status {} (see stderr above)",
            output.status
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_exports(&stdout)
}

/// Parse `export KEY="VALUE";` lines from `text` into a flat map. Lines
/// that don't match are ignored. Empty values are also kept (the script
/// sometimes emits blanks for vars that weren't set).
pub fn parse_exports(text: &str) -> Result<HashMap<String, String>> {
    static LINE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"^\s*export\s+([A-Z_][A-Z0-9_]*)="((?:[^"\\]|\\.)*)";?\s*$"#)
            .expect("regex")
    });
    let mut out = HashMap::new();
    for line in text.lines() {
        if let Some(caps) = LINE.captures(line) {
            let key = caps[1].to_string();
            let val = unescape(&caps[2]);
            out.insert(key, val);
        }
    }
    if out.is_empty() {
        anyhow::bail!(
            "assume-role produced no `export` lines on stdout (was OUTPUT_TO_EVAL respected?)"
        );
    }
    Ok(out)
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Apply the parsed exports to the current process env. Only AWS_*,
/// GEO_ENV, and KUBECONFIG are propagated — we don't want to overwrite
/// arbitrary env from the parent shell.
pub fn apply_to_env(vars: &HashMap<String, String>) {
    for (k, v) in vars {
        if is_propagated(k) {
            // SAFETY: set_var is safe in single-threaded startup before
            // we spawn tokio workers that read env. tokio::main has not
            // yet entered the runtime when this runs in CLI path.
            std::env::set_var(k, v);
        }
    }
}

fn is_propagated(key: &str) -> bool {
    key.starts_with("AWS_") || key == "GEO_ENV" || key == "KUBECONFIG"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_eval_output() {
        let sample = r#"
export AWS_REGION="eu-central-1";
export AWS_ACCESS_KEY_ID="AKIAEXAMPLE";
export AWS_SECRET_ACCESS_KEY="secret";
export AWS_SESSION_TOKEN="token";
export GEO_ENV="prod-app-teach";
not an export line
export AWS_EMPTY="";
"#;
        let map = parse_exports(sample).unwrap();
        assert_eq!(map["AWS_REGION"], "eu-central-1");
        assert_eq!(map["AWS_ACCESS_KEY_ID"], "AKIAEXAMPLE");
        assert_eq!(map["AWS_SESSION_TOKEN"], "token");
        assert_eq!(map["GEO_ENV"], "prod-app-teach");
        assert_eq!(map["AWS_EMPTY"], "");
    }

    #[test]
    fn empty_output_is_error() {
        assert!(parse_exports("nothing here").is_err());
    }

    #[test]
    fn unescape_handles_quotes() {
        assert_eq!(unescape(r#"a\"b"#), r#"a"b"#);
    }

    #[test]
    fn only_aws_keys_propagated() {
        assert!(is_propagated("AWS_REGION"));
        assert!(is_propagated("GEO_ENV"));
        assert!(is_propagated("KUBECONFIG"));
        assert!(!is_propagated("PATH"));
        assert!(!is_propagated("HOME"));
    }
}
