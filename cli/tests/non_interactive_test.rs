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

use fixture_templates::{
    ROUTER_CAPABILITY, RUST_PROJECT_TYPE, START_CAPABILITY, TUI_CAPABILITY, WEB_PROJECT_TYPE,
};

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

/// A complete `--yes` command for the given tags, which is what every test
/// here varies.
fn build(sandbox: &Sandbox, project_type: &str, capabilities: &[&str]) -> Output {
    let mut args = vec![
        "--yes",
        "--name",
        "My App",
        "--dir",
        sandbox.target_dir.to_str().unwrap(),
        "--project-type",
        project_type,
    ];
    for capability in capabilities {
        args.push("--capability");
        args.push(capability);
    }
    run_cli(sandbox, &args)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn flags_alone_scaffold_a_project_without_any_questions() {
    let sandbox = sandbox();
    let output = build(&sandbox, WEB_PROJECT_TYPE, &[ROUTER_CAPABILITY]);
    assert!(output.status.success(), "{}", stderr(&output));

    let project: &Path = &sandbox.target_dir.join("My App");
    assert_eq!(
        std::fs::read_to_string(project.join("README.md")).unwrap(),
        "# My App\nTanStack Router"
    );
    assert!(project.join("vite.config.ts").is_file(), "the chosen tags were overlaid");

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(project.join("package.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["name"], "my-app");

    // The project arrives as a git repository, as an interactive run does.
    assert!(project.join(".git").exists());

    // The report says what was built and how to run it.
    let report = stdout(&output);
    assert!(report.contains("TypeScript web app + TanStack Router"), "{report}");
    assert!(report.contains("pnpm dev"), "{report}");
}

#[test]
fn the_short_capability_flag_builds_the_other_capability() {
    let sandbox = sandbox();
    let output = run_cli(
        &sandbox,
        &[
            "--yes",
            "--name",
            "My App",
            "--dir",
            sandbox.target_dir.to_str().unwrap(),
            "-p",
            WEB_PROJECT_TYPE,
            "-c",
            START_CAPABILITY,
        ],
    );
    assert!(output.status.success(), "{}", stderr(&output));

    let project: &Path = &sandbox.target_dir.join("My App");
    assert!(project.join("src/router.tsx").is_file());
    assert_eq!(
        std::fs::read_to_string(project.join("README.md")).unwrap(),
        "# My App\nTanStack Start"
    );
}

#[test]
fn one_flag_can_carry_a_comma_separated_capability_list() {
    let sandbox = sandbox();
    // The fixture's two capabilities exclude each other, so an error naming
    // both is proof that one flag value produced two capabilities.
    let output = build(&sandbox, WEB_PROJECT_TYPE, &["tanstack-router,tanstack-start"]);

    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(message.contains("mutually exclusive"), "{message}");
    assert!(message.contains(ROUTER_CAPABILITY) && message.contains(START_CAPABILITY), "{message}");
}

#[test]
fn a_project_type_with_no_package_json_scaffolds_without_one() {
    let sandbox = sandbox();
    let output = build(&sandbox, RUST_PROJECT_TYPE, &[]);
    assert!(output.status.success(), "{}", stderr(&output));

    let project: &Path = &sandbox.target_dir.join("My App");
    assert!(project.join("Cargo.toml").is_file());
    assert!(!project.join("package.json").exists());
    assert!(project.join(".git").exists());

    // Nothing filled the seam, so the line it was on is gone: a blank line
    // there is a `cargo fmt --check` diff in the project the user just got.
    assert_eq!(
        std::fs::read_to_string(project.join("src/lib.rs")).unwrap(),
        "pub mod cli;\npub mod theme;\n"
    );

    let report = stdout(&output);
    assert!(report.contains("Rust CLI"), "{report}");
    assert!(report.contains("cargo run"), "{report}");
    assert!(!report.contains("pnpm"), "{report}");
}

#[test]
fn a_capability_of_a_cargo_project_merges_its_manifest_fragment() {
    let sandbox = sandbox();
    let output = build(&sandbox, RUST_PROJECT_TYPE, &[TUI_CAPABILITY]);
    assert!(output.status.success(), "{}", stderr(&output));

    let project: &Path = &sandbox.target_dir.join("My App");
    let manifest = std::fs::read_to_string(project.join("Cargo.toml")).unwrap();

    // Both layers' dependencies, and the comment each carries saying why it is
    // there, survive the merge all the way to the finished project.
    assert!(manifest.contains("name = \"my-app\""), "{manifest}");
    assert!(manifest.contains("# Error handling with context.\nanyhow = \"1\""), "{manifest}");
    assert!(manifest.contains("# Terminal UI rendering.\nratatui = \"0.29\""), "{manifest}");

    assert_eq!(
        std::fs::read_to_string(project.join("src/lib.rs")).unwrap(),
        "pub mod cli;\npub mod tui;\npub mod theme;\n"
    );
    assert!(project.join("src/tui.rs").is_file());
}

#[test]
fn a_template_without_a_skills_manifest_installs_no_skills() {
    let sandbox = sandbox();
    let output = build(&sandbox, WEB_PROJECT_TYPE, &[ROUTER_CAPABILITY]);
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
    assert!(message.contains("--name and --project-type"), "{message}");
    // Nothing was created, and nothing was even fetched.
    assert_eq!(std::fs::read_dir(&sandbox.target_dir).unwrap().count(), 0);
}

#[test]
fn an_unknown_project_type_reports_the_real_choices() {
    let sandbox = sandbox();
    let output = build(&sandbox, "python:web", &[]);

    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(message.contains("no project type called"), "{message}");
    assert!(message.contains(WEB_PROJECT_TYPE) && message.contains(RUST_PROJECT_TYPE), "{message}");
}

#[test]
fn an_unmade_required_choice_lists_what_to_choose_between() {
    let sandbox = sandbox();
    // The two capabilities exclude each other, so a web project has to name
    // one of them when there are no questions to answer.
    let output = build(&sandbox, WEB_PROJECT_TYPE, &[]);

    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(message.contains("has to choose one"), "{message}");
    assert!(message.contains(ROUTER_CAPABILITY) && message.contains(START_CAPABILITY), "{message}");
    assert!(!sandbox.target_dir.join("My App").exists());
}

#[test]
fn a_capability_that_does_not_fit_the_project_type_is_refused() {
    let sandbox = sandbox();
    let output = build(&sandbox, RUST_PROJECT_TYPE, &[ROUTER_CAPABILITY]);

    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(message.contains("only fits a typescript:web project"), "{message}");
    assert!(!sandbox.target_dir.join("My App").exists());
}

#[test]
fn two_capabilities_that_exclude_each_other_are_refused() {
    let sandbox = sandbox();
    let output = build(&sandbox, WEB_PROJECT_TYPE, &[ROUTER_CAPABILITY, START_CAPABILITY]);

    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(message.contains("mutually exclusive"), "{message}");
    assert!(!sandbox.target_dir.join("My App").exists());
}

#[test]
fn an_unknown_capability_reports_the_real_choices() {
    let sandbox = sandbox();
    let output = build(&sandbox, WEB_PROJECT_TYPE, &["svelte"]);

    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(message.contains("no capability called"), "{message}");
    assert!(message.contains(ROUTER_CAPABILITY), "{message}");
}

#[test]
fn an_existing_destination_is_refused() {
    let sandbox = sandbox();
    std::fs::create_dir_all(sandbox.target_dir.join("My App")).unwrap();

    let output = build(&sandbox, WEB_PROJECT_TYPE, &[ROUTER_CAPABILITY]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("already exists"), "{}", stderr(&output));
}
