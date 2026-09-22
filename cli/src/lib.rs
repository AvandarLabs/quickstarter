//! `quickstarter` builds a new front-end project by composing template layers.
//!
//! The tool is a thin client: at runtime it clones the template repository
//! (which holds a shared `base` layer plus one folder per stack `module`),
//! then assembles the chosen combination into the user's target directory.
//! Composing from layers, rather than cloning a whole template per stack
//! combination, means a shared dependency (Mantine, lint config, the theme)
//! lives in exactly one place and every generated project stays in sync.
//!
//! The three composition techniques live under [`compose`]:
//! - whole-file overlay for code that genuinely differs per stack,
//! - deep-merge for `package.json` dependency fragments,
//! - `{{TOKEN}}` substitution for files that are mostly shared but carry a
//!   few stack-specific lines.
//!
//! The agent skills a project gets are not composed but installed: [`skills`]
//! runs `npx skills` inside the finished project, for the capabilities its
//! stack module declares.

pub mod app;
pub mod catalog;
pub mod cli;
pub mod compose;
pub mod git_init;
pub mod skills;
pub mod template;
