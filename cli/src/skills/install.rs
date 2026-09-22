//! Running the selected install commands inside the new project.
//!
//! Installing is best-effort, exactly like the git initialization: one skill
//! that fails to install costs that skill and nothing else, and the project
//! the user just created is never discarded over it. The commands that failed
//! come back in the outcome so the caller can print what to run by hand.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::skills::commands::SkillCommand;
use crate::skills::manifest;

/// Runs one install command. Implemented by [`ProcessRunner`] in production
/// and by a recording fake in the tests, which is what keeps the test suite
/// off the network.
pub trait CommandRunner {
    /// Runs `command` with `dest` as its working directory.
    fn run(&self, command: &SkillCommand, dest: &Path) -> Result<()>;
}

/// Runs commands as real child processes.
pub struct ProcessRunner;

impl CommandRunner for ProcessRunner {
    /// Spawns the command with inherited stdio, so the user watches the
    /// installer's own progress, and turns a non-zero exit into an error
    /// carrying the status.
    fn run(&self, command: &SkillCommand, dest: &Path) -> Result<()> {
        let status = Command::new(&command.program)
            .args(&command.args)
            .current_dir(dest)
            .status()
            .with_context(|| format!("running `{}`", command.shell_line()))?;
        if !status.success() {
            bail!("`{}` exited with {}", command.shell_line(), status);
        }
        Ok(())
    }
}

/// What an install run achieved.
#[derive(Debug, Default)]
pub struct InstallOutcome {
    /// How many skills installed successfully.
    pub installed: usize,
    /// The commands that failed, so the caller can say what to re-run.
    pub failed: Vec<SkillCommand>,
}

impl InstallOutcome {
    /// How many skills the run attempted, successes and failures together.
    pub fn attempted(&self) -> usize {
        self.installed + self.failed.len()
    }
}

/// The skill specs `capabilities` select from the template's manifest.
///
/// Selecting is separate from running so the caller can both tell the new
/// project which of its skills install themselves (a token the templates
/// substitute) and know how many skills are coming before the first one
/// starts.
///
/// A template repository with no manifest selects nothing, which is not an
/// error: it is what lets a minimal template repository be scaffolded offline.
/// A capability the manifest does not declare is an error, raised before any
/// command runs, because it would otherwise install a silently smaller set.
pub fn select_specs(template_root: &Path, capabilities: &[String]) -> Result<Vec<String>> {
    let Some(manifest) = manifest::load(template_root)? else {
        return Ok(Vec::new());
    };
    manifest.specs_for(capabilities)
}

/// Runs every planned command in `dest`, in order.
///
/// A command that fails is recorded and the run continues: one unreachable
/// repository should not cost the user every other skill.
pub fn run_all(
    planned: &[SkillCommand],
    dest: &Path,
    runner: &dyn CommandRunner,
) -> InstallOutcome {
    let mut outcome = InstallOutcome::default();
    for command in planned {
        match runner.run(command, dest) {
            Ok(()) => outcome.installed += 1,
            Err(_) => outcome.failed.push(command.clone()),
        }
    }
    outcome
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::PathBuf;

    use anyhow::bail;

    use super::*;
    use crate::skills::commands;

    const MANIFEST: &str = r#"{
        "capabilities": {
            "global": ["obra/superpowers/skills/brainstorming", "pbakaus/impeccable"],
            "typescript": ["mcollina/skills/skills/typescript-magician"]
        }
    }"#;

    /// Records what it was asked to run instead of spawning anything, so the
    /// tests never reach the network.
    #[derive(Default)]
    struct FakeRunner {
        runs: RefCell<Vec<(String, PathBuf)>>,
        failing_label: Option<String>,
    }

    impl FakeRunner {
        fn failing_on(label: &str) -> FakeRunner {
            FakeRunner {
                failing_label: Some(label.to_string()),
                ..FakeRunner::default()
            }
        }

        fn labels(&self) -> Vec<String> {
            self.runs
                .borrow()
                .iter()
                .map(|(label, _)| label.clone())
                .collect()
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, command: &SkillCommand, dest: &Path) -> Result<()> {
            self.runs
                .borrow_mut()
                .push((command.label.clone(), dest.to_path_buf()));
            if self.failing_label.as_deref() == Some(command.label.as_str()) {
                bail!("exploded");
            }
            Ok(())
        }
    }

    fn template_with_manifest() -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join(manifest::MANIFEST_FILE_NAME),
            MANIFEST,
        )
        .unwrap();
        temp
    }

    fn typescript() -> Vec<String> {
        vec!["typescript".to_string()]
    }

    /// Plans and runs in one step, the way `app.rs` does.
    fn install(
        template_root: &Path,
        capabilities: &[String],
        dest: &Path,
        runner: &dyn CommandRunner,
    ) -> Result<InstallOutcome> {
        let specs = select_specs(template_root, capabilities)?;
        Ok(run_all(&commands::install_commands(&specs), dest, runner))
    }

    #[test]
    fn runs_every_selected_command_in_order_in_the_destination() {
        let template = template_with_manifest();
        let dest = PathBuf::from("/tmp/some-project");
        let runner = FakeRunner::default();

        let outcome = install(template.path(), &typescript(), &dest, &runner).unwrap();

        assert_eq!(
            runner.labels(),
            vec![
                "obra/superpowers/skills/brainstorming",
                "pbakaus/impeccable",
                "mcollina/skills/skills/typescript-magician",
            ]
        );
        assert!(
            runner
                .runs
                .borrow()
                .iter()
                .all(|(_, where_run)| *where_run == dest)
        );
        assert_eq!(outcome.installed, 3);
        assert!(outcome.failed.is_empty());
    }

    #[test]
    fn one_failure_does_not_stop_the_other_skills() {
        let template = template_with_manifest();
        let runner = FakeRunner::failing_on("pbakaus/impeccable");

        let outcome =
            install(template.path(), &typescript(), Path::new("/tmp/p"), &runner).unwrap();

        assert_eq!(runner.labels().len(), 3);
        assert_eq!(outcome.installed, 2);
        let failed: Vec<&str> = outcome
            .failed
            .iter()
            .map(|command| command.label.as_str())
            .collect();
        assert_eq!(failed, vec!["pbakaus/impeccable"]);
    }

    #[test]
    fn a_template_without_a_manifest_installs_nothing_and_is_not_an_error() {
        let template = tempfile::tempdir().unwrap();
        let runner = FakeRunner::default();

        let outcome =
            install(template.path(), &typescript(), Path::new("/tmp/p"), &runner).unwrap();

        assert!(runner.labels().is_empty());
        assert_eq!(outcome.installed, 0);
        assert!(outcome.failed.is_empty());
    }

    #[test]
    fn an_unknown_capability_fails_the_whole_plan() {
        let template = template_with_manifest();
        let runner = FakeRunner::default();

        let error = install(
            template.path(),
            &["svelte".to_string()],
            Path::new("/tmp/p"),
            &runner,
        )
        .unwrap_err();

        assert!(error.to_string().contains("svelte"), "{error}");
        assert!(runner.labels().is_empty());
    }
}
