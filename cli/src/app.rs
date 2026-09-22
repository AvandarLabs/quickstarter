//! End-to-end orchestration of a scaffolding run.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::catalog;
use crate::cli::{Args, Resolver};
use crate::compose::{self, ComposePlan};
use crate::git_init;
use crate::template::{self, ensure_git_available};

/// Runs the scaffolder: resolve the answers (from flags, questions, or
/// defaults), fetch, compose, report.
pub fn run(args: &Args) -> Result<()> {
    // Fail fast, before asking or fetching anything, on a missing git or on a
    // `--yes` command that cannot be completed without questions.
    ensure_git_available()?;
    args.ensure_answerable()?;

    let resolver = Resolver::new(args.interactive());
    let project_name = resolver.project_name(args.name.as_deref())?;
    let package_name = to_package_name(&project_name);
    let location = resolver.location(args.dir.as_deref())?;

    // Resolve and validate the destination before any network work so an
    // existing directory fails immediately.
    let dest = resolve_dest(&location, &project_name)?;
    if dest.exists() {
        bail!("{} already exists. Choose a different name or location.", dest.display());
    }

    println!("Fetching the latest template from {}...", args.repo);
    let checkout = template::fetch::clone(&args.repo)?;

    // The stack is resolved only after the clone, because the choices are
    // whatever modules the template repository ships.
    let modules = catalog::discover_modules(&checkout.path().join("templates"))?;
    let module = resolver.stack(args.stack.as_deref(), &modules)?;

    let plan = ComposePlan {
        template_root: checkout.path(),
        module,
        project_name: &project_name,
        package_name: &package_name,
    };
    compose::compose(&plan, &dest).inspect_err(|_| {
        // Do not leave a half-built directory behind.
        std::fs::remove_dir_all(&dest).ok();
    })?;

    init_git_repository(&dest);
    report_success(&project_name, &dest, module);
    Ok(())
}

/// Turns the new project into a git repository. Best-effort: a git failure (for
/// example, git not being fully configured) must not discard the project the
/// user just successfully created.
fn init_git_repository(dest: &std::path::Path) {
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

fn report_success(project_name: &str, dest: &std::path::Path, module: &catalog::Module) {
    let dev_url = module
        .tokens
        .get("DEV_URL")
        .map(String::as_str)
        .unwrap_or("http://localhost:5173");
    println!("\nCreated {project_name} ({}) at {}", module.name, dest.display());
    println!("\nNext steps:");
    println!("  cd {}", dest.display());
    println!("  pnpm install");
    println!("  pnpm dev        # then open {dev_url}");
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
