//! Guards the invariants of `skills-manifest.json`, the file that decides
//! which agent skills a generated project receives.
//!
//! The manifest is data the scaffolder hands to `npx skills add` verbatim, so
//! a malformed or duplicated entry only shows up as a failed install inside a
//! user's brand-new project. These tests fail the build instead: every spec is
//! well formed, no spec is listed twice under one tag or repeated from the
//! global list, `global` is present and non-empty, and the tags the manifest
//! knows about are exactly the tags `templates/` declares. The manifest has
//! nothing to do with this repository's own `skills-lock.json`, so nothing
//! here compares the two.

use std::collections::BTreeSet;
use std::path::PathBuf;

use quickstarter::catalog;
use quickstarter::skills::manifest::{self, GLOBAL_SECTION, MANIFEST_FILE_NAME};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn manifest_json() -> serde_json::Value {
    let text = std::fs::read_to_string(repo_root().join(MANIFEST_FILE_NAME))
        .expect("reading the manifest");
    serde_json::from_str(&text).expect("the manifest is JSON")
}

/// The specs of the `global` section, in the order authored.
fn global_specs() -> Vec<String> {
    specs_of(&manifest_json()[GLOBAL_SECTION], GLOBAL_SECTION)
}

/// Every tag section entry as (tag, specs), in the order authored, taking the
/// project types and the capabilities together: both are tags, and the shape
/// rules are the same for both.
fn tag_specs() -> Vec<(String, Vec<String>)> {
    let document = manifest_json();
    let mut entries = vec![(GLOBAL_SECTION.to_string(), global_specs())];
    for section in ["projectTypes", "capabilities"] {
        let tags = document[section]
            .as_object()
            .unwrap_or_else(|| panic!("the manifest has a '{section}' object"));
        for (tag, specs) in tags {
            entries.push((tag.clone(), specs_of(specs, tag)));
        }
    }
    entries
}

fn specs_of(value: &serde_json::Value, tag: &str) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("'{tag}' is not a list of specs"))
        .iter()
        .map(|spec| spec.as_str().expect("a spec is a string").to_string())
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
    for (tag, specs) in tag_specs() {
        for spec in specs {
            assert!(
                malformation(&spec).is_none(),
                "'{spec}' in '{tag}' {}",
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
fn no_tag_lists_a_spec_twice() {
    for (tag, specs) in tag_specs() {
        let mut seen: BTreeSet<&String> = BTreeSet::new();
        for spec in &specs {
            assert!(seen.insert(spec), "'{tag}' lists '{spec}' twice");
        }
    }
}

#[test]
fn no_tag_repeats_a_skill_the_global_list_already_installs() {
    // Two project types may legitimately share a skill; repeating one that
    // every project already gets is only noise.
    let global: BTreeSet<String> = global_specs().into_iter().collect();
    for (tag, specs) in tag_specs() {
        if tag == GLOBAL_SECTION {
            continue;
        }
        for spec in specs {
            assert!(!global.contains(&spec), "'{tag}' repeats the global skill '{spec}'");
        }
    }
}

#[test]
fn the_global_section_exists_and_is_not_empty() {
    assert!(!global_specs().is_empty(), "'{GLOBAL_SECTION}' installs nothing");
}

#[test]
fn every_tag_the_templates_declare_has_an_entry() {
    let root = repo_root();
    let manifest = manifest::load(&root).unwrap().expect("the real manifest");
    let catalog = catalog::discover(&root.join("templates")).unwrap();

    let project_types: BTreeSet<&str> = manifest.project_type_names().collect();
    for project_type in &catalog.project_types {
        assert!(
            project_types.contains(project_type.key.as_str()),
            "the templates declare the project type '{}', which the manifest does not: {:?}",
            project_type.key,
            project_types
        );
        // The same check the scaffolder itself makes, so a tag that selects
        // nothing installable fails here rather than in a user's project.
        manifest.specs_for(&project_type.key, &[]).unwrap();
    }

    let capabilities: BTreeSet<&str> = manifest.capability_names().collect();
    for capability in &catalog.capabilities {
        assert!(
            capabilities.contains(capability.key.as_str()),
            "the templates declare the capability '{}', which the manifest does not: {:?}",
            capability.key,
            capabilities
        );
    }
}

#[test]
fn every_tag_the_manifest_names_is_one_the_templates_declare() {
    let root = repo_root();
    let manifest = manifest::load(&root).unwrap().expect("the real manifest");
    let catalog = catalog::discover(&root.join("templates")).unwrap();

    let project_types: BTreeSet<&str> =
        catalog.project_types.iter().map(|one| one.key.as_str()).collect();
    for name in manifest.project_type_names() {
        assert!(
            project_types.contains(name),
            "the manifest names the project type '{name}', which no template declares"
        );
    }

    let capabilities: BTreeSet<&str> =
        catalog.capabilities.iter().map(|one| one.key.as_str()).collect();
    for name in manifest.capability_names() {
        assert!(
            capabilities.contains(name),
            "the manifest names the capability '{name}', which no template declares"
        );
    }
}
