//! Seeding the generated project's `skills-lock.json`.
//!
//! Generated projects do not vendor their agent skills. They track a
//! `skills-lock.json` and let `npx skills` materialize `.agents/skills` from
//! it on `pnpm install` (see `scripts/skills` in the base layer). This module
//! writes that lock by intersecting quickstarter's own `skills-lock.json` with
//! the `produced` bucket of `skills-manifest.json`, so a new project starts
//! with exactly the curated set and nothing Rust-related.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Map, Value};

/// Skills that `npx skills` does not manage. `impeccable` ships its own
/// `impeccable` CLI for installation and updates, so `scripts/skills` drives
/// it directly and it must never enter the generated lock.
pub const CLI_MANAGED_SKILLS: &[&str] = &["impeccable"];

/// File that classifies every skill as `quickstarterDev`, `produced`, or both.
pub const MANIFEST_FILE_NAME: &str = "skills-manifest.json";

/// File `npx skills` reads and writes to track installed skills.
pub const LOCK_FILE_NAME: &str = "skills-lock.json";

#[derive(Deserialize)]
struct SkillsManifest {
    produced: Vec<String>,
}

/// Writes `dest/skills-lock.json` holding the produced subset of the lock in
/// `template_root`.
///
/// A template repository with no skills manifest writes no lock at all, which
/// keeps the composition engine usable with minimal fixture templates. Once a
/// manifest exists it is authoritative: a missing lock, or a produced skill
/// absent from it, is an error rather than a silently smaller project.
pub fn write_produced_lock(template_root: &Path, dest: &Path) -> Result<()> {
    let manifest_path = template_root.join(MANIFEST_FILE_NAME);
    if !manifest_path.exists() {
        return Ok(());
    }

    let manifest_json = read_file(&manifest_path)?;
    let lock_json = read_file(&template_root.join(LOCK_FILE_NAME))?;
    let produced_lock = build_produced_lock(&manifest_json, &lock_json)?;

    let lock_path = dest.join(LOCK_FILE_NAME);
    std::fs::write(&lock_path, produced_lock)
        .with_context(|| format!("writing {}", lock_path.display()))
}

/// Builds the produced project's lock document from quickstarter's manifest
/// and lock. Entries are copied verbatim so the generated project pins the
/// same sources and hashes, and are emitted in name order so regenerating a
/// project twice produces byte-identical output.
pub fn build_produced_lock(manifest_json: &str, lock_json: &str) -> Result<String> {
    let manifest: SkillsManifest =
        serde_json::from_str(manifest_json).context("parsing skills-manifest.json")?;
    let lock: Value = serde_json::from_str(lock_json).context("parsing skills-lock.json")?;

    let locked_skills = lock
        .get("skills")
        .and_then(Value::as_object)
        .context("skills-lock.json has no 'skills' object")?;

    let wanted_names: BTreeSet<&str> = manifest
        .produced
        .iter()
        .map(String::as_str)
        .filter(|name| !CLI_MANAGED_SKILLS.contains(name))
        .collect();

    let mut produced_skills = Map::new();
    let mut unlocked_names: Vec<&str> = Vec::new();
    for name in wanted_names {
        match locked_skills.get(name) {
            Some(entry) => {
                produced_skills.insert(name.to_string(), entry.clone());
            }
            None => unlocked_names.push(name),
        }
    }

    if !unlocked_names.is_empty() {
        bail!(
            "skills-manifest.json lists produced skill(s) that are missing from \
             skills-lock.json: {}. Install them with `skills add` or remove them \
             from the manifest.",
            unlocked_names.join(", ")
        );
    }

    let version = lock.get("version").cloned().unwrap_or(Value::from(1));
    let document = serde_json::json!({
        "version": version,
        "skills": produced_skills,
    });
    Ok(format!("{}\n", serde_json::to_string_pretty(&document)?))
}

fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
        "quickstarterDev": ["rust-skills"],
        "produced": ["test-driven-development", "impeccable", "brainstorming"]
    }"#;

    const LOCK: &str = r#"{
        "version": 1,
        "skills": {
            "rust-skills": { "source": "actionbook/rust-skills" },
            "brainstorming": { "source": "obra/superpowers", "computedHash": "abc" },
            "impeccable": { "source": "pbakaus/impeccable" },
            "test-driven-development": { "source": "obra/superpowers" }
        }
    }"#;

    fn produced_names(lock_json: &str) -> Vec<String> {
        let lock: Value = serde_json::from_str(lock_json).unwrap();
        lock["skills"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }

    #[test]
    fn keeps_only_produced_skills_in_name_order() {
        let produced = build_produced_lock(MANIFEST, LOCK).unwrap();
        assert_eq!(
            produced_names(&produced),
            vec!["brainstorming", "test-driven-development"]
        );
    }

    #[test]
    fn excludes_cli_managed_skills() {
        let produced = build_produced_lock(MANIFEST, LOCK).unwrap();
        assert!(!produced.contains("impeccable"));
    }

    #[test]
    fn copies_lock_entries_verbatim() {
        let produced = build_produced_lock(MANIFEST, LOCK).unwrap();
        let lock: Value = serde_json::from_str(&produced).unwrap();
        assert_eq!(lock["skills"]["brainstorming"]["computedHash"], "abc");
        assert_eq!(lock["version"], 1);
    }

    #[test]
    fn ends_with_a_newline() {
        assert!(build_produced_lock(MANIFEST, LOCK).unwrap().ends_with("}\n"));
    }

    #[test]
    fn fails_when_a_produced_skill_is_not_locked() {
        let lock = r#"{ "version": 1, "skills": { "brainstorming": {} } }"#;
        let error = build_produced_lock(MANIFEST, lock).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("test-driven-development"), "{message}");
        assert!(!message.contains("impeccable"), "{message}");
    }

    #[test]
    fn writes_nothing_when_the_template_has_no_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let template_root = temp.path().join("template");
        let dest = temp.path().join("app");
        std::fs::create_dir_all(&template_root).unwrap();
        std::fs::create_dir_all(&dest).unwrap();

        write_produced_lock(&template_root, &dest).unwrap();

        assert!(!dest.join(LOCK_FILE_NAME).exists());
    }

    #[test]
    fn writes_the_lock_when_the_template_has_a_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let template_root = temp.path().join("template");
        let dest = temp.path().join("app");
        std::fs::create_dir_all(&template_root).unwrap();
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(template_root.join(MANIFEST_FILE_NAME), MANIFEST).unwrap();
        std::fs::write(template_root.join(LOCK_FILE_NAME), LOCK).unwrap();

        write_produced_lock(&template_root, &dest).unwrap();

        let written = std::fs::read_to_string(dest.join(LOCK_FILE_NAME)).unwrap();
        assert_eq!(
            produced_names(&written),
            vec!["brainstorming", "test-driven-development"]
        );
    }
}
