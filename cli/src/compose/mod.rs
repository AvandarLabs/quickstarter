//! Top-level composition: assemble a project from the template layers.
//!
//! The steps mirror the three composition techniques:
//! 1. overlay the base layer's files, then the project type's, then each
//!    chosen capability's;
//! 2. deep-merge the project type's `package.json` with each capability's
//!    fragment, skipping it entirely for a project type that has none;
//! 3. substitute `{{TOKEN}}`s (the tags' tokens plus the project's names).
//!
//! Composition writes authored files only. The agent skills a generated
//! project gets are installed afterwards, by the scaffolder running
//! `npx skills` inside the finished project (see `crate::skills`).

pub mod overlay;
pub mod package_json;
pub mod plan;
pub mod tokens;

use std::path::Path;

use anyhow::{Context, Result, bail};

pub use plan::{ComposePlan, TEMPLATES_DIR_NAME};

use plan::{FILES_DIR_NAME, PACKAGE_JSON_FILE_NAME};

/// Composes the project described by `plan` into `dest`.
///
/// `dest` must not already exist; the caller is responsible for that check so
/// it can fail fast before any network work.
pub fn compose(plan: &ComposePlan, dest: &Path) -> Result<()> {
    overlay_layers(plan, dest)?;
    write_package_json(plan, dest)?;
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

/// Writes the merged `package.json`, unless the project type has none.
///
/// A project type whose language does not use one (a Rust project type) must
/// compose cleanly and must not be handed an empty manifest, so a missing
/// merge base skips the step rather than failing it.
fn write_package_json(plan: &ComposePlan, dest: &Path) -> Result<()> {
    let base = plan.package_json_base();
    if !base.exists() {
        return Ok(());
    }

    let merged = package_json::merge_files(&base, &plan.package_json_fragments())
        .context("merging package.json")?;
    let path = dest.join(PACKAGE_JSON_FILE_NAME);
    std::fs::write(&path, merged).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::catalog::{Capability, ProjectType, test_tags};

    /// A template repository with one base file and one project type, whose
    /// `package.json` the caller decides to ship or not.
    fn template_repo(slug: &str, manifest: Option<&str>) -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("templates/base/files");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("README.md"), "# {{PROJECT_NAME}}").unwrap();

        let project_type = temp.path().join("templates/project-types").join(slug);
        std::fs::create_dir_all(project_type.join("files")).unwrap();
        std::fs::write(project_type.join("files/main.rs"), "fn main() {}").unwrap();
        if let Some(manifest) = manifest {
            std::fs::write(project_type.join("package.json"), manifest).unwrap();
        }
        temp
    }

    fn compose_into(root: &Path, project_type: &ProjectType) -> PathBuf {
        let dest = root.join("out/app");
        let capabilities: Vec<&Capability> = Vec::new();
        let plan = ComposePlan {
            template_root: root,
            project_type,
            capabilities: &capabilities,
            project_name: "My App",
            package_name: "my-app",
            extra_tokens: tokens::Tokens::new(),
        };
        compose(&plan, &dest).unwrap();
        dest
    }

    #[test]
    fn a_project_type_without_a_package_json_gets_none_at_all() {
        let mut project_type = test_tags::project_type("rust:cli", "rust");
        project_type.slug = "rust-cli".to_string();
        let temp = template_repo("rust-cli", None);

        let dest = compose_into(temp.path(), &project_type);

        assert!(dest.join("main.rs").is_file());
        assert!(
            !dest.join("package.json").exists(),
            "a Rust project must not be handed a package.json"
        );
    }

    #[test]
    fn a_project_type_with_a_package_json_gets_the_merged_one() {
        let mut project_type = test_tags::project_type("typescript:web", "typescript");
        project_type.slug = "typescript-web".to_string();
        let temp = template_repo("typescript-web", Some(r#"{ "name": "{{PACKAGE_NAME}}" }"#));

        let dest = compose_into(temp.path(), &project_type);

        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dest.join("package.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["name"], "my-app");
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
