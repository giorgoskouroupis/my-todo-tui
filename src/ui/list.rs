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

fn today_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut days = secs / 86400;
    let mut y = 1970u64;
    loop {
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let yd = if leap { 366 } else { 365 };
        if days < yd {
            break;
        }
        days -= yd;
        y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let month_days: [u64; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1u64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        m += 1;
    }
    let d = days + 1;
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn is_overdue(due_date: &str) -> bool {
    let today = today_iso();
    due_date.as_bytes() <= today.as_bytes() && due_date.len() == 10
}

fn highlight_matches<'a>(text: &str, filter: &str, base_style: Style, match_style: Style) -> Vec<Span<'a>> {
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
        spans.push(Span::styled(text[abs..abs + q.len()].to_string(), match_style));
        start = abs + q.len();
    }
    if start < text.len() {
        spans.push(Span::styled(text[start..].to_string(), base_style));
    }
    spans
}

pub fn render_item(item: &TodoItem, selected: bool, multi_selected: bool, theme: &Theme, text_width: usize, filter: &str, hide_category_badge: bool) -> ListItem<'static> {
    let base_bg = if selected {
        theme.bg_tertiary
    } else if multi_selected {
        theme.accent_selection
    } else {
        theme.bg_secondary
    };

    let is_overdue_item = !item.done && item.due_date.as_deref().map_or(false, is_overdue);

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

    let star = if item.pinned { "\u{2605} " } else { "" };
    let star_style = if item.pinned {
        Style::default().fg(theme.warning)
    } else {
        Style::default()
    };

    if item.done {
        let wrapped = super::wrap_text(&item.text, text_width.max(10));
        let mut lines = Vec::new();
        for (i, seg) in wrapped.iter().enumerate() {
            if i == 0 {
                let mut spans = vec![
                    Span::raw(" "),
                    Span::styled("\u{2713} ", Style::default().fg(theme.success)),
                ];
                if item.pinned {
                    spans.push(Span::styled(star, star_style));
                }
                spans.extend(highlight_matches(seg, filter, title_style, match_style));
                spans.push(Span::raw(" "));
                lines.push(Line::from(spans));
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
            let mut spans = vec![
                Span::raw(" "),
                Span::styled(circle, Style::default().fg(circle_color)),
                Span::styled(diamond, Style::default().fg(diamond_color)),
                Span::raw(" "),
            ];
            if item.pinned {
                spans.push(Span::styled(star, star_style));
            }
            spans.extend(highlight_matches(seg, filter, title_style, match_style));
            if let Some(ref date) = item.due_date {
                spans.push(Span::raw(" "));
                let date_color = if is_overdue_item { theme.error } else { theme.text_muted };
                spans.push(Span::styled(
                    format!("@{}", date),
                    Style::default().fg(date_color),
                ));
            }
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
            lines.push(Line::from(spans));
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
