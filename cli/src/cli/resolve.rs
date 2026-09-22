//! Turning the flags a user passed into the complete set of answers.
//!
//! Each answer has the same three sources, tried in order: the value passed on
//! the command line, a question, and finally a default. `--yes` removes the
//! middle one, which is why an option with no default is required there.

use anyhow::{Result, bail};

use crate::catalog::Module;
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

    /// The stack module to build with. A passed value is matched against the
    /// module keys, case-insensitively.
    pub fn stack<'modules>(
        &self,
        passed: Option<&str>,
        modules: &'modules [Module],
    ) -> Result<&'modules Module> {
        if let Some(key) = passed {
            return find_module(key, modules);
        }
        if !self.interactive {
            bail!("--stack is required when --yes turns off the questions.{}", available(modules));
        }
        prompts::select_stack(modules)
    }
}

/// Finds the module a `--stack` value names, listing the real choices when it
/// names nothing.
fn find_module<'modules>(key: &str, modules: &'modules [Module]) -> Result<&'modules Module> {
    let wanted = key.trim().to_ascii_lowercase();
    match modules.iter().find(|module| module.key.to_ascii_lowercase() == wanted) {
        Some(module) => Ok(module),
        None => bail!("There is no stack called {key:?}.{}", available(modules)),
    }
}

/// A human-readable list of the stacks this template repository offers.
fn available(modules: &[Module]) -> String {
    let choices: Vec<String> = modules
        .iter()
        .map(|module| format!("\n  {:<10} {}", module.key, module.name))
        .collect();
    format!(" Available stacks:{}", choices.concat())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modules() -> Vec<Module> {
        vec![module("router", "TanStack Router"), module("start", "TanStack Start")]
    }

    fn module(key: &str, name: &str) -> Module {
        serde_json::from_value(serde_json::json!({ "key": key, "name": name })).unwrap()
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
    fn a_passed_stack_matches_its_key_case_insensitively() {
        let modules = modules();
        let module = Resolver::new(false).stack(Some("ROUTER"), &modules).unwrap();
        assert_eq!(module.key, "router");
    }

    #[test]
    fn an_unknown_stack_lists_the_real_ones() {
        let modules = modules();
        let error = Resolver::new(true).stack(Some("svelte"), &modules).unwrap_err().to_string();
        assert!(error.contains("no stack called"), "{error}");
        assert!(error.contains("router") && error.contains("start"), "{error}");
    }

    #[test]
    fn a_missing_stack_without_questions_is_an_error() {
        let modules = modules();
        assert!(Resolver::new(false).stack(None, &modules).is_err());
    }
}
