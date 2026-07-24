//! Git preflight and shallow clone into a temporary directory.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use tempfile::TempDir;

/// A cloned template repository living in a temporary directory. Dropping this
/// value deletes the temporary directory and everything under it, so the user
/// is never left with the clone or any intermediate files.
pub struct TemplateCheckout {
    _temp_dir: TempDir,
    repo_path: PathBuf,
}

impl TemplateCheckout {
    /// Root of the cloned repository (the directory containing `templates/`).
    pub fn path(&self) -> &Path {
        &self.repo_path
    }
}

/// Verifies that `git` is available on the PATH, returning a friendly error if
/// it is not.
pub fn ensure_git_available() -> Result<()> {
    let output = Command::new("git").arg("--version").output();
    match output {
        Ok(result) if result.status.success() => Ok(()),
        _ => bail!("git is not installed or not on your PATH. Install git and try again."),
    }
}

/// Shallow-clones `repo_url` into a fresh temporary directory. Credential
/// prompts are disabled so a missing-auth clone fails fast instead of hanging.
pub fn clone(repo_url: &str) -> Result<TemplateCheckout> {
    let temp_dir = tempfile::Builder::new()
        .prefix("quickstarter-template-")
        .tempdir()
        .context("creating a temporary directory for the template clone")?;
    let repo_path = temp_dir.path().join("repo");

    let output = Command::new("git")
        .args(["clone", "--depth", "1", repo_url])
        .arg(&repo_path)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .context("running git clone")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(classify_clone_error(&stderr, repo_url));
    }

    Ok(TemplateCheckout {
        _temp_dir: temp_dir,
        repo_path,
    })
}

/// Turns git's stderr into an actionable error, distinguishing "offline" from
/// "git/GitHub not configured or no access" from everything else.
fn classify_clone_error(stderr: &str, repo_url: &str) -> anyhow::Error {
    let lowered = stderr.to_lowercase();

    if is_offline_error(&lowered) {
        return anyhow!(
            "Cannot reach the template repository, so this tool cannot run offline. \
             Reconnect to the internet and try again.\n\ngit reported:\n{}",
            stderr.trim()
        );
    }

    if is_access_error(&lowered) {
        return anyhow!(
            "Could not access {repo_url}. Check that git and your GitHub credentials are \
             configured and that you have access to the repository.\n\ngit reported:\n{}",
            stderr.trim()
        );
    }

    anyhow!("git clone failed.\n\ngit reported:\n{}", stderr.trim())
}

fn is_offline_error(lowered: &str) -> bool {
    const OFFLINE_MARKERS: &[&str] = &[
        "could not resolve host",
        "could not resolve proxy",
        "temporary failure in name resolution",
        "could not connect to server",
        "failed to connect",
        "network is unreachable",
        "connection timed out",
        "connection refused",
    ];
    OFFLINE_MARKERS.iter().any(|marker| lowered.contains(marker))
}

fn is_access_error(lowered: &str) -> bool {
    const ACCESS_MARKERS: &[&str] = &[
        "authentication failed",
        "permission denied",
        "could not read from remote repository",
        "terminal prompts disabled",
        "repository not found",
        "access denied",
        "invalid username or password",
    ];
    ACCESS_MARKERS.iter().any(|marker| lowered.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_offline_errors() {
        let error =
            classify_clone_error("fatal: unable to access: Could not resolve host: github.com", "u");
        assert!(error.to_string().contains("cannot run offline"));
    }

    #[test]
    fn classifies_access_errors() {
        let error = classify_clone_error("fatal: Authentication failed for 'https://...'", "u");
        assert!(error.to_string().contains("Check that git"));
    }

    #[test]
    fn falls_back_to_generic_error() {
        let error = classify_clone_error("fatal: something unexpected", "u");
        assert!(error.to_string().contains("git clone failed"));
    }

    #[test]
    fn clones_from_a_local_repository() {
        // git can clone from a local path, which lets us exercise the real
        // clone path without any network.
        let source = tempfile::tempdir().unwrap();
        init_local_repo(source.path());

        let checkout = clone(source.path().to_str().unwrap()).unwrap();

        assert!(checkout.path().join("templates").is_dir());
    }

    fn init_local_repo(path: &Path) {
        std::fs::create_dir_all(path.join("templates")).unwrap();
        std::fs::write(path.join("templates/marker.txt"), "hi").unwrap();
        run_git(path, &["init", "-q"]);
        run_git(path, &["config", "user.email", "test@example.com"]);
        run_git(path, &["config", "user.name", "test"]);
        run_git(path, &["add", "."]);
        run_git(path, &["commit", "-q", "-m", "init"]);
    }

    fn run_git(cwd: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }
}
