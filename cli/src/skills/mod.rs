//! Installing the agent skills a generated project gets.
//!
//! A generated project has no single type, so its skills are not a single
//! list. It has a set of **capability tags** (`typescript`, `tanstack-start`,
//! and so on) that its stack module declares in `module.json`, and it receives
//! the union of the lists those tags name in the template repository's
//! `skills-manifest.json`, plus the `global` list every project gets.
//!
//! Nothing is bundled into the project. Right after composition and before the
//! git init, the scaffolder runs one `npx skills add` inside the new project
//! per selected spec, and `npx skills` writes `.agents/skills`, the agent
//! symlinks, and the project's own `skills-lock.json` itself. A teammate who
//! clones the project later restores the same set through the project's own
//! `postinstall`.
//!
//! The work is split in three: [`manifest`] selects the specs for a set of
//! capabilities, [`commands`] turns each spec into the command that installs
//! it, and [`install`] runs them.

pub mod commands;
pub mod install;
pub mod manifest;
