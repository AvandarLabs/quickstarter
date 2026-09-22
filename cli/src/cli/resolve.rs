//! Turning the flags a user passed into the complete set of answers.
//!
//! Each answer has the same three sources, tried in order: the value passed on
//! the command line, a question, and finally a default. `--yes` removes the
//! middle one, which is why an option with no default is required there.

use anyhow::{Result, bail};

use crate::catalog::{
    Capability, Catalog, ChoiceGroup, ProjectType, Selection, compatibility, groups,
};
use crate::cli::prompts;

/// Where the project is created when no location is passed or given.
const DEFAULT_LOCATION: &str = ".";

/// Resolves each answer from what was passed, asking for the rest when asking
/// is allowed.
pub struct Resolver {
    interactive: bool,
}

impl Resolver {
    /// Builds a resolver. `interactive` is false under `--yes`.
    pub fn new(interactive: bool) -> Resolver {
        Resolver { interactive }
    }

    /// The project's display name. A passed name is validated exactly as a
    /// typed one, so `--name 'a/b'` fails the same way.
    pub fn project_name(&self, passed: Option<&str>) -> Result<String> {
        if let Some(name) = passed {
            if let Err(problem) = prompts::validate_project_name(name) {
                bail!("--name {name:?} is not usable: {problem}");
            }
            return Ok(name.trim().to_string());
        }
        if !self.interactive {
            bail!("--name is required when --yes turns off the questions.");
        }
        prompts::project_name()
    }

    /// Where to create the project, defaulting to the current directory.
    pub fn location(&self, passed: Option<&str>) -> Result<String> {
        if let Some(location) = passed {
            return Ok(location.trim().to_string());
        }
        if !self.interactive {
            return Ok(DEFAULT_LOCATION.to_string());
        }
        prompts::location()
    }

    /// What to build: the project type, and the capabilities chosen with it.
    ///
    /// The whole combination is validated here, so nothing downstream has to
    /// wonder whether the tags it was handed go together.
    pub fn selection<'catalog>(
        &self,
        passed_type: Option<&str>,
        passed_capabilities: &[String],
        catalog: &'catalog Catalog,
    ) -> Result<Selection<'catalog>> {
        let project_type = self.project_type(passed_type, &catalog.project_types)?;
        let compatible = compatibility::compatible(&catalog.capabilities, project_type);
        let keys = self.capability_keys(passed_capabilities, project_type, &compatible)?;
        let selection = Selection::resolve(project_type, &keys, &catalog.capabilities)?;
        ensure_every_choice_is_made(&selection, &compatible)?;
        Ok(selection)
    }

    /// The project type to build. A passed value is matched against the keys,
    /// case-insensitively.
    fn project_type<'catalog>(
        &self,
        passed: Option<&str>,
        project_types: &'catalog [ProjectType],
    ) -> Result<&'catalog ProjectType> {
        if let Some(key) = passed {
            return find_project_type(key, project_types);
        }
        if !self.interactive {
            bail!(
                "--project-type is required when --yes turns off the questions.{}",
                available(project_types)
            );
        }
        prompts::select_project_type(project_types)
    }

    /// The capability keys to build with: the ones passed, or the answers to
    /// the questions this project type raises.
    ///
    /// A `--capability` flag is the whole answer rather than a first draft the
    /// questions top up, so a command that names its capabilities behaves the
    /// same with and without `--yes`.
    fn capability_keys(
        &self,
        passed: &[String],
        project_type: &ProjectType,
        compatible: &[&Capability],
    ) -> Result<Vec<String>> {
        if !passed.is_empty() || !self.interactive {
            return Ok(passed.to_vec());
        }
        prompts::select_capabilities(project_type, compatible)
    }
}

/// Finds the project type a `--project-type` value names, listing the real
/// choices when it names nothing.
fn find_project_type<'catalog>(
    key: &str,
    project_types: &'catalog [ProjectType],
) -> Result<&'catalog ProjectType> {
    let wanted = key.trim().to_ascii_lowercase();
    match project_types.iter().find(|one| one.key.to_ascii_lowercase() == wanted) {
        Some(project_type) => Ok(project_type),
        None => bail!("There is no project type called {key:?}.{}", available(project_types)),
    }
}

/// Rejects a selection that leaves a required choice unmade: capabilities that
/// exclude one another are a question, and a project type they fit has to
/// answer it.
fn ensure_every_choice_is_made(
    selection: &Selection,
    compatible: &[&Capability],
) -> Result<()> {
    for group in groups::choice_groups(compatible) {
        if !group.is_covered_by(&selection.capabilities) {
            bail!(
                "A {} project has to choose one of these capabilities:{}\n\
                 Pass one with --capability <KEY>.",
                selection.project_type.key,
                choices(&group)
            );
        }
    }
    Ok(())
}

/// A human-readable list of the project types this template repository offers.
fn available(project_types: &[ProjectType]) -> String {
    let choices: Vec<String> = project_types
        .iter()
        .map(|project_type| format!("\n  {:<16} {}", project_type.key, project_type.name))
        .collect();
    format!(" Available project types:{}", choices.concat())
}

/// A human-readable list of one group's members.
fn choices(group: &ChoiceGroup) -> String {
    let members: Vec<String> = group
        .members
        .iter()
        .map(|member| format!("\n  {:<16} {}", member.key, member.name))
        .collect();
    members.concat()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::test_tags;

    /// Two project types, and two routers that only fit the web one and
    /// exclude each other.
    fn catalog() -> Catalog {
        let mut router = test_tags::capability("tanstack-router");
        router.project_types = test_tags::keys(&["typescript:web"]);
        router.conflicts_with = test_tags::keys(&["tanstack-start"]);
        let mut start = test_tags::capability("tanstack-start");
        start.project_types = test_tags::keys(&["typescript:web"]);
        start.conflicts_with = test_tags::keys(&["tanstack-router"]);
        Catalog {
            project_types: vec![
                test_tags::project_type("typescript:web", "typescript"),
                test_tags::project_type("rust:cli", "rust"),
            ],
            capabilities: vec![router, start],
        }
    }

    #[test]
    fn a_passed_name_is_used_as_is() {
        let name = Resolver::new(false).project_name(Some("  My App  ")).unwrap();
        assert_eq!(name, "My App");
    }

    #[test]
    fn a_passed_name_is_validated() {
        let error = Resolver::new(true).project_name(Some("a/b")).unwrap_err().to_string();
        assert!(error.contains("path separator"), "{error}");
    }

    #[test]
    fn a_missing_name_without_questions_is_an_error() {
        assert!(Resolver::new(false).project_name(None).is_err());
    }

    #[test]
    fn a_missing_location_falls_back_to_the_current_directory() {
        assert_eq!(Resolver::new(false).location(None).unwrap(), ".");
    }

    #[test]
    fn a_passed_location_is_used_as_is() {
        assert_eq!(Resolver::new(false).location(Some(" ~/src ")).unwrap(), "~/src");
    }

    #[test]
    fn a_passed_project_type_matches_its_key_case_insensitively() {
        let catalog = catalog();
        let selection =
            Resolver::new(false).selection(Some("RUST:CLI"), &[], &catalog).unwrap();
        assert_eq!(selection.project_type.key, "rust:cli");
    }

    #[test]
    fn an_unknown_project_type_lists_the_real_ones() {
        let catalog = catalog();
        let error = Resolver::new(true)
            .selection(Some("python:web"), &[], &catalog)
            .unwrap_err()
            .to_string();

        assert!(error.contains("no project type called"), "{error}");
        assert!(error.contains("typescript:web") && error.contains("rust:cli"), "{error}");
    }

    #[test]
    fn a_missing_project_type_without_questions_is_an_error() {
        let catalog = catalog();
        let error = Resolver::new(false).selection(None, &[], &catalog).unwrap_err().to_string();
        assert!(error.contains("--project-type is required"), "{error}");
    }

    #[test]
    fn passed_capabilities_are_resolved_against_the_catalog() {
        let catalog = catalog();
        let keys = test_tags::keys(&["tanstack-start"]);

        let selection =
            Resolver::new(false).selection(Some("typescript:web"), &keys, &catalog).unwrap();

        assert_eq!(selection.capability_keys(), keys);
    }

    #[test]
    fn a_capability_that_does_not_fit_the_project_type_is_refused() {
        let catalog = catalog();
        let keys = test_tags::keys(&["tanstack-router"]);

        let error = Resolver::new(false)
            .selection(Some("rust:cli"), &keys, &catalog)
            .unwrap_err()
            .to_string();

        assert!(error.contains("tanstack-router only fits a typescript:web project"), "{error}");
    }

    #[test]
    fn an_unanswered_required_choice_lists_the_choices() {
        let catalog = catalog();

        let error = Resolver::new(false)
            .selection(Some("typescript:web"), &[], &catalog)
            .unwrap_err()
            .to_string();

        assert!(error.contains("has to choose one"), "{error}");
        assert!(error.contains("tanstack-router") && error.contains("tanstack-start"), "{error}");
        assert!(error.contains("--capability"), "{error}");
    }

    #[test]
    fn a_project_type_with_no_required_choice_needs_no_capability() {
        let catalog = catalog();

        let selection = Resolver::new(false).selection(Some("rust:cli"), &[], &catalog).unwrap();

        assert!(selection.capabilities.is_empty());
    }
}
