use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::ListItem;

use crate::data::{Priority, TodoItem};
use crate::date;

use super::theme::Theme;

fn priority_color(p: Priority, theme: &Theme) -> ratatui::style::Color {
    match p {
        Priority::Low => theme.text_muted,
        Priority::Normal => theme.accent,
        Priority::High => theme.warning,
        Priority::Urgent => theme.error,
    }
}

fn highlight_matches<'a>(
    text: &str,
    filter: &str,
    base_style: Style,
    match_style: Style,
) -> Vec<Span<'a>> {
    if filter.is_empty() {
        return vec![Span::styled(text.to_string(), base_style)];
    }
    let lower = text.to_lowercase();
    let q = filter.to_lowercase();
    let mut spans = Vec::new();
    let mut start = 0;
    while let Some(pos) = lower[start..].find(&q) {
        let abs = start + pos;
        if abs > start {
            spans.push(Span::styled(text[start..abs].to_string(), base_style));
        }
        spans.push(Span::styled(
            text[abs..abs + q.len()].to_string(),
            match_style,
        ));
        start = abs + q.len();
    }
    if start < text.len() {
        spans.push(Span::styled(text[start..].to_string(), base_style));
    }
    spans
}

pub fn render_item(
    item: &TodoItem,
    selected: bool,
    multi_selected: bool,
    theme: &Theme,
    text_width: usize,
    filter: &str,
    hide_category_badge: bool,
) -> ListItem<'static> {
    let base_bg = if selected {
        theme.bg_tertiary
    } else if multi_selected {
        theme.accent_selection
    } else {
        theme.bg_secondary
    };

    let is_overdue_item = !item.done && item.due_date.as_deref().is_some_and(date::is_overdue);

    let title_style = if item.done {
        Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::CROSSED_OUT)
    } else if is_overdue_item {
        Style::default()
            .fg(theme.error)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text_primary)
    };

    let match_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::UNDERLINED);

    let diamond = "\u{25c6}";
    let diamond_color = priority_color(item.priority, theme);

    let wrapped = super::wrap_text(&item.text, text_width.max(10));
    let mut lines = Vec::new();

    // Line 1: bullet, priority, pin, due date
    let mut line1 = vec![Span::raw(" ")];
    if item.done {
        line1.push(Span::styled("\u{2713}", Style::default().fg(theme.success)));
    } else {
        let circle = if item.doing { "\u{25cf}" } else { "\u{25cb}" };
        let circle_color = if item.doing {
            theme.accent
        } else {
            theme.text_primary
        };
        line1.push(Span::styled(circle, Style::default().fg(circle_color)));
    }
    line1.push(Span::raw(" "));
    line1.push(Span::styled(diamond, Style::default().fg(diamond_color)));
    if item.pinned {
        line1.push(Span::raw(" \u{1F4CC}"));
    }
    if let Some(ref date) = item.due_date {
        let date_color = if is_overdue_item {
            theme.error
        } else {
            theme.warning
        };
        line1.push(Span::raw(" \u{1F4C5} "));
        line1.push(Span::styled(date.clone(), Style::default().fg(date_color)));
    }
    line1.push(Span::raw(" "));
    lines.push(Line::from(line1));

    // Line 2+: text at char 4
    for (i, seg) in wrapped.iter().enumerate() {
        let mut spans = vec![Span::raw("   ")];
        if i == 0 {
            spans.extend(highlight_matches(seg, filter, title_style, match_style));
            if let Some(ref cat) = item.category {
                if !hide_category_badge {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        format!("[{}]", cat),
                        Style::default().fg(theme.text_muted),
                    ));
                }
            }
            spans.push(Span::raw(" "));
        } else {
            spans.push(Span::styled(seg.clone(), title_style));
        }
        lines.push(Line::from(spans));
    }

    // Line last: empty
    lines.push(Line::from(Span::raw("")));

    ListItem::new(Text::from(lines)).style(Style::default().bg(base_bg))
}
