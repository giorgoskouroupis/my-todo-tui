use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::ListItem;

use crate::data::{Priority, TodoItem};

use super::theme::Theme;

fn priority_color(p: Priority, theme: &Theme) -> ratatui::style::Color {
    match p {
        Priority::Low => theme.text_muted,
        Priority::Normal => theme.accent,
        Priority::High => theme.warning,
        Priority::Urgent => theme.error,
    }
}

pub fn render_item(item: &TodoItem, selected: bool, multi_selected: bool, theme: &Theme, text_width: usize) -> ListItem<'static> {
    let base_bg = if selected {
        theme.bg_tertiary
    } else if multi_selected {
        theme.accent_selection
    } else {
        theme.bg_secondary
    };

    let title_style = if item.done {
        Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::CROSSED_OUT)
    } else {
        Style::default().fg(theme.text_primary)
    };

    if item.done {
        let wrapped = super::wrap_text(&item.text, text_width.max(10));
        let mut lines = Vec::new();
        for (i, seg) in wrapped.iter().enumerate() {
            if i == 0 {
                lines.push(Line::from(vec![
                    Span::raw(" "),
                    Span::styled("\u{2713} ", Style::default().fg(theme.success)),
                    Span::styled(seg.clone(), title_style),
                    Span::raw(" "),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::styled(seg.clone(), title_style),
                ]));
            }
        }
        lines.push(Line::from(Span::raw("")));
        return ListItem::new(Text::from(lines)).style(Style::default().bg(base_bg));
    }

    let circle = if item.doing { "\u{25cf} " } else { "\u{25cb} " };
    let circle_color = if item.doing { theme.accent } else { theme.text_primary };
    let diamond = "\u{25c6}";
    let diamond_color = priority_color(item.priority, theme);

    let wrapped = super::wrap_text(&item.text, text_width.max(10));
    let mut lines = Vec::new();
    for (i, seg) in wrapped.iter().enumerate() {
        if i == 0 {
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(circle, Style::default().fg(circle_color)),
                Span::styled(diamond, Style::default().fg(diamond_color)),
                Span::raw(" "),
                Span::styled(seg.clone(), title_style),
                Span::raw(" "),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(seg.clone(), title_style),
            ]));
        }
    }
    lines.push(Line::from(Span::raw("")));

    ListItem::new(Text::from(lines)).style(Style::default().bg(base_bg))
}
