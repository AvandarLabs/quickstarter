//! Top-level composition: assemble a project from the template layers.
//!
//! The steps mirror the three composition techniques:
//! 1. overlay the base layer's files, then the module's files on top;
//! 2. deep-merge the base and module `package.json` fragments;
//! 3. substitute `{{TOKEN}}`s (project name plus the module's own tokens).
//!
//! A fourth step seeds `skills-lock.json` with the curated agent skills a
//! generated project should receive. It runs after substitution because the
//! lock is generated data, not an authored template file.

pub mod overlay;
pub mod package_json;
pub mod skills_lock;
pub mod tokens;

use std::path::Path;

use anyhow::{Context, Result};

use crate::catalog::Module;
use crate::compose::tokens::Tokens;

/// Inputs describing what to build and from where.
pub struct ComposePlan<'plan> {
    /// Root of the cloned template repository (the directory containing
    /// `templates/`).
    pub template_root: &'plan Path,
    /// The chosen module (identifies which `templates/modules/<key>` to use
    /// and supplies its tokens).
    pub module: &'plan Module,
    /// Display name of the project (used in titles and headings).
    pub project_name: &'plan str,
    /// npm-safe package name (used for `package.json` `name`).
    pub package_name: &'plan str,
}

/// Composes the project described by `plan` into `dest`.
///
/// `dest` must not already exist; the caller is responsible for that check so
/// it can fail fast before any network work.
pub fn compose(plan: &ComposePlan, dest: &Path) -> Result<()> {
    let templates = plan.template_root.join("templates");
    let base = templates.join("base");
    let module_dir = templates.join("modules").join(&plan.module.key);

    overlay::copy_tree(&base.join("files"), dest).context("overlaying base layer")?;
    overlay::copy_tree(&module_dir.join("files"), dest)
        .with_context(|| format!("overlaying module '{}'", plan.module.key))?;

    let manifest = package_json::merge_files(
        &base.join("package.json"),
        &module_dir.join("package.json"),
    )
    .context("merging package.json")?;
    std::fs::write(dest.join("package.json"), manifest)
        .with_context(|| format!("writing {}", dest.join("package.json").display()))?;

    let tokens = build_tokens(plan);
    tokens::substitute_in_tree(dest, &tokens).context("substituting tokens")?;

    skills_lock::write_produced_lock(plan.template_root, dest)
        .context("writing skills-lock.json")?;
    Ok(())
}

/// Builds the token map: the module's declared tokens plus the built-in
/// project and package names.
fn build_tokens(plan: &ComposePlan) -> Tokens {
    let mut tokens = plan.module.tokens.clone();
    tokens.insert("PROJECT_NAME".to_string(), plan.project_name.to_string());
    tokens.insert("PACKAGE_NAME".to_string(), plan.package_name.to_string());
    tokens
}
