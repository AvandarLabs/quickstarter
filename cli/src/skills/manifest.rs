//! Reading `skills-manifest.json`: which skills each capability selects.
//!
//! The manifest maps a capability tag to the skill source specs a project with
//! that capability receives. Selection is additive: a project always gets the
//! `global` list, plus the list of every capability its stack module declares.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// Name of the manifest file, read from the root of the template repository.
pub const MANIFEST_FILE_NAME: &str = "skills-manifest.json";

/// The capability every generated project has, whatever stack it chose.
pub const GLOBAL_CAPABILITY: &str = "global";

/// The manifest: each capability tag mapped to the skill source specs it
/// contributes, in the order the manifest lists them.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillsManifest {
    /// Capability name to its specs. Any other key in the document (the
    /// `comment` block, for example) is ignored.
    capabilities: BTreeMap<String, Vec<String>>,
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
    /// Every skill spec a project with `capabilities` receives: the `global`
    /// list first, then each capability's list in the order given.
    ///
    /// A spec listed under two capabilities is returned once, at its first
    /// position, so it is installed once.
    pub fn specs_for(&self, capabilities: &[String]) -> Result<Vec<String>> {
        let mut specs = Vec::new();
        let mut seen = BTreeSet::new();
        self.extend_with(GLOBAL_CAPABILITY, &mut specs, &mut seen)?;
        for capability in capabilities {
            self.extend_with(capability, &mut specs, &mut seen)?;
        }
        Ok(specs)
    }

    /// The names of every capability the manifest declares, in name order.
    pub fn capability_names(&self) -> impl Iterator<Item = &str> {
        self.capabilities.keys().map(String::as_str)
    }

    /// Appends one capability's specs, skipping the ones already selected.
    /// An unknown capability is an error: it would otherwise silently install
    /// nothing at all.
    fn extend_with(
        &self,
        capability: &str,
        specs: &mut Vec<String>,
        seen: &mut BTreeSet<String>,
    ) -> Result<()> {
        let Some(declared) = self.capabilities.get(capability) else {
            bail!(
                "{MANIFEST_FILE_NAME} has no capability called '{capability}'. \
                 It declares: {}.",
                self.capability_names().collect::<Vec<&str>>().join(", ")
            );
        };
        for spec in declared {
            if seen.insert(spec.clone()) {
                specs.push(spec.clone());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
        "comment": ["ignored"],
        "capabilities": {
            "global": ["obra/superpowers/skills/brainstorming", "pbakaus/impeccable"],
            "typescript": ["mcollina/skills/skills/typescript-magician", "pbakaus/impeccable"],
            "rust": ["leonardomso/rust-skills"]
        }
    }"#;

    fn capabilities(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    #[test]
    fn parses_every_capability_and_ignores_the_comment() {
        let manifest = parse(MANIFEST).unwrap();
        let names: Vec<&str> = manifest.capability_names().collect();
        assert_eq!(names, vec!["global", "rust", "typescript"]);
    }

    #[test]
    fn rejects_a_document_that_is_not_a_manifest() {
        assert!(parse("not json").is_err());
    }

    #[test]
    fn specs_for_puts_the_global_skills_first_in_manifest_order() {
        let manifest = parse(MANIFEST).unwrap();
        let specs = manifest.specs_for(&capabilities(&["typescript"])).unwrap();
        assert_eq!(
            specs,
            vec![
                "obra/superpowers/skills/brainstorming",
                "pbakaus/impeccable",
                "mcollina/skills/skills/typescript-magician",
            ]
        );
    }

    #[test]
    fn specs_for_installs_a_shared_spec_once() {
        let manifest = parse(MANIFEST).unwrap();
        let specs = manifest.specs_for(&capabilities(&["typescript"])).unwrap();
        let impeccable = specs
            .iter()
            .filter(|spec| *spec == "pbakaus/impeccable")
            .count();
        assert_eq!(impeccable, 1);
    }

    #[test]
    fn specs_for_needs_no_capabilities_to_return_the_global_skills() {
        let manifest = parse(MANIFEST).unwrap();
        assert_eq!(manifest.specs_for(&[]).unwrap().len(), 2);
    }

    #[test]
    fn specs_for_names_an_unknown_capability() {
        let manifest = parse(MANIFEST).unwrap();
        let error = manifest
            .specs_for(&capabilities(&["svelte"]))
            .unwrap_err()
            .to_string();
        assert!(error.contains("svelte"), "{error}");
    }

    #[test]
    fn load_reads_the_manifest_from_the_template_root() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join(MANIFEST_FILE_NAME), MANIFEST).unwrap();

        let manifest = load(temp.path()).unwrap().expect("a manifest");

        assert_eq!(manifest.capability_names().count(), 3);
    }

    #[test]
    fn load_returns_none_when_the_template_has_no_manifest() {
        let temp = tempfile::tempdir().unwrap();
        assert!(load(temp.path()).unwrap().is_none());
    }
}
