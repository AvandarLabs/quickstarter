//! The project type tag: the one tag every generated project has exactly one
//! of.
//!
//! A project type (`typescript:web`, `rust:cli`) decides the build system, the
//! files the project starts from, and the skills that come with its language.
//! It is declared by a `project-type.json` in its own directory under
//! `templates/project-types/`, so adding one is a template change rather than
//! a code change.

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::catalog::discovery;
use crate::compose::tokens::Tokens;

/// Directory under `templates/` holding one directory per project type.
pub const PROJECT_TYPES_DIR_NAME: &str = "project-types";

/// The file that declares a project type, inside its own directory.
pub const PROJECT_TYPE_FILE_NAME: &str = "project-type.json";

/// A single selectable project type, parsed from its `project-type.json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectType {
    /// The tag id, `<language>:<product>`, for example `"rust:cli"`.
    pub key: String,
    /// Human-readable name shown in the selection prompt.
    pub name: String,
    /// One-line description shown alongside the name.
    #[serde(default)]
    pub description: String,
    /// The language half of the key, for example `"rust"`. Capabilities
    /// restrict themselves by it, so it is declared rather than parsed back
    /// out of the key.
    pub language: String,
    /// Sort order for the selection prompt (ascending).
    #[serde(default)]
    pub order: i64,
    /// Token values this project type contributes to substitution.
    #[serde(default)]
    pub tokens: Tokens,
    /// Directory name under `templates/project-types/`, for example
    /// `"rust-cli"`. It is filled in from the directory the declaration was
    /// read from rather than declared, because a key carries a `:` that a
    /// directory name cannot.
    #[serde(skip)]
    pub slug: String,
}

impl ProjectType {
    /// Loads a project type from `<dir>/project-type.json`, taking its slug
    /// from the directory name.
    pub fn load(dir: &Path) -> Result<ProjectType> {
        let path = dir.join(PROJECT_TYPE_FILE_NAME);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let mut project_type: ProjectType = serde_json::from_str(&text)
            .with_context(|| format!("parsing {}", path.display()))?;
        project_type.slug = discovery::slug_of(dir);
        Ok(project_type)
    }
}

/// Discovers every project type under `<templates_root>/project-types`, sorted
/// by `order` then `name`.
///
/// Errors when the repository declares none: a project cannot be built without
/// a project type, so an empty catalog is a malformed template repository
/// rather than an empty list of choices.
pub fn discover(templates_root: &Path) -> Result<Vec<ProjectType>> {
    let root = templates_root.join(PROJECT_TYPES_DIR_NAME);
    let mut project_types = Vec::new();
    for dir in discovery::declaring_dirs(&root, PROJECT_TYPE_FILE_NAME)? {
        project_types.push(ProjectType::load(&dir)?);
    }

    if project_types.is_empty() {
        bail!(
            "no project types found under {}. The template repository may be malformed.",
            root.display()
        );
    }

    project_types.sort_by(|left, right| {
        left.order.cmp(&right.order).then(left.name.cmp(&right.name))
    });
    Ok(project_types)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_project_type(root: &Path, slug: &str, key: &str, order: i64) {
        let dir = root.join(PROJECT_TYPES_DIR_NAME).join(slug);
        std::fs::create_dir_all(&dir).unwrap();
        let declaration = format!(
            r#"{{ "key": "{key}", "name": "{key} name", "language": "rust",
                  "order": {order}, "tokens": {{ "X": "y" }} }}"#
        );
        std::fs::write(dir.join(PROJECT_TYPE_FILE_NAME), declaration).unwrap();
    }

    #[test]
    fn loads_a_project_type_with_its_slug_and_tokens() {
        let temp = tempfile::tempdir().unwrap();
        write_project_type(temp.path(), "rust-cli", "rust:cli", 2);

        let project_type =
            ProjectType::load(&temp.path().join(PROJECT_TYPES_DIR_NAME).join("rust-cli")).unwrap();

        assert_eq!(project_type.key, "rust:cli");
        assert_eq!(project_type.language, "rust");
        assert_eq!(project_type.slug, "rust-cli");
        assert_eq!(project_type.tokens.get("X").unwrap(), "y");
    }

    #[test]
    fn discovers_and_sorts_project_types_by_order() {
        let temp = tempfile::tempdir().unwrap();
        write_project_type(temp.path(), "rust-cli", "rust:cli", 2);
        write_project_type(temp.path(), "typescript-web", "typescript:web", 1);

        let project_types = discover(temp.path()).unwrap();

        let keys: Vec<&str> = project_types.iter().map(|one| one.key.as_str()).collect();
        assert_eq!(keys, vec!["typescript:web", "rust:cli"]);
    }

    #[test]
    fn errors_when_the_repository_declares_no_project_type() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join(PROJECT_TYPES_DIR_NAME)).unwrap();

        let error = discover(temp.path()).unwrap_err().to_string();

        assert!(error.contains("no project types"), "{error}");
    }
}
