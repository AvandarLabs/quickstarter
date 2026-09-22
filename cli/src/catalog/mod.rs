//! The tag catalog: what a generated project can be built from.
//!
//! A project carries tags of two types. It has exactly one **project type**
//! (`typescript:web`, `rust:cli`), which decides its build system and the
//! files it starts from, and any number of **capabilities**
//! (`tanstack-router`), each a library or framework layered on top. A
//! capability says which project types and languages it fits and which
//! capabilities it excludes, so the valid combinations are data rather than
//! code.
//!
//! Both are discovered from the cloned template repository at runtime rather
//! than hardcoded in the binary, so adding either is a matter of adding a
//! folder under `templates/`: an older binary picks it up because it always
//! clones the latest repo.

pub mod capability;
pub mod compatibility;
pub mod discovery;
pub mod groups;
pub mod project_type;
pub mod selection;
#[cfg(test)]
pub mod test_tags;

use std::path::Path;

use anyhow::Result;

pub use capability::Capability;
pub use groups::ChoiceGroup;
pub use project_type::ProjectType;
pub use selection::Selection;

/// Every tag a template repository offers.
pub struct Catalog {
    /// The project types, sorted by `order` then `name`.
    pub project_types: Vec<ProjectType>,
    /// The capabilities, sorted by `order` then `name`, whatever project type
    /// each fits.
    pub capabilities: Vec<Capability>,
}

/// Discovers every tag under `templates_root`.
///
/// A repository with no project type is an error, because nothing can be built
/// from it; one with no capabilities is fine.
pub fn discover(templates_root: &Path) -> Result<Catalog> {
    Ok(Catalog {
        project_types: project_type::discover(templates_root)?,
        capabilities: capability::discover(templates_root)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_both_kinds_of_tag() {
        let temp = tempfile::tempdir().unwrap();
        let project_type_dir = temp.path().join("project-types/rust-cli");
        std::fs::create_dir_all(&project_type_dir).unwrap();
        std::fs::write(
            project_type_dir.join("project-type.json"),
            r#"{ "key": "rust:cli", "name": "Rust CLI", "language": "rust" }"#,
        )
        .unwrap();
        let capability_dir = temp.path().join("capabilities/clap");
        std::fs::create_dir_all(&capability_dir).unwrap();
        std::fs::write(
            capability_dir.join("capability.json"),
            r#"{ "key": "clap", "name": "clap" }"#,
        )
        .unwrap();

        let catalog = discover(temp.path()).unwrap();

        assert_eq!(catalog.project_types.len(), 1);
        assert_eq!(catalog.capabilities.len(), 1);
    }

    #[test]
    fn a_repository_with_no_project_type_is_malformed() {
        let temp = tempfile::tempdir().unwrap();
        assert!(discover(temp.path()).is_err());
    }
}
