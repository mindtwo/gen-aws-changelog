//! Wrapper around the user's `assume-role` script.
//!
//! Invariants we rely on (from inspecting the script):
//!
//! - The script defines an `assume-role` shell function. When *sourced*,
//!   its bottom branch is a noop and only the function is registered;
//!   when *executed* directly, it sets `set -eo pipefail` and immediately
//!   invokes the function — but the function's first command is
//!   `read -r -d '' HELP <<EOF` which always returns 1, killing the
//!   script under `set -e` before anything user-visible happens.
//!   So we always *source* the script via `bash -c` and call the
//!   function ourselves, matching how an interactive user runs it.
//! - `OUTPUT_TO_EVAL=true` suppresses interactive prompts and emits
//!   `export KEY="VALUE";` lines on stdout for `eval` consumers.
//! - Diagnostic messages get wrapped as `echo "...";` lines on **stdout**
//!   too (so a shell `eval` prints them).
//! - `mfa_token_input` is never assigned from a positional arg despite
//!   what the script's usage comment says. It's read straight from the
//!   environment, so we pass it as `mfa_token_input=<token>`.
//!
//! We capture both stdout and stderr. The function returns 0 even on
//! internal failure (its `return` paths don't set an error status), so
//! success is determined by "did we get any `export` lines back?".

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

    // Source the script and invoke the function — see module docs for why
    // we can't execute the script directly.
    let bash_script = format!(
        r#"source "{}"; assume-role "$1"; exit $?"#,
        binary.to_string_lossy().replace('"', r#"\""#),
    );
    let mut cmd = Command::new("bash");
    cmd.arg("-c")
        .arg(&bash_script)
        .arg("aws-utils") // $0 placeholder
        .arg(account)
        .env("OUTPUT_TO_EVAL", "true")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(token) = mfa_token {
        cmd.env("mfa_token_input", token);
    }
    let output = cmd
        .output()
        .map_err(|e| anyhow::anyhow!("spawn bash for assume-role: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed = parse_output(&stdout);

    // Reflect everything the script wrote back to the user. The
    // `echo "..."` messages are user-facing diagnostics that would have
    // shown up in non-eval mode; the raw stderr captures errors from
    // tools the script invokes (aws, jq, set -e deaths).
    for msg in &parsed.messages {
        eprintln!("{msg}");
    }
    if !stderr.trim().is_empty() {
        eprint!("{stderr}");
    }

    if !output.status.success() || parsed.exports.is_empty() {
        let log_path = dump_debug_log(account, &stdout, &stderr, &output.status);
        let log_hint = log_path
            .as_ref()
            .map(|p| format!("\n  (full output: {})", p.display()))
            .unwrap_or_default();
        if !output.status.success() {
            anyhow::bail!(
                "assume-role exited with {}{}{}",
                output.status,
                format_diagnostics(&parsed.messages, &stderr),
                log_hint,
            );
        } else {
            anyhow::bail!(
                "assume-role returned no credentials{}{}",
                format_diagnostics(&parsed.messages, &stderr),
                log_hint,
            );
        }
    }
    Ok(parsed.exports)
}

/// Write the raw stdout + stderr from the failing run to a debug file,
/// so the user can always read the unredacted output even when the TUI
/// truncates the inline error. Best-effort: returns `None` if the
/// destination isn't writable.
fn dump_debug_log(
    account: &str,
    stdout: &str,
    stderr: &str,
    status: &std::process::ExitStatus,
) -> Option<std::path::PathBuf> {
    let dir = dirs::cache_dir()?.join("aws-utils");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("assume-role-last.log");
    let body = format!(
        "account: {account}\nexit: {status}\n\n=== stdout ===\n{stdout}\n=== stderr ===\n{stderr}",
    );
    std::fs::write(&path, body).ok()?;
    Some(path)
}

fn format_diagnostics(messages: &[String], stderr: &str) -> String {
    let mut parts: Vec<String> = messages.to_vec();
    let stderr_trimmed = stderr.trim();
    if !stderr_trimmed.is_empty() {
        parts.extend(stderr_trimmed.lines().map(|l| l.trim().to_string()));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("\n  {}", parts.join("\n  "))
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
    fn format_diagnostics_combines_sources() {
        assert_eq!(format_diagnostics(&[], ""), "");
        assert_eq!(
            format_diagnostics(&["one".into(), "two".into()], ""),
            "\n  one\n  two"
        );
        assert_eq!(
            format_diagnostics(&[], "  An error\noccurred  "),
            "\n  An error\n  occurred"
        );
        assert_eq!(
            format_diagnostics(&["echo msg".into()], "stderr line"),
            "\n  echo msg\n  stderr line"
        );
    }
}
