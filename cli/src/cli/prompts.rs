//! Interactive prompts, built on `dialoguer`.

use anyhow::{Context, Result};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Select};

use crate::catalog::Module;

/// Asks for the new project's name. There is no default: a name is required.
/// The name may not be empty or contain a path separator.
pub fn project_name() -> Result<String> {
    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Project name")
        .validate_with(|input: &String| validate_project_name(input))
        .interact_text()
        .context("reading the project name")?;
    Ok(name.trim().to_string())
}

/// Asks where to create the project, defaulting to the current directory.
pub fn location() -> Result<String> {
    let location: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Where should it be created?")
        .default(".".to_string())
        .interact_text()
        .context("reading the target location")?;
    Ok(location.trim().to_string())
}

/// Asks which stack to use. Presents each module's name and description and
/// returns the chosen module.
pub fn select_stack(modules: &[Module]) -> Result<&Module> {
    let labels: Vec<String> = modules
        .iter()
        .map(|module| format!("{} - {}", module.name, module.description))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which stack?")
        .items(&labels)
        .default(0)
        .interact()
        .context("reading the stack selection")?;

    Ok(&modules[selection])
}

/// Validates a project name for use as both a directory name and the basis of
/// a package name. Shared with [`crate::cli::resolve`] so a name passed as
/// `--name` is held to exactly the same rules as a typed one.
pub fn validate_project_name(input: &str) -> Result<(), String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Please enter a name.".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("The name cannot contain a path separator.".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_names() {
        assert!(validate_project_name("   ").is_err());
    }

    #[test]
    fn rejects_names_with_separators() {
        assert!(validate_project_name("a/b").is_err());
    }

    #[test]
    fn accepts_reasonable_names() {
        assert!(validate_project_name("my-new-project").is_ok());
    }
}
