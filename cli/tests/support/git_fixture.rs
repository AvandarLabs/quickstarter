//! Turning a fixture directory into a git repository.
//!
//! The CLI only ever reads templates through `git clone`, and git clones a
//! local path happily, so an end-to-end test needs no network.

use std::path::Path;
use std::process::Command;

/// Commits everything under `path` to a fresh repository there.
pub fn commit_all(path: &Path) {
    run(path, &["init", "-q", "-b", "main"]);
    run(path, &["config", "user.email", "test@example.com"]);
    run(path, &["config", "user.name", "test"]);
    run(path, &["add", "-A"]);
    run(path, &["commit", "-q", "-m", "fixture"]);
}

fn run(cwd: &Path, args: &[&str]) {
    let status = Command::new("git").args(args).current_dir(cwd).status().unwrap();
    assert!(status.success(), "git {args:?} failed");
}
