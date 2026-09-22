//! {{PROJECT_NAME}}: the library half of the command-line tool.
//!
//! The binary (`src/main.rs`) parses the arguments and hands them here, so
//! every decision this tool makes is reachable from a test without spawning a
//! process. Modules follow the pure/impure split described in
//! `docs/architecture.md`: `cli` and `theme` are pure data and formatting,
//! `commands` and `tui` are the thin shells that touch the terminal.

pub mod cli;
pub mod commands;
pub mod theme;
pub mod tui;

/// The product name shown to people, in help text and in the terminal UI. The
/// crate name (`CARGO_PKG_NAME`) is what a machine types; this is what a
/// person reads.
pub const APP_NAME: &str = "{{PROJECT_NAME}}";
