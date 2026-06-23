use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListItem;

use crate::data::{Priority, TodoItem};

use super::theme;

fn priority_color(p: Priority) -> Color {
    match p {
        Priority::Low => theme::TEXT_MUTED,
        Priority::Normal => theme::ACCENT,
        Priority::High => theme::WARNING,
        Priority::Urgent => theme::ERROR,
    }
}

fn priority_span(item: &TodoItem) -> Span<'static> {
    let color = priority_color(item.priority);
    Span::styled(
        format!(" {} {} ", theme::PRIORITY_ICON, item.priority.label()),
        Style::default().fg(color),
    )
}

pub fn render_item(item: &TodoItem, selected: bool) -> ListItem<'static> {
    let base_bg = if selected {
        theme::BG_TERTIARY
    } else {
        theme::BG_SECONDARY
    };

    let dot = if item.done {
        theme::DONE_ICON
    } else {
        theme::INACTIVE_DOT
    };

    let title_style = if item.done {
        Style::default()
            .fg(theme::SUCCESS)
            .add_modifier(Modifier::CROSSED_OUT)
    } else {
        Style::default().fg(theme::TEXT_PRIMARY)
    };

    let title_line = Line::from(vec![
        Span::styled(
            format!(" {}  {}", dot, item.text),
            title_style,
        ),
        Span::raw(" "),
        priority_span(item),
    ]);

    ListItem::new(vec![title_line, Line::from(Span::raw(""))])
        .style(Style::default().bg(base_bg))
}
