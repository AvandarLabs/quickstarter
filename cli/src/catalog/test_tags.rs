//! Tag values for the unit tests.
//!
//! Compatibility, choice groups, selection, composition, and resolution all
//! need tags to reason about, and building them from JSON in each module would
//! bury the behavior under setup. These constructors give a plain tag that
//! restricts nothing, which each test then narrows through the public fields.

use crate::catalog::{Capability, ProjectType};
use crate::compose::tokens::Tokens;

/// A project type with the given key and language, and no tokens.
pub fn project_type(key: &str, language: &str) -> ProjectType {
    ProjectType {
        key: key.to_string(),
        name: key.to_string(),
        description: String::new(),
        language: language.to_string(),
        order: 0,
        tokens: Tokens::new(),
        slug: key.replace(':', "-"),
    }
}

/// A capability with the given key that fits every project type and conflicts
/// with nothing.
pub fn capability(key: &str) -> Capability {
    Capability {
        key: key.to_string(),
        name: key.to_string(),
        description: String::new(),
        order: 0,
        project_types: Vec::new(),
        languages: Vec::new(),
        conflicts_with: Vec::new(),
        tokens: Tokens::new(),
        slug: key.to_string(),
    }
}

/// Borrowed references to every capability in `capabilities`, which is the
/// shape the compatibility and group functions take.
pub fn all(capabilities: &[Capability]) -> Vec<&Capability> {
    capabilities.iter().collect()
}

/// The given names as owned keys, which is the shape a selection takes.
pub fn keys(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}
