//! Interactive prompts, built on `dialoguer`.
//!
//! Every enum-shaped answer is a list, never free text: the project type is a
//! `Select`, each set of capabilities that exclude one another is a `Select`
//! of its own, and whatever is left over is a `MultiSelect`. A user therefore
//! cannot type a combination the catalog would reject.

use anyhow::{Context, Result};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, MultiSelect, Select};

use crate::catalog::{Capability, ChoiceGroup, ProjectType, groups};

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

/// Asks which project type to build. Presents each name and description and
/// returns the chosen project type.
pub fn select_project_type(project_types: &[ProjectType]) -> Result<&ProjectType> {
    let labels: Vec<String> = project_types.iter().map(label_of).collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What kind of project?")
        .items(&labels)
        .default(0)
        .interact()
        .context("reading the project type selection")?;

    Ok(&project_types[selection])
}

/// Asks which capabilities to add, given the ones that fit the chosen project
/// type, and returns their keys.
///
/// Capabilities that exclude one another are one question each, so an
/// impossible pair cannot be chosen. Whatever excludes nothing is offered as a
/// `MultiSelect`, which is skipped when there is nothing optional to offer.
pub fn select_capabilities(
    project_type: &ProjectType,
    compatible: &[&Capability],
) -> Result<Vec<String>> {
    let mut chosen = Vec::new();
    for group in groups::choice_groups(compatible) {
        chosen.push(choose_one(&group)?);
    }
    chosen.extend(choose_any(project_type, &groups::optional_capabilities(compatible))?);
    Ok(chosen)
}

/// Asks a group's question and returns the key of the member chosen.
fn choose_one(group: &ChoiceGroup) -> Result<String> {
    let labels: Vec<String> = group.members.iter().map(|member| label_of(*member)).collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(group.question())
        .items(&labels)
        .default(0)
        .interact()
        .context("reading a capability selection")?;

    Ok(group.members[selection].key.clone())
}

/// Offers the capabilities that exclude nothing, and returns the keys ticked.
fn choose_any(project_type: &ProjectType, optional: &[&Capability]) -> Result<Vec<String>> {
    if optional.is_empty() {
        return Ok(Vec::new());
    }
    let labels: Vec<String> = optional.iter().map(|capability| label_of(*capability)).collect();

    let selected = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Anything else for your {} project?", project_type.name))
        .items(&labels)
        .interact()
        .context("reading the optional capabilities")?;

    Ok(selected.into_iter().map(|index| optional[index].key.clone()).collect())
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

/// How one tag is shown in a list: its name, and its description when it has
/// one.
fn label_of<T: Labelled>(tag: &T) -> String {
    let (name, description) = (tag.name(), tag.description());
    if description.is_empty() {
        name.to_string()
    } else {
        format!("{name} - {description}")
    }
}

/// A tag that can be offered in a list.
trait Labelled {
    /// The name shown in the list.
    fn name(&self) -> &str;
    /// The one-line description shown alongside it.
    fn description(&self) -> &str;
}

impl Labelled for ProjectType {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }
}

impl Labelled for Capability {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::test_tags;

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

    #[test]
    fn a_tag_with_a_description_shows_it_beside_its_name() {
        let mut capability = test_tags::capability("tanstack-router");
        capability.name = "TanStack Router".to_string();
        capability.description = "Client-side routing".to_string();

        assert_eq!(label_of(&capability), "TanStack Router - Client-side routing");
    }

    #[test]
    fn a_tag_without_a_description_is_shown_by_name_alone() {
        let project_type = test_tags::project_type("rust:cli", "rust");
        assert_eq!(label_of(&project_type), "rust:cli");
    }
}
