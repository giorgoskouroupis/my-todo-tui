pub mod input;
pub mod list;
pub mod theme;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListState, Paragraph};

use crate::app::{
    get_filtered_commands, CategoryPickerTarget, DueFilter, Mode, NoteDisplay, Pane, SortMode,
};
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
    pub sidebar_width: u16,
    pub note_hint: bool,
    pub note_display: NoteDisplay,
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
    let sidebar_w = state
        .sidebar_width
        .min(mid.width.saturating_sub(10).max(6));
    let h_layout = Layout::horizontal([Constraint::Length(sidebar_w), Constraint::Min(1)]).split(mid);

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
        state.pane,
        state.note_hint,
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
    } else if let Mode::BulkActionPicker {
        selected,
        ids,
        category_names,
    } = state.mode
    {
        render_bulk_action_picker(
            frame,
            popup_area,
            *selected,
            ids.len(),
            category_names.len(),
            state.theme,
        );
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
    } else if let Mode::Notes { item_id, selected } = state.mode {
        render_notes_popup(
            frame,
            popup_area,
            state.items,
            *item_id,
            Some(*selected),
            state.theme,
        );
    } else if let Mode::NoteInput {
        item_id,
        edit_index,
    } = state.mode
    {
        render_notes_popup(
            frame,
            popup_area,
            state.items,
            *item_id,
            *edit_index,
            state.theme,
        );
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
                state.pane,
                state.note_hint,
                state.theme,
            );
        }
        let anchor = if *from_normal {
            calendar_item_anchor(
                h_layout[1],
                state.items,
                state.selected_index,
                state.note_display,
            )
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

fn calendar_item_anchor(
    list_area: Rect,
    items: &[TodoItem],
    selected_index: usize,
    note_display: NoteDisplay,
) -> Rect {
    let content_width = list_area.width.saturating_sub(5) as usize;
    let mut y = list_area.y;
    for item in items.iter().take(selected_index) {
        let text_lines = wrap_text(&item.text, content_width.max(10)).len() as u16;
        let note_lines = self::list::note_row_count(item, content_width, note_display) as u16;
        y = y.saturating_add(1 + text_lines + note_lines);
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

    // All is the default, so only call out the states that hide something.
    if !matches!(state.note_display, NoteDisplay::All) {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("[notes: {}]", state.note_display.label()),
            Style::default().fg(theme.warning),
        ));
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

    let is_resizing = matches!(mode, Mode::ResizeSidebar { .. });
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(if is_active || is_resizing {
            theme.accent
        } else {
            theme.border_default
        }).add_modifier(if is_resizing { Modifier::BOLD } else { Modifier::empty() }))
        .title(if is_resizing { " Categories \u{2194} " } else { " Categories " })
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
    let text_width = area.width.saturating_sub(5) as usize;
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
                state.note_display,
            )
        })
        .collect();

    let mut list_state = ListState::default().with_selected(if is_active {
        Some(state.selected_index)
    } else {
        None
    });

    let is_resizing = matches!(state.mode, Mode::ResizeSidebar { .. });
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(if is_active || is_resizing {
            theme.accent
        } else {
            theme.border_default
        }).add_modifier(if is_resizing { Modifier::BOLD } else { Modifier::empty() }))
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
    let desc_width = matches
        .iter()
        .map(|(_, desc)| desc.chars().count())
        .max()
        .unwrap_or(0);
    // Layout: "  " + prefix(1) + cmd(cmd_width) + " " + desc(desc_width) + "  " margin
    let content_width = cmd_width + desc_width + 6;
    let width = (content_width as u16).min(area.width.saturating_sub(2));
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
    let registry = Theme::theme_registry();
    let group_count = 2;
    let height = registry.len() as u16 + group_count + 2;
    // Rows: "    " + circle(2) + name → 6 chars around the widest theme name.
    let content_width = registry
        .iter()
        .map(|(name, _)| name.chars().count())
        .max()
        .unwrap_or(0)
        + 8;
    let title_width = " Themes ".chars().count() + 2;
    let width = ((content_width.max(title_width)) as u16)
        .max(30)
        .min(area.width.saturating_sub(2));
    let popup_y = area.bottom().saturating_sub(height + 1);
    let popup_x = area.x + 2;

    let popup_area = Rect::new(
        popup_x,
        popup_y.min(area.bottom().saturating_sub(height)),
        width,
        height,
    );

    let mut lines = Vec::new();
    let mut last_group: Option<bool> = None;
    for (i, (name, is_light)) in registry.iter().enumerate() {
        if last_group != Some(*is_light) {
            let header = if *is_light { "Light" } else { "Dark" };
            lines.push(
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        header.to_string(),
                        Style::default()
                            .fg(theme.text_muted)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
                .style(Style::default().bg(theme.bg_primary)),
            );
            last_group = Some(*is_light);
        }
        let is_active = *name == theme.name;
        let is_highlighted = i == selected;
        let bg = if is_highlighted {
            theme.bg_tertiary
        } else {
            theme.bg_primary
        };

        lines.push(
            Line::from(vec![
                Span::raw("    "),
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
    min_width: u16,
    theme: &Theme,
) {
    let height = (items.len() as u16 + 2).min(area.height.saturating_sub(1));
    // Layout per row: "  " + label + "  " → 4 chars of margin around the widest label.
    // Title also needs to fit inside the top border, so include its length.
    let content_width = items
        .iter()
        .map(|s| s.chars().count())
        .max()
        .unwrap_or(0)
        + 4;
    let title_width = title.chars().count() + 2;
    let width = (content_width.max(title_width) as u16)
        .max(min_width)
        .min(area.width.saturating_sub(2));
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
        crate::app::archive_picker_labels(),
        selected,
        24,
        theme,
    );
}

fn render_bulk_action_picker(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    item_count: usize,
    category_count: usize,
    theme: &Theme,
) {
    let counts = if category_count == 0 {
        format!(" Action ({} items) ", item_count)
    } else if item_count == 0 {
        format!(" Action ({} cats) ", category_count)
    } else {
        format!(" Action ({} items, {} cats) ", item_count, category_count)
    };
    let single = item_count == 1 && category_count == 0;
    let labels = crate::app::bulk_action_labels(single);
    render_menu_popup(frame, area, &counts, &labels, selected, 32, theme);
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
    let entries: [&str; 1] = [match target {
        CategoryPickerTarget::AssignItem | CategoryPickerTarget::AssignBulk(_) => "None",
        CategoryPickerTarget::MoveCategory(_) => "Root",
    }];
    let title_str = match target {
        CategoryPickerTarget::AssignItem => " Assign Category ",
        CategoryPickerTarget::AssignBulk(_) => " Assign Category (bulk) ",
        CategoryPickerTarget::MoveCategory(_) => " Move Category ",
    };
    let content_width = entries
        .iter()
        .copied()
        .chain(display_categories.iter().copied())
        .map(|s| s.chars().count())
        .max()
        .unwrap_or(0)
        + 4;
    let title_width = title_str.chars().count() + 2;
    let width = ((content_width.max(title_width)) as u16)
        .max(30)
        .min(area.width.saturating_sub(2));
    let popup_y = area.bottom().saturating_sub(height + 1);
    let popup_x = area.x + 2;

    let popup_area = Rect::new(
        popup_x,
        popup_y.min(area.bottom().saturating_sub(height)),
        width,
        height,
    );

    let mut lines = Vec::new();
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
        .title(title_str)
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
    let title_str = " Filter Category ";
    let content_width = std::iter::once("All")
        .chain(categories.iter().map(|entry| entry.path.as_str()))
        .map(|s| s.chars().count())
        .max()
        .unwrap_or(0)
        + 4;
    let title_width = title_str.chars().count() + 2;
    let width = ((content_width.max(title_width)) as u16)
        .max(36)
        .min(area.width.saturating_sub(2));
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
        .title(title_str)
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
    let bg = theme.bg_secondary;
    let primary = Style::default().fg(theme.text_primary);
    let secondary = Style::default().fg(theme.text_secondary);
    let muted = Style::default().fg(theme.text_muted);
    let muted_bold = muted.add_modifier(Modifier::BOLD);

    enum Entry<'a> {
        Blank,
        Welcome,
        Text(String, Style, &'a str),
    }

    let entries: Vec<Entry> = vec![
        Entry::Welcome,
        Entry::Blank,
        Entry::Text("/help           — this screen".into(), primary, "  "),
        Entry::Text("/keybindings    — show all keybindings".into(), primary, "  "),
        Entry::Text("/filter         — filter items (due, priority, category, archived)".into(), primary, "  "),
        Entry::Text("/delete         — bulk delete (multi-select)".into(), primary, "  "),
        Entry::Text("/done           — bulk toggle done".into(), primary, "  "),
        Entry::Text("/clear          — clear completed items".into(), primary, "  "),
        Entry::Text("/archive        — archive items/categories (done, all, restore)".into(), primary, "  "),
        Entry::Text("/move           — move item or highlighted category".into(), primary, "  "),
        Entry::Text("/notes          — note log for the current item (Ctrl+N)".into(), primary, "  "),
        Entry::Text("/notes latest|all|hidden — inline notes (Ctrl+L)".into(), primary, "  "),
        Entry::Text("/rename         — rename current item/category".into(), primary, "  "),
        Entry::Text("/sidebar        — resize sidebar (Left/Right, Enter save)".into(), primary, "  "),
        Entry::Text("/themes         — pick a theme".into(), primary, "  "),
        Entry::Blank,
        Entry::Text("Tab autocompletes commands/subcommands, Esc cancels".into(), muted, "  "),
        Entry::Blank,
        Entry::Text("Files:".into(), muted_bold, "  "),
        Entry::Text(format!("Config: {}", config_path), secondary, "  "),
        Entry::Text(format!("Data:   {}", data_path), secondary, "  "),
    ];

    let raw_widths: Vec<usize> = entries
        .iter()
        .map(|e| match e {
            Entry::Blank => 0,
            Entry::Welcome => "  Welcome to todo-tui".chars().count(),
            Entry::Text(text, _, indent) => indent.chars().count() + text.chars().count(),
        })
        .collect();
    let longest = raw_widths.iter().copied().max().unwrap_or(0);

    let available = area.width.saturating_sub(4) as usize;
    let inner_width = longest.min(available).max(20);
    let width = (inner_width as u16 + 2).min(area.width.saturating_sub(2));

    let mut lines: Vec<Line> = Vec::new();
    for entry in &entries {
        match entry {
            Entry::Blank => {
                lines.push(Line::from(Span::raw("")).style(Style::default().bg(bg)));
            }
            Entry::Welcome => {
                lines.push(
                    Line::from(vec![
                        Span::styled("  Welcome to ", primary),
                        Span::styled("todo-tui", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    ])
                    .style(Style::default().bg(bg)),
                );
            }
            Entry::Text(text, style, indent) => {
                let indent_w = indent.chars().count();
                let body_width = inner_width.saturating_sub(indent_w).max(1);
                let wrapped = wrap_text(text, body_width);
                let cont_indent: String = " ".repeat(indent_w + 2);
                for (i, seg) in wrapped.iter().enumerate() {
                    let full = if i == 0 {
                        format!("{indent}{seg}")
                    } else {
                        format!("{cont_indent}{seg}")
                    };
                    lines.push(
                        Line::from(Span::styled(full, *style)).style(Style::default().bg(bg)),
                    );
                }
            }
        }
    }

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
    let key_lines: &[(&str, &str, bool)] = &[
        ("Global", "", true),
        ("Ctrl+C", "Quit", false),
        ("Ctrl+H", "Help", false),
        ("Ctrl+B", "Resize sidebar", false),
        ("/", "Command mode", false),
        ("", "", false),
        ("Items", "", true),
        ("Enter", "Toggle done", false),
        ("Space", "Toggle doing", false),
        ("Delete", "Delete", false),
        ("Ctrl+E", "Edit", false),
        ("Ctrl+P", "Priority", false),
        ("Ctrl+↑/↓", "Reorder", false),
        ("Ctrl+O", "Assign cat", false),
        ("Ctrl+D", "Due date", false),
        ("Ctrl+N", "Notes", false),
        ("Ctrl+L", "Inline notes", false),
        ("*", "Pin", false),
        ("←/→", "Switch pane", false),
        ("Tab", "Switch pane", false),
        ("PgUp/PgDn", "Cycle filter", false),
        ("printable", "New item", false),
        ("", "", false),
        ("Command", "", true),
        ("↑/↓", "Navigate", false),
        ("Backspace", "Back", false),
        ("Tab", "Autocomplete", false),
        ("", "", false),
        ("Text input", "", true),
        ("↑/↓", "Line start/end", false),
        ("Ctrl+←/→", "Word jump", false),
        ("Ctrl+Shift+V", "Paste", false),
        ("", "", false),
        ("Multi-select", "", true),
        ("Ctrl+A", "Select all", false),
        ("", "", false),
        ("Notes", "", true),
        ("Enter", "Log a note", false),
        ("printable", "Log a note", false),
        ("Ctrl+E", "Edit note", false),
        ("Delete", "Delete note", false),
        ("Ctrl+Y", "Copy note", false),
        ("Ctrl+Z", "Undo delete", false),
        ("", "", false),
        ("Sidebar", "", true),
        ("Ctrl+↑/↓", "Reorder / move", false),
        ("Ctrl+O", "Move", false),
        ("Delete", "Delete branch", false),
        ("PgUp/PgDn", "Cycle filter", false),
        ("printable", "Create cat", false),
        ("", "", false),
        ("Resize sidebar", "", true),
        ("←/→", "\u{00b1}1 column", false),
        ("Shift+←/→", "\u{00b1}5 columns", false),
        ("r", "Reset to default", false),
        ("Enter", "Save & exit", false),
        ("Esc", "Cancel (restore)", false),
        ("", "", false),
        ("Calendar", "", true),
        ("Tab", "Focus toggle", false),
        ("Ctrl+T", "Today", false),
        ("Delete", "Clear date", false),
    ];

    let key_w = 14usize;
    let bg = theme.bg_secondary;
    let cell_w = 38usize;
    let desc_w = cell_w.saturating_sub(key_w + 4);
    let two_col_width = cell_w * 2 + 4;
    let available_inner = area.width.saturating_sub(2) as usize;
    let two_col = available_inner >= two_col_width;

    let mut lines = Vec::new();
    if two_col {
        let split = key_lines.len().div_ceil(2);
        let left = &key_lines[..split];
        let right = &key_lines[split..];
        for i in 0..left.len() {
            let mut spans = Vec::new();
            push_cell(&mut spans, left[i], cell_w, key_w, desc_w, theme);
            if let Some(entry) = right.get(i) {
                spans.push(Span::raw(" "));
                push_cell(&mut spans, *entry, cell_w, key_w, desc_w, theme);
            }
            lines.push(Line::from(spans).style(Style::default().bg(bg)));
        }
    } else {
        let cell_w_single = available_inner.max(key_w + 8);
        let desc_w_single = cell_w_single.saturating_sub(key_w + 4);
        for entry in key_lines.iter() {
            let mut spans = Vec::new();
            push_cell(&mut spans, *entry, cell_w_single, key_w, desc_w_single, theme);
            lines.push(Line::from(spans).style(Style::default().bg(bg)));
        }
    }

    let width = if two_col {
        two_col_width.min(available_inner) as u16
    } else {
        available_inner as u16
    };
    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(1));
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

fn push_cell<'a>(
    spans: &mut Vec<Span<'a>>,
    (key, desc, is_header): (&str, &str, bool),
    cell_w: usize,
    key_w: usize,
    desc_w: usize,
    theme: &Theme,
) {
    if is_header {
        let pad = cell_w.saturating_sub(key.len() + 2);
        spans.push(Span::styled(
            format!("  {key}{:pad$}", "", pad = pad),
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ));
    } else if key.is_empty() {
        spans.push(Span::raw(format!("{:cell$}", "", cell = cell_w)));
    } else {
        let truncated = if desc.len() > desc_w {
            let mut s: String = desc.chars().take(desc_w).collect();
            if desc_w > 1 { s.truncate(desc_w.saturating_sub(1)); s.push('…'); }
            s
        } else {
            format!("{desc:<desc_w$}")
        };
        spans.push(Span::raw("    "));
        spans.push(Span::styled(
            format!("{key:<key_w$}"),
            Style::default().fg(theme.text_secondary),
        ));
        spans.push(Span::styled(truncated, Style::default().fg(theme.text_primary)));
    }
}

fn render_confirm_delete_popup(frame: &mut Frame, area: Rect, texts: &[String], theme: &Theme) {
    let max_content_w = area.width.saturating_sub(6).clamp(20, 60) as usize;

    let mut lines_text: Vec<String> = Vec::new();
    if texts.len() == 1 {
        for wrapped in wrap_text("Are you sure you want to delete?", max_content_w) {
            lines_text.push(wrapped);
        }
        for wrapped in wrap_text(
            &format!("\"{}\"", &texts[0]),
            max_content_w.saturating_sub(2),
        ) {
            lines_text.push(format!("  {wrapped}"));
        }
    } else {
        for wrapped in wrap_text(
            &format!("Are you sure you want to delete these {} items?", texts.len()),
            max_content_w,
        ) {
            lines_text.push(wrapped);
        }
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
    let raw_max = lines_text
        .iter()
        .map(|l| l.chars().count() as u16)
        .max()
        .unwrap_or(0);
    let desired = raw_max.max(30) + 4;
    let width = desired
        .min(area.width.saturating_sub(2))
        .max(area.width.min(20));
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

/// One rendered line of a note log: `date` is set on a note's first line only,
/// `offset` is where the text column starts (wrapped lines align under it).
struct NoteRow {
    date: Option<String>,
    text: String,
    offset: usize,
    muted: bool,
}

fn note_row_width(row: &NoteRow) -> usize {
    row.offset + row.text.chars().count()
}

/// Renders the note log of one item. Rows are grouped per note so a wrapped
/// entry is never split across the scroll window boundary.
fn render_notes_popup(
    frame: &mut Frame,
    area: Rect,
    items: &[TodoItem],
    item_id: u64,
    selected: Option<usize>,
    theme: &Theme,
) {
    let Some(item) = items.iter().find(|candidate| candidate.id == item_id) else {
        return;
    };

    const TITLE_TEXT_MAX: usize = 28;
    let label: String = item.text.chars().take(TITLE_TEXT_MAX).collect();
    let title = if item.text.chars().count() > TITLE_TEXT_MAX {
        format!(" Notes \u{2014} {label}\u{2026} ")
    } else {
        format!(" Notes \u{2014} {label} ")
    };

    // Row layout: indent + "YYYY-MM-DD" + gap + text; wrapped text aligns under the text column.
    const INDENT: usize = 2;
    const DATE_WIDTH: usize = 10;
    const DATE_GAP: usize = 2;
    let text_offset = INDENT + DATE_WIDTH + DATE_GAP;
    let available = area.width.saturating_sub(4) as usize;
    let text_width = available.saturating_sub(text_offset).clamp(12, 72);

    // Same newest-first order as the inline list, so row 0 is the newest entry.
    let mut groups: Vec<(bool, Vec<NoteRow>)> = self::list::notes_newest_first(item)
        .enumerate()
        .map(|(i, note)| {
            let rows: Vec<NoteRow> = wrap_text(&note.text, text_width)
                .into_iter()
                .enumerate()
                .map(|(j, seg)| NoteRow {
                    date: (j == 0).then(|| note.created.clone()),
                    text: seg,
                    offset: text_offset,
                    muted: false,
                })
                .collect();
            (selected == Some(i), rows)
        })
        .collect();

    if groups.is_empty() {
        groups.push((
            false,
            vec![NoteRow {
                date: None,
                text: "No notes yet \u{2014} type or press Enter to log one".to_string(),
                offset: INDENT,
                muted: true,
            }],
        ));
    }

    let max_rows = area.height.saturating_sub(3).max(1) as usize;
    let focus = selected.unwrap_or(groups.len() - 1).min(groups.len() - 1);
    let mut used_rows = groups[focus].1.len().min(max_rows);
    let mut start = focus;
    let mut end = focus + 1;
    while start > 0 {
        let prev = groups[start - 1].1.len();
        if used_rows + prev > max_rows {
            break;
        }
        used_rows += prev;
        start -= 1;
    }
    while end < groups.len() {
        let next = groups[end].1.len();
        if used_rows + next > max_rows {
            break;
        }
        used_rows += next;
        end += 1;
    }

    let longest = groups[start..end]
        .iter()
        .flat_map(|(_, rows)| rows.iter())
        .map(note_row_width)
        .max()
        .unwrap_or(0);
    let title_width = title.chars().count();
    let width = ((longest + 2).max(title_width + 2) as u16)
        .max(44)
        .min(area.width.saturating_sub(2));
    let inner = width.saturating_sub(2) as usize;

    let lines: Vec<Line<'static>> = groups[start..end]
        .iter()
        .flat_map(|(highlighted, rows)| {
            rows.iter().map(move |row| {
                let bg = if *highlighted {
                    theme.bg_tertiary
                } else {
                    theme.bg_secondary
                };
                let text_style = Style::default()
                    .fg(if row.muted {
                        theme.text_muted
                    } else {
                        theme.text_primary
                    })
                    .add_modifier(if *highlighted {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    });

                let mut spans = Vec::new();
                match &row.date {
                    Some(date) => {
                        let cell: String = date.chars().take(DATE_WIDTH).collect();
                        spans.push(Span::raw(" ".repeat(INDENT)));
                        spans.push(Span::styled(
                            format!("{cell:<width$}", width = DATE_WIDTH + DATE_GAP),
                            Style::default().fg(theme.text_muted),
                        ));
                    }
                    None => spans.push(Span::raw(" ".repeat(row.offset))),
                }
                spans.push(Span::styled(row.text.clone(), text_style));
                let pad = inner.saturating_sub(note_row_width(row));
                if pad > 0 {
                    spans.push(Span::raw(" ".repeat(pad)));
                }
                Line::from(spans).style(Style::default().bg(bg))
            })
        })
        .take(max_rows)
        .collect();

    let height = lines.len() as u16 + 2;
    let popup_y = area.bottom().saturating_sub(height + 1);
    let popup_area = Rect::new(
        area.x + 2,
        popup_y.min(area.bottom().saturating_sub(height)),
        width,
        height,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(title)
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
    pane: &Pane,
    note_hint: bool,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme.border_default));
    let input_width = area.width.saturating_sub(4) as usize;

    let (display_text, cursor_pos) = match mode {
        Mode::Searching => {
            let line = if input.is_empty() {
                Line::from(vec![
                    Span::styled("  Search: ", Style::default().fg(theme.accent)),
                    Span::styled(
                        "[type to filter, \u{2191}/\u{2193} nav, Tab bulk-select, Enter action, Esc clear]",
                        Style::default().fg(theme.warning),
                    ),
                ])
            } else {
                render_prompt_input_line(
                    "  Search: ",
                    input.text(),
                    input.cursor(),
                    input_width,
                    theme,
                )
            };
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
                && matches!(
                    cmd,
                    crate::app::MultiSelectCmd::Delete | crate::app::MultiSelectCmd::PickAction
                )
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
                crate::app::MultiSelectCmd::PickAction => "Bulk select",
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
            let select_all = if matches!(
                cmd,
                crate::app::MultiSelectCmd::Delete | crate::app::MultiSelectCmd::PickAction
            ) {
                " Ctrl+A all,"
            } else {
                ""
            };
            let trailing = if matches!(cmd, crate::app::MultiSelectCmd::PickAction) {
                "Enter choose action"
            } else {
                "Enter apply"
            };
            let line = Line::from(vec![Span::styled(
                format!(
                    "  [{action}: {counts} selected | Space toggle {target},{select_all} {trailing}, Esc cancel]"
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
        Mode::BulkActionPicker { .. } => {
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
        Mode::Notes { .. } => {
            let line = Line::from(vec![Span::styled(
                "  [Enter/type add, Ctrl+E edit, Del delete, Ctrl+Y copy, Ctrl+Z undo, Esc close]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        Mode::NoteInput { edit_index, .. } => {
            let label = if edit_index.is_some() {
                "  Edit note: "
            } else {
                "  New note: "
            };
            let line =
                render_prompt_input_line(label, input.text(), input.cursor(), input_width, theme);
            (line, None)
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
        Mode::ResizeSidebar { .. } => {
            let line = Line::from(vec![Span::styled(
                "  [Sidebar: \u{2190}/\u{2192} \u{00b11}, Shift+\u{2190}/\u{2192} \u{00b15}, r reset, Enter save, Esc cancel]",
                Style::default().fg(theme.warning),
            )]);
            (line, None)
        }
        _ if input.is_empty() && matches!(mode, Mode::Normal) => {
            let placeholder = if note_hint {
                Line::from(vec![Span::styled(
                    "  Done \u{2014} Ctrl+N to log what came out of it",
                    Style::default().fg(theme.warning),
                )])
            } else {
                Line::from(vec![Span::styled(
                    "  Type to add or / for commands",
                    Style::default().fg(theme.text_placeholder),
                )])
            };
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
