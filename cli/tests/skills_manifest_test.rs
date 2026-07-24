//! Enforces that `skills-manifest.json` deterministically classifies every
//! installed skill. This is what makes the "which skills go to produced repos
//! vs. stay quickstarter-only vs. both" split deterministic: the test fails if
//! a skill is added or removed without updating the manifest, or if the
//! manifest names a skill that is not installed. A skill may appear in both
//! buckets, which is how "both" is expressed.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::Deserialize;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[derive(Deserialize)]
struct Manifest {
    #[serde(rename = "quickstarterDev")]
    quickstarter_dev: Vec<String>,
    produced: Vec<String>,
}

fn installed_skills(root: &std::path::Path) -> BTreeSet<String> {
    std::fs::read_dir(root.join(".agents/skills"))
        .expect("reading .agents/skills")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry.file_type().ok()?.is_dir().then(|| {
                entry.file_name().to_string_lossy().into_owned()
            })
        })
        .collect()
}

#[test]
fn manifest_classifies_every_installed_skill() {
    let root = repo_root();
    let manifest: Manifest = serde_json::from_str(
        &std::fs::read_to_string(root.join("skills-manifest.json")).unwrap(),
    )
    .unwrap();

    let dev: BTreeSet<String> = manifest.quickstarter_dev.iter().cloned().collect();
    let produced: BTreeSet<String> = manifest.produced.iter().cloned().collect();

    // No duplicate entries within a single bucket (overlap across buckets is
    // allowed and means "both").
    assert_eq!(dev.len(), manifest.quickstarter_dev.len(), "duplicate in quickstarterDev");
    assert_eq!(produced.len(), manifest.produced.len(), "duplicate in produced");

    // Every installed skill must be classified in at least one bucket, and the
    // manifest must not name a skill that is not installed.
    let classified: BTreeSet<String> = dev.union(&produced).cloned().collect();
    let installed = installed_skills(&root);

    let unclassified: Vec<&String> = installed.difference(&classified).collect();
    assert!(unclassified.is_empty(), "installed but unclassified: {unclassified:?}");

    let phantom: Vec<&String> = classified.difference(&installed).collect();
    assert!(phantom.is_empty(), "in manifest but not installed: {phantom:?}");
}

#[test]
fn no_rust_related_skill_is_in_the_produced_bucket() {
    let root = repo_root();
    let manifest: Manifest = serde_json::from_str(
        &std::fs::read_to_string(root.join("skills-manifest.json")).unwrap(),
    )
    .unwrap();

    // Rust is only relevant to the CLI; generated apps must never receive a
    // Rust skill. `coding-guidelines` is Rust code style despite its name.
    let rust_related: Vec<&String> = manifest
        .produced
        .iter()
        .filter(|name| name.starts_with("rust-") || *name == "coding-guidelines")
        .collect();
    assert!(rust_related.is_empty(), "rust-related skill(s) in produced: {rust_related:?}");
}
