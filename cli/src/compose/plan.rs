//! What to compose: the chosen tags, the names, and the layers they imply.
//!
//! The plan is the one place that knows where a tag's files live and how the
//! token map is stacked, so [`crate::compose::compose`] stays a list of steps.

use std::path::{Path, PathBuf};

use crate::catalog::capability::CAPABILITIES_DIR_NAME;
use crate::catalog::project_type::PROJECT_TYPES_DIR_NAME;
use crate::catalog::{Capability, ProjectType};
use crate::compose::tokens::Tokens;

/// Directory inside the template repository that holds every layer.
pub const TEMPLATES_DIR_NAME: &str = "templates";

/// The layer every project type shares.
pub const BASE_DIR_NAME: &str = "base";

/// The directory inside a layer whose contents are overlaid into the project.
pub const FILES_DIR_NAME: &str = "files";

/// The manifest a layer contributes to the merge.
pub const PACKAGE_JSON_FILE_NAME: &str = "package.json";

/// Inputs describing what to build and from where.
pub struct ComposePlan<'plan> {
    /// Root of the cloned template repository (the directory containing
    /// `templates/`).
    pub template_root: &'plan Path,
    /// The chosen project type, which supplies a layer and its tokens.
    pub project_type: &'plan ProjectType,
    /// The chosen capabilities, in the order they are overlaid.
    pub capabilities: &'plan [&'plan Capability],
    /// Display name of the project (used in titles and headings).
    pub project_name: &'plan str,
    /// npm-safe package name (used for `package.json` `name`).
    pub package_name: &'plan str,
    /// Tokens the caller computed rather than a tag declaring them, such as
    /// the self-installing skills the new project's own update script drives.
    /// They win over a tag's token of the same name.
    pub extra_tokens: Tokens,
}

impl ComposePlan<'_> {
    /// The `templates/` directory of the cloned repository.
    pub fn templates_dir(&self) -> PathBuf {
        self.template_root.join(TEMPLATES_DIR_NAME)
    }

    /// The base layer's directory.
    pub fn base_dir(&self) -> PathBuf {
        self.templates_dir().join(BASE_DIR_NAME)
    }

    /// The chosen project type's directory.
    pub fn project_type_dir(&self) -> PathBuf {
        self.templates_dir()
            .join(PROJECT_TYPES_DIR_NAME)
            .join(&self.project_type.slug)
    }

    /// Every `files/` directory to overlay, in order: the base layer, the
    /// project type, then each capability. Later layers win, which is how a
    /// capability owns a whole file its project type also ships.
    pub fn file_layers(&self) -> Vec<PathBuf> {
        let mut layers = vec![
            self.base_dir().join(FILES_DIR_NAME),
            self.project_type_dir().join(FILES_DIR_NAME),
        ];
        layers.extend(self.capability_dirs().map(|dir| dir.join(FILES_DIR_NAME)));
        layers
    }

    /// The project type's `package.json`, which is the merge base. A project
    /// type whose language has no such manifest (a Rust one) ships no file
    /// here, and the project is composed without one.
    pub fn package_json_base(&self) -> PathBuf {
        self.project_type_dir().join(PACKAGE_JSON_FILE_NAME)
    }

    /// The `package.json` fragments to merge onto the base, in order.
    pub fn package_json_fragments(&self) -> Vec<PathBuf> {
        self.capability_dirs()
            .map(|dir| dir.join(PACKAGE_JSON_FILE_NAME))
            .collect()
    }

    /// The token map: the project type's tokens, then each capability's (so a
    /// later capability wins), then the caller's extras, then the built-in
    /// names, which nothing may override.
    pub fn tokens(&self) -> Tokens {
        let mut tokens = self.project_type.tokens.clone();
        for capability in self.capabilities {
            tokens.extend(capability.tokens.clone());
        }
        tokens.extend(self.extra_tokens.clone());
        tokens.insert("PROJECT_NAME".to_string(), self.project_name.to_string());
        tokens.insert("PACKAGE_NAME".to_string(), self.package_name.to_string());
        tokens
    }

    /// Each chosen capability's directory, in the order they are overlaid.
    fn capability_dirs(&self) -> impl Iterator<Item = PathBuf> {
        let capabilities_root = self.templates_dir().join(CAPABILITIES_DIR_NAME);
        self.capabilities
            .iter()
            .map(move |capability| capabilities_root.join(&capability.slug))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::test_tags;

    fn tokens_of(pairs: &[(&str, &str)]) -> Tokens {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    fn plan<'plan>(
        project_type: &'plan ProjectType,
        capabilities: &'plan [&'plan Capability],
        extra_tokens: Tokens,
    ) -> ComposePlan<'plan> {
        ComposePlan {
            template_root: Path::new("/template"),
            project_type,
            capabilities,
            project_name: "My App",
            package_name: "my-app",
            extra_tokens,
        }
    }

    #[test]
    fn layers_run_base_then_project_type_then_capabilities_in_order() {
        let project_type = test_tags::project_type("typescript:web", "typescript");
        let router = test_tags::capability("tanstack-router");
        let prettier = test_tags::capability("prettier");
        let capabilities = vec![&router, &prettier];

        let layers = plan(&project_type, &capabilities, Tokens::new()).file_layers();

        let rendered: Vec<String> =
            layers.iter().map(|path| path.display().to_string()).collect();
        assert_eq!(
            rendered,
            vec![
                "/template/templates/base/files",
                "/template/templates/project-types/typescript-web/files",
                "/template/templates/capabilities/tanstack-router/files",
                "/template/templates/capabilities/prettier/files",
            ]
        );
    }

    #[test]
    fn the_merge_base_is_the_project_types_manifest_and_the_fragments_are_the_capabilities() {
        let project_type = test_tags::project_type("typescript:web", "typescript");
        let router = test_tags::capability("tanstack-router");
        let capabilities = vec![&router];
        let plan = plan(&project_type, &capabilities, Tokens::new());

        assert_eq!(
            plan.package_json_base().display().to_string(),
            "/template/templates/project-types/typescript-web/package.json"
        );
        assert_eq!(
            plan.package_json_fragments()
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<String>>(),
            vec!["/template/templates/capabilities/tanstack-router/package.json"]
        );
    }

    #[test]
    fn a_capability_token_wins_over_the_project_types() {
        let mut project_type = test_tags::project_type("typescript:web", "typescript");
        project_type.tokens = tokens_of(&[("STACK_LINE", "Vite"), ("KEEP", "kept")]);
        let mut router = test_tags::capability("tanstack-router");
        router.tokens = tokens_of(&[("STACK_LINE", "TanStack Router")]);
        let capabilities = vec![&router];

        let tokens = plan(&project_type, &capabilities, Tokens::new()).tokens();

        assert_eq!(tokens.get("STACK_LINE").unwrap(), "TanStack Router");
        assert_eq!(tokens.get("KEEP").unwrap(), "kept");
    }

    #[test]
    fn a_later_capability_token_wins_over_an_earlier_one() {
        let project_type = test_tags::project_type("typescript:web", "typescript");
        let mut first = test_tags::capability("first");
        first.tokens = tokens_of(&[("DEV_URL", "first")]);
        let mut second = test_tags::capability("second");
        second.tokens = tokens_of(&[("DEV_URL", "second")]);
        let capabilities = vec![&first, &second];

        let tokens = plan(&project_type, &capabilities, Tokens::new()).tokens();

        assert_eq!(tokens.get("DEV_URL").unwrap(), "second");
    }

    #[test]
    fn the_callers_tokens_win_over_every_tag_and_the_names_win_over_everything() {
        let mut project_type = test_tags::project_type("typescript:web", "typescript");
        project_type.tokens =
            tokens_of(&[("SELF_INSTALLING_SKILLS", "wrong"), ("PROJECT_NAME", "wrong")]);
        let capabilities: Vec<&Capability> = Vec::new();
        let extra = tokens_of(&[("SELF_INSTALLING_SKILLS", "impeccable"), ("PROJECT_NAME", "no")]);

        let tokens = plan(&project_type, &capabilities, extra).tokens();

        assert_eq!(tokens.get("SELF_INSTALLING_SKILLS").unwrap(), "impeccable");
        assert_eq!(tokens.get("PROJECT_NAME").unwrap(), "My App");
        assert_eq!(tokens.get("PACKAGE_NAME").unwrap(), "my-app");
    }
}
