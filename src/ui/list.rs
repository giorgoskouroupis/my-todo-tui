use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::ListItem;

use crate::data::{category_badge, Priority, TodoItem};
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

fn due_date_color(due_date: &str, theme: &Theme) -> ratatui::style::Color {
    match date::days_until(due_date) {
        Some(days) if days <= 3 => theme.error,
        Some(days) if days <= 10 => theme.warning,
        Some(_) => theme.text_muted,
        None => theme.text_muted,
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
    category_filter: Option<&str>,
) -> ListItem<'static> {
    let base_bg = if selected {
        theme.bg_tertiary
    } else if multi_selected {
        theme.accent_selection
    } else {
        theme.bg_primary
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

    // Line 1: status marker, priority, pin, due date
    let mut line1 = Vec::new();
    line1.push(Span::styled(
        if multi_selected { "x" } else { " " },
        Style::default().fg(theme.accent),
    ));
    if item.done {
        line1.push(Span::styled("\u{2713}", Style::default().fg(theme.success)));
    } else if item.doing {
        line1.push(Span::styled(
            "\u{25e6}",
            Style::default()
                .fg(theme.text_primary)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        line1.push(Span::styled("-", Style::default().fg(theme.text_primary)));
    }
    line1.push(Span::raw(" "));
    line1.push(Span::styled(diamond, Style::default().fg(diamond_color)));
    if item.pinned {
        line1.push(Span::raw(" \u{1F4CC}"));
    }
    if let Some(ref date) = item.due_date {
        let date_color = due_date_color(date, theme);
        line1.push(Span::raw(" \u{1F4C5} "));
        line1.push(Span::styled(date.clone(), Style::default().fg(date_color)));
    }
    let badge_span = item.category.as_ref().and_then(|cat| {
        category_badge(cat, category_filter).map(|badge| {
            Span::styled(
                format!("[{}]", badge),
                Style::default().fg(theme.text_muted),
            )
        })
    });
    match badge_span {
        Some(badge) => {
            let left_width: usize = line1.iter().map(|s| s.width()).sum();
            let badge_width = badge.width();
            let target_width = text_width + 2;
            let gap = target_width.saturating_sub(left_width + badge_width);
            if gap >= 2 {
                line1.push(Span::raw(" ".repeat(gap)));
            } else {
                line1.push(Span::raw(" "));
            }
            line1.push(badge);
        }
        None => line1.push(Span::raw(" ")),
    }
    lines.push(Line::from(line1));

    // Line 2+: text at char 2, justified (last wrap line stays left-aligned)
    let last_idx = wrapped.len().saturating_sub(1);
    let content_width = text_width.max(10);
    for (i, seg) in wrapped.iter().enumerate() {
        let mut spans = vec![Span::raw("  ")];
        let justified;
        let render_text: &str = if i < last_idx {
            justified = justify_line(seg, content_width);
            &justified
        } else {
            seg
        };
        spans.extend(highlight_matches(render_text, filter, title_style, match_style));
        lines.push(Line::from(spans));
    }

    ListItem::new(Text::from(lines)).style(Style::default().bg(base_bg))
}

fn justify_line(text: &str, target_width: usize) -> String {
    let current_width = text.chars().count();
    if current_width >= target_width {
        return text.to_string();
    }
    let slack = target_width - current_width;
    let words: Vec<&str> = text.split(' ').filter(|w| !w.is_empty()).collect();
    if words.len() <= 1 {
        return text.to_string();
    }
    let num_gaps = words.len() - 1;
    let extra_per_gap = slack / num_gaps;
    let remainder = slack % num_gaps;

    let mut result = String::with_capacity(text.len() + slack);
    for (i, word) in words.iter().enumerate() {
        result.push_str(word);
        if i < num_gaps {
            let extra = extra_per_gap + usize::from(i < remainder);
            for _ in 0..(1 + extra) {
                result.push(' ');
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::justify_line;

    #[test]
    fn justify_line_fills_to_target_width() {
        let out = justify_line("hello world foo", 20);
        assert_eq!(out.chars().count(), 20);
    }

    #[test]
    fn justify_line_distributes_remainder_to_earlier_gaps() {
        let out = justify_line("a b c", 8);
        assert_eq!(out, "a   b  c");
        assert_eq!(out.chars().count(), 8);
    }

    #[test]
    fn justify_line_leaves_single_word_alone() {
        assert_eq!(justify_line("hello", 20), "hello");
    }

    #[test]
    fn justify_line_returns_original_when_already_at_or_over_width() {
        assert_eq!(justify_line("hello world", 5), "hello world");
        assert_eq!(justify_line("hello world", 11), "hello world");
    }
}
