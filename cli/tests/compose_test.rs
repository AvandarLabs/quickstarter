//! End-to-end test of the composition engine against a fixture template tree.

#[path = "support/fixture_templates.rs"]
mod fixture_templates;

use quickstarter::catalog::{self, Module};
use quickstarter::compose::{self, ComposePlan};

#[test]
fn composes_base_plus_module_with_merge_and_tokens() {
    let temp = tempfile::tempdir().unwrap();
    let template_root = temp.path().join("template");
    fixture_templates::write(&template_root);

    let modules = catalog::discover_modules(&template_root.join("templates")).unwrap();
    let module: &Module = modules.iter().find(|m| m.key == "router").unwrap();

    let dest = temp.path().join("out/my-app");
    let plan = ComposePlan {
        template_root: &template_root,
        module,
        project_name: "My App",
        package_name: "my-app",
        extra_tokens: Default::default(),
    };
    compose::compose(&plan, &dest).unwrap();

    // Base file present, with tokens substituted (including a module token).
    let readme = std::fs::read_to_string(dest.join("README.md")).unwrap();
    assert_eq!(readme, "# My App\nTanStack Router");

    // Module file overlaid on top.
    assert_eq!(
        std::fs::read_to_string(dest.join("vite.config.ts")).unwrap(),
        "// router vite"
    );

    // package.json deep-merged: base + module deps, name token filled in.
    let manifest = std::fs::read_to_string(dest.join("package.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(parsed["name"], "my-app");
    assert_eq!(parsed["dependencies"]["react"], "^19.0.0");
    assert_eq!(parsed["dependencies"]["@tanstack/react-router"], "^1.170.0");

    // Symlink preserved as a link, not dereferenced.
    #[cfg(unix)]
    {
        let link = dest.join("CLAUDE.md");
        let meta = std::fs::symlink_metadata(&link).unwrap();
        assert!(meta.file_type().is_symlink());
    }
}
