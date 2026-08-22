//! Composes the real, authored templates (not fixtures) to guard against
//! authoring mistakes such as an unfilled `{{TOKEN}}` or a misfiled module
//! file. `template_root` is the repository root, one level above this crate.

use std::path::{Path, PathBuf};

use quickstarter::catalog::{self, Module};
use quickstarter::compose::{self, ComposePlan};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn compose_stack(key: &str, dest: &Path) -> Module {
    let root = repo_root();
    let modules = catalog::discover_modules(&root.join("templates")).unwrap();
    let module = modules.iter().find(|m| m.key == key).unwrap().clone();
    let plan = ComposePlan {
        template_root: &root,
        module: &module,
        project_name: "Demo App",
        package_name: "demo-app",
    };
    compose::compose(&plan, dest).unwrap();
    module
}

/// Fails if any composed text file still contains an unfilled `{{TOKEN}}`.
fn assert_no_unfilled_tokens(dir: &Path) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let meta = std::fs::symlink_metadata(&path).unwrap();
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            assert_no_unfilled_tokens(&path);
        } else if let Ok(text) = std::fs::read_to_string(&path) {
            assert!(
                !text.contains("{{"),
                "unfilled token in {}",
                path.display()
            );
        }
    }
}

#[test]
fn router_stack_composes_cleanly() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("app");
    compose_stack("router", &dest);

    assert!(dest.join("index.html").is_file());
    assert!(dest.join("src/main.tsx").is_file());
    assert!(dest.join("src/routes/__root.tsx").is_file());

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dest.join("package.json")).unwrap()).unwrap();
    assert_eq!(manifest["name"], "demo-app");
    assert!(manifest["dependencies"]["@tanstack/react-router"].is_string());
    assert!(manifest["devDependencies"]["@tanstack/router-plugin"].is_string());
    assert!(manifest["dependencies"]["@mantine/core"].is_string());

    // Shared symlink survives and the title token was filled in.
    assert!(dest.join("CLAUDE.md").is_symlink() || std::fs::symlink_metadata(dest.join("CLAUDE.md")).is_ok());
    let html = std::fs::read_to_string(dest.join("index.html")).unwrap();
    assert!(html.contains("<title>Demo App</title>"));

    assert_no_unfilled_tokens(&dest);
}

#[test]
fn start_stack_composes_cleanly() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("app");
    compose_stack("start", &dest);

    // Start has no index.html or main.tsx; it defines its document in __root.
    assert!(!dest.join("index.html").exists());
    assert!(!dest.join("src/main.tsx").exists());
    assert!(dest.join("src/router.tsx").is_file());
    assert!(dest.join("src/routes/__root.tsx").is_file());

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dest.join("package.json")).unwrap()).unwrap();
    assert!(manifest["dependencies"]["@tanstack/react-start"].is_string());
    assert_eq!(manifest["scripts"]["start"], "node .output/server/index.mjs");

    let vite = std::fs::read_to_string(dest.join("vite.config.ts")).unwrap();
    assert!(vite.contains("tanstackStart"));

    assert_no_unfilled_tokens(&dest);
}

#[test]
fn every_stack_gets_the_produced_skills_lock() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("app");
    compose_stack("router", &dest);

    let lock: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dest.join("skills-lock.json")).unwrap())
            .unwrap();
    let skills = lock["skills"].as_object().unwrap();

    // The produced bucket, minus the skills that are not `npx skills`-managed.
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(repo_root().join("skills-manifest.json")).unwrap())
            .unwrap();
    let produced: Vec<&str> = manifest["produced"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
        .filter(|name| !quickstarter::compose::skills_lock::CLI_MANAGED_SKILLS.contains(name))
        .collect();

    assert_eq!(skills.len(), produced.len());
    for name in produced {
        assert!(skills.contains_key(name), "missing {name} from the produced lock");
        assert!(skills[name]["source"].is_string(), "{name} has no source");
    }

    // Rust skills stay in quickstarter; a generated app must never see one.
    for name in skills.keys() {
        assert!(!name.starts_with("rust-"), "rust skill leaked into a project: {name}");
    }

    // `impeccable` is installed by its own CLI, so it must not be locked.
    assert!(!skills.contains_key("impeccable"));
}
