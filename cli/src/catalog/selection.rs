//! A validated choice: one project type plus the capabilities chosen with it.
//!
//! Everything the scaffolder does afterwards (which skills to install, which
//! layers to overlay, what to report) reads a `Selection`, so this is the one
//! place a combination is checked. Building one is the check: a `Selection`
//! that exists is a combination that can be built.

use anyhow::{Result, bail};

use crate::catalog::compatibility;
use crate::catalog::{Capability, ProjectType};

/// One project type and the capabilities chosen alongside it.
#[derive(Debug)]
pub struct Selection<'catalog> {
    /// The project type the repository will have.
    pub project_type: &'catalog ProjectType,
    /// The chosen capabilities, in catalog order (`order` then `name`), which
    /// is the order composition overlays them in.
    pub capabilities: Vec<&'catalog Capability>,
}

impl<'catalog> Selection<'catalog> {
    /// Resolves `keys` against `catalog`, checking that every capability
    /// exists, fits `project_type`, and excludes none of the others.
    ///
    /// A key given twice selects its capability once, and the result is in
    /// catalog order however the keys were ordered, so `--capability a
    /// --capability b` and `--capability b --capability a` compose the same
    /// project.
    pub fn resolve(
        project_type: &'catalog ProjectType,
        keys: &[String],
        catalog: &'catalog [Capability],
    ) -> Result<Selection<'catalog>> {
        for key in keys {
            let capability = find(key, project_type, catalog)?;
            if let Some(reason) = compatibility::misfit_reason(capability, project_type) {
                bail!("{reason}.");
            }
        }

        let capabilities: Vec<&Capability> = catalog
            .iter()
            .filter(|capability| keys.iter().any(|key| key == &capability.key))
            .collect();
        ensure_nothing_excluded(&capabilities)?;

        Ok(Selection { project_type, capabilities })
    }

    /// The chosen capability keys, in catalog order.
    pub fn capability_keys(&self) -> Vec<String> {
        self.capabilities
            .iter()
            .map(|capability| capability.key.clone())
            .collect()
    }
}

/// Finds the capability a key names, listing the ones this project type can
/// actually use when it names nothing.
fn find<'catalog>(
    key: &str,
    project_type: &ProjectType,
    catalog: &'catalog [Capability],
) -> Result<&'catalog Capability> {
    let wanted = key.trim().to_ascii_lowercase();
    match catalog.iter().find(|capability| capability.key.to_ascii_lowercase() == wanted) {
        Some(capability) => Ok(capability),
        None => bail!("There is no capability called {key:?}.{}", available(project_type, catalog)),
    }
}

/// Rejects a pair of chosen capabilities that exclude each other.
fn ensure_nothing_excluded(chosen: &[&Capability]) -> Result<()> {
    for (index, capability) in chosen.iter().enumerate() {
        for other in &chosen[index + 1..] {
            if compatibility::conflict(capability, other) {
                bail!(
                    "{} and {} are mutually exclusive: choose one of them.",
                    capability.key,
                    other.key
                );
            }
        }
    }
    Ok(())
}

/// A human-readable list of the capabilities `project_type` can use.
fn available(project_type: &ProjectType, catalog: &[Capability]) -> String {
    let usable = compatibility::compatible(catalog, project_type);
    if usable.is_empty() {
        return format!(" A {} project has no capabilities to choose from.", project_type.key);
    }
    let keys: Vec<&str> = usable.iter().map(|capability| capability.key.as_str()).collect();
    format!(" A {} project can use: {}.", project_type.key, keys.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::test_tags;

    /// Two routers that exclude each other and only fit the web project type,
    /// plus one capability that fits anything.
    fn catalog() -> Vec<Capability> {
        let mut router = test_tags::capability("tanstack-router");
        router.project_types = test_tags::keys(&["typescript:web"]);
        router.conflicts_with = test_tags::keys(&["tanstack-start"]);
        let mut start = test_tags::capability("tanstack-start");
        start.project_types = test_tags::keys(&["typescript:web"]);
        start.conflicts_with = test_tags::keys(&["tanstack-router"]);
        vec![router, start, test_tags::capability("prettier")]
    }

    fn web() -> ProjectType {
        test_tags::project_type("typescript:web", "typescript")
    }

    fn cli() -> ProjectType {
        test_tags::project_type("rust:cli", "rust")
    }

    #[test]
    fn a_project_type_alone_selects_no_capability() {
        let catalog = catalog();
        let project_type = cli();

        let selection = Selection::resolve(&project_type, &[], &catalog).unwrap();

        assert!(selection.capabilities.is_empty());
    }

    #[test]
    fn a_selection_keeps_catalog_order_whatever_order_the_keys_came_in() {
        let catalog = catalog();
        let project_type = web();
        let keys = test_tags::keys(&["prettier", "tanstack-router"]);

        let selection = Selection::resolve(&project_type, &keys, &catalog).unwrap();

        assert_eq!(selection.capability_keys(), test_tags::keys(&["tanstack-router", "prettier"]));
    }

    #[test]
    fn a_key_given_twice_selects_its_capability_once() {
        let catalog = catalog();
        let project_type = web();
        let keys = test_tags::keys(&["prettier", "prettier"]);

        let selection = Selection::resolve(&project_type, &keys, &catalog).unwrap();

        assert_eq!(selection.capability_keys(), test_tags::keys(&["prettier"]));
    }

    #[test]
    fn an_unknown_capability_lists_the_usable_ones() {
        let catalog = catalog();
        let project_type = web();

        let error = Selection::resolve(&project_type, &test_tags::keys(&["svelte"]), &catalog)
            .unwrap_err()
            .to_string();

        assert!(error.contains("no capability called \"svelte\""), "{error}");
        assert!(error.contains("tanstack-router") && error.contains("prettier"), "{error}");
    }

    #[test]
    fn a_capability_that_does_not_fit_the_project_type_is_refused() {
        let catalog = catalog();
        let project_type = cli();

        let error =
            Selection::resolve(&project_type, &test_tags::keys(&["tanstack-router"]), &catalog)
                .unwrap_err()
                .to_string();

        assert!(error.contains("tanstack-router only fits a typescript:web project"), "{error}");
        assert!(error.contains("rust:cli"), "{error}");
    }

    #[test]
    fn two_capabilities_that_exclude_each_other_are_refused() {
        let catalog = catalog();
        let project_type = web();
        let keys = test_tags::keys(&["tanstack-router", "tanstack-start"]);

        let error = Selection::resolve(&project_type, &keys, &catalog).unwrap_err().to_string();

        assert!(error.contains("tanstack-router and tanstack-start"), "{error}");
        assert!(error.contains("mutually exclusive"), "{error}");
    }

    #[test]
    fn a_project_type_with_no_usable_capability_says_so() {
        // Only the two web-only routers exist, so a Rust project has nothing
        // to be offered.
        let catalog = &catalog()[..2];
        let project_type = cli();

        let error = Selection::resolve(&project_type, &test_tags::keys(&["nope"]), catalog)
            .unwrap_err()
            .to_string();

        assert!(error.contains("no capabilities to choose from"), "{error}");
    }
}
