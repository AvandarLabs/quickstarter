//! Reading `skills-manifest.json`: which skills each tag selects.
//!
//! The manifest is keyed by tag, in three sections: `global` for the skills
//! every generated project gets, `projectTypes` for the ones that come with a
//! language and its build system, and `capabilities` for the ones a library
//! brings. Selection is additive: a project receives `global`, plus its
//! project type's list, plus the list of every capability it chose.
//!
//! Which tags exist, and which of them can be combined, is declared in
//! `templates/` rather than here. This file only says what each tag installs,
//! and `cli/tests/skills_manifest_test.rs` checks that the two agree.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

/// Name of the manifest file, read from the root of the template repository.
pub const MANIFEST_FILE_NAME: &str = "skills-manifest.json";

/// The section every generated project receives, whatever its tags.
pub const GLOBAL_SECTION: &str = "global";

/// A tag section, mapping each tag to the specs it contributes.
type Section = BTreeMap<String, Vec<String>>;

/// The manifest: the global skills plus the skills each tag contributes, in
/// the order the manifest lists them.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsManifest {
    /// The skills every generated project receives.
    #[serde(default)]
    global: Vec<String>,
    /// Project type key to its specs.
    #[serde(default)]
    project_types: Section,
    /// Capability key to its specs. Any other key in the document (the
    /// `comment` block, for example) is ignored.
    #[serde(default)]
    capabilities: Section,
}

/// Parses a manifest document.
pub fn parse(json: &str) -> Result<SkillsManifest> {
    serde_json::from_str(json).with_context(|| format!("parsing {MANIFEST_FILE_NAME}"))
}

/// Loads the manifest at the root of a template repository.
///
/// Returns `Ok(None)` when the repository ships no manifest, which is what
/// lets a minimal template repository compose without selecting any skill.
pub fn load(template_root: &Path) -> Result<Option<SkillsManifest>> {
    let path = template_root.join(MANIFEST_FILE_NAME);
    if !path.exists() {
        return Ok(None);
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    parse(&text).map(Some)
}

impl SkillsManifest {
    /// Every skill spec a project with these tags receives: the global list
    /// first, then the project type's, then each capability's in the order
    /// given.
    ///
    /// A spec listed under two tags is returned once, at its first position,
    /// so it is installed once. A tag the manifest does not declare is an
    /// error naming it: it would otherwise silently install nothing.
    pub fn specs_for(&self, project_type: &str, capabilities: &[String]) -> Result<Vec<String>> {
        let mut specs = Vec::new();
        let mut seen = BTreeSet::new();
        extend(&self.global, &mut specs, &mut seen);
        extend(declared(&self.project_types, project_type, "project type")?, &mut specs, &mut seen);
        for capability in capabilities {
            extend(declared(&self.capabilities, capability, "capability")?, &mut specs, &mut seen);
        }
        Ok(specs)
    }

    /// The skills every generated project receives.
    pub fn global_specs(&self) -> &[String] {
        &self.global
    }

    /// The project type keys the manifest declares, in key order.
    pub fn project_type_names(&self) -> impl Iterator<Item = &str> {
        self.project_types.keys().map(String::as_str)
    }

    /// The capability keys the manifest declares, in key order.
    pub fn capability_names(&self) -> impl Iterator<Item = &str> {
        self.capabilities.keys().map(String::as_str)
    }
}

/// One tag's specs, or an error naming the tag and the ones that do exist.
fn declared<'manifest>(
    section: &'manifest Section,
    tag: &str,
    kind: &str,
) -> Result<&'manifest [String]> {
    section.get(tag).map(Vec::as_slice).ok_or_else(|| {
        anyhow!(
            "{MANIFEST_FILE_NAME} has no {kind} called '{tag}'. It declares: {}.",
            section.keys().cloned().collect::<Vec<String>>().join(", ")
        )
    })
}

/// Appends the specs not already selected, so each is installed once.
fn extend(declared: &[String], specs: &mut Vec<String>, seen: &mut BTreeSet<String>) {
    for spec in declared {
        if seen.insert(spec.clone()) {
            specs.push(spec.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
        "comment": ["ignored"],
        "global": ["obra/superpowers/skills/brainstorming", "pbakaus/impeccable"],
        "projectTypes": {
            "typescript:web": ["mcollina/skills/skills/typescript-magician", "pbakaus/impeccable"],
            "rust:cli": ["leonardomso/rust-skills"]
        },
        "capabilities": {
            "tanstack-router": ["owner/repo/skills/router"],
            "tanstack-start": []
        }
    }"#;

    fn keys(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    #[test]
    fn parses_every_section_and_ignores_the_comment() {
        let manifest = parse(MANIFEST).unwrap();

        assert_eq!(manifest.global_specs().len(), 2);
        assert_eq!(
            manifest.project_type_names().collect::<Vec<&str>>(),
            vec!["rust:cli", "typescript:web"]
        );
        assert_eq!(
            manifest.capability_names().collect::<Vec<&str>>(),
            vec!["tanstack-router", "tanstack-start"]
        );
    }

    #[test]
    fn rejects_a_document_that_is_not_a_manifest() {
        assert!(parse("not json").is_err());
    }

    #[test]
    fn specs_come_global_first_then_the_project_type_then_the_capabilities() {
        let manifest = parse(MANIFEST).unwrap();

        let specs = manifest
            .specs_for("typescript:web", &keys(&["tanstack-router"]))
            .unwrap();

        assert_eq!(
            specs,
            vec![
                "obra/superpowers/skills/brainstorming",
                "pbakaus/impeccable",
                "mcollina/skills/skills/typescript-magician",
                "owner/repo/skills/router",
            ]
        );
    }

    #[test]
    fn a_spec_two_tags_share_is_installed_once() {
        let manifest = parse(MANIFEST).unwrap();

        let specs = manifest.specs_for("typescript:web", &[]).unwrap();

        let impeccable = specs.iter().filter(|spec| *spec == "pbakaus/impeccable").count();
        assert_eq!(impeccable, 1);
    }

    #[test]
    fn a_project_type_needs_no_capability_to_select_its_skills() {
        let manifest = parse(MANIFEST).unwrap();

        let specs = manifest.specs_for("rust:cli", &[]).unwrap();

        assert_eq!(
            specs,
            vec![
                "obra/superpowers/skills/brainstorming",
                "pbakaus/impeccable",
                "leonardomso/rust-skills",
            ]
        );
    }

    #[test]
    fn a_capability_that_brings_no_skill_changes_nothing() {
        let manifest = parse(MANIFEST).unwrap();

        let bare = manifest.specs_for("typescript:web", &[]).unwrap();
        let with_start = manifest
            .specs_for("typescript:web", &keys(&["tanstack-start"]))
            .unwrap();

        assert_eq!(bare, with_start);
    }

    #[test]
    fn an_unknown_project_type_is_named() {
        let manifest = parse(MANIFEST).unwrap();

        let error = manifest.specs_for("python:web", &[]).unwrap_err().to_string();

        assert!(error.contains("project type called 'python:web'"), "{error}");
        assert!(error.contains("rust:cli"), "{error}");
    }

    #[test]
    fn an_unknown_capability_is_named() {
        let manifest = parse(MANIFEST).unwrap();

        let error = manifest
            .specs_for("typescript:web", &keys(&["svelte"]))
            .unwrap_err()
            .to_string();

        assert!(error.contains("capability called 'svelte'"), "{error}");
    }

    #[test]
    fn load_reads_the_manifest_from_the_template_root() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join(MANIFEST_FILE_NAME), MANIFEST).unwrap();

        let manifest = load(temp.path()).unwrap().expect("a manifest");

        assert_eq!(manifest.project_type_names().count(), 2);
    }

    #[test]
    fn load_returns_none_when_the_template_has_no_manifest() {
        let temp = tempfile::tempdir().unwrap();
        assert!(load(temp.path()).unwrap().is_none());
    }
}
