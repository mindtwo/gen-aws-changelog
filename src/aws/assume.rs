//! Wrapper around the user's `assume-role` script.
//!
//! The script supports `OUTPUT_TO_EVAL=true` which:
//! - suppresses interactive prompts (account/role/region)
//! - emits `export KEY="VALUE";` lines on stdout (for `eval` consumers)
//! - emits diagnostic messages as `echo "...";` lines on **stdout** too,
//!   so a shell `eval` prints them when it executes the captured output
//! - expects the MFA token as the third positional arg (no prompt)
//!
//! We collect the MFA token ourselves (so the script's stdout stays clean
//! for parsing), then spawn the script and pull both kinds of lines out
//! of stdout. Echo lines are re-emitted on stderr so the user actually
//! sees diagnostics (instead of them being silently dropped by us), and
//! they get folded into the error message when the script fails.

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

#[derive(Debug, Default)]
pub struct AssumeOutput {
    pub exports: HashMap<String, String>,
    /// Lines the script wrote as `echo "..."` on stdout — these are the
    /// user-facing diagnostic messages it would have printed in
    /// non-eval mode.
    pub messages: Vec<String>,
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = parse_output(&stdout);

    // Always reflect the script's diagnostic messages back to the user.
    // The script puts these on stdout (wrapped as `echo "...";`) so a
    // shell `eval` prints them; we capture stdout to extract the
    // exports, so we'd silently drop them otherwise.
    for msg in &parsed.messages {
        eprintln!("{msg}");
    }

    if !output.status.success() {
        anyhow::bail!(
            "assume-role exited with {}{}",
            output.status,
            format_messages(&parsed.messages),
        );
    }
    if parsed.exports.is_empty() {
        anyhow::bail!(
            "assume-role returned no credentials{}",
            format_messages(&parsed.messages),
        );
    }
    Ok(parsed.exports)
}

fn format_messages(messages: &[String]) -> String {
    if messages.is_empty() {
        String::new()
    } else {
        format!(": {}", messages.join(" | "))
    }
}

/// Parse the assume-role script's stdout into structured output.
///
/// Recognizes:
/// - `export KEY="VALUE";` → captured into [`AssumeOutput::exports`]
/// - `echo "MESSAGE";` → captured into [`AssumeOutput::messages`]
///
/// Anything else is ignored.
pub fn parse_output(text: &str) -> AssumeOutput {
    static EXPORT: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"^\s*export\s+([A-Z_][A-Z0-9_]*)="((?:[^"\\]|\\.)*)";?\s*$"#).expect("regex")
    });
    static ECHO: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"^\s*echo\s+"((?:[^"\\]|\\.)*)";?\s*$"#).expect("regex"));
    let mut out = AssumeOutput::default();
    for line in text.lines() {
        if let Some(caps) = EXPORT.captures(line) {
            out.exports.insert(caps[1].to_string(), unescape(&caps[2]));
        } else if let Some(caps) = ECHO.captures(line) {
            let msg = unescape(&caps[1]);
            if !msg.trim().is_empty() {
                out.messages.push(msg);
            }
        }
    }
    out
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
        let parsed = parse_output(sample);
        assert_eq!(parsed.exports["AWS_REGION"], "eu-central-1");
        assert_eq!(parsed.exports["AWS_ACCESS_KEY_ID"], "AKIAEXAMPLE");
        assert_eq!(parsed.exports["AWS_SESSION_TOKEN"], "token");
        assert_eq!(parsed.exports["GEO_ENV"], "prod-app-teach");
        assert_eq!(parsed.exports["AWS_EMPTY"], "");
        assert!(parsed.messages.is_empty());
    }

    #[test]
    fn captures_echo_messages() {
        let sample = "
echo \"aws sts get-session-token error\";
echo \"Failed to export session envars.\";
";
        let parsed = parse_output(sample);
        assert!(parsed.exports.is_empty());
        assert_eq!(
            parsed.messages,
            vec![
                "aws sts get-session-token error".to_string(),
                "Failed to export session envars.".to_string(),
            ]
        );
    }

    #[test]
    fn echo_and_export_intermixed() {
        let sample = r#"
echo "Using source profile mindtwo";
export AWS_REGION="eu-central-1";
echo "switched role";
export AWS_ACCESS_KEY_ID="AKIA";
"#;
        let parsed = parse_output(sample);
        assert_eq!(parsed.exports.len(), 2);
        assert_eq!(parsed.messages.len(), 2);
    }

    #[test]
    fn empty_output() {
        let parsed = parse_output("nothing here");
        assert!(parsed.exports.is_empty());
        assert!(parsed.messages.is_empty());
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

    #[test]
    fn format_messages_renders() {
        assert_eq!(format_messages(&[]), "");
        assert_eq!(
            format_messages(&["one".into(), "two".into()]),
            ": one | two"
        );
    }
}
