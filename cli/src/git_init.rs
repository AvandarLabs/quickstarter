//! Initializing the freshly scaffolded project as a git repository.
//!
//! After the files are composed into the destination, we turn it into a git
//! repository with a single initial commit so the user starts from a clean,
//! version-controlled slate. This is best-effort: a failure here is reported as
//! a warning by the caller and never discards the successfully created project.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Message for the commit that captures the scaffolded files.
const INITIAL_COMMIT_MESSAGE: &str = "Initial commit from quickstarter";

/// Branch the new repository starts on.
///
/// Named explicitly rather than left to `git init`: git falls back to `master`
/// on any machine that has not set `init.defaultBranch`, so without this the
/// branch a generated project lands on depends on whose laptop ran the
/// scaffolder.
const DEFAULT_BRANCH_NAME: &str = "main";

/// Initializes `dest` as a git repository and creates an initial commit
/// containing every scaffolded file.
///
/// Uses the user's configured git identity when one is available, falling back
/// to a generic identity so the commit still succeeds on a machine that has no
/// global `user.name` / `user.email` set.
pub fn init_and_commit(dest: &Path) -> Result<()> {
    run_git(dest, &["init", "-q", "-b", DEFAULT_BRANCH_NAME])
        .context("initializing a git repository")?;
    run_git(dest, &["add", "-A"]).context("staging the scaffolded files")?;
    commit_all(dest).context("creating the initial commit")?;
    Ok(())
}

/// Commits everything staged, supplying a fallback identity only when the user
/// has not configured one so we do not override a real author.
fn commit_all(dest: &Path) -> Result<()> {
    let mut args: Vec<&str> = Vec::new();
    if !has_identity(dest) {
        args.extend_from_slice(&[
            "-c",
            "user.name=quickstarter",
            "-c",
            "user.email=quickstarter@localhost",
        ]);
    }
    args.extend_from_slice(&["commit", "-q", "-m", INITIAL_COMMIT_MESSAGE]);
    run_git(dest, &args)
}

/// Reports whether both `user.name` and `user.email` resolve to a value.
fn has_identity(dest: &Path) -> bool {
    config_value(dest, "user.name").is_some() && config_value(dest, "user.email").is_some()
}

/// Reads a git config value in `dest`, returning `None` when it is unset.
fn config_value(dest: &Path, key: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["config", key])
        .current_dir(dest)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

/// Runs a git command in `dest`, turning a non-zero exit into an error that
/// carries git's own stderr.
fn run_git(dest: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dest)
        .output()
        .context("running git")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_repo_and_commits_every_file() {
        let dest = tempfile::tempdir().unwrap();
        std::fs::write(dest.path().join("file.txt"), "hi").unwrap();

        init_and_commit(dest.path()).unwrap();

        assert!(dest.path().join(".git").is_dir());

        // Exactly one commit exists.
        let log = Command::new("git")
            .args(["log", "--oneline"])
            .current_dir(dest.path())
            .output()
            .unwrap();
        assert!(log.status.success());
        assert_eq!(String::from_utf8_lossy(&log.stdout).lines().count(), 1);

        // The tree is clean, so nothing was left unstaged or uncommitted.
        let status = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(dest.path())
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&status.stdout).trim().is_empty());
    }

    #[test]
    fn starts_the_repository_on_the_default_branch() {
        let dest = tempfile::tempdir().unwrap();
        std::fs::write(dest.path().join("file.txt"), "hi").unwrap();

        init_and_commit(dest.path()).unwrap();

        // Naming the branch explicitly is what makes this independent of the
        // machine: without it, git falls back to `master` on any machine that
        // has not set `init.defaultBranch`.
        let branch = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(dest.path())
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&branch.stdout).trim(), DEFAULT_BRANCH_NAME);
        assert_eq!(DEFAULT_BRANCH_NAME, "main");
    }
}
