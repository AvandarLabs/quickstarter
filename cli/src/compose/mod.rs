//! Top-level composition: assemble a project from the template layers.
//!
//! The steps mirror the three composition techniques:
//! 1. overlay the base layer's files, then the project type's, then each
//!    chosen capability's;
//! 2. deep-merge the project type's dependency manifest (`package.json` or
//!    `Cargo.toml`, whichever its layer ships) with each capability's
//!    fragment, skipping it entirely for a project type that has neither;
//! 3. substitute `{{TOKEN}}`s (the tags' tokens plus the project's names).
//!
//! Composition writes authored files only. The agent skills a generated
//! project gets are installed afterwards, by the scaffolder running
//! `npx skills` inside the finished project (see `crate::skills`).

pub mod cargo_toml;
pub mod overlay;
pub mod package_json;
pub mod plan;
pub mod tokens;

use std::path::Path;

use anyhow::{Context, Result, bail};

pub use plan::{ComposePlan, ManifestKind, TEMPLATES_DIR_NAME};

use plan::FILES_DIR_NAME;

/// Composes the project described by `plan` into `dest`.
///
/// `dest` must not already exist; the caller is responsible for that check so
/// it can fail fast before any network work.
pub fn compose(plan: &ComposePlan, dest: &Path) -> Result<()> {
    overlay_layers(plan, dest)?;
    write_manifest(plan, dest)?;
    tokens::substitute_in_tree(dest, &plan.tokens()).context("substituting tokens")?;
    Ok(())
}

/// Copies every layer's files into `dest`, in order, so a later layer wins.
///
/// A tag that ships no `files/` directory contributes nothing here, which is
/// how a capability can be a `package.json` fragment and a token or two. The
/// base layer is the exception: a repository without it is malformed.
fn overlay_layers(plan: &ComposePlan, dest: &Path) -> Result<()> {
    let base = plan.base_dir().join(FILES_DIR_NAME);
    if !base.is_dir() {
        bail!(
            "the template repository has no base layer at {}. It may be malformed.",
            base.display()
        );
    }

    for layer in plan.file_layers() {
        if !layer.is_dir() {
            continue;
        }
        overlay::copy_tree(&layer, dest)
            .with_context(|| format!("overlaying {}", layer.display()))?;
    }
    Ok(())
}

/// Writes the merged dependency manifest, unless the project type ships none.
///
/// Which merge runs is whichever manifest the project type's layer ships: a
/// `package.json` is deep-merged as JSON, a `Cargo.toml` as TOML with its
/// comments intact. A project type whose language uses neither must compose
/// cleanly and must not be handed an empty manifest, so shipping neither skips
/// the step rather than failing it.
fn write_manifest(plan: &ComposePlan, dest: &Path) -> Result<()> {
    let kind = plan.manifest_kind();
    refuse_foreign_manifests(plan, kind)?;
    let Some(kind) = kind else {
        return Ok(());
    };

    let base = plan.manifest_base(kind);
    let fragments = plan.manifest_fragments(kind);
    let merged = match kind {
        ManifestKind::PackageJson => package_json::merge_files(&base, &fragments),
        ManifestKind::CargoToml => cargo_toml::merge_files(&base, &fragments),
    }
    .with_context(|| format!("merging {}", kind.file_name()))?;

    let path = dest.join(kind.file_name());
    std::fs::write(&path, merged).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Refuses a capability whose manifest fragment this project's merge cannot
/// use, naming both the fragment and the manifest the project type actually
/// builds. Skipping it silently would cost the generated project a dependency
/// it needs, and the first sign of that would be a build failure.
fn refuse_foreign_manifests(plan: &ComposePlan, kind: Option<ManifestKind>) -> Result<()> {
    let foreign = plan.foreign_manifests(kind);
    let Some((capability, path)) = foreign.first() else {
        return Ok(());
    };
    let file = path.file_name().unwrap_or_default().to_string_lossy();
    match kind {
        Some(kind) => bail!(
            "the capability '{capability}' ships a {file} fragment, but the project type '{}' \
             builds its manifest from {}. Nothing would merge that fragment.",
            plan.project_type.key,
            kind.file_name()
        ),
        None => bail!(
            "the capability '{capability}' ships a {file} fragment, but the project type '{}' \
             ships no manifest to merge it onto.",
            plan.project_type.key
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::catalog::{Capability, ProjectType, test_tags};

    /// A template repository with one base file and one project type, whose
    /// manifest the caller decides to ship or not, and of which kind.
    fn template_repo(slug: &str, manifest: Option<(&str, &str)>) -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("templates/base/files");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("README.md"), "# {{PROJECT_NAME}}").unwrap();

        let project_type = temp.path().join("templates/project-types").join(slug);
        std::fs::create_dir_all(project_type.join("files")).unwrap();
        std::fs::write(project_type.join("files/main.rs"), "fn main() {}").unwrap();
        if let Some((file, contents)) = manifest {
            std::fs::write(project_type.join(file), contents).unwrap();
        }
        temp
    }

    /// Adds a capability shipping one manifest fragment and nothing else.
    fn fragment_capability(root: &Path, slug: &str, file: &str, contents: &str) {
        let dir = root.join("templates/capabilities").join(slug);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(file), contents).unwrap();
    }

    fn compose_with(
        root: &Path,
        project_type: &ProjectType,
        capabilities: &[&Capability],
    ) -> Result<PathBuf> {
        let dest = root.join("out/app");
        let plan = ComposePlan {
            template_root: root,
            project_type,
            capabilities,
            project_name: "My App",
            package_name: "my-app",
            extra_tokens: tokens::Tokens::new(),
        };
        compose(&plan, &dest)?;
        Ok(dest)
    }

    fn compose_into(root: &Path, project_type: &ProjectType) -> PathBuf {
        compose_with(root, project_type, &[]).unwrap()
    }

    fn rust_project_type() -> ProjectType {
        let mut project_type = test_tags::project_type("rust:cli", "rust");
        project_type.slug = "rust-cli".to_string();
        project_type
    }

    #[test]
    fn a_project_type_without_a_manifest_gets_none_at_all() {
        let temp = template_repo("rust-cli", None);

        let dest = compose_into(temp.path(), &rust_project_type());

        assert!(dest.join("main.rs").is_file());
        assert!(
            !dest.join("package.json").exists(),
            "a project type that ships no manifest must not be handed one"
        );
        assert!(!dest.join("Cargo.toml").exists());
    }

    #[test]
    fn a_project_type_with_a_package_json_gets_the_merged_one() {
        let mut project_type = test_tags::project_type("typescript:web", "typescript");
        project_type.slug = "typescript-web".to_string();
        let temp =
            template_repo("typescript-web", Some(("package.json", r#"{ "name": "{{PACKAGE_NAME}}" }"#)));

        let dest = compose_into(temp.path(), &project_type);

        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dest.join("package.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["name"], "my-app");
    }

    #[test]
    fn a_project_type_with_a_cargo_toml_gets_the_merged_one() {
        let temp = template_repo(
            "rust-cli",
            Some((
                "Cargo.toml",
                "[package]\nname = \"{{PACKAGE_NAME}}\"\n\n[dependencies]\n# Errors.\nanyhow = \"1\"\n",
            )),
        );
        fragment_capability(
            temp.path(),
            "rust-tui",
            "Cargo.toml",
            "[dependencies]\n# Terminal UI.\nratatui = \"0.29\"\n",
        );
        let tui = test_tags::capability("rust-tui");

        let dest = compose_with(temp.path(), &rust_project_type(), &[&tui]).unwrap();

        let manifest = std::fs::read_to_string(dest.join("Cargo.toml")).unwrap();
        assert!(manifest.contains("name = \"my-app\""), "{manifest}");
        assert!(manifest.contains("# Errors.\nanyhow = \"1\""), "{manifest}");
        assert!(manifest.contains("# Terminal UI.\nratatui = \"0.29\""), "{manifest}");
        assert!(!dest.join("package.json").exists());
    }

    #[test]
    fn a_capability_fragment_of_the_wrong_kind_is_refused_rather_than_skipped() {
        let temp = template_repo("rust-cli", Some(("Cargo.toml", "[package]\nname = \"a\"\n")));
        fragment_capability(temp.path(), "prettier", "package.json", r#"{ "name": "x" }"#);
        let prettier = test_tags::capability("prettier");

        let error = compose_with(temp.path(), &rust_project_type(), &[&prettier])
            .unwrap_err()
            .to_string();

        assert!(error.contains("prettier"), "{error}");
        assert!(error.contains("package.json"), "{error}");
        assert!(error.contains("Cargo.toml"), "{error}");
    }

    #[test]
    fn a_fragment_is_refused_when_the_project_type_has_no_manifest_to_merge_it_onto() {
        let temp = template_repo("rust-cli", None);
        fragment_capability(temp.path(), "rust-tui", "Cargo.toml", "[dependencies]\na = \"1\"\n");
        let tui = test_tags::capability("rust-tui");

        let error = compose_with(temp.path(), &rust_project_type(), &[&tui])
            .unwrap_err()
            .to_string();

        assert!(error.contains("no manifest to merge it onto"), "{error}");
    }

    #[test]
    fn a_repository_with_no_base_layer_is_refused() {
        let mut project_type = test_tags::project_type("rust:cli", "rust");
        project_type.slug = "rust-cli".to_string();
        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().join("out/app");
        let capabilities: Vec<&Capability> = Vec::new();
        let plan = ComposePlan {
            template_root: temp.path(),
            project_type: &project_type,
            capabilities: &capabilities,
            project_name: "My App",
            package_name: "my-app",
            extra_tokens: tokens::Tokens::new(),
        };

        let error = compose(&plan, &dest).unwrap_err().to_string();

        assert!(error.contains("no base layer"), "{error}");
    }
}
