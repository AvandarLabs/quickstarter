//! What a keystroke means. Pure: a key event in, an action out.
//!
//! Key handling is where a terminal UI hides its bugs, so it is deliberately
//! kept out of the event loop and tested directly. Adding a binding means
//! adding a variant and a test, never an `if` inside the loop.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// What the UI should do about a keystroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Leave the UI and return to the shell.
    Quit,
    /// Nothing is bound to this key.
    Ignore,
}

/// Decides what a keystroke means.
///
/// `q`, `Esc` and `Ctrl+C` all quit, because a person in a hurry will try
/// whichever one they know. Key releases (Windows terminals report them) are
/// never actions: otherwise every keypress would fire twice.
#[must_use]
pub const fn action_for(key: &KeyEvent) -> Action {
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return Action::Ignore;
    }
    match key.code {
        KeyCode::Char('c' | 'C') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        KeyCode::Char('q' | 'Q') | KeyCode::Esc => Action::Quit,
        _ => Action::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn q_quits() {
        assert_eq!(action_for(&press(KeyCode::Char('q'))), Action::Quit);
        assert_eq!(action_for(&press(KeyCode::Char('Q'))), Action::Quit);
    }

    #[test]
    fn escape_quits() {
        assert_eq!(action_for(&press(KeyCode::Esc)), Action::Quit);
    }

    #[test]
    fn control_c_quits() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(action_for(&key), Action::Quit);
    }

    #[test]
    fn a_plain_c_does_not_quit() {
        assert_eq!(action_for(&press(KeyCode::Char('c'))), Action::Ignore);
    }

    #[test]
    fn an_unbound_key_does_nothing() {
        assert_eq!(action_for(&press(KeyCode::Char('x'))), Action::Ignore);
        assert_eq!(action_for(&press(KeyCode::Up)), Action::Ignore);
    }

    #[test]
    fn a_key_release_is_not_a_second_press() {
        let release = KeyEvent::new_with_kind(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        );
        assert_eq!(action_for(&release), Action::Ignore);
    }
}
