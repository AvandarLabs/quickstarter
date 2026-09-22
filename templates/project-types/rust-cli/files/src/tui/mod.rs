//! The terminal UI shell: set up the terminal, loop, put it back.
//!
//! This module moves bytes and nothing else. Every decision belongs to
//! [`keymap`], which is why there are no tests here and plenty there. It draws
//! on stderr so that stdout stays a clean data channel even while the UI is up.

pub mod keymap;

use std::io::{self, Stderr};

use anyhow::{Context, Result};
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Alignment;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::theme::Tone;
use keymap::Action;

/// The terminal this UI draws on.
type Screen = Terminal<CrosstermBackend<Stderr>>;

/// Opens the UI, runs it until the user quits, and restores the terminal
/// whether the loop ended well or badly.
pub fn run(title: &str) -> Result<()> {
    let mut screen = enter().context("opening the terminal UI")?;
    let outcome = event_loop(&mut screen, title);
    leave(&mut screen).context("restoring the terminal")?;
    outcome
}

fn enter() -> Result<Screen> {
    enable_raw_mode()?;
    let mut output = io::stderr();
    execute!(output, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(output))?)
}

fn leave(screen: &mut Screen) -> Result<()> {
    disable_raw_mode()?;
    execute!(screen.backend_mut(), LeaveAlternateScreen)?;
    screen.show_cursor()?;
    Ok(())
}

fn event_loop(screen: &mut Screen, title: &str) -> Result<()> {
    loop {
        screen.draw(|frame| draw(frame, title))?;
        if let Event::Key(key) = event::read()?
            && keymap::action_for(&key) == Action::Quit
        {
            return Ok(());
        }
    }
}

fn draw(frame: &mut ratatui::Frame, title: &str) {
    let block = Block::bordered().title(Span::styled(format!(" {title} "), tone(Tone::Heading)));
    let body = Paragraph::new(vec![
        Line::from(Span::styled(title, tone(Tone::Value))),
        Line::raw(""),
        Line::from(Span::styled("q, Esc or Ctrl+C quits", tone(Tone::Muted))),
    ])
    .block(block)
    .alignment(Alignment::Center);
    frame.render_widget(body, frame.area());
}

/// The one place the UI turns a semantic tone into a terminal color, so the
/// palette still lives in `theme.rs` alone.
const fn tone(tone: Tone) -> Style {
    Style::new().fg(Color::Indexed(tone.ansi_index()))
}
