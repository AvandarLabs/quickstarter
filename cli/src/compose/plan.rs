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

/// The manifest a JavaScript layer contributes to the merge.
pub const PACKAGE_JSON_FILE_NAME: &str = "package.json";

/// The manifest a Rust layer contributes to the merge.
pub const CARGO_TOML_FILE_NAME: &str = "Cargo.toml";

/// The dependency manifest a project type builds its project from, and so the
/// merge composition runs.
///
/// It is detected from what the project type's layer ships rather than
/// declared, which is what lets a project type opt into a merge simply by
/// putting its language's manifest at the root of its layer. A project type
/// that ships neither composes with no manifest at all, because a language
/// without one must not be handed an empty file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestKind {
    /// `package.json`, deep-merged as JSON by
    /// [`crate::compose::package_json`].
    PackageJson,
    /// `Cargo.toml`, merged as TOML by [`crate::compose::cargo_toml`], which
    /// keeps the comment above every dependency.
    CargoToml,
}

impl ManifestKind {
    /// Every kind there is, in the order a layer is searched for one.
    pub const ALL: [ManifestKind; 2] = [ManifestKind::PackageJson, ManifestKind::CargoToml];

    /// The file a layer ships to take part in this merge. The names live here,
    /// with the rest of the layout, rather than in the merges themselves.
    pub fn file_name(self) -> &'static str {
        match self {
            ManifestKind::PackageJson => PACKAGE_JSON_FILE_NAME,
            ManifestKind::CargoToml => CARGO_TOML_FILE_NAME,
        }
    }
}

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

    /// The manifest the chosen project type ships, or `None` when its language
    /// has none and the project is composed without one.
    pub fn manifest_kind(&self) -> Option<ManifestKind> {
        let dir = self.project_type_dir();
        ManifestKind::ALL
            .into_iter()
            .find(|kind| dir.join(kind.file_name()).is_file())
    }

    /// The project type's manifest of `kind`, which is the merge base.
    pub fn manifest_base(&self, kind: ManifestKind) -> PathBuf {
        self.project_type_dir().join(kind.file_name())
    }

    /// The fragments of `kind` to merge onto the base, in order. A capability
    /// that ships none contributes nothing, so the path need not exist.
    pub fn manifest_fragments(&self, kind: ManifestKind) -> Vec<PathBuf> {
        self.capability_dirs().map(|dir| dir.join(kind.file_name())).collect()
    }

    /// Every manifest a chosen capability ships that a project expecting
    /// `expected` cannot merge, as (capability key, path).
    ///
    /// A `Cargo.toml` fragment on a `package.json` project (or any fragment at
    /// all when the project type ships no manifest) is an authoring mistake
    /// whose only symptom would be a dependency quietly missing from the
    /// generated project, so composition refuses it instead.
    pub fn foreign_manifests(&self, expected: Option<ManifestKind>) -> Vec<(&str, PathBuf)> {
        let mut foreign = Vec::new();
        for (capability, dir) in self.capabilities.iter().zip(self.capability_dirs()) {
            for kind in ManifestKind::ALL {
                let path = dir.join(kind.file_name());
                if Some(kind) != expected && path.is_file() {
                    foreign.push((capability.key.as_str(), path));
                }
            }
        }
        foreign
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
            plan.manifest_base(ManifestKind::PackageJson).display().to_string(),
            "/template/templates/project-types/typescript-web/package.json"
        );
        assert_eq!(
            plan.manifest_fragments(ManifestKind::PackageJson)
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<String>>(),
            vec!["/template/templates/capabilities/tanstack-router/package.json"]
        );
    }

    #[test]
    fn a_cargo_manifest_has_the_same_shape_as_a_package_one() {
        let project_type = test_tags::project_type("rust:cli", "rust");
        let tui = test_tags::capability("rust-tui");
        let capabilities = vec![&tui];
        let plan = plan(&project_type, &capabilities, Tokens::new());

        assert_eq!(
            plan.manifest_base(ManifestKind::CargoToml).display().to_string(),
            "/template/templates/project-types/rust-cli/Cargo.toml"
        );
        assert_eq!(
            plan.manifest_fragments(ManifestKind::CargoToml)
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<String>>(),
            vec!["/template/templates/capabilities/rust-tui/Cargo.toml"]
        );
    }

    /// A template repository on disk, because which merge runs is read from
    /// what a layer actually ships.
    struct Layers {
        temp: tempfile::TempDir,
    }

    impl Layers {
        fn new() -> Layers {
            Layers { temp: tempfile::tempdir().unwrap() }
        }

        /// Puts `file` at the root of a layer's directory.
        fn ship(&self, dir: &str, slug: &str, file: &str) -> &Layers {
            let layer = self.temp.path().join("templates").join(dir).join(slug);
            std::fs::create_dir_all(&layer).unwrap();
            std::fs::write(layer.join(file), "").unwrap();
            self
        }
    }

    fn rooted_plan<'plan>(
        root: &'plan Path,
        project_type: &'plan ProjectType,
        capabilities: &'plan [&'plan Capability],
    ) -> ComposePlan<'plan> {
        ComposePlan {
            template_root: root,
            project_type,
            capabilities,
            project_name: "My App",
            package_name: "my-app",
            extra_tokens: Tokens::new(),
        }
    }

    #[test]
    fn a_project_type_shipping_a_package_json_merges_json() {
        let layers = Layers::new();
        layers.ship("project-types", "typescript-web", "package.json");
        let project_type = test_tags::project_type("typescript:web", "typescript");

        let plan = rooted_plan(layers.temp.path(), &project_type, &[]);

        assert_eq!(plan.manifest_kind(), Some(ManifestKind::PackageJson));
    }

    #[test]
    fn a_project_type_shipping_a_cargo_toml_merges_toml() {
        let layers = Layers::new();
        layers.ship("project-types", "rust-cli", "Cargo.toml");
        let project_type = test_tags::project_type("rust:cli", "rust");

        let plan = rooted_plan(layers.temp.path(), &project_type, &[]);

        assert_eq!(plan.manifest_kind(), Some(ManifestKind::CargoToml));
    }

    #[test]
    fn a_project_type_shipping_neither_merges_nothing() {
        let layers = Layers::new();
        layers.ship("project-types", "rust-cli", "project-type.json");
        let project_type = test_tags::project_type("rust:cli", "rust");

        let plan = rooted_plan(layers.temp.path(), &project_type, &[]);

        assert_eq!(plan.manifest_kind(), None);
    }

    #[test]
    fn a_capability_fragment_of_another_kind_is_reported_as_foreign() {
        let layers = Layers::new();
        layers.ship("project-types", "rust-cli", "Cargo.toml");
        layers.ship("capabilities", "prettier", "package.json");
        let project_type = test_tags::project_type("rust:cli", "rust");
        let prettier = test_tags::capability("prettier");
        let capabilities = vec![&prettier];

        let plan = rooted_plan(layers.temp.path(), &project_type, &capabilities);
        let foreign = plan.foreign_manifests(Some(ManifestKind::CargoToml));

        assert_eq!(foreign.len(), 1, "{foreign:?}");
        assert_eq!(foreign[0].0, "prettier");
        assert!(foreign[0].1.ends_with("prettier/package.json"), "{foreign:?}");
    }

    #[test]
    fn a_capability_fragment_of_the_projects_own_kind_is_not_foreign() {
        let layers = Layers::new();
        layers.ship("project-types", "rust-cli", "Cargo.toml");
        layers.ship("capabilities", "rust-tui", "Cargo.toml");
        let project_type = test_tags::project_type("rust:cli", "rust");
        let tui = test_tags::capability("rust-tui");
        let capabilities = vec![&tui];

        let plan = rooted_plan(layers.temp.path(), &project_type, &capabilities);

        assert!(plan.foreign_manifests(Some(ManifestKind::CargoToml)).is_empty());
    }

    #[test]
    fn every_fragment_is_foreign_to_a_project_type_with_no_manifest_at_all() {
        let layers = Layers::new();
        layers.ship("project-types", "rust-cli", "project-type.json");
        layers.ship("capabilities", "rust-tui", "Cargo.toml");
        let project_type = test_tags::project_type("rust:cli", "rust");
        let tui = test_tags::capability("rust-tui");
        let capabilities = vec![&tui];

        let plan = rooted_plan(layers.temp.path(), &project_type, &capabilities);

        assert_eq!(plan.foreign_manifests(None).len(), 1);
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
