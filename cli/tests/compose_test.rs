//! End-to-end test of the composition engine against a fixture template tree.

use std::path::Path;

use quickstarter::catalog::{self, Module};
use quickstarter::compose::{self, ComposePlan};

/// Builds a minimal but realistic template repo layout in `root`.
fn write_fixture_templates(root: &Path) {
    let base = root.join("templates/base");
    std::fs::create_dir_all(base.join("files/src")).unwrap();
    std::fs::write(base.join("files/README.md"), "# {{PROJECT_NAME}}\n{{STACK_LINE}}").unwrap();
    std::fs::write(base.join("files/src/theme.ts"), "export const theme = {}").unwrap();
    std::fs::write(
        base.join("package.json"),
        r#"{ "name": "{{PACKAGE_NAME}}", "dependencies": { "react": "^19.0.0" } }"#,
    )
    .unwrap();
    // A symlink that must survive composition as a link.
    std::fs::write(base.join("files/AGENTS.md"), "rules").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("AGENTS.md", base.join("files/CLAUDE.md")).unwrap();

    let router = root.join("templates/modules/router");
    std::fs::create_dir_all(router.join("files")).unwrap();
    std::fs::write(router.join("files/vite.config.ts"), "// router vite").unwrap();
    std::fs::write(
        router.join("package.json"),
        r#"{ "dependencies": { "@tanstack/react-router": "^1.170.0" } }"#,
    )
    .unwrap();
    std::fs::write(
        router.join("module.json"),
        r#"{ "key": "router", "name": "Router", "order": 1,
            "tokens": { "STACK_LINE": "TanStack Router" } }"#,
    )
    .unwrap();
}

#[test]
fn composes_base_plus_module_with_merge_and_tokens() {
    let temp = tempfile::tempdir().unwrap();
    let template_root = temp.path().join("template");
    write_fixture_templates(&template_root);

    let modules = catalog::discover_modules(&template_root.join("templates")).unwrap();
    let module: &Module = modules.iter().find(|m| m.key == "router").unwrap();

    let dest = temp.path().join("out/my-app");
    let plan = ComposePlan {
        template_root: &template_root,
        module,
        project_name: "My App",
        package_name: "my-app",
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
