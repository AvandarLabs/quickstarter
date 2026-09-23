//! The terminal UI, driven from outside the crate and without a terminal.
//!
//! Every decision the UI makes is a pure function, so this file plays whole
//! sessions by hand: classify a key, apply the action, look at the state.
//! Nothing here opens a screen, which is the point of keeping `tui::mod` a
//! shell with no decisions in it.

use app::tui::command::{CATALOG, Command};
use app::tui::keymap::{Action, action_for};
use app::tui::palette::{Palette, matches_query};
use app::tui::shortcuts;
use app::tui::state::{Overlay, State, update};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// One turn of the event loop: exactly what `tui::mod` does with a key.
fn press(state: State, key: KeyEvent) -> State {
    let action = action_for(state.overlay(), &key);
    update(state, action)
}

const fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

const fn control(letter: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(letter), KeyModifiers::CONTROL)
}

const fn alt(letter: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(letter), KeyModifiers::ALT)
}

fn typed(state: State, text: &str) -> State {
    let mut state = state;
    for letter in text.chars() {
        state = press(state, key(KeyCode::Char(letter)));
    }
    state
}

#[test]
fn a_command_can_be_run_without_ever_learning_its_shortcut() {
    // The palette is the whole point: type what you want, press Enter.
    let state = press(State::new(), control('p'));
    assert_eq!(state.overlay(), Overlay::Palette);
    let state = typed(state, "details hide");
    assert_eq!(state.palette().visible_rows().len(), 1);
    let state = press(state, key(KeyCode::Enter));
    assert_eq!(state.overlay(), Overlay::None);
    assert!(!state.details_visible());
}

#[test]
fn what_a_key_means_depends_on_what_is_open() {
    let closed = State::new();
    assert_eq!(
        action_for(closed.overlay(), &control('p')),
        Action::OpenPalette
    );
    let open = press(closed, control('p'));
    assert_eq!(
        action_for(open.overlay(), &control('p')),
        Action::SelectPrevious
    );
    // A letter is a binding on the main view and a query letter in the palette.
    assert_eq!(
        action_for(Overlay::None, &key(KeyCode::Char('q'))),
        Action::Ignore
    );
    assert_eq!(
        action_for(Overlay::Palette, &key(KeyCode::Char('q'))),
        Action::Type('q')
    );
}

#[test]
fn escape_closes_an_overlay_without_ending_the_session() {
    let state = press(State::new(), alt('s'));
    assert_eq!(state.overlay(), Overlay::Shortcuts);
    let state = press(state, key(KeyCode::Esc));
    assert_eq!(state.overlay(), Overlay::None);
    assert!(state.running());
}

#[test]
fn control_q_leaves_from_wherever_the_user_is() {
    for opening in [None, Some(control('p')), Some(alt('s'))] {
        let state = opening.map_or_else(State::new, |key| press(State::new(), key));
        let state = press(state, control('q'));
        assert!(!state.running(), "Ctrl+Q was swallowed");
    }
}

#[test]
fn the_palette_filter_takes_the_query_words_in_any_order() {
    assert!(matches_query(
        "show shortcut",
        "2. Show every keyboard shortcut"
    ));
    assert!(matches_query(
        "shortcut show",
        "2. Show every keyboard shortcut"
    ));
    assert!(!matches_query(
        "show details",
        "2. Show every keyboard shortcut"
    ));
}

#[test]
fn the_selection_wraps_rather_than_stopping_at_the_ends() {
    let state = press(State::new(), control('p'));
    assert_eq!(state.palette().selected(), 0);
    let state = press(state, key(KeyCode::Up));
    assert_eq!(state.palette().selected(), CATALOG.len() - 1);
    let state = press(state, key(KeyCode::Down));
    assert_eq!(state.palette().selected(), 0);
}

#[test]
fn every_shortcut_that_runs_something_appears_in_the_palette() {
    // The same guard `src/tui/shortcuts.rs` carries, checked here through the
    // public surface: the palette is the parent set of the shortcuts, so no
    // feature is reachable by keystroke alone.
    let listed: Vec<Command> = Palette::from_catalog()
        .rows()
        .iter()
        .map(|row| row.command)
        .collect();
    for shortcut in shortcuts::ALL {
        for command in shortcut.commands {
            assert!(
                listed.contains(command),
                "{} runs {command:?}, which the palette does not list",
                shortcut.keys
            );
        }
    }
}

#[test]
fn every_command_the_palette_lists_can_be_run_from_it() {
    for command in CATALOG {
        let state = press(State::new(), control('p'));
        let state = typed(state, command.label());
        assert_eq!(
            state.palette().selected_command(),
            Some(*command),
            "typing {:?} does not select it",
            command.label()
        );
    }
}
