use crate::error::Result;
use git2::{Cred, ObjectType, PushOptions, RemoteCallbacks, Repository, Signature};
use std::path::Path;

/// Open the git repo at `path` (or any of its parents).
pub fn open(path: &Path) -> Result<Repository> {
    Repository::discover(path)
        .map_err(|e| anyhow::anyhow!("not inside a git repo ({}): {e}", path.display()))
}

/// Create an annotated tag pointing at `sha`. Tag name follows the v1
/// convention `release-DD-MM-YYYY`.
pub fn tag_release(repo: &Repository, sha: &str, tag_name: &str, message: &str) -> Result<()> {
    let oid = repo
        .revparse_single(sha)
        .map_err(|e| anyhow::anyhow!("revparse {sha}: {e}"))?
        .id();
    let object = repo.find_object(oid, Some(ObjectType::Commit))?;

    let signature = author_signature(repo)?;
    repo.tag(tag_name, &object, &signature, message, false)
        .map_err(|e| anyhow::anyhow!("create tag {tag_name}: {e}"))?;
    Ok(())
}

/// Push the tag to `remote_name` (typically `origin`). Uses the default
/// SSH-agent / system credential helper.
pub fn push_tag(repo: &Repository, remote_name: &str, tag_name: &str) -> Result<()> {
    let mut remote = repo.find_remote(remote_name)?;
    let refspec = format!("refs/tags/{tag_name}:refs/tags/{tag_name}");

    let mut cbs = RemoteCallbacks::new();
    cbs.credentials(|_url, username_from_url, allowed| {
        if allowed.contains(git2::CredentialType::SSH_KEY) {
            return Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"));
        }
        if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            if let Ok(token) = std::env::var("GITHUB_TOKEN") {
                return Cred::userpass_plaintext("x-access-token", &token);
            }
        }
        Cred::default()
    });
    let mut opts = PushOptions::new();
    opts.remote_callbacks(cbs);
    remote
        .push(&[refspec.as_str()], Some(&mut opts))
        .map_err(|e| anyhow::anyhow!("push tag {tag_name} to {remote_name}: {e}"))?;
    Ok(())
}

fn author_signature(repo: &Repository) -> Result<Signature<'static>> {
    if let Ok(sig) = repo.signature() {
        return Ok(sig.to_owned());
    }
    // Fall back to env so we don't fail just because git config is missing
    // on CI.
    let name = std::env::var("GIT_AUTHOR_NAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "aws-utils".to_string());
    let email = std::env::var("GIT_AUTHOR_EMAIL").unwrap_or_else(|_| "aws-utils@local".to_string());
    Ok(Signature::now(&name, &email)?)
}
