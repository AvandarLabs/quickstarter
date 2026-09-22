//! The stack catalog: the set of modules a user can choose between.
//!
//! Modules are discovered from the cloned template repo at runtime rather than
//! hardcoded in the binary. Adding a new stack is therefore a matter of adding
//! a `templates/modules/<key>/` folder with a `module.json`; an older binary
//! picks it up automatically because it always clones the latest repo.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::compose::tokens::Tokens;

/// A single selectable stack module, parsed from its `module.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Module {
    /// The directory name under `templates/modules/`, e.g. `"router"`.
    pub key: String,
    /// Human-readable name shown in the selection prompt.
    pub name: String,
    /// One-line description shown alongside the name.
    #[serde(default)]
    pub description: String,
    /// Sort order for the selection prompt (ascending).
    #[serde(default)]
    pub order: i64,
    /// Capability tags this stack has, naming the skill lists a generated
    /// project receives from `skills-manifest.json` on top of the global one.
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Token values this module contributes to substitution.
    #[serde(default)]
    pub tokens: Tokens,
}

impl Module {
    /// Loads a module's metadata from `<module_dir>/module.json`.
    pub fn load(module_dir: &Path) -> Result<Module> {
        let manifest_path = module_dir.join("module.json");
        let text = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("reading {}", manifest_path.display()))?;
        let module: Module = serde_json::from_str(&text)
            .with_context(|| format!("parsing {}", manifest_path.display()))?;
        Ok(module)
    }
}

/// Discovers all modules under `<templates_root>/modules`, sorted by `order`
/// then `name`. Errors if the directory is missing or contains no modules.
pub fn discover_modules(templates_root: &Path) -> Result<Vec<Module>> {
    let modules_dir = templates_root.join("modules");
    let mut modules = Vec::new();

    for entry in std::fs::read_dir(&modules_dir)
        .with_context(|| format!("reading modules dir {}", modules_dir.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        if entry.path().join("module.json").exists() {
            modules.push(Module::load(&entry.path())?);
        }
    }

    if modules.is_empty() {
        anyhow::bail!(
            "no template modules found under {}. The template repository may be malformed.",
            modules_dir.display()
        );
    }

    modules.sort_by(|left, right| left.order.cmp(&right.order).then(left.name.cmp(&right.name)));
    Ok(modules)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_module(root: &Path, key: &str, name: &str, order: i64) {
        let dir = root.join("modules").join(key);
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = format!(
            r#"{{ "key": "{key}", "name": "{name}", "order": {order}, "tokens": {{ "X": "y" }} }}"#
        );
        std::fs::write(dir.join("module.json"), manifest).unwrap();
    }

    #[test]
    fn discovers_and_sorts_modules_by_order() {
        let temp = tempfile::tempdir().unwrap();
        write_module(temp.path(), "start", "Start", 2);
        write_module(temp.path(), "router", "Router", 1);

        let modules = discover_modules(temp.path()).unwrap();

        let keys: Vec<&str> = modules.iter().map(|module| module.key.as_str()).collect();
        assert_eq!(keys, vec!["router", "start"]);
        assert_eq!(modules[0].tokens.get("X").unwrap(), "y");
    }

    #[test]
    fn reads_the_capabilities_a_module_declares() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("modules/start");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("module.json"),
            r#"{ "key": "start", "name": "Start",
                 "capabilities": ["typescript", "tanstack-start"] }"#,
        )
        .unwrap();

        let module = Module::load(&dir).unwrap();

        assert_eq!(module.capabilities, vec!["typescript", "tanstack-start"]);
    }

    #[test]
    fn a_module_that_declares_no_capabilities_has_none() {
        let temp = tempfile::tempdir().unwrap();
        write_module(temp.path(), "router", "Router", 1);

        let module = Module::load(&temp.path().join("modules/router")).unwrap();

        assert!(module.capabilities.is_empty());
    }

    #[test]
    fn errors_when_no_modules_present() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("modules")).unwrap();
        assert!(discover_modules(temp.path()).is_err());
    }
}
