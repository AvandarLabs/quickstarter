//! End-to-end test of the composition engine against a fixture template tree.

#[path = "support/fixture_templates.rs"]
mod fixture_templates;

use std::path::Path;

use quickstarter::catalog::{self, Catalog, Selection};
use quickstarter::compose::{self, ComposePlan};

/// A template repository plus somewhere to compose it into.
struct Fixture {
    _temp: tempfile::TempDir,
    template_root: std::path::PathBuf,
    catalog: Catalog,
}

fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let template_root = temp.path().join("template");
    fixture_templates::write(&template_root);
    let catalog = catalog::discover(&template_root.join("templates")).unwrap();
    Fixture { _temp: temp, template_root, catalog }
}

/// Composes one project type and capability combination, the way `app.rs`
/// does.
fn compose_into(fixture: &Fixture, project_type_key: &str, capability_keys: &[&str], dest: &Path) {
    let project_type = fixture
        .catalog
        .project_types
        .iter()
        .find(|one| one.key == project_type_key)
        .expect("a project type with that key");
    let keys: Vec<String> = capability_keys.iter().map(|key| (*key).to_string()).collect();
    let selection =
        Selection::resolve(project_type, &keys, &fixture.catalog.capabilities).unwrap();

    let plan = ComposePlan {
        template_root: &fixture.template_root,
        project_type: selection.project_type,
        capabilities: &selection.capabilities,
        project_name: "My App",
        package_name: "my-app",
        extra_tokens: Default::default(),
    };
    compose::compose(&plan, dest).unwrap();
}

#[test]
fn composes_base_plus_project_type_plus_capability_with_merge_and_tokens() {
    let fixture = fixture();
    let dest = fixture.template_root.join("../out/my-app");
    compose_into(
        &fixture,
        fixture_templates::WEB_PROJECT_TYPE,
        &[fixture_templates::ROUTER_CAPABILITY],
        &dest,
    );

    // Base file present, with tokens substituted. The capability's token wins
    // over the project type's.
    let readme = std::fs::read_to_string(dest.join("README.md")).unwrap();
    assert_eq!(readme, "# My App\nTanStack Router");

    // Project type file overlaid on the base, capability file on top of that.
    assert!(dest.join("src/theme.ts").is_file());
    assert_eq!(
        std::fs::read_to_string(dest.join("vite.config.ts")).unwrap(),
        "// router vite"
    );

    // package.json deep-merged: project type + capability deps, name filled in.
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

#[test]
fn a_project_type_whose_manifest_is_a_cargo_toml_composes_without_a_package_json() {
    let fixture = fixture();
    let dest = fixture.template_root.join("../out/rust-app");
    compose_into(&fixture, fixture_templates::RUST_PROJECT_TYPE, &[], &dest);

    // Its own files arrived, tokens and all, and the base layer came with them.
    let manifest = std::fs::read_to_string(dest.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("name = \"my-app\""), "{manifest}");
    assert!(manifest.contains("# Error handling with context.\nanyhow = \"1\""), "{manifest}");
    assert!(dest.join("src/main.rs").is_file());
    assert_eq!(
        std::fs::read_to_string(dest.join("README.md")).unwrap(),
        "# My App\nRust CLI"
    );

    // No capability filled the seam, so its line is gone rather than blank.
    assert_eq!(
        std::fs::read_to_string(dest.join("src/lib.rs")).unwrap(),
        "pub mod cli;\npub mod theme;\n"
    );

    // The JSON merge is skipped entirely rather than producing an empty
    // manifest, and nothing from the other project type leaked in.
    assert!(!dest.join("package.json").exists());
    assert!(!dest.join("vite.config.ts").exists());
}

#[test]
fn a_capability_merges_its_cargo_toml_fragment_and_fills_the_seam_it_owns() {
    let fixture = fixture();
    let dest = fixture.template_root.join("../out/tui-app");
    compose_into(
        &fixture,
        fixture_templates::RUST_PROJECT_TYPE,
        &[fixture_templates::TUI_CAPABILITY],
        &dest,
    );

    // The fragment's dependency arrived with the comment saying why it is
    // there, and the project type's own dependencies and comments survived.
    let manifest = std::fs::read_to_string(dest.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("name = \"my-app\""), "{manifest}");
    assert!(manifest.contains("# Error handling with context.\nanyhow = \"1\""), "{manifest}");
    assert!(manifest.contains("# Terminal UI rendering.\nratatui = \"0.29\""), "{manifest}");

    // The seam is filled in place rather than removed, and the capability's
    // own file came with it.
    assert_eq!(
        std::fs::read_to_string(dest.join("src/lib.rs")).unwrap(),
        "pub mod cli;\npub mod tui;\npub mod theme;\n"
    );
    assert!(dest.join("src/tui.rs").is_file());
}

#[test]
fn composing_the_same_project_twice_produces_the_same_manifest_bytes() {
    let fixture = fixture();
    let first = fixture.template_root.join("../out/first");
    let second = fixture.template_root.join("../out/second");
    for dest in [&first, &second] {
        compose_into(
            &fixture,
            fixture_templates::RUST_PROJECT_TYPE,
            &[fixture_templates::TUI_CAPABILITY],
            dest,
        );
    }

    assert_eq!(
        std::fs::read_to_string(first.join("Cargo.toml")).unwrap(),
        std::fs::read_to_string(second.join("Cargo.toml")).unwrap()
    );
}

#[test]
fn a_different_capability_composes_a_different_project() {
    let fixture = fixture();
    let dest = fixture.template_root.join("../out/start-app");
    compose_into(
        &fixture,
        fixture_templates::WEB_PROJECT_TYPE,
        &[fixture_templates::START_CAPABILITY],
        &dest,
    );

    assert!(dest.join("src/router.tsx").is_file());
    // This capability overlays no vite config, so the project type's stands.
    assert_eq!(std::fs::read_to_string(dest.join("vite.config.ts")).unwrap(), "// web vite");
    assert_eq!(
        std::fs::read_to_string(dest.join("README.md")).unwrap(),
        "# My App\nTanStack Start"
    );

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dest.join("package.json")).unwrap()).unwrap();
    assert_eq!(manifest["dependencies"]["@tanstack/react-start"], "^1.0.0");
    assert!(manifest["dependencies"]["@tanstack/react-router"].is_null());
}

#[test]
fn a_project_type_composes_with_no_capability_at_all() {
    let fixture = fixture();
    let dest = fixture.template_root.join("../out/bare-app");
    compose_into(&fixture, fixture_templates::WEB_PROJECT_TYPE, &[], &dest);

    // Nothing overrode the project type, so its own token and file stand.
    assert_eq!(std::fs::read_to_string(dest.join("README.md")).unwrap(), "# My App\nVite");
    assert_eq!(std::fs::read_to_string(dest.join("vite.config.ts")).unwrap(), "// web vite");
}
