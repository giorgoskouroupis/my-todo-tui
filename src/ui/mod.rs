pub mod input;
pub mod list;
pub mod theme;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, List, ListState};

use crate::app::Mode;
use crate::data::TodoItem;

use self::input::InputBuffer;

pub fn render(
    frame: &mut Frame,
    items: &[TodoItem],
    selected_index: usize,
    input: &InputBuffer,
    mode: &Mode,
    pending_count: usize,
    filter: &str,
) {
    let area = frame.area();
    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);

    render_header(frame, layout[0], pending_count);
    render_list(frame, layout[1], items, selected_index);
    render_input(frame, layout[2], input, mode, filter);
}

fn render_header(frame: &mut Frame, area: Rect, pending_count: usize) {
    let pending_text = format!("{} pending", pending_count);
    let line = Line::from(vec![
        Span::styled(
            format!(" {}  Todos", theme::TODO_ICON),
            Style::default().fg(theme::ACCENT),
        ),
        Span::raw(" "),
        Span::styled(
            pending_text,
            Style::default().fg(theme::TEXT_SECONDARY),
        ),
    ]);

    let paragraph = Paragraph::new(line).style(Style::default().bg(theme::BG_PRIMARY));
    frame.render_widget(paragraph, area);
}

fn render_list(frame: &mut Frame, area: Rect, items: &[TodoItem], selected_index: usize) {
    let list_items: Vec<_> = items
        .iter()
        .enumerate()
        .map(|(i, item)| self::list::render_item(item, i == selected_index))
        .collect();

    let mut list_state = ListState::default().with_selected(Some(selected_index));

    let list = List::new(list_items)
        .style(Style::default().bg(theme::BG_PRIMARY))
        .highlight_style(
            Style::default().bg(theme::BG_TERTIARY),
        );

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_input(frame: &mut Frame, area: Rect, input: &InputBuffer, mode: &Mode, filter: &str) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme::BORDER_DEFAULT));

    let (display_text, cursor_pos) = match mode {
        Mode::Searching => {
            let line = Line::from(vec![
                Span::styled(
                    "  Search: ",
                    Style::default().fg(theme::ACCENT),
                ),
                Span::styled(
                    filter,
                    Style::default().fg(theme::TEXT_PRIMARY),
                ),
                Span::styled(
                    "\u{2588}",
                    Style::default().fg(theme::ACCENT),
                ),
            ]);
            (line, None::<u16>)
        }
        _ if input.is_empty() && matches!(mode, Mode::Normal) => {
            let placeholder = Line::from(vec![
                Span::styled(
                    "  Add todo...",
                    Style::default().fg(theme::TEXT_PLACEHOLDER),
                ),
            ]);
            (placeholder, None)
        }
        _ => {
            let text = input.text();
            let cursor = input.cursor();

            if cursor == 0 {
                let line = Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "\u{2588}",
                        Style::default().fg(theme::ACCENT),
                    ),
                    Span::styled(
                        text,
                        Style::default().fg(theme::TEXT_PRIMARY),
                    ),
                ]);
                (line, None)
            } else {
                let before = &text[..cursor];
                let after = &text[cursor..];
                let line = Line::from(vec![
                    Span::raw("  "),
                    Span::styled(before, Style::default().fg(theme::TEXT_PRIMARY)),
                    Span::styled(
                        "\u{2588}",
                        Style::default().fg(theme::ACCENT),
                    ),
                    Span::styled(after, Style::default().fg(theme::TEXT_PRIMARY)),
                ]);
                (line, None)
            }
        }
    };

    let paragraph = Paragraph::new(display_text)
        .style(Style::default().bg(theme::BG_PRIMARY))
        .block(block);

    frame.render_widget(paragraph, area);

    if let Some(col) = cursor_pos {
        let x = area.x + col as u16;
        let y = area.y + 1;
        #[allow(deprecated)]
        frame.set_cursor(x, y);
    }
}
