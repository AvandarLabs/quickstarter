//! Composes the real, authored templates (not fixtures) to guard against
//! authoring mistakes such as an unfilled `{{TOKEN}}` or a misfiled module
//! file. `template_root` is the repository root, one level above this crate.

use std::path::{Path, PathBuf};

use quickstarter::catalog::{self, Module};
use quickstarter::compose::{self, ComposePlan};
use quickstarter::skills::manifest;

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
fn composition_writes_no_skills_and_leaves_that_to_the_scaffolder() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("app");
    compose_stack("router", &dest);

    // The skills are installed by `npx skills` after composition, inside the
    // finished project, so composition itself must ship neither the lock that
    // tool writes nor the directories it fills.
    assert!(!dest.join("skills-lock.json").exists());
    assert!(!dest.join(".agents").exists());
    assert!(!dest.join(".claude/skills").exists());
}

#[test]
fn every_stack_selects_the_global_and_typescript_skills_and_no_rust_one() {
    let root = repo_root();
    let manifest = manifest::load(&root).unwrap().expect("the real manifest");
    let global = manifest.specs_for(&[]).unwrap();

    for module in catalog::discover_modules(&root.join("templates")).unwrap() {
        let specs = manifest.specs_for(&module.capabilities).unwrap();

        for spec in &global {
            assert!(specs.contains(spec), "{}: missing global skill {spec}", module.key);
        }
        assert!(
            specs.iter().any(|spec| spec.ends_with("typescript-magician")),
            "{}: no TypeScript skill selected",
            module.key
        );

        // Rust skills belong to this repository, never to a generated project.
        for spec in &specs {
            assert!(
                !spec.starts_with("actionbook/") && !spec.starts_with("leonardomso/"),
                "{}: rust skill leaked into a project: {spec}",
                module.key
            );
        }
    }
}

#[test]
fn every_stack_gets_the_skills_tooling() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("app");
    compose_stack("router", &dest);

    assert!(dest.join("scripts/skills/SkillsCli.ts").is_file());
    assert!(dest.join("vitest.config.ts").is_file());

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dest.join("package.json")).unwrap()).unwrap();
    let scripts = &manifest["scripts"];

    // The scaffolder installs the skills once, at creation. Restoring them in
    // a later clone is the project's own job, so the wiring that does it has
    // to survive composition.
    assert_eq!(scripts["skills"], "tsx scripts/skills/SkillsCli.ts");
    assert_eq!(scripts["skills:install"], "tsx scripts/skills/SkillsCli.ts install");
    assert_eq!(scripts["skills:update"], "tsx scripts/skills/SkillsCli.ts update");

    // `postinstall` must install, never update: `pnpm install` does not
    // upgrade an installed package and must not upgrade an installed skill.
    let postinstall = scripts["postinstall"].as_str().unwrap();
    assert!(postinstall.contains("install"), "{postinstall}");
    assert!(!postinstall.contains("update"), "{postinstall}");
    assert_eq!(scripts["test"], "vitest run");
    assert!(manifest["devDependencies"]["@avandar/acclimate"].is_string());
    assert!(manifest["devDependencies"]["tsx"].is_string());
    assert!(manifest["devDependencies"]["vitest"].is_string());

    // The restored directories must not be committed by the generated repo.
    let gitignore = std::fs::read_to_string(dest.join(".gitignore")).unwrap();
    assert!(gitignore.contains(".agents/"));
    assert!(gitignore.contains(".claude/skills/"));

    // `npx skills` writes the lock into the new project as it installs, and it
    // is the one skills file the project tracks, so it must not be ignored.
    // Comments mentioning it do not count.
    let is_lock_ignored = gitignore
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .any(|line| line.contains("skills-lock.json"));
    assert!(!is_lock_ignored, "skills-lock.json must stay tracked");
}

#[test]
fn every_stack_builds_before_it_type_checks() {
    for stack in ["router", "start"] {
        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().join("app");
        compose_stack(stack, &dest);

        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dest.join("package.json")).unwrap())
                .unwrap();
        let build = manifest["scripts"]["build"].as_str().unwrap();

        // Both stacks generate `src/routeTree.gen.ts` from their Vite plugin,
        // and every entry point imports it. Running `tsc` first means a freshly
        // scaffolded project cannot type-check or build at all.
        let vite_position = build.find("vite build").expect("build must run vite");
        let tsc_position = build.find("tsc").expect("build must run tsc");
        assert!(
            vite_position < tsc_position,
            "{stack}: vite build must precede tsc: {build}"
        );

        // `check` gets its type-checking from `build` for the same reason.
        let check = manifest["scripts"]["check"].as_str().unwrap();
        assert!(check.contains("pnpm build"), "{stack}: {check}");
        assert!(!check.contains("type-check"), "{stack}: {check}");
    }
}
