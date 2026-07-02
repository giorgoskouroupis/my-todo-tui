pub mod input;
pub mod list;
pub mod theme;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListState, Paragraph};

use crate::app::{get_filtered_commands, CategoryPickerTarget, DueFilter, Mode, Pane, SortMode};
use crate::data::{CategoryEntry, Priority, TodoItem};
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
    pub due_filter: &'a Option<DueFilter>,
    pub show_archived: bool,
    pub pane: &'a Pane,
    pub category_index: usize,
    pub categories: &'a [String],
    pub category_entries: &'a [CategoryEntry],
    pub theme: &'a Theme,
    pub sort_mode: &'a SortMode,
}

pub fn render(frame: &mut Frame, state: RenderState<'_>) {
    let area = frame.area();
    let dim_theme;
    let base_theme = if matches!(state.mode, Mode::DueDateCalendar { .. }) {
        dim_theme = dimmed_theme(state.theme);
        &dim_theme
    } else {
        state.theme
    };
    let layout = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);

    render_header(frame, layout[0], &state, base_theme);

    let mid = layout[1];
    let h_layout = Layout::horizontal([Constraint::Length(22), Constraint::Min(1)]).split(mid);

    render_sidebar(
        frame,
        h_layout[0],
        state.mode,
        state.pane,
        state.category_index,
        state.category_entries,
        base_theme,
    );
    render_list(frame, h_layout[1], &state, base_theme);
    render_input(
        frame,
        layout[2],
        state.input,
        state.mode,
        state.filter,
        state.pane,
        base_theme,
    );

    let popup_area = mid;
    if let Mode::Command { selected } = state.mode {
        render_cmd_completions(frame, popup_area, state.input, *selected, state.theme);
    } else if let Mode::ThemePicker { selected, .. } = state.mode {
        render_theme_picker(frame, popup_area, *selected, state.theme);
    } else if let Mode::PriorityPicker { selected } = state.mode {
        render_priority_picker(frame, popup_area, *selected, state.theme);
    } else if let Mode::ArchivePicker { selected } = state.mode {
        render_archive_picker(frame, popup_area, *selected, state.theme);
    } else if let Mode::FilterPicker { selected } = state.mode {
        render_filter_picker(frame, popup_area, *selected, state.theme);
    } else if let Mode::DueDateFilterPicker { selected } = state.mode {
        render_due_date_filter_picker(frame, popup_area, *selected, state.theme);
    } else if let Mode::CategoryPicker { selected, target } = state.mode {
        render_category_picker(
            frame,
            popup_area,
            *selected,
            target,
            state.categories,
            state.theme,
        );
    } else if let Mode::CategoryFilterPicker { selected } = state.mode {
        render_category_filter_picker(
            frame,
            popup_area,
            *selected,
            state.category_entries,
            state.theme,
        );
    } else if let Mode::CategoryCreateChoice {
        selected,
        parent,
        name,
    } = state.mode
    {
        render_category_create_choice(
            frame,
            popup_area,
            *selected,
            parent.as_deref(),
            name,
            state.theme,
        );
    } else if let Mode::CategoryParentPicker { selected, name } = state.mode {
        render_category_parent_picker(
            frame,
            popup_area,
            *selected,
            name,
            state.categories,
            state.theme,
        );
    } else if let Mode::SortPicker { selected, .. } = state.mode {
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
        from_normal,
        ..
    } = state.mode
    {
        if *prompt_focused {
            render_input(
                frame,
                layout[2],
                state.input,
                state.mode,
                state.filter,
                state.pane,
                state.theme,
            );
        }
        let anchor = if *from_normal {
            calendar_item_anchor(h_layout[1], state.items, state.selected_index)
        } else {
            calendar_prompt_anchor(layout[2])
        };
        render_calendar_popup(
            frame,
            popup_area,
            anchor,
            *selected,
            *prompt_focused,
            state.theme,
        );
    }
}

fn render_prompt_input_line<'a>(
    label: &str,
    text: &str,
    cursor: usize,
    width: usize,
    theme: &Theme,
) -> Line<'a> {
    let label_width = label.chars().count();
    let visible_width = width.saturating_sub(label_width).max(1);
    let cursor = cursor.min(text.len());
    let cursor_char = text[..cursor].chars().count();
    let text_chars: Vec<char> = text.chars().collect();
    let total_chars = text_chars.len();

    let mut content_width = visible_width.saturating_sub(1).max(1);
    let mut start = 0usize;
    let mut end = total_chars.min(content_width);

    for _ in 0..4 {
        start = if cursor_char >= content_width {
            cursor_char + 1 - content_width
        } else {
            0
        };
        end = (start + content_width).min(total_chars);
        let clipped_left = start > 0;
        let clipped_right = end < total_chars;
        let marker_width = usize::from(clipped_left) * 3 + usize::from(clipped_right) * 3;
        let adjusted_content_width = visible_width.saturating_sub(1 + marker_width).max(1);
        if adjusted_content_width == content_width {
            break;
        }
        content_width = adjusted_content_width;
    }

    let clipped_left = start > 0;
    let clipped_right = end < total_chars;
    let before_cursor: String = text_chars[start..cursor_char.min(end)].iter().collect();
    let after_cursor: String = if cursor_char < end {
        text_chars[cursor_char..end].iter().collect()
    } else {
        String::new()
    };

    let mut spans = vec![Span::styled(
        label.to_string(),
        Style::default().fg(theme.accent),
    )];
    if clipped_left {
        spans.push(Span::styled("...", Style::default().fg(theme.text_muted)));
    }
    spans.push(Span::styled(
        before_cursor,
        Style::default().fg(theme.text_primary),
    ));
    spans.push(Span::styled("\u{2588}", Style::default().fg(theme.accent)));
    spans.push(Span::styled(
        after_cursor,
        Style::default().fg(theme.text_primary),
    ));
    if clipped_right {
        spans.push(Span::styled("...", Style::default().fg(theme.text_muted)));
    }

    Line::from(spans)
}

fn dimmed_theme(theme: &Theme) -> Theme {
    let mut dimmed = theme.clone();
    dimmed.accent = theme.text_muted;
    dimmed.success = theme.text_muted;
    dimmed.warning = theme.text_muted;
    dimmed.error = theme.text_muted;
    dimmed.text_primary = theme.text_muted;
    dimmed.text_secondary = theme.text_muted;
    dimmed.border_default = theme.text_muted;
    dimmed.bg_tertiary = theme.bg_secondary;
    dimmed.accent_selection = theme.bg_secondary;
    dimmed
}

fn calendar_prompt_anchor(input_area: Rect) -> Rect {
    Rect::new(
        input_area.x,
        input_area.y,
        input_area.width,
        input_area.height,
    )
}

fn calendar_item_anchor(list_area: Rect, items: &[TodoItem], selected_index: usize) -> Rect {
    let content_width = list_area.width.saturating_sub(7) as usize;
    let mut y = list_area.y;
    for item in items.iter().take(selected_index) {
        let text_lines = wrap_text(&item.text, content_width.max(10)).len() as u16;
        y = y.saturating_add(1 + text_lines);
        if y >= list_area.bottom().saturating_sub(1) {
            break;
        }
    }

    Rect::new(
        list_area.x.saturating_add(1),
        y.min(list_area.bottom().saturating_sub(1)),
        list_area.width.saturating_sub(2),
        1,
    )
}

fn render_header(frame: &mut Frame, area: Rect, state: &RenderState<'_>, theme: &Theme) {
    let mut spans = vec![
        Span::raw(" "),
        Span::styled(
            "TODO",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if !state.filter.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("[search: {}]", state.filter),
            Style::default().fg(theme.warning),
        ));
    }

    if let Some(p) = state.priority_filter {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("[filter: {:?}]", p).to_lowercase(),
            Style::default().fg(theme.warning),
        ));
    }

    if let Some(filter) = state.due_filter {
        spans.push(Span::raw("  "));
        let label = match filter {
            DueFilter::Today => "today",
            DueFilter::Week => "week",
            DueFilter::Overdue => "overdue",
        };
        spans.push(Span::styled(
            format!("[due: {label}]"),
            Style::default().fg(theme.warning),
        ));
    }

    if state.show_archived {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            "[archived]",
            Style::default().fg(theme.warning),
        ));
    }

    if state.category_index > 0 {
        if let Some(cat) = state.category_entries.get(state.category_index - 1) {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("[cat: {}]", display_category_path(&cat.path)),
                Style::default().fg(theme.warning),
            ));
        }
    }

    match state.sort_mode {
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
        format!("{} pending", state.pending_count),
        Style::default().fg(theme.text_secondary),
    ));

    if !state.items.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{}/{}", state.selected_index + 1, state.items.len()),
            Style::default().fg(theme.text_muted),
        ));
    }

    let line = Line::from(spans);

    let paragraph = Paragraph::new(line)
        .style(Style::default().bg(theme.bg_primary))
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, area);
}

fn display_category_path(path: &str) -> String {
    path.replace('/', "|")
}

fn render_sidebar(
    frame: &mut Frame,
    area: Rect,
    mode: &Mode,
    pane: &Pane,
    category_index: usize,
    categories: &[CategoryEntry],
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
    let separator = "\u{2500}".repeat(content_width.saturating_sub(1));
    lines.push(
        Line::from(vec![
            Span::raw(" "),
            Span::styled(separator, Style::default().fg(theme.text_muted)),
        ])
        .style(Style::default().bg(theme.bg_primary)),
    );

    for (i, cat) in categories.iter().enumerate() {
        let idx = i + 1;
        let hl = is_active && category_index == idx;
        let is_filter = !is_active && category_index == idx;
        let multi_selected =
            selected_categories.is_some_and(|selected| selected.contains(&cat.path));
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

        let prefix = if cat.depth == 0 { "" } else { " - " };
        let label_style = if cat.is_all {
            style.fg(theme.text_muted)
        } else {
            style
        };
        let wrapped = wrap_text(&cat.label, name_width.saturating_sub(prefix.len()));
        for (j, seg) in wrapped.iter().enumerate() {
            let mut spans = vec![Span::raw(" ")];
            if j == 0 {
                spans.push(bullet.clone());
                spans.push(Span::raw(prefix));
                spans.push(Span::styled(seg.clone(), label_style));
            } else {
                spans.push(Span::raw(if cat.depth == 0 { "   " } else { "      " }));
                spans.push(Span::styled(seg.clone(), label_style));
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

fn render_list(frame: &mut Frame, area: Rect, state: &RenderState<'_>, theme: &Theme) {
    let selected_ids = match state.mode {
        Mode::MultiSelect { ref selected, .. } => Some(selected),
        _ => None,
    };

    let is_active = *state.pane == Pane::Items;
    let text_width = area.width.saturating_sub(7) as usize;
    let category_filter = if state.show_archived {
        None
    } else {
        state
            .category_index
            .checked_sub(1)
            .and_then(|idx| state.category_entries.get(idx))
            .map(|entry| entry.path.as_str())
    };

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
                theme,
                text_width,
                state.filter,
                category_filter,
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
            theme.accent
        } else {
            theme.border_default
        }))
        .title(" Items ")
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme.bg_primary));

    let list = List::new(list_items)
        .block(block)
        .style(Style::default().bg(theme.bg_primary))
        .highlight_style(Style::default().bg(theme.bg_tertiary));

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

    let safe_selected = selected.min(matches.len().saturating_sub(1));
    let parent_command = if prefix.contains(char::is_whitespace) {
        prefix.split_whitespace().next().map(str::to_string)
    } else {
        None
    };
    let max_rows = area.height.saturating_sub(3).max(1) as usize;
    let visible_rows = matches.len().min(max_rows);
    let height = visible_rows as u16 + 2;
    let display_labels: Vec<String> = matches
        .iter()
        .map(|(cmd, _)| command_display_label(cmd, parent_command.as_deref()))
        .collect();
    let cmd_width = display_labels
        .iter()
        .map(String::len)
        .max()
        .unwrap_or(18)
        .max(14);
    let width = ((cmd_width + 48) as u16).min(area.width.saturating_sub(2));
    let popup_y = area.bottom().saturating_sub(height + 1);
    let popup_x = area.x + 2;

    let popup_area = Rect::new(
        popup_x,
        popup_y.min(area.bottom().saturating_sub(height)),
        width,
        height,
    );

    let start = safe_selected.saturating_sub(visible_rows.saturating_sub(1));
    let end = (start + visible_rows).min(matches.len());
    let mut lines = Vec::new();
    for (i, ((cmd, desc), display_label)) in matches
        .iter()
        .zip(display_labels.iter())
        .enumerate()
        .take(end)
        .skip(start)
    {
        let highlighted = i == safe_selected;
        let bg = if highlighted {
            theme.bg_tertiary
        } else {
            theme.bg_primary
        };

        lines.push(
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {}{:<2$}",
                        command_display_prefix(cmd, parent_command.as_deref()),
                        display_label,
                        cmd_width
                    ),
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
        .title(command_popup_title(parent_command.as_deref()))
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

fn command_popup_title(parent_command: Option<&str>) -> String {
    match parent_command {
        Some(parent) => format!(" /{parent} "),
        None => " Commands ".to_string(),
    }
}

fn command_display_label(command: &str, parent_command: Option<&str>) -> String {
    if let Some(parent) = parent_command {
        if let Some(rest) = command.strip_prefix(&format!("{parent} ")) {
            return rest.to_string();
        }
    }
    command.to_string()
}

fn command_display_prefix(command: &str, parent_command: Option<&str>) -> &'static str {
    if parent_command.is_some_and(|parent| command.starts_with(&format!("{parent} "))) {
        " "
    } else {
        "/"
    }
}

fn render_theme_picker(frame: &mut Frame, area: Rect, selected: usize, theme: &Theme) {
    let names = Theme::theme_names();
    let height = names.len() as u16 + 2;
    let width = 30u16.min(area.width.saturating_sub(2));
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
        .title(" Themes ")
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
    render_menu_popup(
        frame,
        area,
        " Priorities ",
        &["All priorities", "Urgent", "High", "Normal", "Low"],
        selected,
        24,
        theme,
    );
}

fn render_menu_popup(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: &[&str],
    selected: usize,
    width: u16,
    theme: &Theme,
) {
    let height = (items.len() as u16 + 2).min(area.height.saturating_sub(1));
    let width = width.min(area.width.saturating_sub(2));
    let popup_y = area.bottom().saturating_sub(height + 1);
    let popup_x = area.x + 2;
    let popup_area = Rect::new(
        popup_x,
        popup_y.min(area.bottom().saturating_sub(height)),
        width,
        height,
    );

    let visible_rows = height.saturating_sub(2) as usize;
    let safe_selected = selected.min(items.len().saturating_sub(1));
    let start = safe_selected.saturating_sub(visible_rows.saturating_sub(1));
    let lines: Vec<_> = items
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_rows)
        .map(|(i, label)| {
            let is_highlighted = i == safe_selected;
            let bg = if is_highlighted {
                theme.bg_tertiary
            } else {
                theme.bg_primary
            };
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    (*label).to_string(),
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
            .style(Style::default().bg(bg))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_default))
        .title(title)
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

fn render_archive_picker(frame: &mut Frame, area: Rect, selected: usize, theme: &Theme) {
    render_menu_popup(
        frame,
        area,
        " Archive ",
        &[
            "Archive bulk",
            "Archive done",
            "Archive one",
            "Archive all",
            "Archived view",
            "Restore bulk",
            "Restore one",
            "Restore all",
        ],
        selected,
        24,
        theme,
    );
}

fn render_filter_picker(frame: &mut Frame, area: Rect, selected: usize, theme: &Theme) {
    render_menu_popup(
        frame,
        area,
        " Filter ",
        &["Archived", "Category", "Due date", "Priority", "Clear all"],
        selected,
        24,
        theme,
    );
}

fn render_due_date_filter_picker(frame: &mut Frame, area: Rect, selected: usize, theme: &Theme) {
    render_menu_popup(
        frame,
        area,
        " Due Filter ",
        &["Overdue", "This week", "Today", "Clear filter"],
        selected,
        24,
        theme,
    );
}

fn render_category_picker(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    target: &CategoryPickerTarget,
    categories: &[String],
    theme: &Theme,
) {
    let is_move = matches!(target, CategoryPickerTarget::MoveCategory(_));
    let display_categories: Vec<&str> = if is_move {
        categories.iter().filter(|c| !c.contains('/')).map(|s| s.as_str()).collect()
    } else {
        categories.iter().map(|s| s.as_str()).collect()
    };
    let total = display_categories.len() + 1;
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
    let entries: [&str; 1] = [match target {
        CategoryPickerTarget::AssignItem => "None",
        CategoryPickerTarget::MoveCategory(_) => "Root",
    }];
    for (i, label) in entries
        .iter()
        .copied()
        .chain(display_categories.iter().copied())
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
        .title(match target {
            CategoryPickerTarget::AssignItem => " Assign Category ",
            CategoryPickerTarget::MoveCategory(_) => " Move Category ",
        })
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

fn render_category_filter_picker(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    categories: &[CategoryEntry],
    theme: &Theme,
) {
    let total = categories.len() + 1;
    let height = (total as u16 + 2).min(area.height.saturating_sub(1));
    let width = 36u16.min(area.width.saturating_sub(2));
    let popup_y = area.bottom().saturating_sub(height + 1);
    let popup_x = area.x + 2;

    let popup_area = Rect::new(
        popup_x,
        popup_y.min(area.bottom().saturating_sub(height)),
        width,
        height,
    );

    let mut lines = Vec::new();
    let visible_rows = height.saturating_sub(2) as usize;
    let safe_selected = selected.min(total.saturating_sub(1));
    let start = safe_selected.saturating_sub(visible_rows.saturating_sub(1));

    for (i, label) in std::iter::once("All")
        .chain(categories.iter().map(|entry| entry.path.as_str()))
        .enumerate()
        .skip(start)
        .take(visible_rows)
    {
        let is_highlighted = i == safe_selected;
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
        .title(" Filter Category ")
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

fn render_category_create_choice(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    parent: Option<&str>,
    name: &str,
    theme: &Theme,
) {
    let first = match parent {
        Some(parent) => format!("Attach to {parent}"),
        None => "Attach to existing category".to_string(),
    };
    let items = [first, "Create at root".to_string()];
    let height = items.len() as u16 + 3;
    let width = items
        .iter()
        .map(|item| item.len() as u16)
        .max()
        .unwrap_or(24)
        .saturating_add(8)
        .max(32);
    let popup_y = area.bottom().saturating_sub(height + 1);
    let popup_x = area.x + 2;
    let popup_area = Rect::new(
        popup_x,
        popup_y.min(area.bottom().saturating_sub(height)),
        width,
        height,
    );

    let mut lines = vec![Line::from(vec![
        Span::raw("  "),
        Span::styled(name.to_string(), Style::default().fg(theme.text_secondary)),
    ])
    .style(Style::default().bg(theme.bg_primary))];
    lines.extend(items.iter().enumerate().map(|(i, label)| {
        let is_highlighted = i == selected.min(items.len().saturating_sub(1));
        let bg = if is_highlighted {
            theme.bg_tertiary
        } else {
            theme.bg_primary
        };
        Line::from(vec![
            Span::styled(format!("  {}. ", i + 1), Style::default().fg(theme.accent)),
            Span::styled(
                label.clone(),
                Style::default()
                    .fg(theme.text_primary)
                    .add_modifier(if is_highlighted {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
        ])
        .style(Style::default().bg(bg))
    }));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_default))
        .title(" New Category ")
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

fn render_category_parent_picker(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    name: &str,
    categories: &[String],
    theme: &Theme,
) {
    let mut roots: Vec<&str> = categories
        .iter()
        .filter_map(|cat| cat.split('/').next())
        .filter(|root| !root.is_empty())
        .collect();
    roots.sort_unstable();
    roots.dedup();
    if roots.is_empty() {
        return;
    }

    let height = roots.len().min(10) as u16 + 2;
    let width = roots
        .iter()
        .map(|root| root.len() as u16)
        .max()
        .unwrap_or(20)
        .saturating_add(4)
        .max(28);
    let popup_y = area.bottom().saturating_sub(height + 1);
    let popup_x = area.x + 2;
    let popup_area = Rect::new(
        popup_x,
        popup_y.min(area.bottom().saturating_sub(height)),
        width,
        height,
    );

    let safe_selected = selected.min(roots.len().saturating_sub(1));
    let lines: Vec<Line<'static>> = roots
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, root)| {
            let is_highlighted = i == safe_selected;
            let bg = if is_highlighted {
                theme.bg_tertiary
            } else {
                theme.bg_primary
            };
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    root.to_string(),
                    Style::default()
                        .fg(theme.text_primary)
                        .add_modifier(if is_highlighted {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
            ])
            .style(Style::default().bg(bg))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_default))
        .title(format!(" Attach {name} Under "))
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
    render_menu_popup(
        frame,
        area,
        " Sort ",
        &["Default", "Due date", "Priority"],
        selected,
        24,
        theme,
    );
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
            "  /filter         — filter items (due, priority, category, archived)",
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
            "  /archive        — archive items/categories (done, all, restore)",
            Style::default().fg(theme.text_primary),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            "  /move           — move item or highlighted category",
            Style::default().fg(theme.text_primary),
        ))
        .style(Style::default().bg(theme.bg_secondary)),
        Line::from(Span::styled(
            "  /rename         — rename current item/category",
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
            "  Tab autocompletes commands/subcommands, Esc cancels",
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

    let width = 64.min(area.width.saturating_sub(2));
    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(1));
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
        ("Global", "", true),
        ("Ctrl+C", "Quit from any mode", false),
        ("Ctrl+/ or Ctrl+_", "Show keybindings", false),
        (
            "/",
            "Open command mode, except command/category typing",
            false,
        ),
        ("", "", false),
        ("Items", "", true),
        ("↑/↓", "Navigate", false),
        ("Enter", "Edit selected item, create if list empty", false),
        (
            "printable key",
            "Start a new item with that character",
            false,
        ),
        ("Space", "Toggle done", false),
        ("d", "Toggle doing", false),
        ("Delete", "Delete item", false),
        ("p / P", "Cycle priority forward/back", false),
        ("Alt+↑/↓", "Move item up/down", false),
        ("u or Ctrl+Shift+Z", "Undo last delete", false),
        ("*", "Toggle pin/star", false),
        ("s", "Open sort picker", false),
        ("Ctrl+K", "Open category picker", false),
        ("Ctrl+D", "Open due-date calendar", false),
        ("←/→", "Switch items/sidebar pane", false),
        ("PgUp/PgDn", "Switch category filter", false),
        ("Ctrl+PgUp/PgDn", "Switch top-level category filter", false),
        ("", "", false),
        ("Sidebar", "", true),
        ("↑/↓", "Navigate categories", false),
        ("Enter", "Select category and switch to items", false),
        ("printable key", "Start category creation", false),
        ("Delete", "Delete highlighted category", false),
        ("→", "Switch to items pane", false),
        ("PgUp/PgDn", "Switch category filter", false),
        ("Ctrl+PgUp/PgDn", "Switch top-level category filter", false),
        ("", "", false),
        ("Command", "", true),
        ("type", "Filter command list / edit command text", false),
        ("↑/↓", "Move highlighted command", false),
        ("Tab", "Autocomplete highlighted command/subcommand", false),
        ("Enter", "Execute typed or highlighted command", false),
        ("Esc", "Cancel", false),
        ("", "", false),
        ("Multi-select", "", true),
        ("↑/↓", "Navigate targets", false),
        ("Space", "Toggle current target", false),
        ("Ctrl+A", "Delete all targets in /delete", false),
        ("Enter", "Apply selected targets", false),
        ("Esc", "Cancel", false),
        ("", "", false),
        ("Text input", "", true),
        ("printable key", "Insert character", false),
        ("Enter", "Submit edit/search/category/rename", false),
        ("Esc", "Cancel or clear search", false),
        ("←/→, Home/End", "Move cursor", false),
        ("Shift+arrows", "Extend text selection", false),
        ("Ctrl/Alt+←/→", "Jump by word", false),
        ("Backspace", "Delete left", false),
        ("Ctrl+Backspace/w", "Delete word left", false),
        ("Delete", "Delete right", false),
        ("Ctrl+Delete", "Delete word right", false),
        ("Ctrl+A / Ctrl+E", "Move to start/end", false),
        ("Ctrl+D", "Open due-date calendar while editing item", false),
        ("Ctrl+Shift+C", "Copy selection", false),
        ("Ctrl+Shift+V", "Paste", false),
        ("", "", false),
        ("Search", "", true),
        ("type", "Append to query", false),
        ("Backspace/Delete", "Remove last query character", false),
        ("Enter", "Apply search", false),
        ("Esc", "Clear search", false),
        ("", "", false),
        ("Category creation", "", true),
        (
            "Enter",
            "Submit category text / choose highlighted option",
            false,
        ),
        ("1 / 2", "Choose attach/root option", false),
        ("↑/↓", "Move choice or parent highlight", false),
        ("Esc", "Cancel category creation", false),
        ("", "", false),
        ("Pickers", "", true),
        (
            "↑/↓",
            "Navigate theme/category/filter/archive/priority",
            false,
        ),
        (
            "Enter",
            "Select highlighted option or typed category",
            false,
        ),
        ("Category picker type", "Create category text", false),
        ("Esc", "Cancel picker", false),
        (
            "Sort: p / d / n",
            "Priority / due date / default sort",
            false,
        ),
        ("", "", false),
        ("Due date", "", true),
        ("Tab", "Switch calendar/prompt", false),
        ("Calendar ←/→", "Move selected date by day", false),
        ("Calendar ↑/↓", "Move selected date by week", false),
        ("PgUp/PgDn", "Previous/next month in calendar", false),
        ("Ctrl+T", "Jump to today", false),
        ("Prompt digits/-", "Edit YYYY-MM-DD text", false),
        ("Prompt arrows/Home/End", "Move prompt cursor", false),
        ("Prompt Backspace/Delete", "Edit prompt text", false),
        ("Calendar Delete", "Clear due date", false),
        ("Enter", "Save valid date", false),
        ("Esc", "Cancel", false),
        ("", "", false),
        ("Popups", "", true),
        (
            "Delete confirm",
            "Y/Enter delete, N/Esc cancel, A archive",
            false,
        ),
        ("Help/keybindings", "q or Esc closes", false),
    ];

    let split_at = key_lines.len().div_ceil(2);
    let left = &key_lines[..split_at];
    let right = &key_lines[split_at..];
    let cell_width = 52usize;
    let key_width = 18usize;
    let desc_width = cell_width.saturating_sub(key_width + 4);
    let mut lines = Vec::new();

    for (i, entry) in left.iter().enumerate() {
        let mut spans = Vec::new();
        push_keybinding_cell(&mut spans, *entry, cell_width, key_width, desc_width, theme);
        spans.push(Span::raw("  "));
        if let Some(entry) = right.get(i) {
            push_keybinding_cell(&mut spans, *entry, cell_width, key_width, desc_width, theme);
        }
        lines.push(Line::from(spans).style(Style::default().bg(theme.bg_secondary)));
    }

    let width = 110.min(area.width.saturating_sub(2));
    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(1));
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

fn push_keybinding_cell<'a>(
    spans: &mut Vec<Span<'a>>,
    entry: (&str, &str, bool),
    cell_width: usize,
    key_width: usize,
    desc_width: usize,
    theme: &Theme,
) {
    let (key, desc, is_header) = entry;
    if is_header {
        spans.push(Span::styled(
            format!("  {:<width$}", key, width = cell_width.saturating_sub(2)),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
    } else if key.is_empty() {
        spans.push(Span::raw(format!("{:<width$}", "", width = cell_width)));
    } else {
        spans.push(Span::raw("    "));
        spans.push(Span::styled(
            format!("{:<width$}", key, width = key_width),
            Style::default().fg(theme.text_secondary),
        ));
        spans.push(Span::styled(
            truncate_for_cell(desc, desc_width),
            Style::default().fg(theme.text_primary),
        ));
    }
}

fn truncate_for_cell(text: &str, width: usize) -> String {
    let mut chars = text.chars();
    let mut truncated: String = chars.by_ref().take(width).collect();
    if chars.next().is_some() && width > 1 {
        truncated.pop();
        truncated.push('…');
    }
    format!("{truncated:<width$}")
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
        let preview_count = texts.len().min(8);
        for t in texts.iter().take(preview_count) {
            for wrapped in wrap_text(&format!("\"{t}\""), max_content_w.saturating_sub(2)) {
                lines_text.push(format!("  {wrapped}"));
            }
        }
        if texts.len() > preview_count {
            lines_text.push(format!("  ... and {} more", texts.len() - preview_count));
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
    anchor: Rect,
    selected: Date,
    prompt_focused: bool,
    theme: &Theme,
) {
    let width = 40;
    let height = 13;
    let popup_x = calendar_popup_x(area, anchor, width);
    let popup_y = calendar_popup_y(area, anchor, height, prompt_focused);
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
                "  [Tab cal, Enter save, Esc cancel]"
            } else {
                "  [Tab prompt, Arrows, Pg, Del clear]"
            },
            Style::default().fg(theme.warning),
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

fn calendar_popup_x(area: Rect, anchor: Rect, width: u16) -> u16 {
    let centered = anchor
        .x
        .saturating_add(anchor.width / 2)
        .saturating_sub(width / 2);
    let min = area.x;
    let max = area.right().saturating_sub(width);
    centered.clamp(min, max.max(min))
}

fn calendar_popup_y(area: Rect, anchor: Rect, height: u16, prefer_above: bool) -> u16 {
    let below = anchor.bottom().saturating_add(1);
    let above = anchor.y.saturating_sub(height.saturating_add(1));
    let min = area.y;
    let max = area.bottom().saturating_sub(height);

    let y = if prefer_above {
        if anchor.y.saturating_sub(area.y) > height {
            above
        } else {
            below
        }
    } else if area.bottom().saturating_sub(anchor.bottom()) > height {
        below
    } else {
        above
    };

    y.clamp(min, max.max(min))
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
    let input_width = area.width.saturating_sub(4) as usize;

    let (display_text, cursor_pos) = match mode {
        Mode::Searching => {
            let line =
                render_prompt_input_line("  Search: ", filter, filter.len(), input_width, theme);
            (line, None::<u16>)
        }
        Mode::Command { .. } => {
            let line = if input.is_empty() {
                Line::from(vec![Span::styled(
                    "  [Type to filter, Up/Down choose, Tab complete/options, Enter open/run]",
                    Style::default().fg(theme.warning),
                )])
            } else {
                render_prompt_input_line("  /", input.text(), input.cursor(), input_width, theme)
            };
            (line, None)
        }
        Mode::MultiSelect {
            cmd,
            selected,
            selected_categories,
        } => {
            let target = if matches!(pane, Pane::Categories)
                && matches!(cmd, crate::app::MultiSelectCmd::Delete)
            {
                "categories"
            } else {
                "items"
            };
            let action = match cmd {
                crate::app::MultiSelectCmd::Delete => "Bulk delete",
                crate::app::MultiSelectCmd::ToggleDone => "Bulk done",
                crate::app::MultiSelectCmd::Archive => "Bulk archive",
                crate::app::MultiSelectCmd::RestoreArchive => "Bulk restore",
            };
            let counts = if selected_categories.is_empty() {
                format!("{} items", selected.len())
            } else {
                format!(
                    "{} items, {} categories",
                    selected.len(),
                    selected_categories.len()
                )
            };
            let select_all = if matches!(cmd, crate::app::MultiSelectCmd::Delete) {
                " Ctrl+A all,"
            } else {
                ""
            };
            let line = Line::from(vec![Span::styled(
                format!(
                    "  [{action}: {counts} selected | select {target},{select_all} Enter apply, Esc cancel]"
                ),
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::ThemePicker { .. } => {
            let line = Line::from(vec![Span::styled(
                "  [Up/Down preview, Enter save, Esc restore]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::ConfirmDelete { .. } => {
            let line = Line::from(vec![Span::styled(
                "  [Y/Enter delete, N/Esc cancel, A archive]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::PriorityPicker { .. } => {
            let line = Line::from(vec![Span::styled(
                "  [Up/Down choose, Enter apply, Esc cancel]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::DueDateFilterPicker { .. } => {
            let line = Line::from(vec![Span::styled(
                "  [Up/Down choose, Enter apply, Esc cancel]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::Help => {
            let line = Line::from(vec![Span::styled(
                "  [Esc close]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::Keybindings => {
            let line = Line::from(vec![Span::styled(
                "  [Esc close]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::CategoryCreateChoice { .. } => {
            let line = Line::from(vec![Span::styled(
                "  [1/2 choose, Enter apply, Esc cancel]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::CategoryParentPicker { .. } => {
            let line = Line::from(vec![Span::styled(
                "  [Up/Down choose parent, Enter apply, Esc cancel]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::CategoryAdd { parent } => {
            let label = match parent {
                Some(parent) => format!("  New subcategory under {parent}: "),
                None => "  New category: ".to_string(),
            };
            let line =
                render_prompt_input_line(&label, input.text(), input.cursor(), input_width, theme);
            (line, None)
        }
        Mode::CategoryPicker { .. } => {
            let line = render_prompt_input_line(
                "  Assign category: ",
                input.text(),
                input.cursor(),
                input_width,
                theme,
            );
            (line, None)
        }
        Mode::CategoryFilterPicker { .. } => {
            let line = Line::from(vec![Span::styled(
                "  [Up/Down choose, Enter apply, Esc cancel]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::SortPicker { .. } => {
            let line = Line::from(vec![Span::styled(
                "  [Up/Down preview, Enter keep, Esc restore]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::ArchivePicker { .. } => {
            let line = Line::from(vec![Span::styled(
                "  [Up/Down choose, Enter apply, Esc cancel]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::FilterPicker { .. } => {
            let line = Line::from(vec![Span::styled(
                "  [Up/Down choose, Enter apply, Esc cancel]",
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
                    "  [Tab calendar, Enter save]"
                } else {
                    "  [Tab prompt, calendar active]"
                },
                Style::default().fg(theme.warning),
            ));
            (line.style(Style::default().bg(theme.bg_primary)), None)
        }
        Mode::RenameInput { .. } => {
            let line = render_prompt_input_line(
                "  Rename to: ",
                input.text(),
                input.cursor(),
                input_width,
                theme,
            );
            (line, None)
        }
        Mode::Editing { .. } => {
            let line =
                render_prompt_input_line("  ", input.text(), input.cursor(), input_width, theme);
            (line, None)
        }
        _ if input.is_empty() && matches!(mode, Mode::Normal) => {
            let placeholder = Line::from(vec![Span::styled(
                "  Type to add or / for commands",
                Style::default().fg(theme.text_placeholder),
            )]);
            (placeholder, None)
        }
        _ => {
            let line =
                render_prompt_input_line("  ", input.text(), input.cursor(), input_width, theme);
            (line, None)
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
