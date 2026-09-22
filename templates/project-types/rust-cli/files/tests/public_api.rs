//! The library surface, exercised from outside the crate.
//!
//! The binary's name is decided when the project is created, so these tests
//! drive the library rather than spawning `target/debug/<binary>`. Everything
//! `main.rs` calls is public and reachable here, which is the point of keeping
//! the binary thin.

use app::cli::{Cli, Command};
use app::commands::greet::greeting;
use app::commands::info::facts;
use app::theme::{Theme, Tone};
use app::tui::keymap::{Action, action_for};
use clap::{CommandFactory, Parser};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn the_command_line_surface_is_well_formed() {
    Cli::command().debug_assert();
}

#[test]
fn every_action_is_reachable_without_a_prompt() {
    let greet = Cli::try_parse_from(["tool", "greet", "--name", "Ada"]).expect("greet parses");
    assert_eq!(
        greet.command,
        Some(Command::Greet {
            name: Some("Ada".to_owned())
        })
    );
    let info = Cli::try_parse_from(["tool", "info"]).expect("info parses");
    assert_eq!(info.command, Some(Command::Info));
    let ui = Cli::try_parse_from(["tool", "ui", "--verbose"]).expect("ui parses");
    assert_eq!(ui.command, Some(Command::Ui));
    assert!(ui.verbose);
}

#[test]
fn the_ui_quits_on_every_documented_key() {
    let quits = [
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    ];
    for key in quits {
        assert_eq!(action_for(&key), Action::Quit, "{key:?} should quit");
    }
}

#[test]
fn a_greeting_names_the_person() {
    assert_eq!(greeting("Ada"), "Hello, Ada!");
}

#[test]
fn info_reports_the_running_version() {
    assert!(facts().iter().any(|(key, _)| *key == "version"));
}

#[test]
fn piped_output_carries_no_escape_sequences() {
    let plain = Theme::dark(false);
    assert_eq!(plain.paint(Tone::Success, "done"), "done");
}
