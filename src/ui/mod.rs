pub mod input;
pub mod list;
pub mod theme;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListState, Paragraph};

use crate::app::{get_filtered_commands, Mode};
use crate::data::TodoItem;

use self::input::InputBuffer;
use self::theme::Theme;

pub fn render(
    frame: &mut Frame,
    items: &[TodoItem],
    selected_index: usize,
    input: &InputBuffer,
    mode: &Mode,
    pending_count: usize,
    filter: &str,
    theme: &Theme,
) {
    let area = frame.area();
    let layout = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);

    render_header(frame, layout[0], pending_count, theme);
    render_list(frame, layout[1], items, selected_index, mode, theme);
    render_input(frame, layout[2], input, mode, filter, theme);

    if let Mode::Command { selected } = mode {
        render_cmd_completions(frame, layout[1], input, *selected, theme);
    } else if let Mode::ThemePicker { selected } = mode {
        render_theme_picker(frame, layout[1], *selected, theme);
    }
}

fn render_header(frame: &mut Frame, area: Rect, pending_count: usize, theme: &Theme) {
    let pending_text = format!("{} pending", pending_count);
    let line = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            "TODO",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            pending_text,
            Style::default().fg(theme.text_secondary),
        ),
    ]);

    let paragraph = Paragraph::new(line)
        .style(Style::default().bg(theme.bg_primary))
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, area);
}

fn render_list(frame: &mut Frame, area: Rect, items: &[TodoItem], selected_index: usize, mode: &Mode, theme: &Theme) {
    let selected_ids = match mode {
        Mode::MultiSelect { ref selected, .. } => Some(selected),
        _ => None,
    };

    let list_items: Vec<_> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let multi_sel = selected_ids.map_or(false, |ids| ids.contains(&item.id));
            self::list::render_item(item, i == selected_index, multi_sel, theme)
        })
        .collect();

    let mut list_state = ListState::default().with_selected(Some(selected_index));

    let list = List::new(list_items)
        .style(Style::default().bg(theme.bg_primary))
        .highlight_style(
            Style::default().bg(theme.bg_tertiary),
        );

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_cmd_completions(frame: &mut Frame, area: Rect, input: &InputBuffer, selected: usize, theme: &Theme) {
    let prefix = input.text().to_lowercase();
    let matches: Vec<(&str, &str)> = get_filtered_commands(&prefix);

    if matches.is_empty() {
        return;
    }

    let height = matches.len().min(8) as u16 + 2;
    let width = 48;
    let popup_y = area.bottom().saturating_sub(height + 1);
    let popup_x = area.x + 2;

    let popup_area = Rect::new(
        popup_x,
        popup_y.min(area.bottom().saturating_sub(height)),
        width,
        height,
    );

    let safe_selected = selected.min(matches.len().saturating_sub(1));
    let mut lines = Vec::new();
    for (i, (cmd, desc)) in matches.iter().enumerate() {
        let highlighted = i == safe_selected;
        let bg = if highlighted { theme.bg_tertiary } else { theme.bg_primary };

        lines.push(
            Line::from(vec![
                Span::styled(
                    format!("  /{:<8}", cmd),
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(if highlighted { Modifier::BOLD } else { Modifier::empty() }),
                ),
                Span::styled(
                    desc.to_string(),
                    Style::default().fg(theme.text_secondary),
                ),
            ])
            .style(Style::default().bg(bg)),
        );
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_default))
        .title(" Commands ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.bg_primary));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_primary));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn render_theme_picker(frame: &mut Frame, area: Rect, selected: usize, theme: &Theme) {
    let names = Theme::theme_names();
    let height = names.len() as u16 + 2;
    let width = 30;
    let popup_y = area.bottom().saturating_sub(height + 1);
    let popup_x = area.x + 2;

    let popup_area = Rect::new(
        popup_x,
        popup_y.min(area.bottom().saturating_sub(height)),
        width,
        height,
    );

    let mut lines = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let is_active = *name == theme.name;
        let is_highlighted = i == selected;
        let bg = if is_highlighted { theme.bg_tertiary } else { theme.bg_primary };

        lines.push(
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    if is_active { "\u{25cf} " } else { "  " },
                    Style::default().fg(theme.accent),
                ),
                Span::styled(
                    name.to_string(),
                    Style::default()
                        .fg(if is_active { theme.accent } else { theme.text_primary })
                        .add_modifier(if is_highlighted || is_active { Modifier::BOLD } else { Modifier::empty() }),
                ),
                Span::raw("  "),
            ])
            .style(Style::default().bg(bg)),
        );
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_default))
        .title(" Themes (Enter to select) ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.bg_primary));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_primary));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn render_input(frame: &mut Frame, area: Rect, input: &InputBuffer, mode: &Mode, filter: &str, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme.border_default));

    let (display_text, cursor_pos) = match mode {
        Mode::Searching => {
            let line = Line::from(vec![
                Span::styled(
                    "  Search: ",
                    Style::default().fg(theme.accent),
                ),
                Span::styled(
                    filter,
                    Style::default().fg(theme.text_primary),
                ),
                Span::styled(
                    "\u{2588}",
                    Style::default().fg(theme.accent),
                ),
            ]);
            (line, None::<u16>)
        }
        Mode::Command { .. } => {
            let mut spans = vec![
                Span::styled(
                    "  /",
                    Style::default().fg(theme.accent),
                ),
            ];

            let text = input.text();
            if text.is_empty() {
                spans.push(Span::styled(
                    "\u{2588}",
                    Style::default().fg(theme.accent),
                ));
            } else {
                spans.push(Span::styled(
                    text.to_string(),
                    Style::default().fg(theme.text_primary),
                ));
                spans.push(Span::styled(
                    "\u{2588}",
                    Style::default().fg(theme.accent),
                ));
            }

            (Line::from(spans), None)
        }
        Mode::MultiSelect { .. } => {
            let line = Line::from(vec![
                Span::styled(
                    "  [select items, Enter to confirm, Esc to cancel]",
                    Style::default().fg(theme.warning),
                ),
            ]);
            (line, None)
        }
        Mode::ThemePicker { .. } => {
            let line = Line::from(vec![
                Span::styled(
                    "  [up/down: navigate, Enter: select theme, Esc: cancel]",
                    Style::default().fg(theme.warning),
                ),
            ]);
            (line, None)
        }
        _ if input.is_empty() && matches!(mode, Mode::Normal) => {
            let placeholder = Line::from(vec![
                Span::styled(
                    "  Type to add or / for commands",
                    Style::default().fg(theme.text_placeholder),
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
                        Style::default().fg(theme.accent),
                    ),
                    Span::styled(
                        text,
                        Style::default().fg(theme.text_primary),
                    ),
                ]);
                (line, None)
            } else {
                let before = &text[..cursor];
                let after = &text[cursor..];
                let line = Line::from(vec![
                    Span::raw("  "),
                    Span::styled(before, Style::default().fg(theme.text_primary)),
                    Span::styled(
                        "\u{2588}",
                        Style::default().fg(theme.accent),
                    ),
                    Span::styled(after, Style::default().fg(theme.text_primary)),
                ]);
                (line, None)
            }
        }
    };

    let paragraph = Paragraph::new(display_text)
        .style(Style::default().bg(theme.bg_primary))
        .block(block);

    frame.render_widget(paragraph, area);

    if let Some(col) = cursor_pos {
        let x = area.x + col as u16;
        let y = area.y + 1;
        #[allow(deprecated)]
        frame.set_cursor(x, y);
    }
}
