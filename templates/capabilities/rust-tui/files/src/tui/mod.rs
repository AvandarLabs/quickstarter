//! The terminal UI: set the terminal up, loop, put it back.
//!
//! This module moves bytes and nothing else. It reads a key, hands it to
//! [`keymap`] to classify, hands the action to [`state::update`], and draws
//! the result with [`render`]. Every decision lives in one of those three, so
//! there is nothing here a test would want to assert and plenty there.
//!
//! The terminal is borrowed, not owned: raw mode and the alternate screen are
//! the user's shell, and this module gives them back on every exit path, a
//! panic included. That is what [`TerminalGuard`] and the panic hook are for.

pub mod command;
pub mod keymap;
pub mod palette;
pub mod render;
pub mod shortcuts;
pub mod state;

use std::io::{self, Stderr};
use std::sync::Once;

use anyhow::{Context, Result};
use crossterm::cursor::Show;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::theme::Theme;
use state::State;

/// The terminal this UI draws on. Stderr, not stdout: stdout carries data,
/// and piping the tool must not send it a screenful of escape sequences.
type Screen = Terminal<CrosstermBackend<Stderr>>;

/// Opens the UI, runs it until the user quits, and restores the terminal
/// whether the loop ended well, badly, or not at all.
pub fn run(theme: Theme) -> Result<()> {
    install_panic_hook();
    let (_guard, mut screen) = enter().context("opening the terminal UI")?;
    event_loop(&mut screen, theme)
}

/// Owns the borrowed terminal. Dropping it puts the terminal back, so an
/// early `?` cannot leave a shell in raw mode with no cursor.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Nothing useful can be done about a failure here: the UI is already
        // on its way out, and the error would be printed onto the screen this
        // call was meant to restore.
        let _ = restore();
    }
}

/// Takes the terminal: raw mode on, alternate screen entered, renderer built.
/// The guard is created first, so every failure after it still restores.
fn enter() -> Result<(TerminalGuard, Screen)> {
    enable_raw_mode().context("switching the terminal to raw mode")?;
    let guard = TerminalGuard;
    let mut output = io::stderr();
    execute!(output, EnterAlternateScreen).context("entering the alternate screen")?;
    let screen = Terminal::new(CrosstermBackend::new(output)).context("starting the renderer")?;
    Ok((guard, screen))
}

/// Gives the terminal back exactly as it was found.
fn restore() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stderr(), LeaveAlternateScreen, Show)
}

/// Restores the terminal before a panic prints anything.
///
/// Without this, a crash leaves the alternate screen up and raw mode on: the
/// message lands on a screen nobody sees, and the shell it returns to needs a
/// blind `reset`. The hook is installed once however often the UI is opened,
/// and it still runs the hook it replaced, so the backtrace is not lost.
fn install_panic_hook() {
    static INSTALLED: Once = Once::new();
    INSTALLED.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            let _ = restore();
            previous(panic);
        }));
    });
}

/// Read a key, classify it, apply it, draw. The whole loop.
fn event_loop(screen: &mut Screen, theme: Theme) -> Result<()> {
    let mut state = State::new();
    while state.running() {
        screen
            .draw(|frame| render::draw(frame, &state, theme))
            .context("drawing the terminal UI")?;
        if let Event::Key(key) = event::read().context("reading a key")? {
            let action = keymap::action_for(state.overlay(), &key);
            state = state::update(state, action);
        }
    }
    Ok(())
}
