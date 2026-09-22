//! Composes the real, authored templates (not fixtures) to guard against
//! authoring mistakes such as an unfilled `{{TOKEN}}` or a misfiled layer.
//! `template_root` is the repository root, one level above this crate.

use std::path::{Path, PathBuf};

use quickstarter::catalog::{self, Catalog, Selection};
use quickstarter::compose::{self, ComposePlan};
use quickstarter::skills::commands;
use quickstarter::skills::install;
use quickstarter::skills::manifest;

/// The project type of the web app.
const WEB: &str = "typescript:web";

/// The project type of the command-line tool.
const RUST: &str = "rust:cli";

/// The two capabilities a web app chooses between.
const ROUTER: &str = "tanstack-router";
const START: &str = "tanstack-start";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn catalog() -> Catalog {
    catalog::discover(&repo_root().join("templates")).unwrap()
}

/// The selection a user's answers would produce, validated by the same code
/// the CLI uses.
fn selection<'catalog>(
    catalog: &'catalog Catalog,
    project_type_key: &str,
    capability_keys: &[&str],
) -> Selection<'catalog> {
    let project_type = catalog
        .project_types
        .iter()
        .find(|one| one.key == project_type_key)
        .unwrap_or_else(|| panic!("the templates declare no '{project_type_key}' project type"));
    let keys: Vec<String> = capability_keys.iter().map(|key| (*key).to_string()).collect();
    Selection::resolve(project_type, &keys, &catalog.capabilities).unwrap()
}

/// Composes one combination of real tags into `dest`, exactly as a run does.
fn compose_project(project_type_key: &str, capability_keys: &[&str], dest: &Path) {
    let root = repo_root();
    let catalog = catalog();
    let selection = selection(&catalog, project_type_key, capability_keys);

    // The real run selects the skills first, because the templates carry a
    // token naming the ones that install themselves.
    let specs =
        install::select_specs(&root, &selection.project_type.key, &selection.capability_keys())
            .unwrap();
    let mut extra_tokens = compose::tokens::Tokens::new();
    extra_tokens.insert(
        "SELF_INSTALLING_SKILLS".to_string(),
        commands::self_installing_skill_names(&specs).join(" "),
    );

    let plan = ComposePlan {
        template_root: &root,
        project_type: selection.project_type,
        capabilities: &selection.capabilities,
        project_name: "Demo App",
        package_name: "demo-app",
        extra_tokens,
    };
    compose::compose(&plan, dest).unwrap();
}

/// The composed `package.json` of a project.
fn manifest_of(dest: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(dest.join("package.json")).unwrap()).unwrap()
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
            assert!(!text.contains("{{"), "unfilled token in {}", path.display());
        }
    }
}

#[test]
fn the_router_capability_composes_cleanly() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("app");
    compose_project(WEB, &[ROUTER], &dest);

    assert!(dest.join("index.html").is_file());
    assert!(dest.join("src/main.tsx").is_file());
    assert!(dest.join("src/routes/__root.tsx").is_file());

    let manifest = manifest_of(&dest);
    assert_eq!(manifest["name"], "demo-app");
    assert!(manifest["dependencies"]["@tanstack/react-router"].is_string());
    assert!(manifest["devDependencies"]["@tanstack/router-plugin"].is_string());
    assert!(manifest["dependencies"]["@mantine/core"].is_string());

    // Shared symlink survives and the title token was filled in.
    assert!(std::fs::symlink_metadata(dest.join("CLAUDE.md")).unwrap().file_type().is_symlink());
    let html = std::fs::read_to_string(dest.join("index.html")).unwrap();
    assert!(html.contains("<title>Demo App</title>"));

    assert_no_unfilled_tokens(&dest);
}

#[test]
fn the_start_capability_composes_cleanly() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("app");
    compose_project(WEB, &[START], &dest);

    // Start has no index.html or main.tsx; it defines its document in __root.
    assert!(!dest.join("index.html").exists());
    assert!(!dest.join("src/main.tsx").exists());
    assert!(dest.join("src/router.tsx").is_file());
    assert!(dest.join("src/routes/__root.tsx").is_file());

    let manifest = manifest_of(&dest);
    assert!(manifest["dependencies"]["@tanstack/react-start"].is_string());
    assert_eq!(manifest["scripts"]["start"], "node .output/server/index.mjs");

    let vite = std::fs::read_to_string(dest.join("vite.config.ts")).unwrap();
    assert!(vite.contains("tanstackStart"));

    assert_no_unfilled_tokens(&dest);
}

#[test]
fn the_rust_project_type_composes_cleanly_and_without_a_package_json() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("app");
    compose_project(RUST, &[], &dest);

    assert!(dest.join("Cargo.toml").is_file());
    assert!(dest.join("src/main.rs").is_file());

    // cargo is the build system here, so no npm manifest may appear, empty or
    // otherwise, and nothing from the web project type may leak in.
    assert!(!dest.join("package.json").exists());
    assert!(!dest.join("vite.config.ts").exists());
    assert!(!dest.join("tsconfig.json").exists());

    // The shared base layer still arrives, tokens filled in from this project
    // type rather than from a web one.
    assert!(dest.join("AGENTS.md").is_file());
    assert!(dest.join("README.md").is_file());
    let manifest = std::fs::read_to_string(dest.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("name = \"demo-app\""), "{manifest}");

    assert_no_unfilled_tokens(&dest);
}

#[test]
fn composition_writes_no_skills_and_leaves_that_to_the_scaffolder() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("app");
    compose_project(WEB, &[ROUTER], &dest);

    // The skills are installed by `npx skills` after composition, inside the
    // finished project, so composition itself must ship neither the lock that
    // tool writes nor the directories it fills.
    assert!(!dest.join("skills-lock.json").exists());
    assert!(!dest.join(".agents").exists());
    assert!(!dest.join(".claude/skills").exists());
}

#[test]
fn every_project_type_selects_the_global_skills() {
    let root = repo_root();
    let manifest = manifest::load(&root).unwrap().expect("the real manifest");

    for project_type in &catalog().project_types {
        let specs = manifest.specs_for(&project_type.key, &[]).unwrap();
        for spec in manifest.global_specs() {
            assert!(specs.contains(spec), "{}: missing global skill {spec}", project_type.key);
        }
    }
}

#[test]
fn a_typescript_project_gets_the_typescript_skills_and_no_rust_one() {
    let root = repo_root();
    let manifest = manifest::load(&root).unwrap().expect("the real manifest");

    let specs = manifest.specs_for(WEB, &[ROUTER.to_string()]).unwrap();

    assert!(
        specs.iter().any(|spec| spec.ends_with("typescript-magician")),
        "no TypeScript skill selected: {specs:?}"
    );
    // Rust skills belong to a Rust project, never to a web one.
    for spec in &specs {
        assert!(
            !spec.starts_with("actionbook/") && !spec.starts_with("leonardomso/"),
            "rust skill leaked into a web project: {spec}"
        );
    }
}

#[test]
fn a_rust_project_gets_the_rust_skills_and_no_web_one() {
    let root = repo_root();
    let manifest = manifest::load(&root).unwrap().expect("the real manifest");

    let specs = manifest.specs_for(RUST, &[]).unwrap();

    assert!(
        specs.iter().any(|spec| spec.contains("rust")),
        "no Rust skill selected: {specs:?}"
    );
    for spec in &specs {
        assert!(!spec.contains("mantine"), "a Mantine skill reached a CLI project: {spec}");
        assert!(!spec.contains("playwright"), "a browser skill reached a CLI project: {spec}");
    }
}

#[test]
fn the_web_project_type_gets_the_skills_tooling() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("app");
    compose_project(WEB, &[ROUTER], &dest);

    assert!(dest.join("scripts/skills/SkillsCli.ts").is_file());
    assert!(dest.join("vitest.config.ts").is_file());

    let manifest = manifest_of(&dest);
    let scripts = &manifest["scripts"];

    // The scaffolder installs the skills once, at creation. Restoring them in
    // a later clone is the project's own job, so the wiring that does it has
    // to survive composition.
    assert_eq!(scripts["skills"], "tsx scripts/skills/SkillsCli.ts");
    assert_eq!(scripts["skills:install"], "tsx scripts/skills/SkillsCli.ts install");
    assert_eq!(scripts["skills:update"], "./scripts/skills/update-skills.sh");

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
fn every_project_type_can_update_all_of_its_skills_with_one_script() {
    for (project_type, capabilities) in [(WEB, vec![ROUTER]), (RUST, vec![])] {
        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().join("app");
        compose_project(project_type, &capabilities, &dest);

        let script_path = dest.join("scripts/skills/update-skills.sh");
        assert!(script_path.is_file(), "{project_type}: the update script must reach the project");

        // The script is run directly, so composition has to preserve its mode.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&script_path).unwrap().permissions().mode();
            assert!(mode & 0o111 != 0, "{project_type}: update-skills.sh is not executable");
        }

        // The scaffolder tells the project which of its skills install
        // themselves, so the script drives their own CLI and `npx skills` does
        // the rest.
        let script = std::fs::read_to_string(&script_path).unwrap();
        assert!(
            script.contains(r#"SELF_INSTALLING_SKILLS="impeccable""#),
            "{project_type}: the self-installing skills were not substituted: {script}"
        );
        assert!(script.contains("npx --yes skills update"), "{project_type}: {script}");
    }
}

#[test]
fn every_web_capability_builds_before_it_type_checks() {
    for capability in [ROUTER, START] {
        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().join("app");
        compose_project(WEB, &[capability], &dest);

        let manifest = manifest_of(&dest);
        let build = manifest["scripts"]["build"].as_str().unwrap();

        // Both capabilities generate `src/routeTree.gen.ts` from their Vite
        // plugin, and every entry point imports it. Running `tsc` first means a
        // freshly scaffolded project cannot type-check or build at all.
        let vite_position = build.find("vite build").expect("build must run vite");
        let tsc_position = build.find("tsc").expect("build must run tsc");
        assert!(
            vite_position < tsc_position,
            "{capability}: vite build must precede tsc: {build}"
        );

        // `check` gets its type-checking from `build` for the same reason.
        let check = manifest["scripts"]["check"].as_str().unwrap();
        assert!(check.contains("pnpm build"), "{capability}: {check}");
        assert!(!check.contains("type-check"), "{capability}: {check}");
    }
}
