//! A minimal but realistic template repository, built in a temp directory.
//!
//! Sharing one fixture between the composition test and the end-to-end CLI
//! test keeps them describing the same template layout.
//!
//! The fixture deliberately ships no `skills-manifest.json`: a template
//! repository that declares no skills installs none, which is what lets the
//! end-to-end test drive the real binary without ever running `npx`.

use std::path::Path;

/// Writes `templates/base` plus a single `router` module under `root`.
pub fn write(root: &Path) {
    write_base(root);
    write_router_module(root);
}

fn write_base(root: &Path) {
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
}

fn write_router_module(root: &Path) {
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
