use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
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

fn priority_span(item: &TodoItem, theme: &Theme) -> Span<'static> {
    let color = priority_color(item.priority, theme);
    Span::styled(
        format!(" {} ", item.priority.label()),
        Style::default().fg(color),
    )
}

pub fn render_item(item: &TodoItem, selected: bool, multi_selected: bool, theme: &Theme) -> ListItem<'static> {
    let base_bg = if selected {
        theme.bg_tertiary
    } else if multi_selected {
        theme.accent_selection
    } else {
        theme.bg_secondary
    };

    let icon = if item.done {
        "\u{2713} "
    } else if multi_selected {
        "\u{25cf} "
    } else {
        "\u{25cb} "
    };

    let icon_color = if multi_selected && !item.done {
        theme.accent
    } else if item.done {
        theme.success
    } else {
        theme.text_primary
    };

    let title_style = if item.done {
        Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::CROSSED_OUT)
    } else {
        Style::default().fg(theme.text_primary)
    };

    let line = Line::from(vec![
        Span::raw(" "),
        Span::styled(icon, Style::default().fg(icon_color)),
        Span::styled(item.text.clone(), title_style),
        Span::raw(" "),
        priority_span(item, theme),
        Span::raw(" "),
    ]);

    ListItem::new(line).style(Style::default().bg(base_bg))
}
