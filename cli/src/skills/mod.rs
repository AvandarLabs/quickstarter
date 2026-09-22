//! Installing the agent skills a generated project gets.
//!
//! A generated project's skills are not one list. It carries tags of two
//! types: exactly one **project type** (`typescript:web`, `rust:cli`), which
//! brings the skills of its language and build system, and any number of
//! **capabilities** (`tanstack-router`), each bringing the skills of the
//! library it adds. The project receives the union of the lists those tags
//! name in the template repository's `skills-manifest.json`, plus the `global`
//! list every project gets.
//!
//! Nothing is bundled into the project. Right after composition and before the
//! git init, the scaffolder runs one `npx skills add` inside the new project
//! per selected spec, and `npx skills` writes `.agents/skills`, the agent
//! symlinks, and the project's own `skills-lock.json` itself. A teammate who
//! clones the project later restores the same set through the project's own
//! `postinstall`.
//!
//! The work is split in three: [`manifest`] selects the specs for a set of
//! tags, [`commands`] turns each spec into the command that installs it, and
//! [`install`] runs them.

pub mod commands;
pub mod install;
pub mod manifest;
