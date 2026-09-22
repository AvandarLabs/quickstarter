//! The rules that say which tags can be combined.
//!
//! Two rules, both pure and both read from the tags themselves rather than
//! hardcoded here:
//!
//! - a capability fits a project type when it restricts neither the project
//!   types nor the languages it allows, or names this one;
//! - two capabilities exclude each other when either names the other in
//!   `conflictsWith`.
//!
//! Exclusion is treated as symmetric even when only one side declares it, so a
//! single missing `conflictsWith` entry cannot let an impossible pair through.

use crate::catalog::{Capability, ProjectType};

/// Whether `capability` can be chosen for a `project_type` project.
pub fn fits(capability: &Capability, project_type: &ProjectType) -> bool {
    misfit_reason(capability, project_type).is_none()
}

/// Why `capability` does not fit `project_type`, phrased for the user, or
/// `None` when it does fit. The reason names both tags, because "that does not
/// fit" is only useful when it says what does.
pub fn misfit_reason(capability: &Capability, project_type: &ProjectType) -> Option<String> {
    if restricts(&capability.project_types, &project_type.key) {
        return Some(format!(
            "{} only fits a {} project, and this one is {}",
            capability.key,
            or_list(&capability.project_types),
            project_type.key
        ));
    }
    if restricts(&capability.languages, &project_type.language) {
        return Some(format!(
            "{} only fits a {} project, and {} is a {} one",
            capability.key,
            or_list(&capability.languages),
            project_type.key,
            project_type.language
        ));
    }
    None
}

/// Whether `left` and `right` exclude each other.
pub fn conflict(left: &Capability, right: &Capability) -> bool {
    left.conflicts_with.contains(&right.key) || right.conflicts_with.contains(&left.key)
}

/// The capabilities that fit `project_type`, in catalog order.
pub fn compatible<'catalog>(
    capabilities: &'catalog [Capability],
    project_type: &ProjectType,
) -> Vec<&'catalog Capability> {
    capabilities
        .iter()
        .filter(|capability| fits(capability, project_type))
        .collect()
}

/// Whether `allowed` is a restriction that `value` fails. An empty list is no
/// restriction at all.
fn restricts(allowed: &[String], value: &str) -> bool {
    !allowed.is_empty() && !allowed.iter().any(|one| one == value)
}

/// The allowed values as `a`, `a or b`, `a, b or c`.
fn or_list(values: &[String]) -> String {
    match values {
        [] => String::new(),
        [only] => only.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::test_tags;

    fn web() -> ProjectType {
        test_tags::project_type("typescript:web", "typescript")
    }

    fn cli() -> ProjectType {
        test_tags::project_type("rust:cli", "rust")
    }

    #[test]
    fn a_capability_that_restricts_nothing_fits_every_project_type() {
        let capability = test_tags::capability("prettier");
        assert!(fits(&capability, &web()));
        assert!(fits(&capability, &cli()));
    }

    #[test]
    fn a_capability_fits_only_the_project_types_it_names() {
        let mut capability = test_tags::capability("tanstack-router");
        capability.project_types = test_tags::keys(&["typescript:web"]);

        assert!(fits(&capability, &web()));
        assert!(!fits(&capability, &cli()));
    }

    #[test]
    fn a_capability_fits_only_the_languages_it_names() {
        let mut capability = test_tags::capability("serde");
        capability.languages = test_tags::keys(&["rust"]);

        assert!(fits(&capability, &cli()));
        assert!(!fits(&capability, &web()));
    }

    #[test]
    fn a_misfit_names_both_tags_and_why() {
        let mut capability = test_tags::capability("tanstack-router");
        capability.project_types = test_tags::keys(&["typescript:web"]);

        let reason = misfit_reason(&capability, &cli()).unwrap();

        assert!(reason.contains("tanstack-router only fits a typescript:web project"), "{reason}");
        assert!(reason.contains("rust:cli"), "{reason}");
    }

    #[test]
    fn a_language_misfit_names_the_language() {
        let mut capability = test_tags::capability("serde");
        capability.languages = test_tags::keys(&["rust"]);

        let reason = misfit_reason(&capability, &web()).unwrap();

        assert!(reason.contains("serde only fits a rust project"), "{reason}");
        assert!(reason.contains("typescript:web"), "{reason}");
    }

    #[test]
    fn exclusion_holds_even_when_only_one_side_declares_it() {
        let mut router = test_tags::capability("tanstack-router");
        router.conflicts_with = test_tags::keys(&["tanstack-start"]);
        let start = test_tags::capability("tanstack-start");

        assert!(conflict(&router, &start));
        assert!(conflict(&start, &router));
    }

    #[test]
    fn unrelated_capabilities_do_not_conflict() {
        let router = test_tags::capability("tanstack-router");
        let prettier = test_tags::capability("prettier");
        assert!(!conflict(&router, &prettier));
    }

    #[test]
    fn compatible_keeps_catalog_order_and_drops_the_misfits() {
        let mut router = test_tags::capability("tanstack-router");
        router.project_types = test_tags::keys(&["typescript:web"]);
        let capabilities = vec![router, test_tags::capability("prettier")];

        let fitting = compatible(&capabilities, &cli());

        let keys: Vec<&str> = fitting.iter().map(|one| one.key.as_str()).collect();
        assert_eq!(keys, vec!["prettier"]);
    }

    #[test]
    fn a_restriction_of_several_values_reads_as_a_list() {
        let mut capability = test_tags::capability("tailwind");
        capability.project_types = test_tags::keys(&["typescript:web", "typescript:mobile"]);

        let reason = misfit_reason(&capability, &cli()).unwrap();

        assert!(reason.contains("typescript:web or typescript:mobile"), "{reason}");
    }
}
