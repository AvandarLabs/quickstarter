//! The capability tag: a library or framework layered on a project type.
//!
//! A project has any number of capabilities (`tanstack-router`,
//! `tanstack-start`), each declared by a `capability.json` in its own
//! directory under `templates/capabilities/`. A capability says what it fits
//! (`projectTypes`, `languages`) and what it excludes (`conflictsWith`); the
//! rules that read those fields live in [`crate::catalog::compatibility`].

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::catalog::discovery;
use crate::compose::tokens::Tokens;

/// Directory under `templates/` holding one directory per capability.
pub const CAPABILITIES_DIR_NAME: &str = "capabilities";

/// The file that declares a capability, inside its own directory.
pub const CAPABILITY_FILE_NAME: &str = "capability.json";

/// A single capability, parsed from its `capability.json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    /// The tag id, for example `"tanstack-router"`.
    pub key: String,
    /// Human-readable name shown in the selection prompts.
    pub name: String,
    /// One-line description shown alongside the name.
    #[serde(default)]
    pub description: String,
    /// Sort order, which is also the order capabilities are overlaid in
    /// (ascending).
    #[serde(default)]
    pub order: i64,
    /// Project type keys this capability fits. Empty means it fits every
    /// project type.
    #[serde(default)]
    pub project_types: Vec<String>,
    /// Languages this capability fits. Empty means it fits every language.
    #[serde(default)]
    pub languages: Vec<String>,
    /// Capabilities that cannot be chosen alongside this one.
    #[serde(default)]
    pub conflicts_with: Vec<String>,
    /// Token values this capability contributes to substitution. They win over
    /// the project type's tokens of the same name.
    #[serde(default)]
    pub tokens: Tokens,
    /// Directory name under `templates/capabilities/`. It matches the key
    /// today, but it is read from the directory rather than assumed so a
    /// capability is free to be filed under any name.
    #[serde(skip)]
    pub slug: String,
}

impl Capability {
    /// Loads a capability from `<dir>/capability.json`, taking its slug from
    /// the directory name.
    pub fn load(dir: &Path) -> Result<Capability> {
        let path = dir.join(CAPABILITY_FILE_NAME);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let mut capability: Capability = serde_json::from_str(&text)
            .with_context(|| format!("parsing {}", path.display()))?;
        capability.slug = discovery::slug_of(dir);
        Ok(capability)
    }
}

/// Discovers every capability under `<templates_root>/capabilities`, sorted by
/// `order` then `name`.
///
/// A repository that ships none is not an error, unlike one that ships no
/// project type: a project type with no capabilities at all is a perfectly
/// good project type.
pub fn discover(templates_root: &Path) -> Result<Vec<Capability>> {
    let root = templates_root.join(CAPABILITIES_DIR_NAME);
    let mut capabilities = Vec::new();
    for dir in discovery::declaring_dirs(&root, CAPABILITY_FILE_NAME)? {
        capabilities.push(Capability::load(&dir)?);
    }

    capabilities.sort_by(|left, right| {
        left.order.cmp(&right.order).then(left.name.cmp(&right.name))
    });
    Ok(capabilities)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_capability(root: &Path, slug: &str, declaration: &str) {
        let dir = root.join(CAPABILITIES_DIR_NAME).join(slug);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(CAPABILITY_FILE_NAME), declaration).unwrap();
    }

    #[test]
    fn loads_the_rules_a_capability_declares() {
        let temp = tempfile::tempdir().unwrap();
        write_capability(
            temp.path(),
            "tanstack-router",
            r#"{ "key": "tanstack-router", "name": "TanStack Router", "order": 1,
                 "projectTypes": ["typescript:web"], "languages": ["typescript"],
                 "conflictsWith": ["tanstack-start"],
                 "tokens": { "STACK_LINE": "TanStack Router" } }"#,
        );

        let dir = temp.path().join(CAPABILITIES_DIR_NAME).join("tanstack-router");
        let capability = Capability::load(&dir).unwrap();

        assert_eq!(capability.key, "tanstack-router");
        assert_eq!(capability.slug, "tanstack-router");
        assert_eq!(capability.project_types, vec!["typescript:web"]);
        assert_eq!(capability.languages, vec!["typescript"]);
        assert_eq!(capability.conflicts_with, vec!["tanstack-start"]);
        assert_eq!(capability.tokens.get("STACK_LINE").unwrap(), "TanStack Router");
    }

    #[test]
    fn a_capability_that_restricts_nothing_restricts_nothing() {
        let temp = tempfile::tempdir().unwrap();
        write_capability(temp.path(), "plain", r#"{ "key": "plain", "name": "Plain" }"#);

        let dir = temp.path().join(CAPABILITIES_DIR_NAME).join("plain");
        let capability = Capability::load(&dir).unwrap();

        assert!(capability.project_types.is_empty());
        assert!(capability.languages.is_empty());
        assert!(capability.conflicts_with.is_empty());
    }

    #[test]
    fn discovers_and_sorts_capabilities_by_order() {
        let temp = tempfile::tempdir().unwrap();
        write_capability(
            temp.path(),
            "tanstack-start",
            r#"{ "key": "tanstack-start", "name": "TanStack Start", "order": 2 }"#,
        );
        write_capability(
            temp.path(),
            "tanstack-router",
            r#"{ "key": "tanstack-router", "name": "TanStack Router", "order": 1 }"#,
        );

        let capabilities = discover(temp.path()).unwrap();

        let keys: Vec<&str> = capabilities.iter().map(|one| one.key.as_str()).collect();
        assert_eq!(keys, vec!["tanstack-router", "tanstack-start"]);
    }

    #[test]
    fn a_repository_with_no_capabilities_discovers_none_and_is_not_an_error() {
        let temp = tempfile::tempdir().unwrap();
        assert!(discover(temp.path()).unwrap().is_empty());
    }
}
