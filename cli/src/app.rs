//! End-to-end orchestration of a scaffolding run.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::catalog::{self, ProjectType, Selection};
use crate::cli::{Args, Resolver};
use crate::compose::{self, ComposePlan, TEMPLATES_DIR_NAME};
use crate::git_init;
use crate::skills::commands;
use crate::skills::install::{self, InstallOutcome, ProcessRunner};
use crate::template::{self, ensure_git_available};

/// The token a project type uses to say how its project is run, which differs
/// with its build system (`pnpm install` on a web project, `cargo run` on a
/// Rust one).
const NEXT_STEPS_TOKEN: &str = "NEXT_STEPS";

/// What to print after a project whose type says nothing about how to run it.
const GENERIC_NEXT_STEPS: &str = "cat README.md   # how to install, build, and run it";

/// Runs the scaffolder: resolve the answers (from flags, questions, or
/// defaults), fetch, compose, report.
pub fn run(args: &Args) -> Result<()> {
    // Fail fast, before asking or fetching anything, on a missing git or on a
    // `--yes` command that cannot be completed without questions.
    ensure_git_available()?;
    args.ensure_answerable()?;

    let resolver = Resolver::new(args.interactive());
    let project_name = resolver.project_name(args.name.as_deref())?;
    let location = resolver.location(args.dir.as_deref())?;

    // Resolve and validate the destination before any network work so an
    // existing directory fails immediately.
    let dest = resolve_dest(&location, &project_name)?;
    if dest.exists() {
        bail!("{} already exists. Choose a different name or location.", dest.display());
    }

    println!("Fetching the latest template from {}...", args.repo);
    let checkout = template::fetch::clone(&args.repo)?;

    // The tags are resolved only after the clone, because the choices are
    // whatever the template repository ships.
    let catalog = catalog::discover(&checkout.path().join(TEMPLATES_DIR_NAME))?;
    let selection =
        resolver.selection(args.project_type.as_deref(), &args.capabilities, &catalog)?;

    // The skills are selected before composition, because the new project's
    // update script has to be told which of them install themselves.
    let specs = select_skill_specs(checkout.path(), &selection);
    build_project(checkout.path(), &selection, &project_name, &specs, &dest)?;

    install_agent_skills(&specs, &dest);
    init_git_repository(&dest);
    report_success(&project_name, &dest, &selection);
    Ok(())
}

/// Composes the selected layers into `dest`, leaving no half-built directory
/// behind when something goes wrong.
fn build_project(
    template_root: &Path,
    selection: &Selection,
    project_name: &str,
    specs: &[String],
    dest: &Path,
) -> Result<()> {
    let package_name = to_package_name(project_name);
    let plan = ComposePlan {
        template_root,
        project_type: selection.project_type,
        capabilities: &selection.capabilities,
        project_name,
        package_name: &package_name,
        extra_tokens: skill_tokens(specs),
    };
    compose::compose(&plan, dest).inspect_err(|_| {
        std::fs::remove_dir_all(dest).ok();
    })
}

/// The skills the selected tags name.
///
/// Best-effort: a template repository whose manifest cannot be read costs the
/// user their skills, not their project, so the problem is reported and the
/// selection is empty.
fn select_skill_specs(template_root: &Path, selection: &Selection) -> Vec<String> {
    let selected = install::select_specs(
        template_root,
        &selection.project_type.key,
        &selection.capability_keys(),
    );
    match selected {
        Ok(specs) => specs,
        Err(error) => {
            eprintln!("Warning: no agent skills were selected: {error:#}");
            Vec::new()
        }
    }
}

/// The tokens the templates need about the selected skills: the names of the
/// ones that install themselves, which the new project's update script drives
/// through their own CLI.
fn skill_tokens(specs: &[String]) -> compose::tokens::Tokens {
    let mut tokens = compose::tokens::Tokens::new();
    tokens.insert(
        "SELF_INSTALLING_SKILLS".to_string(),
        commands::self_installing_skill_names(specs).join(" "),
    );
    tokens
}

/// Installs the selected agent skills inside the new project. Best-effort like
/// the git initialization: a skill that will not install is reported with the
/// command to run by hand, and never costs the user the project they just
/// created.
fn install_agent_skills(specs: &[String], dest: &Path) {
    let planned = commands::install_commands(specs);
    if planned.is_empty() {
        return;
    }

    println!("\nInstalling {} agent skills...", planned.len());
    report_failed_skills(&install::run_all(&planned, dest, &ProcessRunner), dest);
}

/// Prints the commands the user can run themselves for every skill that failed
/// to install.
fn report_failed_skills(outcome: &InstallOutcome, dest: &Path) {
    if outcome.failed.is_empty() {
        return;
    }
    eprintln!(
        "\nWarning: {} of {} agent skills could not be installed. The project is \
         ready; run these yourself in {}:",
        outcome.failed.len(),
        outcome.attempted(),
        dest.display()
    );
    for command in &outcome.failed {
        eprintln!("  {}", command.shell_line());
    }
}

/// Turns the new project into a git repository. Best-effort: a git failure (for
/// example, git not being fully configured) must not discard the project the
/// user just successfully created.
fn init_git_repository(dest: &Path) {
    if let Err(error) = git_init::init_and_commit(dest) {
        eprintln!(
            "Warning: could not initialize a git repository in {}: {error:#}\n\
             The project was created successfully; run `git init` yourself to add \
             version control.",
            dest.display()
        );
    }
}

/// Converts a display name into an npm-safe package name: lowercase, with runs
/// of unsupported characters collapsed to single hyphens and the ends trimmed.
pub fn to_package_name(display_name: &str) -> String {
    let mut package_name = String::with_capacity(display_name.len());
    let mut last_was_hyphen = false;
    for character in display_name.trim().chars() {
        if character.is_ascii_alphanumeric() {
            package_name.push(character.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen {
            package_name.push('-');
            last_was_hyphen = true;
        }
    }
    let trimmed = package_name.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "app".to_string()
    } else {
        trimmed
    }
}

/// Resolves the destination directory from the location and project name,
/// expanding a leading `~` to the home directory.
fn resolve_dest(location: &str, project_name: &str) -> Result<PathBuf> {
    let base = if location.is_empty() || location == "." {
        std::env::current_dir().context("reading the current directory")?
    } else if let Some(rest) = location.strip_prefix("~/") {
        home_dir()?.join(rest)
    } else if location == "~" {
        home_dir()?
    } else {
        PathBuf::from(location)
    };
    Ok(base.join(project_name))
}

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set, so '~' cannot be expanded")
}

/// Reports what was built and how to run it.
fn report_success(project_name: &str, dest: &Path, selection: &Selection) {
    println!("\nCreated {project_name} ({}) at {}", built_with(selection), dest.display());
    println!("\nNext steps:");
    println!("  cd {}", dest.display());
    for line in next_steps(selection.project_type).lines() {
        println!("  {line}");
    }
}

/// What the project was built from: its project type, plus every capability it
/// chose.
fn built_with(selection: &Selection) -> String {
    let mut description = selection.project_type.name.clone();
    for capability in &selection.capabilities {
        description.push_str(" + ");
        description.push_str(&capability.name);
    }
    description
}

/// The commands to run next. They belong to the project type rather than to
/// the scaffolder, because a Rust project is not started with `pnpm`, so they
/// come from its own token with a generic line as the fallback.
fn next_steps(project_type: &ProjectType) -> &str {
    project_type
        .tokens
        .get(NEXT_STEPS_TOKEN)
        .map(String::as_str)
        .unwrap_or(GENERIC_NEXT_STEPS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{Capability, test_tags};

    #[test]
    fn package_name_lowercases_and_hyphenates() {
        assert_eq!(to_package_name("My New Project"), "my-new-project");
    }

    #[test]
    fn package_name_collapses_and_trims_separators() {
        assert_eq!(to_package_name("  Foo__Bar!! "), "foo-bar");
    }

    #[test]
    fn package_name_falls_back_when_empty() {
        assert_eq!(to_package_name("!!!"), "app");
    }

    #[test]
    fn resolve_dest_joins_name_onto_location() {
        let dest = resolve_dest("/tmp/projects", "my-app").unwrap();
        assert_eq!(dest, PathBuf::from("/tmp/projects/my-app"));
    }

    #[test]
    fn a_project_is_described_by_its_type_and_its_capabilities() {
        let mut project_type = test_tags::project_type("typescript:web", "typescript");
        project_type.name = "TypeScript web app".to_string();
        let mut router = test_tags::capability("tanstack-router");
        router.name = "TanStack Router".to_string();
        let capabilities: Vec<&Capability> = vec![&router];

        let selection = Selection { project_type: &project_type, capabilities };

        assert_eq!(built_with(&selection), "TypeScript web app + TanStack Router");
    }

    #[test]
    fn a_project_with_no_capability_is_described_by_its_type_alone() {
        let mut project_type = test_tags::project_type("rust:cli", "rust");
        project_type.name = "Rust CLI".to_string();

        let selection = Selection { project_type: &project_type, capabilities: Vec::new() };

        assert_eq!(built_with(&selection), "Rust CLI");
    }

    #[test]
    fn the_next_steps_come_from_the_project_type() {
        let mut project_type = test_tags::project_type("rust:cli", "rust");
        project_type
            .tokens
            .insert(NEXT_STEPS_TOKEN.to_string(), "cargo run".to_string());

        assert_eq!(next_steps(&project_type), "cargo run");
    }

    #[test]
    fn a_project_type_that_says_nothing_still_gets_a_next_step() {
        let project_type = test_tags::project_type("rust:cli", "rust");
        assert_eq!(next_steps(&project_type), GENERIC_NEXT_STEPS);
    }
}
