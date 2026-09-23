//! Guards the invariants of the `{{TOKEN}}`s the real templates declare.
//!
//! A sibling of `skills_manifest_test.rs`: that file guards what a tag
//! installs, this one guards what a tag substitutes. Both read the authored
//! `templates/` rather than a fixture, because the mistake each catches is an
//! authoring mistake.
//!
//! The invariant here is about **seams**. A shared file carries a line that is
//! nothing but a token, every project type defines that token empty, and a
//! capability that needs the line overrides it. The token map is flat
//! (`ComposePlan::tokens`), so two capabilities that a project could carry at
//! once and that both define one token would not both be applied: the later
//! one would silently win and the earlier one's line would never appear. That
//! is invisible in the generated project, which is why it fails here instead.

use std::path::PathBuf;

use quickstarter::catalog::{self, Capability, ProjectType, compatibility};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// The pairs of capabilities one project could carry at the same time: both
/// fit `project_type`, and neither excludes the other. Capabilities that
/// exclude each other are free to define the same token, because that is what
/// a choice group is for: exactly one of them is ever chosen.
fn combinable_pairs<'catalog>(
    capabilities: &[&'catalog Capability],
    project_type: &ProjectType,
) -> Vec<(&'catalog Capability, &'catalog Capability)> {
    let fitting: Vec<&Capability> = capabilities
        .iter()
        .copied()
        .filter(|one| compatibility::fits(one, project_type))
        .collect();

    let mut pairs = Vec::new();
    for (index, left) in fitting.iter().enumerate() {
        for right in &fitting[index + 1..] {
            if !compatibility::conflict(left, right) {
                pairs.push((*left, *right));
            }
        }
    }
    pairs
}

/// The token keys both capabilities define.
fn shared_tokens<'a>(left: &'a Capability, right: &Capability) -> Vec<&'a str> {
    left.tokens
        .keys()
        .filter(|key| right.tokens.contains_key(*key))
        .map(String::as_str)
        .collect()
}

/// Every collision the guard would report among `capabilities` for one project
/// type, as (left key, right key, the tokens both define).
fn collisions<'a>(
    capabilities: &[&'a Capability],
    project_type: &ProjectType,
) -> Vec<(&'a str, &'a str, Vec<&'a str>)> {
    combinable_pairs(capabilities, project_type)
        .into_iter()
        .map(|(left, right)| (left.key.as_str(), right.key.as_str(), shared_tokens(left, right)))
        .filter(|(_, _, shared)| !shared.is_empty())
        .collect()
}

/// A capability that fits everything, conflicts with nothing, and defines the
/// given tokens. Used to prove the check itself bites.
fn capability_defining(key: &str, tokens: &[&str]) -> Capability {
    Capability {
        key: key.to_string(),
        name: key.to_string(),
        description: String::new(),
        order: 0,
        project_types: Vec::new(),
        languages: Vec::new(),
        conflicts_with: Vec::new(),
        tokens: tokens
            .iter()
            .map(|token| ((*token).to_string(), "value".to_string()))
            .collect(),
        slug: key.to_string(),
    }
}

/// A project type with nothing but a key and a language, which is all the
/// pairing reads.
fn project_type_named(key: &str, language: &str) -> ProjectType {
    ProjectType {
        key: key.to_string(),
        name: key.to_string(),
        description: String::new(),
        language: language.to_string(),
        order: 0,
        tokens: Default::default(),
        slug: key.replace(':', "-"),
    }
}

#[test]
fn no_two_capabilities_of_one_project_type_define_the_same_token() {
    let catalog = catalog::discover(&repo_root().join("templates")).unwrap();
    let capabilities: Vec<&Capability> = catalog.capabilities.iter().collect();

    for project_type in &catalog.project_types {
        for (left, right) in combinable_pairs(&capabilities, project_type) {
            let shared = shared_tokens(left, right);
            assert!(
                shared.is_empty(),
                "'{}' and '{}' both fit '{}' and both define {shared:?}. \
                 One would silently overwrite the other, so at most one of them \
                 may own a token: move the value into the tag that knows the answer.",
                left.key,
                right.key,
                project_type.key
            );
        }
    }
}

#[test]
fn the_collision_check_catches_two_capabilities_that_share_a_seam() {
    // Keeps the guard above from passing vacuously. Today no two capabilities
    // can be chosen together at all, so the real templates would satisfy a
    // check that never compared anything.
    let first = capability_defining("first", &["EXTRA_MODULES", "DEV_URL"]);
    let second = capability_defining("second", &["EXTRA_MODULES"]);
    let third = capability_defining("third", &["EXTRA_RULES"]);
    let capabilities = vec![&first, &second, &third];

    let found = collisions(&capabilities, &project_type_named("rust:cli", "rust"));

    assert_eq!(found, vec![("first", "second", vec!["EXTRA_MODULES"])]);
}

#[test]
fn capabilities_that_exclude_each_other_may_define_the_same_seam() {
    // This is the real templates' shape: both routers define `STACK_LINE`, and
    // that is correct precisely because a project chooses exactly one of them.
    let mut first = capability_defining("first", &["STACK_LINE"]);
    first.conflicts_with = vec!["second".to_string()];
    let second = capability_defining("second", &["STACK_LINE"]);
    let capabilities = vec![&first, &second];

    let found = collisions(&capabilities, &project_type_named("typescript:web", "typescript"));

    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_capability_that_does_not_fit_the_project_type_is_not_paired_with_one_that_does() {
    let mut web_only = capability_defining("web-only", &["EXTRA_RULES"]);
    web_only.project_types = vec!["typescript:web".to_string()];
    let everywhere = capability_defining("everywhere", &["EXTRA_RULES"]);
    let capabilities = vec![&web_only, &everywhere];

    let rust = collisions(&capabilities, &project_type_named("rust:cli", "rust"));
    let web = collisions(&capabilities, &project_type_named("typescript:web", "typescript"));

    assert!(rust.is_empty(), "{rust:?}");
    assert_eq!(web, vec![("web-only", "everywhere", vec!["EXTRA_RULES"])]);
}
