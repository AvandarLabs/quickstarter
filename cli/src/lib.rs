//! `quickstarter` builds a new project by composing template layers.
//!
//! The tool is a thin client: at runtime it clones the template repository
//! (which holds a shared `base` layer, one folder per `project-type`, and one
//! per `capability`), then assembles the chosen combination into the user's
//! target directory. Composing from layers, rather than keeping a whole
//! template repository per combination, means a shared dependency (Mantine,
//! lint config, the theme) lives in exactly one place and every generated
//! project stays in sync.
//!
//! What a project can be is described by its tags ([`catalog`]): exactly one
//! project type (`typescript:web`, `rust:cli`), which decides the build system
//! and the files it starts from, plus any number of capabilities
//! (`tanstack-router`) layered on top. Each capability declares what it fits
//! and what it excludes, so the combinations that make sense are data in the
//! template repository rather than rules in this binary.
//!
//! The three composition techniques live under [`compose`]:
//! - whole-file overlay for code that genuinely differs per tag,
//! - deep-merge for dependency-manifest fragments, `package.json` or
//!   `Cargo.toml` depending on what the project type ships,
//! - `{{TOKEN}}` substitution for files that are mostly shared but carry a
//!   few tag-specific lines. A line holding nothing but a token that no tag
//!   filled in disappears, so a capability can inject one without leaving a
//!   blank line in the projects that skip it.
//!
//! The agent skills a project gets are not composed but installed: [`skills`]
//! runs `npx skills` inside the finished project, for the tags it carries.

pub mod app;
pub mod catalog;
pub mod cli;
pub mod compose;
pub mod git_init;
pub mod skills;
pub mod template;
