//! Drawing, as a function of the state it is handed.
//!
//! Nothing here decides anything: every branch below reads the state and
//! nothing else, so what the screen shows is settled before this module runs.
//! That is what makes the UI testable without a terminal, and it is why the
//! only tests here are for the two pure helpers that do arithmetic.
//!
//! Color goes through [`Tone`], the same semantic palette the command-line
//! half of this tool paints with, so this file never writes an escape
//! sequence or names a color of its own.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::APP_NAME;
use crate::theme::{Theme, Tone};

use super::palette::{Palette, Row};
use super::shortcuts::{self, Group};
use super::state::{Overlay, State};

/// Draws the whole frame: the main view, the footer, and whichever overlay is
/// open on top of them.
pub fn draw(frame: &mut Frame, state: &State, theme: Theme) {
    let screen = frame.area();
    let [body, footer] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(screen);
    draw_body(frame, state, theme, body);
    frame.render_widget(footer_bar(state, theme), footer);
    match state.overlay() {
        Overlay::None => {}
        Overlay::Palette => draw_palette(frame, state.palette(), theme, screen),
        Overlay::Shortcuts => draw_shortcuts(frame, theme, screen),
    }
}

/// The main pane, with the details panel beside it when it is showing.
fn draw_body(frame: &mut Frame, state: &State, theme: Theme, area: Rect) {
    if state.details_visible() {
        let [main, details] =
            Layout::horizontal([Constraint::Min(24), Constraint::Length(30)]).areas(area);
        frame.render_widget(main_pane(theme), main);
        frame.render_widget(details_pane(theme), details);
    } else {
        frame.render_widget(main_pane(theme), area);
    }
}

/// What the tool has to say for itself, in a box carrying its name.
fn main_pane(theme: Theme) -> Paragraph<'static> {
    let block = Block::bordered().title(Span::styled(
        format!(" {APP_NAME} "),
        style(theme, Tone::Heading),
    ));
    Paragraph::new(vec![
        Line::raw(""),
        Line::from(Span::styled(
            "A terminal UI worth building on.",
            style(theme, Tone::Value),
        )),
        Line::raw(""),
        hint_line(theme, "Ctrl+P", "every command, in one palette"),
        hint_line(theme, "Alt+S", "every keyboard shortcut"),
        Line::raw(""),
        Line::from(Span::styled(
            "This pane is a function of the state and nothing else. Put your own content here.",
            style(theme, Tone::Muted),
        )),
    ])
    .block(block)
    .wrap(Wrap { trim: true })
}

/// One "these keys reach that" line of the main pane. The keys go in a column
/// of their own, so the eye can run down them.
fn hint_line(theme: Theme, keys: &'static str, text: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{keys:<9}"), style(theme, Tone::Accent)),
        Span::styled(text, style(theme, Tone::Value)),
    ])
}

/// Facts about this build, as `key  value` rows. A panel that is worth
/// toggling has to say something: fill it with what your tool knows.
fn details_pane(theme: Theme) -> Paragraph<'static> {
    let block = Block::bordered().title(Span::styled(" Details ", style(theme, Tone::Heading)));
    let facts = [
        ("name", APP_NAME),
        ("package", env!("CARGO_PKG_NAME")),
        ("version", env!("CARGO_PKG_VERSION")),
    ];
    let lines: Vec<Line<'static>> = facts
        .into_iter()
        .map(|(key, value)| {
            Line::from(vec![
                Span::styled(format!("{key:<9}"), style(theme, Tone::Muted)),
                Span::styled(value, style(theme, Tone::Value)),
            ])
        })
        .collect();
    Paragraph::new(lines).block(block)
}

/// The one-line footer: what just happened, then the keys worth remembering.
fn footer_bar(state: &State, theme: Theme) -> Paragraph<'static> {
    let mut spans = vec![Span::raw(" ")];
    if let Some(status) = state.status() {
        spans.push(Span::styled(status.to_owned(), style(theme, Tone::Info)));
        spans.push(Span::raw("  "));
    }
    for shortcut in shortcuts::footer() {
        spans.push(Span::styled(shortcut.keys, style(theme, Tone::Accent)));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(shortcut.label, style(theme, Tone::Muted)));
        spans.push(Span::raw("   "));
    }
    Paragraph::new(Line::from(spans))
}

/// The command palette, floating over the main view.
fn draw_palette(frame: &mut Frame, palette: &Palette, theme: Theme, screen: Rect) {
    let rows = palette.visible_rows();
    let listed = u16::try_from(rows.len().max(1)).unwrap_or(u16::MAX);
    let area = centered(64, listed.saturating_add(4), screen);
    let block = Block::bordered().title(Span::styled(
        " Command palette ",
        style(theme, Tone::Heading),
    ));
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let [query, list] = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).areas(inner);
    frame.render_widget(query_line(palette.query(), theme), query);
    let mut selection = ListState::default();
    selection.select((!rows.is_empty()).then(|| palette.selected()));
    frame.render_stateful_widget(
        List::new(palette_items(&rows, theme))
            .highlight_style(style(theme, Tone::Accent).add_modifier(Modifier::REVERSED))
            .highlight_symbol("> "),
        list,
        &mut selection,
    );
}

/// The query, with the prompt character the eye looks for.
fn query_line(query: &str, theme: Theme) -> Paragraph<'static> {
    Paragraph::new(Line::from(vec![
        Span::styled(" > ", style(theme, Tone::Prompt)),
        Span::styled(query.to_owned(), style(theme, Tone::Value)),
    ]))
}

/// One list item per visible row: the numbered label, then the shortcut that
/// reaches the same command, dim.
fn palette_items(rows: &[&Row], theme: Theme) -> Vec<ListItem<'static>> {
    if rows.is_empty() {
        return vec![ListItem::new(Line::from(Span::styled(
            "  No command matches.",
            style(theme, Tone::Muted),
        )))];
    }
    rows.iter()
        .map(|row| {
            let mut spans = vec![Span::styled(row.text(), style(theme, Tone::Value))];
            if let Some(shortcut) = row.shortcut {
                spans.push(Span::styled(
                    format!("  ({shortcut})"),
                    style(theme, Tone::Muted),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect()
}

/// The shortcuts modal: every binding, grouped by where it applies.
fn draw_shortcuts(frame: &mut Frame, theme: Theme, screen: Rect) {
    let lines = shortcut_lines(theme);
    // Sized to its content, so the table can grow without anybody remembering
    // to grow a number here as well.
    let height = u16::try_from(lines.len().saturating_add(2)).unwrap_or(u16::MAX);
    let area = centered(78, height, screen);
    let block = Block::bordered().title(Span::styled(
        " Keyboard shortcuts ",
        style(theme, Tone::Heading),
    ));
    frame.render_widget(Clear, area);
    frame.render_widget(
        // `trim: false`, because trimming would eat the leading spaces that
        // line the keys up into a column.
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

/// The modal's text: a heading per group, then its bindings.
fn shortcut_lines(theme: Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for group in Group::ORDER {
        lines.push(Line::from(Span::styled(
            group.title(),
            style(theme, Tone::Heading),
        )));
        for shortcut in shortcuts::in_group(*group) {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:>15}  ", shortcut.keys),
                    style(theme, Tone::Accent),
                ),
                Span::styled(shortcut.description, style(theme, Tone::Value)),
            ]));
        }
        lines.push(Line::raw(""));
    }
    lines.push(Line::from(Span::styled(
        "Esc closes this.",
        style(theme, Tone::Muted),
    )));
    lines
}

/// Centers a box of at most `width` by `height` inside `area`. A terminal can
/// be smaller than any box you had in mind, so the size is a wish and the
/// area is the law.
#[must_use]
pub const fn centered(width: u16, height: u16, area: Rect) -> Rect {
    let width = if width < area.width {
        width
    } else {
        area.width
    };
    let height = if height < area.height {
        height
    } else {
        area.height
    };
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

/// Whether the theme is painting at all.
///
/// `NO_COLOR` and a piped stderr both turn the command-line half of this tool
/// monochrome, and the UI follows: asking the theme is the only way to stay
/// in step with it.
#[must_use]
pub fn paints(theme: Theme) -> bool {
    !theme.paint(Tone::Value, "").is_empty()
}

/// Turns a semantic tone into a terminal style, or into no style at all when
/// color is off.
fn style(theme: Theme, tone: Tone) -> Style {
    if paints(theme) {
        Style::new().fg(Color::Indexed(tone.ansi_index()))
    } else {
        Style::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Rect = Rect {
        x: 0,
        y: 0,
        width: 100,
        height: 40,
    };

    #[test]
    fn a_box_is_centered_in_the_area() {
        let area = centered(40, 10, SCREEN);
        assert_eq!((area.x, area.y), (30, 15));
        assert_eq!((area.width, area.height), (40, 10));
    }

    #[test]
    fn a_box_larger_than_the_terminal_is_the_terminal() {
        let area = centered(200, 200, SCREEN);
        assert_eq!(area, SCREEN);
    }

    #[test]
    fn centering_respects_an_offset_area() {
        let area = centered(
            10,
            10,
            Rect {
                x: 10,
                y: 4,
                width: 30,
                height: 20,
            },
        );
        assert_eq!((area.x, area.y), (20, 9));
    }

    #[test]
    fn a_theme_with_color_off_paints_nothing() {
        assert!(!paints(Theme::dark(false)));
        assert_eq!(style(Theme::dark(false), Tone::Accent), Style::new());
    }

    #[test]
    fn a_theme_with_color_on_paints_the_tones_own_color() {
        assert!(paints(Theme::dark(true)));
        assert_eq!(
            style(Theme::dark(true), Tone::Accent),
            Style::new().fg(Color::Indexed(Tone::Accent.ansi_index()))
        );
    }
}
