pub mod input;
pub mod list;
pub mod theme;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListState, Paragraph};

use crate::app::{get_filtered_commands, Mode, Pane, SortMode};
use crate::data::{Priority, TodoItem};
use crate::date::{self, Date};

use self::input::InputBuffer;
use self::theme::Theme;

pub struct RenderState<'a> {
    pub items: &'a [TodoItem],
    pub selected_index: usize,
    pub input: &'a InputBuffer,
    pub mode: &'a Mode,
    pub pending_count: usize,
    pub filter: &'a str,
    pub priority_filter: &'a Option<Priority>,
    pub pane: &'a Pane,
    pub category_index: usize,
    pub categories: &'a [String],
    pub theme: &'a Theme,
    pub sort_mode: &'a SortMode,
}

pub fn render(frame: &mut Frame, state: RenderState<'_>) {
    let area = frame.area();
    let layout = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);

    render_header(frame, layout[0], &state);

    let mid = layout[1];
    let h_layout = Layout::horizontal([Constraint::Length(22), Constraint::Min(1)]).split(mid);

    render_sidebar(
        frame,
        h_layout[0],
        state.mode,
        state.pane,
        state.category_index,
        state.categories,
        state.theme,
    );
    render_list(frame, h_layout[1], &state);
    render_input(
        frame,
        layout[2],
        state.input,
        state.mode,
        state.filter,
        state.pane,
        state.theme,
    );

    let popup_area = mid;
    if let Mode::Command { selected } = state.mode {
        render_cmd_completions(frame, popup_area, state.input, *selected, state.theme);
    } else if let Mode::ThemePicker { selected } = state.mode {
        render_theme_picker(frame, popup_area, *selected, state.theme);
    } else if let Mode::PriorityPicker { selected } = state.mode {
        render_priority_picker(frame, popup_area, *selected, state.theme);
    } else if let Mode::CategoryPicker { selected } = state.mode {
        render_category_picker(frame, popup_area, *selected, state.categories, state.theme);
    } else if let Mode::SortPicker { selected } = state.mode {
        render_sort_picker(frame, popup_area, *selected, state.theme);
    } else if let Mode::Help = state.mode {
        render_help_popup(frame, popup_area, state.theme);
    } else if let Mode::Keybindings = state.mode {
        render_keybindings_popup(frame, popup_area, state.theme);
    } else if let Mode::ConfirmDelete { texts, .. } = state.mode {
        render_confirm_delete_popup(frame, popup_area, texts, state.theme);
    } else if let Mode::DueDateCalendar {
        selected,
        prompt_focused,
        ..
    } = state.mode
    {
        render_calendar_popup(frame, popup_area, *selected, *prompt_focused, state.theme);
    }
}

fn render_header(frame: &mut Frame, area: Rect, state: &RenderState<'_>) {
    let mut spans = vec![
        Span::raw(" "),
        Span::styled(
            "TODO",
            Style::default()
                .fg(state.theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if !state.filter.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("[search: {}]", state.filter),
            Style::default().fg(state.theme.warning),
        ));
    }

    if let Some(p) = state.priority_filter {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("[filter: {:?}]", p).to_lowercase(),
            Style::default().fg(state.theme.warning),
        ));
    }

    if state.category_index > 0 {
        if let Some(cat) = state.categories.get(state.category_index - 1) {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("[cat: {}]", cat),
                Style::default().fg(state.theme.warning),
            ));
        }
    }

    match state.sort_mode {
        SortMode::Priority => {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                "[sort: priority]",
                Style::default().fg(state.theme.warning),
            ));
        }
        SortMode::DueDate => {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                "[sort: due]",
                Style::default().fg(state.theme.warning),
            ));
        }
        _ => {}
    }

    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        format!("{} pending", state.pending_count),
        Style::default().fg(state.theme.text_secondary),
    ));

    if !state.items.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{}/{}", state.selected_index + 1, state.items.len()),
            Style::default().fg(state.theme.text_muted),
        ));
    }

    let line = Line::from(spans);

    let paragraph = Paragraph::new(line)
        .style(Style::default().bg(state.theme.bg_primary))
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, area);
}

fn render_sidebar(
    frame: &mut Frame,
    area: Rect,
    mode: &Mode,
    pane: &Pane,
    category_index: usize,
    categories: &[String],
    theme: &Theme,
) {
    let selected_categories = match mode {
        Mode::MultiSelect {
            selected_categories,
            ..
        } => Some(selected_categories),
        _ => None,
    };
    let is_active = *pane == Pane::Categories;
    let content_width = area.width.saturating_sub(1).max(10) as usize;
    let name_width = content_width.saturating_sub(4).max(4);

    let mut lines = Vec::new();
    let all_filtered = !is_active && category_index == 0;
    let all_hl = is_active && category_index == 0;
    lines.push(
        Line::from(vec![
            Span::raw(" "),
            Span::styled("  ", Style::default().fg(theme.accent)),
            Span::styled(
                "All",
                Style::default()
                    .fg(theme.text_primary)
                    .add_modifier(if all_hl || all_filtered {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
        ])
        .style(Style::default().bg(if all_hl || all_filtered {
            theme.bg_tertiary
        } else {
            theme.bg_primary
        })),
    );

    for (i, cat) in categories.iter().enumerate() {
        let idx = i + 1;
        let hl = is_active && category_index == idx;
        let is_filter = !is_active && category_index == idx;
        let multi_selected = selected_categories.is_some_and(|selected| selected.contains(cat));
        let bg = if hl || is_filter {
            theme.bg_tertiary
        } else if multi_selected {
            theme.accent_selection
        } else {
            theme.bg_primary
        };
        let style = Style::default()
            .fg(theme.text_primary)
            .add_modifier(if hl || is_filter {
                Modifier::BOLD
            } else {
                Modifier::empty()
            })
            .bg(bg);

        let bullet = Span::styled(
            if multi_selected { "x " } else { "  " },
            Style::default().fg(theme.accent),
        );

        let wrapped = wrap_text(cat, name_width);
        for (j, seg) in wrapped.iter().enumerate() {
            let mut spans = vec![Span::raw(" ")];
            if j == 0 {
                spans.push(bullet.clone());
                spans.push(Span::styled(seg.clone(), style));
            } else {
                spans.push(Span::raw("    "));
                spans.push(Span::styled(seg.clone(), style));
            }
            lines.push(Line::from(spans).style(Style::default().bg(bg)));
        }
    }

    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(if is_active {
            theme.accent
        } else {
            theme.border_default
        }))
        .title(" Categories ")
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme.bg_primary));

    let list = List::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_primary));

    frame.render_widget(list, area);
}

fn render_list(frame: &mut Frame, area: Rect, state: &RenderState<'_>) {
    let selected_ids = match state.mode {
        Mode::MultiSelect { ref selected, .. } => Some(selected),
        _ => None,
    };

    let is_active = *state.pane == Pane::Items;
    let text_width = area.width.saturating_sub(7) as usize;
    let hide_category_badge = state.category_index > 0;

    let list_items: Vec<_> = state
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let multi_sel = selected_ids.is_some_and(|ids| ids.contains(&item.id));
            self::list::render_item(
                item,
                is_active && i == state.selected_index,
                multi_sel,
                state.theme,
                text_width,
                state.filter,
                hide_category_badge,
            )
        })
        .collect();

    let mut list_state = ListState::default().with_selected(if is_active {
        Some(state.selected_index)
    } else {
        None
    });

    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(if is_active {
            state.theme.accent
        } else {
            state.theme.border_default
        }))
        .title(" Items ")
        .title_style(
            Style::default()
                .fg(state.theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(state.theme.bg_primary));

    let list = List::new(list_items)
        .block(block)
        .style(Style::default().bg(state.theme.bg_primary))
        .highlight_style(Style::default().bg(state.theme.bg_tertiary));

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_cmd_completions(
    frame: &mut Frame,
    area: Rect,
    input: &InputBuffer,
    selected: usize,
    theme: &Theme,
) {
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
        let bg = if highlighted {
            theme.bg_tertiary
        } else {
            theme.bg_primary
        };

        lines.push(
            Line::from(vec![
                Span::styled(
                    format!("  /{:<1$}", cmd, cmd_width),
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(if highlighted {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                Span::styled(desc.to_string(), Style::default().fg(theme.text_secondary)),
            ])
            .style(Style::default().bg(bg)),
        );
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_default))
        .title(" Commands ")
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
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
        let bg = if is_highlighted {
            theme.bg_tertiary
        } else {
            theme.bg_primary
        };

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
                        .fg(if is_active {
                            theme.accent
                        } else {
                            theme.text_primary
                        })
                        .add_modifier(if is_highlighted || is_active {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
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
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
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
        let bg = if is_highlighted {
            theme.bg_tertiary
        } else {
            theme.bg_primary
        };

        lines.push(
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    label.to_string(),
                    Style::default()
                        .fg(theme.text_primary)
                        .add_modifier(if is_highlighted {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
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
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme.bg_primary));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_primary));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn render_category_picker(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    categories: &[String],
    theme: &Theme,
) {
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
    for (i, label) in entries
        .iter()
        .copied()
        .chain(categories.iter().map(|s| s.as_str()))
        .enumerate()
    {
        let is_highlighted = i == selected;
        let bg = if is_highlighted {
            theme.bg_tertiary
        } else {
            theme.bg_primary
        };

        lines.push(
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    label.to_string(),
                    Style::default()
                        .fg(if i == 0 {
                            theme.text_muted
                        } else {
                            theme.text_primary
                        })
                        .add_modifier(if is_highlighted {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
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
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
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
        let bg = if is_highlighted {
            theme.bg_tertiary
        } else {
            theme.bg_primary
        };

        lines.push(
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    label.to_string(),
                    Style::default()
                        .fg(theme.text_primary)
                        .add_modifier(if is_highlighted {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
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
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
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
        .join("todo-tui")
        .join("config.json");
    let data = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/state"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("todo-tui")
        .join("todos.json");
    (config.display().to_string(), data.display().to_string())
}

fn render_help_popup(frame: &mut Frame, area: Rect, theme: &Theme) {
    let (config_path, data_path) = help_file_paths();
    let lines = vec![
        Line::from(vec![
            Span::styled("  Welcome to ", Style::default().fg(theme.text_primary)),
            Span::styled(
                "todo-tui",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::raw("")).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            "  /command or /<alias> — run a command",
            Style::default().fg(theme.text_primary),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            "  /help           — this screen",
            Style::default().fg(theme.text_primary),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            "  /keybindings    — show all keybindings",
            Style::default().fg(theme.text_primary),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            "  /priorities     — filter by priority",
            Style::default().fg(theme.text_primary),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            "  /search <q>     — filter items by text",
            Style::default().fg(theme.text_primary),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            "  /delete         — bulk delete (multi-select)",
            Style::default().fg(theme.text_primary),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            "  /done           — bulk toggle done",
            Style::default().fg(theme.text_primary),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            "  /clear          — clear completed items",
            Style::default().fg(theme.text_primary),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            "  /themes         — pick a theme",
            Style::default().fg(theme.text_primary),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::raw("")).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            "  Tab autocompletes commands, Esc cancels",
            Style::default().fg(theme.text_muted),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::raw("")).style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            "  Files:",
            Style::default()
                .fg(theme.text_muted)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            format!("  Config: {}", config_path),
            Style::default().fg(theme.text_secondary),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            format!("  Data:   {}", data_path),
            Style::default().fg(theme.text_secondary),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
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
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
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
        ("/delete", "Bulk-select categories", false),
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
        ("", "", false),
        ("Due date", "", true),
        ("Tab", "Switch calendar/prompt", false),
        ("calendar arrows", "Move day/week", false),
        ("prompt arrows", "Move cursor", false),
        ("digits / -", "Edit date prompt", false),
        ("PgUp/PgDn", "Previous/next month", false),
        ("t", "Today", false),
        ("Backspace", "Edit date text", false),
        ("Delete", "Clear due date", false),
        ("Enter", "Save valid date", false),
        ("Esc", "Cancel", false),
    ];

    let mut lines = Vec::new();
    for (key, desc, is_header) in &key_lines {
        if *is_header {
            lines.push(
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        key.to_string(),
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
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
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme.bg_secondary));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_secondary));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn render_confirm_delete_popup(frame: &mut Frame, area: Rect, texts: &[String], theme: &Theme) {
    let max_content_w = area.width.saturating_sub(6).clamp(20, 60) as usize;

    let mut lines_text: Vec<String> = Vec::new();
    if texts.len() == 1 {
        lines_text.push("Are you sure you want to delete?".into());
        for wrapped in wrap_text(
            &format!("\"{}\"", &texts[0]),
            max_content_w.saturating_sub(2),
        ) {
            lines_text.push(format!("  {wrapped}"));
        }
    } else {
        lines_text.push(format!(
            "Are you sure you want to delete these {} items?",
            texts.len()
        ));
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
            Span::styled("(", Style::default().fg(theme.text_muted)),
            Span::styled(
                "Y",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("/n)", Style::default().fg(theme.text_muted)),
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
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
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

fn render_calendar_popup(
    frame: &mut Frame,
    area: Rect,
    selected: Date,
    prompt_focused: bool,
    theme: &Theme,
) {
    let width = 40;
    let height = 13;
    let popup_x = area.x + (area.width.saturating_sub(width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, width, height);

    let today = Date::today();
    let first = Date::new(selected.year, selected.month, 1).unwrap_or(selected);
    let first_weekday = first.weekday_monday0();
    let days_in_month = date::days_in_month(selected.year, selected.month);

    let mut lines = Vec::new();
    lines.push(
        Line::from(vec![Span::styled(
            format!("  {} {}  ", date::month_name(selected.month), selected.year),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )])
        .alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg_secondary)),
    );
    lines.push(
        Line::from(vec![Span::styled(
            "  Mon  Tue  Wed  Thu  Fri  Sat  Sun",
            Style::default().fg(theme.text_muted),
        )])
        .style(Style::default().bg(theme.bg_secondary)),
    );

    let mut day = 1u32;
    for week in 0..6 {
        let mut spans = vec![Span::raw(" ")];
        for weekday in 0..7 {
            let cell_index = week * 7 + weekday;
            if cell_index < first_weekday || day > days_in_month {
                spans.push(Span::raw("     "));
                continue;
            }

            let date = Date {
                year: selected.year,
                month: selected.month,
                day,
            };
            let is_selected = date == selected;
            let is_today = date == today;
            let mut style = Style::default().fg(if prompt_focused {
                theme.text_secondary
            } else {
                theme.text_primary
            });
            if is_today {
                style = style.fg(theme.warning).add_modifier(Modifier::BOLD);
            }
            if is_selected {
                style = if prompt_focused {
                    style.fg(theme.accent).add_modifier(Modifier::BOLD)
                } else {
                    style
                        .fg(theme.text_primary)
                        .bg(theme.accent_selection)
                        .add_modifier(Modifier::BOLD)
                };
            }

            spans.push(Span::styled(format!(" {:>2}  ", day), style));
            day += 1;
        }
        lines.push(Line::from(spans).style(Style::default().bg(theme.bg_secondary)));
    }

    lines.push(
        Line::from(vec![Span::styled(
            if prompt_focused {
                "  Tab calendar   Enter save   Esc cancel"
            } else {
                "  Tab prompt   Arrows move   Pg month   t today   Del clear"
            },
            Style::default().fg(theme.text_muted),
        )])
        .style(Style::default().bg(theme.bg_secondary)),
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if prompt_focused {
            theme.border_default
        } else {
            theme.accent
        }))
        .title(if prompt_focused {
            " Due Date - Prompt * "
        } else {
            " Due Date - Calendar * "
        })
        .title_style(
            Style::default()
                .fg(if prompt_focused {
                    theme.text_muted
                } else {
                    theme.accent
                })
                .add_modifier(if prompt_focused {
                    Modifier::empty()
                } else {
                    Modifier::BOLD
                }),
        )
        .style(Style::default().bg(theme.bg_secondary));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_secondary));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn render_calendar_date_field(
    text: &str,
    cursor: usize,
    typed_valid: bool,
    prompt_focused: bool,
    theme: &Theme,
) -> Line<'static> {
    let focus_color = if prompt_focused {
        theme.accent
    } else {
        theme.text_muted
    };
    let mut spans = vec![Span::styled("  Date: ", Style::default().fg(focus_color))];
    let date_style = Style::default().fg(if typed_valid {
        if prompt_focused {
            theme.text_primary
        } else {
            theme.text_secondary
        }
    } else {
        theme.warning
    });
    if text.is_empty() {
        spans.push(Span::styled("\u{2588}", Style::default().fg(focus_color)));
    } else {
        let cursor = cursor.min(text.len());
        let before = &text[..cursor];
        let after = &text[cursor..];
        spans.push(Span::styled(before.to_string(), date_style));
        spans.push(Span::styled("\u{2588}", Style::default().fg(focus_color)));
        spans.push(Span::styled(after.to_string(), date_style));
    }
    Line::from(spans)
}

fn render_input(
    frame: &mut Frame,
    area: Rect,
    input: &InputBuffer,
    mode: &Mode,
    filter: &str,
    pane: &Pane,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme.border_default));

    let (display_text, cursor_pos) = match mode {
        Mode::Searching => {
            let line = Line::from(vec![
                Span::styled("  Search: ", Style::default().fg(theme.accent)),
                Span::styled(filter, Style::default().fg(theme.text_primary)),
                Span::styled("\u{2588}", Style::default().fg(theme.accent)),
            ]);
            (line, None::<u16>)
        }
        Mode::Command { .. } => {
            let mut spans = vec![Span::styled("  /", Style::default().fg(theme.accent))];

            let text = input.text();
            if text.is_empty() {
                spans.push(Span::styled("\u{2588}", Style::default().fg(theme.accent)));
            } else {
                spans.push(Span::styled(
                    text.to_string(),
                    Style::default().fg(theme.text_primary),
                ));
                spans.push(Span::styled("\u{2588}", Style::default().fg(theme.accent)));
            }

            (Line::from(spans), None)
        }
        Mode::MultiSelect { cmd, .. } => {
            let target = if matches!(pane, Pane::Categories)
                && matches!(cmd, crate::app::MultiSelectCmd::Delete)
            {
                "categories"
            } else {
                "items"
            };
            let line = Line::from(vec![Span::styled(
                format!("  [select {target}, Enter to confirm, Esc to cancel]"),
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::ThemePicker { .. } => {
            let line = Line::from(vec![Span::styled(
                "  [up/down: navigate, Enter: select theme, Esc: cancel]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::ConfirmDelete { .. } => {
            let line = Line::from(vec![Span::styled(
                "  Y/Enter to confirm, any other key to cancel",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::PriorityPicker { .. } => {
            let line = Line::from(vec![Span::styled(
                "  [up/down: navigate, Enter: select priority, Esc: cancel]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::Help => {
            let line = Line::from(vec![Span::styled(
                "  Esc to close help",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::Keybindings => {
            let line = Line::from(vec![Span::styled(
                "  Esc to close keybindings",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::CategoryAdd => {
            let text = input.text();
            let line = Line::from(vec![
                Span::styled("  Category: ", Style::default().fg(theme.accent)),
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
                Span::styled("  Assign category: ", Style::default().fg(theme.accent)),
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
            let line = Line::from(vec![Span::styled(
                "  [p: priority, d: due date, n: none, Enter: select, Esc: cancel]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::DueDateCalendar { prompt_focused, .. } => {
            let typed = input.text();
            let typed_valid = date::Date::parse(typed).is_some();
            let mut line = render_calendar_date_field(
                typed,
                input.cursor(),
                typed_valid,
                *prompt_focused,
                theme,
            );
            line.spans.push(Span::styled(
                if *prompt_focused {
                    "  Tab calendar   Enter save"
                } else {
                    "  Tab prompt   calendar active"
                },
                Style::default().fg(if *prompt_focused {
                    theme.accent
                } else {
                    theme.text_muted
                }),
            ));
            (line.style(Style::default().bg(theme.bg_primary)), None)
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
            let placeholder = Line::from(vec![Span::styled(
                "  Type to add or / for commands",
                Style::default().fg(theme.text_placeholder),
            )]);
            (placeholder, None)
        }
        _ => {
            let text = input.text();
            let cursor = input.cursor();

            if cursor == 0 {
                let line = Line::from(vec![
                    Span::raw("  "),
                    Span::styled("\u{2588}", Style::default().fg(theme.accent)),
                    Span::styled(text, Style::default().fg(theme.text_primary)),
                ]);
                (line, None)
            } else {
                let before = &text[..cursor];
                let after = &text[cursor..];
                let line = Line::from(vec![
                    Span::raw("  "),
                    Span::styled(before, Style::default().fg(theme.text_primary)),
                    Span::styled("\u{2588}", Style::default().fg(theme.accent)),
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
        let x = area.x + col;
        let y = area.y + 1;
        #[allow(deprecated)]
        frame.set_cursor(x, y);
    }
}
