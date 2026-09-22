//! Guards the invariants of `skills-manifest.json`, the file that decides
//! which agent skills a generated project receives.
//!
//! The manifest is data the scaffolder hands to `npx skills add` verbatim, so
//! a malformed or duplicated entry only shows up as a failed install inside a
//! user's brand-new project. These tests fail the build instead: every spec is
//! well formed, no spec is listed twice, `global` is present and non-empty,
//! and every capability a stack module declares exists here. The manifest has
//! nothing to do with this repository's own `skills-lock.json`, so nothing
//! here compares the two.

use std::collections::BTreeSet;
use std::path::PathBuf;

use quickstarter::catalog;
use quickstarter::skills::manifest::{self, GLOBAL_CAPABILITY, MANIFEST_FILE_NAME};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn manifest_json() -> String {
    std::fs::read_to_string(repo_root().join(MANIFEST_FILE_NAME)).expect("reading the manifest")
}

/// The manifest as capability name to its specs, in the order authored.
fn capabilities() -> Vec<(String, Vec<String>)> {
    let document: serde_json::Value = serde_json::from_str(&manifest_json()).unwrap();
    document["capabilities"]
        .as_object()
        .expect("the manifest has a 'capabilities' object")
        .iter()
        .map(|(name, specs)| {
            let specs = specs
                .as_array()
                .unwrap_or_else(|| panic!("capability '{name}' is not a list"))
                .iter()
                .map(|spec| spec.as_str().expect("a spec is a string").to_string())
                .collect();
            (name.clone(), specs)
        })
        .collect()
}

/// Reports why `spec` is not something `npx skills add` can resolve, or `None`
/// when it is well formed: `owner/repo`, optionally followed by the path to
/// the skill inside that repository.
fn malformation(spec: &str) -> Option<&'static str> {
    if spec.chars().any(char::is_whitespace) {
        return Some("contains whitespace");
    }
    if spec.starts_with('/') || spec.ends_with('/') {
        return Some("has a leading or trailing slash");
    }
    if spec.ends_with(".git") {
        return Some("ends with .git");
    }
    let segments: Vec<&str> = spec.split('/').collect();
    if segments.len() < 2 {
        return Some("is not owner/repo");
    }
    if segments.iter().any(|segment| segment.is_empty()) {
        return Some("has an empty path segment");
    }
    None
}

#[test]
fn every_spec_is_a_well_formed_source_spec() {
    for (capability, specs) in capabilities() {
        for spec in specs {
            assert!(
                malformation(&spec).is_none(),
                "'{spec}' in '{capability}' {}",
                malformation(&spec).unwrap()
            );
        }
    }
}

#[test]
fn the_shape_check_rejects_a_malformed_spec() {
    // Keeps `every_spec_is_a_well_formed_source_spec` from passing vacuously.
    for bad in [
        "obra superpowers",
        "/owner/repo",
        "owner/repo/",
        "owner/repo.git",
        "owner",
        "owner//repo",
    ] {
        assert!(malformation(bad).is_some(), "'{bad}' should be rejected");
    }
    assert!(malformation("owner/repo").is_none());
    assert!(malformation("owner/repo/skills/one").is_none());
}

#[test]
fn no_spec_is_listed_twice_in_one_capability_or_in_two() {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (capability, specs) in capabilities() {
        for spec in specs {
            assert!(
                seen.insert(spec.clone()),
                "'{spec}' is listed twice ('{capability}')"
            );
        }
    }
}

#[test]
fn the_global_capability_exists_and_is_not_empty() {
    let global = capabilities()
        .into_iter()
        .find(|(name, _)| name == GLOBAL_CAPABILITY)
        .map(|(_, specs)| specs)
        .unwrap_or_else(|| panic!("the manifest declares no '{GLOBAL_CAPABILITY}' capability"));
    assert!(!global.is_empty(), "'{GLOBAL_CAPABILITY}' installs nothing");
}

#[test]
fn every_capability_a_module_declares_exists_in_the_manifest() {
    let root = repo_root();
    let manifest = manifest::parse(&manifest_json()).unwrap();
    let declared: BTreeSet<&str> = manifest.capability_names().collect();

    for module in catalog::discover_modules(&root.join("templates")).unwrap() {
        for capability in &module.capabilities {
            assert!(
                declared.contains(capability.as_str()),
                "module '{}' declares '{capability}', which the manifest does not: {declared:?}",
                module.key
            );
        }
        // The same check the scaffolder itself makes, so a module that selects
        // nothing installable fails here rather than in a user's project.
        manifest.specs_for(&module.capabilities).unwrap();
    }
}
