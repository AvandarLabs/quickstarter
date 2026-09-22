//! End-to-end test of the non-interactive path: a `--yes` run that answers
//! every question with a flag, driven through the real binary.
//!
//! The template repository is a local fixture, which git clones without a
//! network, so this exercises argument parsing, resolution, fetching, and
//! composition exactly as a user's run does.

#[path = "support/fixture_templates.rs"]
mod fixture_templates;
#[path = "support/git_fixture.rs"]
mod git_fixture;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A temp directory holding the cloneable template fixture and the target dir.
struct Sandbox {
    _temp: tempfile::TempDir,
    template_repo: PathBuf,
    target_dir: PathBuf,
}

fn sandbox() -> Sandbox {
    let temp = tempfile::tempdir().unwrap();
    let template_repo = temp.path().join("template-repo");
    fixture_templates::write(&template_repo);
    git_fixture::commit_all(&template_repo);

    let target_dir = temp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    Sandbox { _temp: temp, template_repo, target_dir }
}

fn run_cli(sandbox: &Sandbox, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quickstarter"))
        .args(args)
        .args(["--repo", sandbox.template_repo.to_str().unwrap()])
        .output()
        .unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn flags_alone_scaffold_a_project_without_any_questions() {
    let sandbox = sandbox();
    let output = run_cli(
        &sandbox,
        &["--yes", "--name", "My App", "--dir", sandbox.target_dir.to_str().unwrap(), "--stack",
          "router"],
    );
    assert!(output.status.success(), "{}", stderr(&output));

    let project: &Path = &sandbox.target_dir.join("My App");
    assert_eq!(
        std::fs::read_to_string(project.join("README.md")).unwrap(),
        "# My App\nTanStack Router"
    );
    assert!(project.join("vite.config.ts").is_file(), "the chosen module was overlaid");

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(project.join("package.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["name"], "my-app");

    // The project arrives as a git repository, as an interactive run does.
    assert!(project.join(".git").exists());
}

#[test]
fn a_template_without_a_skills_manifest_installs_no_skills() {
    let sandbox = sandbox();
    let output = run_cli(
        &sandbox,
        &["--yes", "--name", "My App", "--dir", sandbox.target_dir.to_str().unwrap(), "--stack",
          "router"],
    );
    assert!(output.status.success(), "{}", stderr(&output));

    // The fixture template ships no `skills-manifest.json`, so the run selects
    // nothing and never reaches `npx`. That is what keeps this suite, which
    // drives the real binary, free of the network.
    let project: &Path = &sandbox.target_dir.join("My App");
    assert!(!project.join(".agents").exists());
    assert!(!project.join("skills-lock.json").exists());
    assert!(!stderr(&output).contains("Warning"), "{}", stderr(&output));
}

#[test]
fn yes_without_the_required_flags_fails_before_doing_any_work() {
    let sandbox = sandbox();
    let output = run_cli(&sandbox, &["--yes"]);

    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(message.contains("--name and --stack"), "{message}");
    // Nothing was created, and nothing was even fetched.
    assert_eq!(std::fs::read_dir(&sandbox.target_dir).unwrap().count(), 0);
}

#[test]
fn an_unknown_stack_reports_the_real_choices() {
    let sandbox = sandbox();
    let output = run_cli(
        &sandbox,
        &["--yes", "--name", "My App", "--dir", sandbox.target_dir.to_str().unwrap(), "--stack",
          "svelte"],
    );

    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(message.contains("no stack called"), "{message}");
    assert!(message.contains("router"), "{message}");
}

#[test]
fn an_existing_destination_is_refused() {
    let sandbox = sandbox();
    std::fs::create_dir_all(sandbox.target_dir.join("My App")).unwrap();

    let output = run_cli(
        &sandbox,
        &["--yes", "--name", "My App", "--dir", sandbox.target_dir.to_str().unwrap(), "--stack",
          "router"],
    );

    assert!(!output.status.success());
    assert!(stderr(&output).contains("already exists"), "{}", stderr(&output));
}
