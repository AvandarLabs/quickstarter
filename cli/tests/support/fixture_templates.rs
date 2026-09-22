//! A minimal but realistic template repository, built in a temp directory.
//!
//! Sharing one fixture between the composition test and the end-to-end CLI
//! test keeps them describing the same template layout. It exercises the tag
//! model rather than mirroring the real templates: two project types, one with
//! a `package.json` and one without, and two capabilities that fit only the
//! first and exclude each other.
//!
//! The fixture deliberately ships no `skills-manifest.json`: a template
//! repository that declares no skills installs none, which is what lets the
//! end-to-end test drive the real binary without ever running `npx`.

use std::path::{Path, PathBuf};

/// The project type with a `package.json`, and the only one the capabilities
/// fit.
pub const WEB_PROJECT_TYPE: &str = "typescript:web";

/// The project type without a `package.json`.
pub const RUST_PROJECT_TYPE: &str = "rust:cli";

/// One of the two capabilities that exclude each other.
pub const ROUTER_CAPABILITY: &str = "tanstack-router";

/// The other one.
pub const START_CAPABILITY: &str = "tanstack-start";

/// Writes the whole template repository under `root`.
pub fn write(root: &Path) {
    write_base(root);
    write_web_project_type(root);
    write_rust_project_type(root);
    write_router_capability(root);
    write_start_capability(root);
}

/// The layer every project type shares, tokens and all.
fn write_base(root: &Path) {
    let base = root.join("templates/base/files");
    std::fs::create_dir_all(&base).unwrap();
    write_file(&base.join("README.md"), "# {{PROJECT_NAME}}\n{{STACK_LINE}}");
    write_file(&base.join("AGENTS.md"), "rules");
    // A symlink that must survive composition as a link.
    #[cfg(unix)]
    std::os::unix::fs::symlink("AGENTS.md", base.join("CLAUDE.md")).unwrap();
}

/// A project type whose language has a `package.json`.
fn write_web_project_type(root: &Path) {
    let dir = project_type_dir(root, "typescript-web");
    write_file(
        &dir.join("project-type.json"),
        r#"{ "key": "typescript:web", "name": "TypeScript web app",
             "description": "Vite, React and Mantine", "language": "typescript", "order": 1,
             "tokens": { "STACK_LINE": "Vite", "NEXT_STEPS": "pnpm install\npnpm dev" } }"#,
    );
    write_file(
        &dir.join("package.json"),
        r#"{ "name": "{{PACKAGE_NAME}}", "dependencies": { "react": "^19.0.0" } }"#,
    );
    write_file(&dir.join("files/src/theme.ts"), "export const theme = {}");
    write_file(&dir.join("files/vite.config.ts"), "// web vite");
}

/// A project type whose language has no `package.json` at all.
fn write_rust_project_type(root: &Path) {
    let dir = project_type_dir(root, "rust-cli");
    write_file(
        &dir.join("project-type.json"),
        r#"{ "key": "rust:cli", "name": "Rust CLI",
             "description": "A command-line tool", "language": "rust", "order": 2,
             "tokens": { "STACK_LINE": "Rust CLI", "NEXT_STEPS": "cargo run" } }"#,
    );
    write_file(&dir.join("files/Cargo.toml"), "[package]\nname = \"{{PACKAGE_NAME}}\"\n");
    write_file(&dir.join("files/src/main.rs"), "fn main() {}");
}

/// A capability that fits only the web project type and excludes the other.
fn write_router_capability(root: &Path) {
    let dir = capability_dir(root, "tanstack-router");
    write_file(
        &dir.join("capability.json"),
        r#"{ "key": "tanstack-router", "name": "TanStack Router",
             "description": "Client-side routing", "order": 1,
             "projectTypes": ["typescript:web"], "conflictsWith": ["tanstack-start"],
             "tokens": { "STACK_LINE": "TanStack Router" } }"#,
    );
    write_file(
        &dir.join("package.json"),
        r#"{ "dependencies": { "@tanstack/react-router": "^1.170.0" } }"#,
    );
    write_file(&dir.join("files/vite.config.ts"), "// router vite");
}

/// The capability the router excludes, declaring the exclusion from its side
/// too.
fn write_start_capability(root: &Path) {
    let dir = capability_dir(root, "tanstack-start");
    write_file(
        &dir.join("capability.json"),
        r#"{ "key": "tanstack-start", "name": "TanStack Start",
             "description": "Server-side rendering", "order": 2,
             "projectTypes": ["typescript:web"], "conflictsWith": ["tanstack-router"],
             "tokens": { "STACK_LINE": "TanStack Start" } }"#,
    );
    write_file(
        &dir.join("package.json"),
        r#"{ "dependencies": { "@tanstack/react-start": "^1.0.0" } }"#,
    );
    write_file(&dir.join("files/src/router.tsx"), "// start router");
}

fn project_type_dir(root: &Path, slug: &str) -> PathBuf {
    root.join("templates/project-types").join(slug)
}

fn capability_dir(root: &Path, slug: &str) -> PathBuf {
    root.join("templates/capabilities").join(slug)
}

/// Writes `contents` at `path`, creating the directories it needs.
fn write_file(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}
