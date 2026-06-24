pub mod input;
pub mod list;
pub mod theme;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListState, Paragraph};

use crate::app::{get_filtered_commands, Mode, Pane, SortMode};
use crate::data::{Priority, TodoItem};

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
    priority_filter: &Option<Priority>,
    pane: &Pane,
    category_index: usize,
    categories: &[String],
    theme: &Theme,
    sort_mode: &SortMode,
) {
    let area = frame.area();
    let layout = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);

    render_header(frame, layout[0], pending_count, filter, priority_filter, selected_index, items.len(), categories, category_index, theme, sort_mode);

    let mid = layout[1];
    let h_layout = Layout::horizontal([
        Constraint::Length(22),
        Constraint::Min(1),
    ])
    .split(mid);

    render_sidebar(frame, h_layout[0], pane, category_index, categories, theme);
    render_list(frame, h_layout[1], items, selected_index, mode, filter, theme, pane, category_index);
    render_input(frame, layout[2], input, mode, filter, theme);

    let popup_area = mid;
    if let Mode::Command { selected } = mode {
        render_cmd_completions(frame, popup_area, input, *selected, theme);
    } else if let Mode::ThemePicker { selected } = mode {
        render_theme_picker(frame, popup_area, *selected, theme);
    } else if let Mode::PriorityPicker { selected } = mode {
        render_priority_picker(frame, popup_area, *selected, theme);
    } else if let Mode::CategoryPicker { selected } = mode {
        render_category_picker(frame, popup_area, *selected, categories, theme);
    } else if let Mode::SortPicker { selected } = mode {
        render_sort_picker(frame, popup_area, *selected, theme);
    } else if let Mode::Help = mode {
        render_help_popup(frame, popup_area, theme);
    } else if let Mode::Keybindings = mode {
        render_keybindings_popup(frame, popup_area, theme);
    } else if let Mode::ConfirmDelete { texts, .. } = mode {
        render_confirm_delete_popup(frame, popup_area, texts, theme);
    }
}

fn render_header(frame: &mut Frame, area: Rect, pending_count: usize, filter: &str, priority_filter: &Option<Priority>, selected: usize, total: usize, categories: &[String], category_index: usize, theme: &Theme, sort_mode: &SortMode) {
    let mut spans = vec![
        Span::raw(" "),
        Span::styled(
            "TODO",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if !filter.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("[search: {}]", filter),
            Style::default().fg(theme.warning),
        ));
    }

    if let Some(p) = priority_filter {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("[filter: {:?}]", p).to_lowercase(),
            Style::default().fg(theme.warning),
        ));
    }

    if category_index > 0 {
        if let Some(cat) = categories.get(category_index - 1) {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("[cat: {}]", cat),
                Style::default().fg(theme.warning),
            ));
        }
    }

    match sort_mode {
        SortMode::Priority => {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                "[sort: priority]",
                Style::default().fg(theme.warning),
            ));
        }
        SortMode::DueDate => {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                "[sort: due]",
                Style::default().fg(theme.warning),
            ));
        }
        _ => {}
    }

    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        format!("{} pending", pending_count),
        Style::default().fg(theme.text_secondary),
    ));

    if total > 0 {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{}/{}", selected + 1, total),
            Style::default().fg(theme.text_muted),
        ));
    }

    let line = Line::from(spans);

    let paragraph = Paragraph::new(line)
        .style(Style::default().bg(theme.bg_primary))
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, area);
}

fn render_sidebar(frame: &mut Frame, area: Rect, pane: &Pane, category_index: usize, categories: &[String], theme: &Theme) {
    let is_active = *pane == Pane::Categories;

    let mut lines = Vec::new();
    let all_filtered = !is_active && category_index == 0;
    let all_hl = is_active && category_index == 0;
    lines.push(
        Line::from(vec![
            Span::raw(" "),
            Span::styled(
                if category_index == 0 { "\u{25cf} " } else { "  " },
                Style::default().fg(theme.accent),
            ),
            Span::styled(
                "All",
                Style::default()
                    .fg(if all_filtered { theme.text_primary } else { theme.text_primary })
                    .add_modifier(if all_hl || all_filtered { Modifier::BOLD } else { Modifier::empty() }),
            ),
        ])
        .style(Style::default().bg(if all_hl || all_filtered { theme.bg_tertiary } else { theme.bg_primary })),
    );

    for (i, cat) in categories.iter().enumerate() {
        let idx = i + 1;
        let hl = is_active && category_index == idx;
        let is_filter = !is_active && category_index == idx;
        lines.push(
            Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    if is_filter { "\u{25cf} " } else { "  " },
                    Style::default().fg(theme.accent),
                ),
                Span::styled(
                    cat.clone(),
                    Style::default()
                        .fg(if is_filter { theme.text_primary } else { theme.text_primary })
                        .add_modifier(if hl || is_filter { Modifier::BOLD } else { Modifier::empty() }),
                ),
            ])
            .style(Style::default().bg(if hl || is_filter { theme.bg_tertiary } else { theme.bg_primary })),
        );
    }

    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(if is_active { theme.accent } else { theme.border_default }))
        .title(" Categories ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.bg_primary));

    let list = List::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_primary));

    frame.render_widget(list, area);
}

fn render_list(frame: &mut Frame, area: Rect, items: &[TodoItem], selected_index: usize, mode: &Mode, filter: &str, theme: &Theme, pane: &Pane, category_index: usize) {
    let selected_ids = match mode {
        Mode::MultiSelect { ref selected, .. } => Some(selected),
        _ => None,
    };

    let is_active = *pane == Pane::Items;
    let text_width = area.width.saturating_sub(7) as usize;
    let hide_category_badge = category_index > 0;

    let list_items: Vec<_> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let multi_sel = selected_ids.map_or(false, |ids| ids.contains(&item.id));
            self::list::render_item(item, is_active && i == selected_index, multi_sel, theme, text_width, filter, hide_category_badge)
        })
        .collect();

    let mut list_state = ListState::default().with_selected(if is_active { Some(selected_index) } else { None });

    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(if is_active { theme.accent } else { theme.border_default }))
        .title(" Items ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.bg_primary));

    let list = List::new(list_items)
        .block(block)
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

    let height = matches.len().min(12) as u16 + 2;
    let cmd_width = 12usize;
    let width = (cmd_width + 30) as u16;
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
                    format!("  /{:<1$}", cmd, cmd_width),
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

fn render_priority_picker(frame: &mut Frame, area: Rect, selected: usize, theme: &Theme) {
    let items: [(&str, Option<Priority>); 5] = [
        ("All priorities", None),
        ("Urgent", Some(Priority::Urgent)),
        ("High", Some(Priority::High)),
        ("Normal", Some(Priority::Normal)),
        ("Low", Some(Priority::Low)),
    ];
    let height = items.len() as u16 + 2;
    let width = 24;
    let popup_y = area.bottom().saturating_sub(height + 1);
    let popup_x = area.x + 2;

    let popup_area = Rect::new(
        popup_x,
        popup_y.min(area.bottom().saturating_sub(height)),
        width,
        height,
    );

    let mut lines = Vec::new();
    for (i, (label, _)) in items.iter().enumerate() {
        let is_highlighted = i == selected;
        let bg = if is_highlighted { theme.bg_tertiary } else { theme.bg_primary };

        lines.push(
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    label.to_string(),
                    Style::default()
                        .fg(theme.text_primary)
                        .add_modifier(if is_highlighted { Modifier::BOLD } else { Modifier::empty() }),
                ),
                Span::raw("  "),
            ])
            .style(Style::default().bg(bg)),
        );
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_default))
        .title(" Priorities ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.bg_primary));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_primary));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn render_category_picker(frame: &mut Frame, area: Rect, selected: usize, categories: &[String], theme: &Theme) {
    let total = categories.len() + 1;
    let height = total as u16 + 2;
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
    let entries: [&str; 1] = ["None"];
    for (i, label) in entries.iter().copied().chain(categories.iter().map(|s| s.as_str())).enumerate() {
        let is_highlighted = i == selected;
        let bg = if is_highlighted { theme.bg_tertiary } else { theme.bg_primary };

        lines.push(
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    label.to_string(),
                    Style::default()
                        .fg(if i == 0 { theme.text_muted } else { theme.text_primary })
                        .add_modifier(if is_highlighted { Modifier::BOLD } else { Modifier::empty() }),
                ),
                Span::raw("  "),
            ])
            .style(Style::default().bg(bg)),
        );
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_default))
        .title(" Assign Category ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.bg_primary));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_primary));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn render_sort_picker(frame: &mut Frame, area: Rect, selected: usize, theme: &Theme) {
    let items: [(&str, SortMode); 3] = [
        ("Priority", SortMode::Priority),
        ("Due date", SortMode::DueDate),
        ("Default", SortMode::Default),
    ];
    let height = items.len() as u16 + 2;
    let width = 24;
    let popup_y = area.bottom().saturating_sub(height + 1);
    let popup_x = area.x + 2;

    let popup_area = Rect::new(
        popup_x,
        popup_y.min(area.bottom().saturating_sub(height)),
        width,
        height,
    );

    let mut lines = Vec::new();
    for (i, (label, _)) in items.iter().enumerate() {
        let is_highlighted = i == selected;
        let bg = if is_highlighted { theme.bg_tertiary } else { theme.bg_primary };

        lines.push(
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    label.to_string(),
                    Style::default()
                        .fg(theme.text_primary)
                        .add_modifier(if is_highlighted { Modifier::BOLD } else { Modifier::empty() }),
                ),
                Span::raw("  "),
            ])
            .style(Style::default().bg(bg)),
        );
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_default))
        .title(" Sort (p/d/n) ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.bg_primary));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_primary));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn help_file_paths() -> (String, String) {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("todo-tui").join("config.json");
    let data = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("todo-tui").join("todos.json");
    (config.display().to_string(), data.display().to_string())
}

fn render_help_popup(frame: &mut Frame, area: Rect, theme: &Theme) {
    let (config_path, data_path) = help_file_paths();
    let lines = vec![
        Line::from(vec![
            Span::styled("  Welcome to ", Style::default().fg(theme.text_primary)),
            Span::styled("todo-tui", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        ]).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::raw("")).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled("  /command or /<alias> — run a command", Style::default().fg(theme.text_primary))).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled("  /help           — this screen", Style::default().fg(theme.text_primary))).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled("  /keybindings    — show all keybindings", Style::default().fg(theme.text_primary))).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled("  /priorities     — filter by priority", Style::default().fg(theme.text_primary))).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled("  /search <q>     — filter items by text", Style::default().fg(theme.text_primary))).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled("  /delete         — bulk delete (multi-select)", Style::default().fg(theme.text_primary))).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled("  /done           — bulk toggle done", Style::default().fg(theme.text_primary))).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled("  /clear          — clear completed items", Style::default().fg(theme.text_primary))).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled("  /themes         — pick a theme", Style::default().fg(theme.text_primary))).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::raw("")).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled("  Tab autocompletes commands, Esc cancels", Style::default().fg(theme.text_muted))).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::raw("")).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled("  Files:", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD))).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(format!("  Config: {}", config_path), Style::default().fg(theme.text_secondary))).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(format!("  Data:   {}", data_path), Style::default().fg(theme.text_secondary))).style(Style::default().bg(theme.bg_secondary)),
    ];

    let width = 64;
    let height = lines.len() as u16 + 2;
    let popup_x = area.x + (area.width.saturating_sub(width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, width, height);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" Help ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.bg_secondary));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_secondary));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn render_keybindings_popup(frame: &mut Frame, area: Rect, theme: &Theme) {
    let key_lines = [
        ("Normal", "", true),
        ("j/k or ↑/↓", "Navigate", false),
        ("Enter", "Edit item / create new", false),
        ("Space", "Toggle done", false),
        ("d", "Toggle doing", false),
        ("Delete", "Delete item", false),
        ("p / P", "Cycle priority", false),
        ("Alt+↑/↓", "Reorder item", false),
        ("u", "Undo delete", false),
        ("*", "Toggle pin", false),
        ("s", "Sort picker", false),
        ("←/→", "Switch pane", false),
        ("Ctrl+K", "Assign category", false),
        ("/", "Command mode", false),
        ("Ctrl+C", "Quit (from any mode)", false),
        ("", "", false),
        ("Sidebar", "", true),
        ("j/k or ↑/↓", "Navigate", false),
        ("Enter", "Select/filter category", false),
        ("a", "Add category", false),
        ("Delete", "Delete category", false),
        ("", "", false),
        ("Editing", "", true),
        ("Enter", "Submit", false),
        ("Esc", "Cancel", false),
        ("Ctrl+D", "Set/clear due date", false),
        ("←/→, Home/End", "Move cursor", false),
        ("Shift+←/→", "Select text", false),
        ("Ctrl+←/→", "Word jump", false),
        ("Ctrl+Shift+C", "Copy selection", false),
        ("Ctrl+Shift+V", "Paste", false),
    ];

    let mut lines = Vec::new();
    for (key, desc, is_header) in &key_lines {
        if *is_header {
            lines.push(
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        key.to_string(),
                        Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
                    ),
                ])
                .style(Style::default().bg(theme.bg_secondary)),
            );
        } else if key.is_empty() {
            lines.push(Line::from(Span::raw("")).style(Style::default().bg(theme.bg_secondary)));
        } else {
            lines.push(
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(
                        format!("{:<20}", key),
                        Style::default().fg(theme.text_secondary),
                    ),
                    Span::styled(desc.to_string(), Style::default().fg(theme.text_primary)),
                ])
                .style(Style::default().bg(theme.bg_secondary)),
            );
        }
    }

    let width = 52;
    let height = lines.len() as u16 + 2;
    let popup_x = area.x + (area.width.saturating_sub(width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, width, height);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" Keybindings ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.bg_secondary));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_secondary));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn render_confirm_delete_popup(frame: &mut Frame, area: Rect, texts: &[String], theme: &Theme) {
    let max_content_w = (area.width.saturating_sub(6)).min(60).max(20) as usize;

    let mut lines_text: Vec<String> = Vec::new();
    if texts.len() == 1 {
        lines_text.push("Are you sure you want to delete?".into());
        for wrapped in wrap_text(&format!("\"{}\"", &texts[0]), max_content_w.saturating_sub(2)) {
            lines_text.push(format!("  {wrapped}"));
        }
    } else {
        lines_text.push(format!("Are you sure you want to delete these {} items?", texts.len()));
        for t in texts {
            for wrapped in wrap_text(&format!("\"{t}\""), max_content_w.saturating_sub(2)) {
                lines_text.push(format!("  {wrapped}"));
            }
        }
    }

    let mut lines = Vec::new();
    for txt in &lines_text {
        lines.push(
            Line::from(Span::styled(
                txt.clone(),
                Style::default().fg(theme.text_primary),
            ))
            .style(Style::default().bg(theme.bg_secondary)),
        );
    }
    // blank line
    lines.push(Line::from(Span::raw("")).style(Style::default().bg(theme.bg_secondary)));
    // Y/n prompt
    lines.push(
        Line::from(vec![
            Span::styled(
                "(",
                Style::default().fg(theme.text_muted),
            ),
            Span::styled(
                "Y",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "/n)",
                Style::default().fg(theme.text_muted),
            ),
        ])
        .style(Style::default().bg(theme.bg_secondary)),
    );

    let content_width = lines_text
        .iter()
        .map(|l| l.len() as u16)
        .max()
        .unwrap_or(0)
        .max(30)
        + 4;
    let width = content_width.min(area.width.saturating_sub(4)).max(30);
    let height = lines.len() as u16 + 2;
    let popup_x = area.x + (area.width.saturating_sub(width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, width, height);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" Confirm Delete ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.bg_secondary));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg_secondary));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

pub(super) fn wrap_text(s: &str, max_width: usize) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_width {
        return vec![s.to_string()];
    }
    let mut result = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        if start + max_width >= chars.len() {
            result.push(chars[start..].iter().collect());
            break;
        }
        let end = start + max_width;
        // Find last space within max_width to break at word boundary
        if let Some(offset) = chars[start..end].iter().rposition(|c| *c == ' ') {
            let pos = start + offset;
            result.push(chars[start..pos].iter().collect());
            start = pos + 1; // skip the space
        } else {
            // No space found, hard break
            result.push(chars[start..end].iter().collect());
            start = end;
        }
    }
    result
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
        Mode::ConfirmDelete { .. } => {
            let line = Line::from(vec![
                Span::styled(
                    "  Y/Enter to confirm, any other key to cancel",
                    Style::default().fg(theme.warning),
                ),
            ]);
            (line, None)
        }
        Mode::PriorityPicker { .. } => {
            let line = Line::from(vec![
                Span::styled(
                    "  [up/down: navigate, Enter: select priority, Esc: cancel]",
                    Style::default().fg(theme.warning),
                ),
            ]);
            (line, None)
        }
        Mode::Help => {
            let line = Line::from(vec![
                Span::styled(
                    "  Esc to close help",
                    Style::default().fg(theme.warning),
                ),
            ]);
            (line, None)
        }
        Mode::Keybindings => {
            let line = Line::from(vec![
                Span::styled(
                    "  Esc to close keybindings",
                    Style::default().fg(theme.warning),
                ),
            ]);
            (line, None)
        }
        Mode::CategoryAdd => {
            let text = input.text();
            let line = Line::from(vec![
                Span::styled(
                    "  Category: ",
                    Style::default().fg(theme.accent),
                ),
                Span::styled(
                    if text.is_empty() { "\u{2588}" } else { text },
                    Style::default().fg(theme.text_primary),
                ),
                Span::styled(
                    if text.is_empty() { "" } else { "\u{2588}" },
                    Style::default().fg(theme.accent),
                ),
            ]);
            (line, None)
        }
        Mode::CategoryPicker { .. } => {
            let text = input.text();
            let line = Line::from(vec![
                Span::styled(
                    "  Assign category: ",
                    Style::default().fg(theme.accent),
                ),
                Span::styled(
                    if text.is_empty() { "\u{2588}" } else { text },
                    Style::default().fg(theme.text_primary),
                ),
                Span::styled(
                    if text.is_empty() { "" } else { "\u{2588}" },
                    Style::default().fg(theme.accent),
                ),
            ]);
            (line, None)
        }
        Mode::SortPicker { .. } => {
            let line = Line::from(vec![
                Span::styled(
                    "  [p: priority, d: due date, n: none, Enter: select, Esc: cancel]",
                    Style::default().fg(theme.warning),
                ),
            ]);
            (line, None)
        }
        Mode::DueDateInput { .. } => {
            let text = input.text();
            let cursor = input.cursor();
            let spans = if text.is_empty() {
                vec![
                    Span::styled(
                        "  Due date (YYYY-MM-DD, Esc to skip): ",
                        Style::default().fg(theme.accent),
                    ),
                    Span::styled("\u{2588}", Style::default().fg(theme.accent)),
                ]
            } else if cursor == 0 {
                vec![
                    Span::styled(
                        "  Due date (YYYY-MM-DD, Esc to skip): ",
                        Style::default().fg(theme.accent),
                    ),
                    Span::styled("\u{2588}", Style::default().fg(theme.accent)),
                    Span::styled(text, Style::default().fg(theme.text_primary)),
                ]
            } else {
                let before = &text[..cursor];
                let after = &text[cursor..];
                vec![
                    Span::styled(
                        "  Due date (YYYY-MM-DD, Esc to skip): ",
                        Style::default().fg(theme.accent),
                    ),
                    Span::styled(before, Style::default().fg(theme.text_primary)),
                    Span::styled("\u{2588}", Style::default().fg(theme.accent)),
                    Span::styled(after, Style::default().fg(theme.text_primary)),
                ]
            };
            (Line::from(spans), None)
        }
        Mode::Editing { .. } => {
            let text = input.text();
            let cursor = input.cursor();
            let spans = if cursor == 0 {
                vec![
                    Span::raw("  "),
                    Span::styled("\u{2588}", Style::default().fg(theme.accent)),
                    Span::styled(text, Style::default().fg(theme.text_primary)),
                ]
            } else {
                let before = &text[..cursor];
                let after = &text[cursor..];
                vec![
                    Span::raw("  "),
                    Span::styled(before, Style::default().fg(theme.text_primary)),
                    Span::styled("\u{2588}", Style::default().fg(theme.accent)),
                    Span::styled(after, Style::default().fg(theme.text_primary)),
                ]
            };
            (Line::from(spans), None)
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
