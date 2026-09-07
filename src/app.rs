mod commands;
mod defs;

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};

use crate::config;
use crate::data::{
    category_matches, normalize_category, CategoryEntry, Note, Priority, TodoData, TodoItem,
};
use crate::date::Date;
use crate::keys;
use crate::ui::input::InputBuffer;
use crate::ui::theme::Theme;

pub use commands::get_filtered_commands;
use commands::{
    contains_whitespace, get_completions, has_visible_subcommands, resolve_command_input,
};
use defs::PopupBackTarget;
pub use defs::{
    Action, CategoryPickerTarget, DueFilter, Mode, MultiSelectCmd, Pane, RenameTarget, SortMode,
};

pub struct App {
    pub data: TodoData,
    pub selected_index: usize,
    pub input: InputBuffer,
    pub mode: Mode,
    pub filter: String,
    pub priority_filter: Option<Priority>,
    pub due_filter: Option<DueFilter>,
    pub category_filter: Option<String>,
    pub show_archived: bool,
    pub pane: Pane,
    pub category_index: usize,
    pub sort_mode: SortMode,
    pub theme: Theme,
    pub completions: Vec<String>,
    pub completion_index: usize,
    pub dirty: bool,
    pub last_mutated: Instant,
    pub pending_due_date: Option<String>,
    pub sidebar_width: u16,
    pub note_hint: bool,
    category_selection_memory: HashMap<String, usize>,
    popup_back_stack: Vec<PopupBackTarget>,
    undo_stack: Vec<UndoEntry>,
    saved_search: Option<String>,
}

pub const SIDEBAR_WIDTH_DEFAULT: u16 = 22;
pub const SIDEBAR_WIDTH_MIN: u16 = 12;
pub const SIDEBAR_WIDTH_MAX: u16 = 60;

enum UndoEntry {
    Delete(Vec<TodoItem>),
    Archive {
        item_ids: Vec<u64>,
        category_names: Vec<String>,
        previous_archived: bool,
    },
    NoteDelete {
        item_id: u64,
        index: usize,
        note: Note,
    },
}

const UNDO_STACK_LIMIT: usize = 20;

impl App {
    pub fn new() -> Self {
        let theme = config::load_theme();
        let data = TodoData::load();
        let sidebar_width = config::load_sidebar_width()
            .unwrap_or(SIDEBAR_WIDTH_DEFAULT)
            .clamp(SIDEBAR_WIDTH_MIN, SIDEBAR_WIDTH_MAX);
        Self {
            data,
            selected_index: 0,
            input: InputBuffer::new(),
            mode: Mode::Normal,
            filter: String::new(),
            priority_filter: None,
            due_filter: None,
            category_filter: None,
            show_archived: false,
            pane: Pane::Items,
            category_index: 0,
            sort_mode: SortMode::Default,
            theme,
            completions: Vec::new(),
            completion_index: 0,
            dirty: false,
            last_mutated: Instant::now(),
            pending_due_date: None,
            sidebar_width,
            note_hint: false,
            category_selection_memory: HashMap::new(),
            popup_back_stack: Vec::new(),
            undo_stack: Vec::new(),
            saved_search: None,
        }
    }

    fn return_after_secondary_mode(&mut self) {
        if let Some(search) = self.saved_search.take() {
            self.input.set_text(&search);
            self.filter = search;
            self.mode = Mode::Searching;
        } else {
            self.input.clear();
            self.mode = Mode::Normal;
        }
        self.clamp_selection();
    }

    fn push_undo_delete(&mut self, items: Vec<TodoItem>) {
        if items.is_empty() {
            return;
        }
        self.push_undo(UndoEntry::Delete(items));
    }

    fn push_undo_archive(
        &mut self,
        item_ids: Vec<u64>,
        category_names: Vec<String>,
        previous_archived: bool,
    ) {
        if item_ids.is_empty() && category_names.is_empty() {
            return;
        }
        self.push_undo(UndoEntry::Archive {
            item_ids,
            category_names,
            previous_archived,
        });
    }

    fn push_undo(&mut self, entry: UndoEntry) {
        self.undo_stack.push(entry);
        if self.undo_stack.len() > UNDO_STACK_LIMIT {
            self.undo_stack.remove(0);
        }
    }

    fn mark_dirty(&mut self) {
        if !self.dirty {
            self.dirty = true;
            self.last_mutated = Instant::now();
        }
    }

    pub fn items(&self) -> Vec<TodoItem> {
        let items: Vec<TodoItem> = self
            .data
            .items()
            .iter()
            .filter(|item| item.archived == self.show_archived)
            .cloned()
            .collect();
        let items: Vec<TodoItem> = match self.priority_filter {
            Some(p) => items.into_iter().filter(|i| i.priority == p).collect(),
            None => items,
        };
        let items: Vec<TodoItem> = match &self.category_filter {
            Some(cat) => items
                .into_iter()
                .filter(|i| category_matches(i.category.as_deref(), cat))
                .collect(),
            None => items,
        };
        let items: Vec<TodoItem> = match self.due_filter {
            Some(DueFilter::Today) => items
                .into_iter()
                .filter(|i| i.due_date.as_deref().and_then(crate::date::days_until) == Some(0))
                .collect(),
            Some(DueFilter::Week) => items
                .into_iter()
                .filter(|i| {
                    i.due_date
                        .as_deref()
                        .and_then(crate::date::days_until)
                        .is_some_and(|days| (0..=7).contains(&days))
                })
                .collect(),
            Some(DueFilter::Overdue) => items
                .into_iter()
                .filter(|i| {
                    i.due_date
                        .as_deref()
                        .and_then(crate::date::days_until)
                        .is_some_and(|days| days < 0)
                })
                .collect(),
            None => items,
        };
        let mut items: Vec<TodoItem> = if self.filter.is_empty() {
            items
        } else {
            let q = self.filter.to_lowercase();
            items
                .into_iter()
                .filter(|i| i.text.to_lowercase().contains(&q))
                .collect()
        };
        match self.sort_mode {
            SortMode::Default => {
                items.sort_by_key(|i| !i.pinned);
            }
            SortMode::Priority => {
                items.sort_by_key(|i| (!i.pinned, std::cmp::Reverse(i.priority as u8)));
            }
            SortMode::DueDate => {
                items.sort_by_key(|i| {
                    let due = i.due_date.as_deref().and_then(Date::parse).unwrap_or(Date {
                        year: 9999,
                        month: 12,
                        day: 31,
                    });
                    (!i.pinned, due)
                });
            }
        }
        items
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
            .min(self.items().len().saturating_sub(1))
    }

    pub fn pending_count(&self) -> usize {
        self.data.pending_count()
    }

    fn clamp_selection(&mut self) {
        let max = self.items().len().saturating_sub(1);
        self.selected_index = self.selected_index.min(max);
    }

    fn category_selection_key(&self) -> String {
        self.category_filter.clone().unwrap_or_default()
    }

    fn remember_category_selection(&mut self) {
        let key = self.category_selection_key();
        self.category_selection_memory
            .insert(key, self.selected_index());
    }

    fn restore_category_selection(&mut self) {
        let key = self.category_selection_key();
        self.selected_index = self
            .category_selection_memory
            .get(&key)
            .copied()
            .unwrap_or(0);
        self.clamp_selection();
    }

    fn root_command_index(command: &str) -> Option<usize> {
        get_filtered_commands("")
            .iter()
            .position(|(name, _)| *name == command)
    }

    fn command_popup_back_target(&self, command: &str) -> Option<PopupBackTarget> {
        let Mode::Command { selected } = self.mode else {
            return None;
        };

        let input = self.input.text().to_string();
        if contains_whitespace(&input) {
            Some(PopupBackTarget::Command { input, selected })
        } else {
            Some(PopupBackTarget::Command {
                input: String::new(),
                selected: Self::root_command_index(command).unwrap_or(selected),
            })
        }
    }

    fn push_command_popup_back_target(&mut self, command: &str) {
        if let Some(target) = self.command_popup_back_target(command) {
            self.popup_back_stack.push(target);
        }
    }

    fn pop_popup_back_target_mode(&mut self) -> Option<Mode> {
        let target = self.popup_back_stack.pop()?;

        Some(match target {
            PopupBackTarget::Command { input, selected } => {
                self.input.set_text(&input);
                self.completions.clear();
                self.completion_index = 0;
                let max = get_filtered_commands(&input).len().saturating_sub(1);
                Mode::Command {
                    selected: selected.min(max),
                }
            }
            PopupBackTarget::FilterPicker { selected } => Mode::FilterPicker { selected },
        })
    }

    fn initial_due_date(&self, edit_id: Option<u64>) -> Date {
        edit_id
            .and_then(|id| self.data.get(id))
            .and_then(|item| item.due_date.as_deref())
            .and_then(Date::parse)
            .or_else(|| self.pending_due_date.as_deref().and_then(Date::parse))
            .unwrap_or_else(Date::today)
    }

    fn restore_after_due_date(
        &mut self,
        edit_id: Option<u64>,
        saved_text: String,
        from_new: bool,
        from_normal: bool,
    ) {
        if from_new || from_normal {
            self.return_after_secondary_mode();
        } else {
            self.input.set_text(&saved_text);
            self.mode = Mode::Editing { edit_id };
        }
    }

    fn current_category_parent(&self) -> Option<String> {
        if self.category_index == 0 {
            return None;
        }
        let entries = self.category_entries();
        let path = entries.get(self.category_index - 1)?.path.as_str();
        let parent = path.split_once('/').map_or(path, |(parent, _)| parent);
        normalize_category(parent)
    }

    fn categories(&self) -> Vec<String> {
        self.data.categories_for_archived(self.show_archived)
    }

    fn category_entries(&self) -> Vec<CategoryEntry> {
        self.data.category_entries_for_archived(self.show_archived)
    }

    fn root_categories(&self) -> Vec<String> {
        let mut roots: Vec<String> = self
            .categories()
            .iter()
            .filter_map(|cat| cat.split('/').next())
            .filter_map(normalize_category)
            .collect();
        roots.sort();
        roots.dedup();
        roots
    }

    fn category_from_input(parent: Option<&str>, name: &str) -> Option<String> {
        let name = normalize_category(name)?;
        if name.contains('/') {
            Some(name)
        } else if let Some(parent) = parent {
            normalize_category(&format!("{parent}/{name}"))
        } else {
            Some(name)
        }
    }

    fn set_category_index(&mut self, idx: usize) {
        self.remember_category_selection();
        let cats = self.category_entries();
        self.category_index = idx.min(cats.len());
        self.category_filter = if self.category_index == 0 {
            None
        } else {
            cats.get(self.category_index - 1)
                .map(|entry| entry.path.clone())
        };
        self.restore_category_selection();
    }

    fn parent_category_index_for(&self, idx: usize) -> usize {
        if idx == 0 {
            return 0;
        }

        let cats = self.category_entries();
        let Some(current) = cats.get(idx - 1) else {
            return 0;
        };

        if current.is_all {
            return cats
                .iter()
                .position(|entry| entry.path == current.path && entry.depth == 0)
                .map_or(0, |idx| idx + 1);
        }

        let Some((parent, _)) = current.path.rsplit_once('/') else {
            return 0;
        };

        cats.iter()
            .position(|entry| entry.path == parent && entry.depth == 0)
            .map_or(0, |idx| idx + 1)
    }



    fn selected_category_path(&self) -> Option<String> {
        if self.category_index == 0 {
            None
        } else {
            self.category_entries()
                .get(self.category_index - 1)
                .map(|entry| entry.path.clone())
        }
    }

    fn set_current_category_archived(&mut self, archived: bool) {
        if self.pane != Pane::Categories {
            return;
        }

        let (item_ids, category_names) = if let Some(category) = self.selected_category_path() {
            let ids: Vec<u64> = self
                .data
                .items()
                .iter()
                .filter(|item| {
                    category_matches(item.category.as_deref(), &category)
                        && item.archived != archived
                })
                .map(|item| item.id)
                .collect();
            self.data.set_category_archived(&category, archived);
            (ids, vec![category])
        } else {
            let ids: Vec<u64> = self
                .items()
                .iter()
                .filter(|item| item.archived != archived)
                .map(|item| item.id)
                .collect();
            let categories = self.categories();
            for id in &ids {
                self.data.set_archived(*id, archived);
            }
            for category in &categories {
                self.data.set_category_archived(category, archived);
            }
            (ids, categories)
        };
        if !item_ids.is_empty() || !category_names.is_empty() {
            self.push_undo_archive(item_ids, category_names, !archived);
        }
        self.mark_dirty();
        self.category_index = self.category_index.min(self.category_entries().len());
        self.clamp_selection();
    }

    fn set_visible_archived(&mut self, archived: bool) {
        let ids: Vec<u64> = self
            .items()
            .iter()
            .filter(|item| item.archived != archived)
            .map(|item| item.id)
            .collect();
        let categories = if self.pane == Pane::Categories {
            self.categories()
        } else {
            Vec::new()
        };

        for id in &ids {
            self.data.set_archived(*id, archived);
        }
        for category in &categories {
            self.data.set_category_archived(category, archived);
        }
        if !ids.is_empty() || !categories.is_empty() {
            self.push_undo_archive(ids, categories, !archived);
        }
        self.mark_dirty();
        self.category_index = self.category_index.min(self.category_entries().len());
        self.clamp_selection();
    }

    fn archive_single_visible_item(&mut self, archived: bool) {
        if let Some(id) = self.items().get(self.selected_index()).map(|item| item.id) {
            if self.data.get(id).is_some_and(|item| item.archived != archived) {
                self.data.set_archived(id, archived);
                self.push_undo_archive(vec![id], Vec::new(), !archived);
                self.mark_dirty();
            }
        }
    }

    fn build_confirm_delete_mode(
        &self,
        selected_ids: Vec<u64>,
        selected_categories: Vec<String>,
    ) -> Mode {
        let mut all_ids: Vec<u64> = selected_ids;
        let mut seen: HashSet<u64> = all_ids.iter().copied().collect();
        for name in &selected_categories {
            for item in self.data.items() {
                if category_matches(item.category.as_deref(), name) && seen.insert(item.id) {
                    all_ids.push(item.id);
                }
            }
        }
        let mut texts: Vec<String> = all_ids
            .iter()
            .filter_map(|id| self.data.get(*id).map(|item| item.text.clone()))
            .collect();
        texts.extend(
            selected_categories
                .iter()
                .map(|name| format!("[category] {name}")),
        );
        Mode::ConfirmDelete {
            ids: all_ids,
            texts,
            category_names: selected_categories,
        }
    }

    fn archive_done_items(&mut self) {
        let ids: Vec<u64> = self
            .data
            .items()
            .iter()
            .filter(|item| item.done && !item.archived)
            .map(|item| item.id)
            .collect();
        if ids.is_empty() {
            return;
        }
        for id in &ids {
            self.data.set_archived(*id, true);
        }
        self.push_undo_archive(ids, Vec::new(), false);
        self.mark_dirty();
    }

    fn move_category_to(&mut self, old_category: &str, destination: Option<&str>) {
        let Some(old_category) = normalize_category(old_category) else {
            return;
        };
        let leaf = old_category.rsplit('/').next().unwrap_or(&old_category);
        let new_category = match destination.and_then(normalize_category) {
            Some(parent) => {
                if category_matches(Some(&parent), &old_category) {
                    return;
                }
                normalize_category(&format!("{parent}/{leaf}"))
            }
            None => normalize_category(leaf),
        };
        let Some(new_category) = new_category else {
            return;
        };
        if new_category == old_category || category_matches(Some(&new_category), &old_category) {
            return;
        }

        if self.data.rename_category(&old_category, &new_category) {
            self.category_filter = Some(new_category.clone());
            let cats = self.category_entries();
            if let Some(pos) = cats.iter().position(|entry| entry.path == new_category) {
                self.category_index = pos + 1;
            }
            self.mark_dirty();
        }
    }

    fn switch_category_filter(&mut self, direction: i32, parent_only: bool) {
        let cats = self.category_entries();
        if parent_only {
            let parent_indices: Vec<usize> = std::iter::once(0)
                .chain(
                    cats.iter()
                        .enumerate()
                        .filter(|(_, entry)| entry.depth == 0)
                        .map(|(idx, _)| idx + 1),
                )
                .collect();
            let current_parent = parent_indices
                .iter()
                .rev()
                .copied()
                .find(|idx| *idx <= self.category_index)
                .unwrap_or(0);
            let pos = parent_indices
                .iter()
                .position(|idx| *idx == current_parent)
                .unwrap_or(0);
            let next_pos = if direction < 0 {
                pos.saturating_sub(1)
            } else {
                (pos + 1).min(parent_indices.len().saturating_sub(1))
            };
            if let Some(idx) = parent_indices.get(next_pos) {
                self.set_category_index(*idx);
            }
        } else {
            let max = cats.len();
            let next = if direction < 0 {
                self.category_index.saturating_sub(1)
            } else {
                (self.category_index + 1).min(max)
            };
            self.set_category_index(next);
        }
    }

    fn finish_add_category(&mut self, name: String) {
        self.data.add_category(&name);
        self.mark_dirty();
        let cats = self.category_entries();
        if let Some(pos) = cats.iter().position(|c| c.path == name) {
            self.category_index = pos + 1;
        }
        self.category_filter = Some(name);
        self.pane = Pane::Categories;
        self.clamp_selection();
        self.input.clear();
        self.mode = Mode::Normal;
    }

    /// Opens the note log for `id` with the newest entry highlighted, since that
    /// is the one you just wrote or came back to read.
    fn open_notes(&mut self, id: u64) {
        if self.data.get(id).is_none() {
            return;
        }
        let selected = self.data.last_note_index(id);
        self.input.clear();
        self.mode = Mode::Notes {
            item_id: id,
            selected,
        };
    }

    fn mode_selection_max(&self) -> Option<usize> {
        match &self.mode {
            Mode::Command { .. } => Some(
                get_filtered_commands(self.input.text())
                    .len()
                    .saturating_sub(1),
            ),
            Mode::ThemePicker { .. } => Some(Theme::theme_names().len().saturating_sub(1)),
            Mode::PriorityPicker { .. } => Some(4),
            Mode::SortPicker { .. } => Some(2),
            Mode::ArchivePicker { .. } => Some(archive_picker_labels().len().saturating_sub(1)),
            Mode::FilterPicker { .. } => Some(4),
            Mode::DueDateFilterPicker { .. } => Some(3),
            Mode::CategoryPicker { target, .. } => Some(match target {
                CategoryPickerTarget::MoveCategory(_) => self.root_categories().len(),
                CategoryPickerTarget::AssignItem | CategoryPickerTarget::AssignBulk(_) => {
                    self.categories().len()
                }
            }),
            Mode::BulkActionPicker { ids, category_names, .. } => {
                let single = ids.len() == 1 && category_names.is_empty();
                Some(bulk_action_labels(single).len().saturating_sub(1))
            }
            Mode::Notes { item_id, .. } => Some(self.data.last_note_index(*item_id)),
            Mode::CategoryFilterPicker { .. } => Some(self.category_entries().len()),
            Mode::CategoryCreateChoice { .. } => Some(1),
            Mode::CategoryParentPicker { .. } => {
                Some(self.root_categories().len().saturating_sub(1))
            }
            _ => None,
        }
    }

    fn preview_selected_mode(&mut self) {
        match self.mode {
            Mode::ThemePicker { selected, .. } => {
                if let Some(theme) = theme_by_index(selected) {
                    self.theme = theme;
                }
            }
            Mode::SortPicker { selected, .. } => {
                if let Some(mode) = sort_by_index(selected) {
                    self.sort_mode = mode;
                }
            }
            _ => {}
        }
    }

    fn move_mode_selection(&mut self, direction: i32) -> bool {
        let Some(max) = self.mode_selection_max() else {
            return false;
        };
        let Some(selected) = self.mode.selected_mut() else {
            return false;
        };

        let next = if direction < 0 {
            selected.saturating_sub(1)
        } else {
            (*selected + 1).min(max)
        };
        if next != *selected {
            *selected = next;
            self.preview_selected_mode();
        }
        true
    }

    pub fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
        let mut app = Self::new();

        terminal.hide_cursor()?;

        loop {
            terminal.draw(|f| {
                let items = app.items();
                let categories = app.categories();
                let category_entries = app.category_entries();
                crate::ui::render(
                    f,
                    crate::ui::RenderState {
                        items: &items,
                        selected_index: app.selected_index(),
                        input: &app.input,
                        mode: &app.mode,
                        pending_count: app.pending_count(),
                        filter: &app.filter,
                        priority_filter: &app.priority_filter,
                        due_filter: &app.due_filter,
                        show_archived: app.show_archived,
                        pane: &app.pane,
                        category_index: app.category_index,
                        categories: &categories,
                        category_entries: &category_entries,
                        theme: &app.theme,
                        sort_mode: &app.sort_mode,
                        sidebar_width: app.sidebar_width,
                        note_hint: app.note_hint,
                    },
                )
            })?;

            let debounce = Duration::from_secs(2);
            let timeout = if app.dirty {
                let elapsed = app.last_mutated.elapsed();
                if elapsed >= debounce {
                    app.data.save();
                    app.dirty = false;
                    Duration::from_millis(500)
                } else {
                    debounce - elapsed
                }
            } else {
                Duration::from_millis(500)
            };

            if !crossterm::event::poll(timeout)? {
                if app.dirty {
                    app.data.save();
                    app.dirty = false;
                }
                continue;
            }

            let event = crossterm::event::read()?;
            let action = match event {
                Event::Key(key) => app.dispatch_key(key),
                _ => None,
            };

            if let Some(action) = action {
                if app.handle_action(action) {
                    if app.dirty {
                        app.data.save();
                    }
                    break;
                }
            }
        }

        Ok(())
    }

    fn dispatch_key(&mut self, key: KeyEvent) -> Option<Action> {
        let is_text_mode = matches!(
            self.mode,
            Mode::Editing { .. }
                | Mode::Command { .. }
                | Mode::Searching
                | Mode::CategoryAdd { .. }
                | Mode::RenameInput { .. }
                | Mode::CategoryPicker { .. }
                | Mode::DueDateCalendar { .. }
                | Mode::Notes { .. }
                | Mode::NoteInput { .. }
        );
        if key.code == KeyCode::Char('k')
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && !is_text_mode
        {
            return Some(Action::ExecuteCommand("keybindings".to_string()));
        }
        if key.code == KeyCode::Char('/') && !is_text_mode {
            return Some(Action::StartCommand);
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(Action::Quit);
        }
        if key.code == KeyCode::Char('h')
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && !is_text_mode
        {
            return Some(Action::ExecuteCommand("help".to_string()));
        }
        if key.code == KeyCode::Char('d')
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(&self.mode, Mode::Normal | Mode::Searching)
            && self.pane == Pane::Items
            && self.selected_index() < self.items().len()
        {
            return Some(Action::SetDueDate);
        }
        if key.code == KeyCode::Char('n')
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(&self.mode, Mode::Normal | Mode::Searching)
            && self.pane == Pane::Items
        {
            if let Some(id) = self.items().get(self.selected_index()).map(|item| item.id) {
                if matches!(&self.mode, Mode::Searching) {
                    self.saved_search = Some(self.input.text().to_string());
                    self.input.clear();
                }
                return Some(Action::OpenNotes(id));
            }
        }
        if key.code == KeyCode::Char('z')
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(&self.mode, Mode::Normal | Mode::Notes { .. })
            && !self.undo_stack.is_empty()
        {
            return Some(Action::Undo);
        }
        if key.code == KeyCode::Char('b')
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(&self.mode, Mode::Normal)
        {
            return Some(Action::EnterResizeSidebar);
        }
        if let Mode::ResizeSidebar { .. } = &self.mode {
            return match key.code {
                KeyCode::Left => {
                    let step = if key.modifiers.contains(KeyModifiers::SHIFT) { -5 } else { -1 };
                    Some(Action::ResizeSidebar(step))
                }
                KeyCode::Right => {
                    let step = if key.modifiers.contains(KeyModifiers::SHIFT) { 5 } else { 1 };
                    Some(Action::ResizeSidebar(step))
                }
                KeyCode::Char('r') if key.modifiers.is_empty() => Some(Action::ResetSidebarWidth),
                KeyCode::Enter => Some(Action::ConfirmResizeSidebar),
                KeyCode::Esc => Some(Action::CancelResizeSidebar),
                KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Action::ConfirmResizeSidebar)
                }
                _ => None,
            };
        }

        if matches!(self.mode, Mode::Searching) && key.code == KeyCode::Tab {
            self.filter = self.input.text().to_string();
            self.input.clear();
            self.mode = Mode::MultiSelect {
                cmd: MultiSelectCmd::PickAction,
                selected: HashSet::new(),
                selected_categories: HashSet::new(),
            };
            self.pane = Pane::Items;
            self.clamp_selection();
            return None;
        }

        if matches!(self.mode, Mode::Searching)
            && key.modifiers.contains(KeyModifiers::CONTROL)
        {
            let items = self.items();
            let selected_id = items.get(self.selected_index()).map(|item| item.id);
            if let Some(id) = selected_id {
                match key.code {
                    KeyCode::Char('p') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                        return Some(Action::CyclePriority(id, true));
                    }
                    KeyCode::Char('P') | KeyCode::Char('p') => {
                        return Some(Action::CyclePriority(id, false));
                    }
                    KeyCode::Up => return Some(Action::Reorder(id, -1)),
                    KeyCode::Down => return Some(Action::Reorder(id, 1)),
                    KeyCode::Char('*') => return Some(Action::TogglePin(id)),
                    KeyCode::Char('e') => {
                        self.saved_search = Some(self.input.text().to_string());
                        return Some(Action::EditItem(id));
                    }
                    KeyCode::Char('o') => {
                        self.saved_search = Some(self.input.text().to_string());
                        self.input.clear();
                        return Some(Action::OpenCategoryPicker);
                    }
                    _ => {}
                }
            }
        }

        let is_pane_mode = matches!(self.mode, Mode::Normal | Mode::MultiSelect { .. });
        if is_pane_mode {
            match key.code {
                KeyCode::Left if self.pane == Pane::Items => return Some(Action::SwitchPane),
                KeyCode::Right if self.pane == Pane::Categories => return Some(Action::SwitchPane),
                KeyCode::Tab if key.modifiers.is_empty() => return Some(Action::SwitchPane),
                KeyCode::PageUp => {
                    return Some(Action::SwitchCategoryFilter(-1, true));
                }
                KeyCode::PageDown => {
                    return Some(Action::SwitchCategoryFilter(1, true));
                }
                _ => {}
            }
        }

        if key.modifiers.is_empty() {
            match key.code {
                KeyCode::Right => {
                    if let Some(action) = self.popup_list_enter_action() {
                        return Some(action);
                    }
                }
                KeyCode::Left => {
                    if matches!(self.mode, Mode::Command { .. }) {
                        if let Some(mode) = self.pop_popup_back_target_mode() {
                            self.mode = mode;
                        } else if !self.input.is_empty() {
                            self.input.clear();
                            self.completions.clear();
                            self.completion_index = 0;
                            if let Mode::Command { ref mut selected } = &mut self.mode {
                                *selected = 0;
                            }
                        }
                        return None;
                    }
                    if is_popup_list_mode(&self.mode) {
                        return Some(Action::PopupBack);
                    }
                }
                KeyCode::Backspace if matches!(self.mode, Mode::Command { .. }) => {
                    if !self.input.is_empty() {
                        if let Some(mode) = self.pop_popup_back_target_mode() {
                            self.mode = mode;
                        } else {
                            self.input.clear();
                            self.completions.clear();
                            self.completion_index = 0;
                            if let Mode::Command { ref mut selected } = &mut self.mode {
                                *selected = 0;
                            }
                        }
                    }
                    return None;
                }
                KeyCode::Backspace if is_popup_list_mode(&self.mode) => {
                    return Some(Action::PopupBack);
                }
                _ => {}
            }
        }

        match &self.mode {
            Mode::Normal => {
                if self.pane == Pane::Categories {
                    let entries = self.category_entries();
                    keys::handle_sidebar(key, &entries, self.category_index)
                } else {
                    let items = self.items();
                    keys::handle_normal(key, &items, self.selected_index)
                }
            }
            Mode::Editing { edit_id: None } => {
                let action = keys::handle_editing(key, &mut self.input);
                if action.is_none() && self.input.is_empty() {
                    Some(Action::CancelEdit)
                } else {
                    action
                }
            }
            Mode::Editing { .. } => keys::handle_editing(key, &mut self.input),
            Mode::RenameInput { .. } => {
                if let KeyCode::Enter = key.code {
                    Some(Action::SubmitRename)
                } else if let KeyCode::Esc = key.code {
                    Some(Action::CancelEdit)
                } else {
                    keys::handle_editing(key, &mut self.input)
                }
            }
            Mode::Searching => {
                let action = keys::handle_search(key, &mut self.input);
                self.filter = self.input.text().to_string();
                action
            }
            Mode::Command { selected } => {
                if let KeyCode::Enter = key.code {
                    return resolve_command_input(self.input.text(), *selected)
                        .map(Action::ExecuteCommand);
                }

                let action = keys::handle_command(key, &mut self.input);
                if action.is_some() {
                    self.completions.clear();
                    self.completion_index = 0;
                } else if let KeyCode::Tab = key.code {
                    return Some(Action::TabComplete);
                } else if matches!(key.code, KeyCode::Up) && !key.modifiers.is_empty() {
                    return action;
                } else if matches!(key.code, KeyCode::Up) {
                    return Some(Action::SelectPrev);
                } else if matches!(key.code, KeyCode::Down) {
                    return Some(Action::SelectNext);
                } else if matches!(key.code, KeyCode::Char(_)) {
                    self.completions.clear();
                    self.completion_index = 0;
                    if let Mode::Command { ref mut selected } = &mut self.mode {
                        *selected = 0;
                    }
                }
                action
            }
            Mode::MultiSelect { .. } => {
                if self.pane == Pane::Categories {
                    let entries = self.category_entries();
                    keys::handle_category_multiselect(key, &entries, self.category_index)
                } else {
                    let items = self.items();
                    keys::handle_multiselect(key, &items, self.selected_index)
                }
            }
            Mode::ThemePicker { selected, .. } => {
                let action = keys::handle_theme_picker(key);
                if action.is_none() && key.code == KeyCode::Enter {
                    return Some(Action::ThemeSelect(*selected));
                }
                action
            }
            Mode::PriorityPicker { selected } => {
                let action = keys::handle_priority_picker(key);
                if action.is_none() && key.code == KeyCode::Enter {
                    return Some(Action::PrioritySelect(*selected));
                }
                action
            }
            Mode::CategoryPicker { selected, target } => {
                let nav_action = keys::handle_category_picker(key);
                if nav_action.is_some() {
                    return nav_action;
                }
                match key.code {
                    KeyCode::Enter => {
                        let text = self.input.text().to_string();
                        if text.is_empty()
                            || matches!(target, CategoryPickerTarget::MoveCategory(_))
                        {
                            Some(Action::CategorySelect(*selected))
                        } else {
                            Some(Action::CreateAndAssignCategory(text))
                        }
                    }
                    KeyCode::Esc => {
                        self.input.clear();
                        Some(Action::CancelCategoryPicker)
                    }
                    _ => {
                        if let KeyCode::Char(c) = key.code {
                            if matches!(target, CategoryPickerTarget::AssignItem)
                                && !key.modifiers.contains(KeyModifiers::CONTROL)
                                && !key.modifiers.contains(KeyModifiers::ALT)
                            {
                                self.input.insert_char(c);
                            }
                        }
                        None
                    }
                }
            }
            Mode::CategoryFilterPicker { selected } => {
                let action = keys::handle_category_picker(key);
                if action.is_none() {
                    return match key.code {
                        KeyCode::Backspace => Some(Action::CategoryFilterParent(*selected)),
                        KeyCode::Enter => Some(Action::CategoryFilterSelect(*selected)),
                        _ => None,
                    };
                }
                action
            }
            Mode::CategoryCreateChoice { .. } => keys::handle_category_create_choice(key),
            Mode::CategoryParentPicker { .. } => keys::handle_category_parent_picker(key),
            Mode::CategoryAdd { .. } => {
                let action = keys::handle_category_add(key, &mut self.input);
                if action.is_none() && self.input.is_empty() {
                    Some(Action::CancelCategoryAdd)
                } else {
                    action
                }
            }
            Mode::SortPicker { selected, .. } => {
                let action = keys::handle_sort_picker(key);
                if action.is_none() && key.code == KeyCode::Enter {
                    return Some(Action::SortSelect(*selected));
                }
                action
            }
            Mode::ArchivePicker { selected } => {
                let action = keys::handle_archive_picker(key);
                if action.is_none() && key.code == KeyCode::Enter {
                    return Some(Action::ArchiveSelect(*selected));
                }
                action
            }
            Mode::BulkActionPicker { selected, .. } => {
                let action = keys::handle_bulk_action_picker(key);
                if action.is_none() && key.code == KeyCode::Enter {
                    return Some(Action::BulkActionSelect(*selected));
                }
                action
            }
            Mode::FilterPicker { selected } => {
                let action = keys::handle_filter_picker(key);
                if action.is_none() && key.code == KeyCode::Enter {
                    return Some(Action::FilterSelect(*selected));
                }
                action
            }
            Mode::DueDateFilterPicker { selected } => {
                let action = keys::handle_due_date_filter_picker(key);
                if action.is_none() && key.code == KeyCode::Enter {
                    return Some(Action::DueDateFilterSelect(*selected));
                }
                action
            }
            Mode::ConfirmDelete { .. } => keys::handle_confirm_delete(key),
            Mode::DueDateCalendar { prompt_focused, .. } => {
                keys::handle_due_date_calendar(key, &mut self.input, *prompt_focused)
            }
            Mode::Notes { .. } => keys::handle_notes(key),
            Mode::NoteInput { .. } => keys::handle_note_input(key, &mut self.input),
            Mode::Help | Mode::Keybindings => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => Some(Action::CancelEdit),
                _ => None,
            },
            Mode::ResizeSidebar { .. } => None,
        }
    }

    fn popup_list_enter_action(&self) -> Option<Action> {
        match &self.mode {
            Mode::Command { selected } => {
                resolve_command_input(self.input.text(), *selected).map(Action::ExecuteCommand)
            }
            Mode::ThemePicker { selected, .. } => Some(Action::ThemeSelect(*selected)),
            Mode::PriorityPicker { selected } => Some(Action::PrioritySelect(*selected)),
            Mode::CategoryPicker { selected, target } => {
                if self.input.is_empty() || matches!(target, CategoryPickerTarget::MoveCategory(_))
                {
                    Some(Action::CategorySelect(*selected))
                } else {
                    Some(Action::CreateAndAssignCategory(
                        self.input.text().to_string(),
                    ))
                }
            }
            Mode::CategoryFilterPicker { selected } => {
                Some(Action::CategoryFilterSelect(*selected))
            }
            Mode::CategoryCreateChoice { .. } => Some(Action::AddCategoryChoice(usize::MAX)),
            Mode::CategoryParentPicker { .. } => Some(Action::SelectCategoryParent(usize::MAX)),
            Mode::SortPicker { selected, .. } => Some(Action::SortSelect(*selected)),
            Mode::ArchivePicker { selected } => Some(Action::ArchiveSelect(*selected)),
            Mode::BulkActionPicker { selected, .. } => Some(Action::BulkActionSelect(*selected)),
            Mode::FilterPicker { selected } => Some(Action::FilterSelect(*selected)),
            Mode::DueDateFilterPicker { selected } => Some(Action::DueDateFilterSelect(*selected)),
            _ => None,
        }
    }

    fn handle_action(&mut self, action: Action) -> bool {
        self.note_hint = false;
        match action {
            Action::SelectPrev => {
                if self.move_mode_selection(-1) {
                    return false;
                }
                if self.pane == Pane::Categories {
                    if self.category_index > 0 {
                        self.set_category_index(self.category_index - 1);
                    }
                } else if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            Action::SelectNext => {
                if self.move_mode_selection(1) {
                    return false;
                }
                if self.pane == Pane::Categories {
                    let cats = self.category_entries();
                    if self.category_index < cats.len() {
                        self.set_category_index(self.category_index + 1);
                    }
                } else {
                    let max = self.items().len().saturating_sub(1);
                    if self.selected_index < max {
                        self.selected_index += 1;
                    }
                }
            }
            Action::StartNewItem => {
                self.input.clear();
                self.mode = Mode::Editing { edit_id: None };
            }
            Action::StartNewItemWithChar(c) => {
                self.input.clear();
                self.input.insert_char(c);
                self.mode = Mode::Editing { edit_id: None };
            }
            Action::EditItem(id) => {
                if let Some(item) = self.data.get(id) {
                    self.input.set_text(&item.text);
                    self.mode = Mode::Editing { edit_id: Some(id) };
                }
            }
            Action::SubmitRename => {
                let target = if let Mode::RenameInput { target } = &self.mode {
                    Some(target.clone())
                } else {
                    None
                };

                if let Some(target) = target {
                    let new_name = self.input.text().trim().to_string();
                    match target {
                        RenameTarget::Item(id) => {
                            if !new_name.is_empty() && self.data.update_text(id, &new_name) {
                                self.mark_dirty();
                            }
                        }
                        RenameTarget::Category(old_category) => {
                            if !new_name.is_empty()
                                && self.data.rename_category(&old_category, &new_name)
                            {
                                if self.category_filter.as_deref() == Some(old_category.as_str()) {
                                    self.category_filter = Some(new_name.clone());
                                }
                                if self.pane == Pane::Categories {
                                    let entries = self.category_entries();
                                    if let Some(pos) =
                                        entries.iter().position(|entry| entry.path == new_name)
                                    {
                                        self.category_index = pos + 1;
                                    }
                                }
                                self.mark_dirty();
                            }
                        }
                    }
                }
                self.input.clear();
                self.mode = Mode::Normal;
                self.clamp_selection();
            }
            Action::SubmitEdit => {
                let mut created_id = None;
                if let Mode::Editing { edit_id } = &self.mode {
                    let text = self.input.text().to_string();
                    let text = text.trim().to_string();

                    if !text.is_empty() && text.len() <= 500 {
                        match edit_id {
                            None => {
                                let id = self.data.add(&text);
                                self.mark_dirty();
                                if let Some(ref cat) = self.category_filter {
                                    self.data.set_category(id, Some(cat.clone()));
                                    let cats = self.category_entries();
                                    if let Some(pos) = cats.iter().position(|c| c.path == *cat) {
                                        self.category_index = pos + 1;
                                    }
                                }
                                if let Some(ref date) = self.pending_due_date {
                                    self.data.set_due_date(id, Some(date.clone()));
                                }
                                self.pending_due_date = None;
                                created_id = Some(id);
                            }
                            Some(id) => {
                                self.data.update_text(*id, &text);
                                self.mark_dirty();
                            }
                        }
                    } else if edit_id.is_none() {
                        if let Some(ref cat) = self.category_filter {
                            if !self
                                .data
                                .categories()
                                .iter()
                                .any(|category| category_matches(Some(category), cat))
                            {
                                self.category_filter = None;
                                self.category_index = 0;
                            }
                        }
                        self.pending_due_date = None;
                    }
                }
                if let Some(id) = created_id {
                    let selected = self.initial_due_date(Some(id));
                    self.input.set_text(&selected.iso());
                    self.mode = Mode::DueDateCalendar {
                        edit_id: Some(id),
                        saved_text: String::new(),
                        from_new: true,
                        from_normal: false,
                        selected,
                        prompt_focused: false,
                    };
                } else {
                    self.return_after_secondary_mode();
                }
            }
            Action::CancelEdit => {
                let was_new_edit = matches!(self.mode, Mode::Editing { edit_id: None });
                if was_new_edit {
                    if let Some(ref cat) = self.category_filter {
                        if !self
                            .categories()
                            .iter()
                            .any(|category| category_matches(Some(category), cat))
                        {
                            self.category_filter = None;
                            self.category_index = 0;
                        }
                    }
                    self.pending_due_date = None;
                }
                self.return_after_secondary_mode();
            }
            Action::ToggleDone(id) => {
                self.data.toggle_done(id);
                // Just-completed items are the moment you have the outcome in hand,
                // so nudge towards logging it — but only while there is nothing logged yet.
                self.note_hint = self
                    .data
                    .get(id)
                    .is_some_and(|item| item.done && item.notes.is_empty());
                self.mark_dirty();
            }
            Action::ToggleDoing(id) => {
                self.data.toggle_doing(id);
                self.mark_dirty();
            }
            Action::DeleteItem(id) => {
                if let Some(item) = self.data.get(id) {
                    self.mode = Mode::ConfirmDelete {
                        ids: vec![id],
                        texts: vec![item.text.clone()],
                        category_names: Vec::new(),
                    };
                }
            }
            Action::CyclePriority(id, forward) => {
                self.data.cycle_priority(id, forward);
                self.mark_dirty();
                let new_idx = self.items().iter().position(|i| i.id == id);
                if let Some(pos) = new_idx {
                    self.selected_index = pos;
                }
            }
            Action::Reorder(id, direction) => {
                if self.data.reorder(id, direction) {
                    if direction < 0 && self.selected_index > 0 {
                        self.selected_index -= 1;
                    } else if direction > 0
                        && self.selected_index < self.items().len().saturating_sub(1)
                    {
                        self.selected_index += 1;
                    }
                    self.mark_dirty();
                }
                self.clamp_selection();
            }
            Action::ReorderCategory(path, direction) => {
                if self.data.reorder_category(&path, direction) {
                    let entries = self.category_entries();
                    if let Some(pos) = entries.iter().position(|e| e.path == path) {
                        self.category_index = pos + 1;
                    }
                    self.mark_dirty();
                } else if path.contains('/') {
                    return self.handle_action(Action::OpenCategoryMovePicker(path));
                }
            }
            Action::ApplySearch => {
                self.filter = self.input.text().to_string();
                self.input.clear();
                let selected_id = self
                    .items()
                    .get(self.selected_index())
                    .map(|item| item.id);
                if let Some(id) = selected_id {
                    self.saved_search = Some(self.filter.clone());
                    self.mode = Mode::BulkActionPicker {
                        ids: vec![id],
                        category_names: Vec::new(),
                        selected: 0,
                    };
                } else {
                    self.mode = Mode::Normal;
                }
                self.clamp_selection();
            }
            Action::ClearSearch => {
                self.filter.clear();
                self.input.clear();
                self.mode = Mode::Normal;
                self.clamp_selection();
            }
            Action::StartCommand => {
                self.input.clear();
                self.completions.clear();
                self.completion_index = 0;
                self.filter.clear();
                self.popup_back_stack.clear();
                self.mode = Mode::Command { selected: 0 };
            }
            Action::TabComplete => {
                let text = self.input.text().to_string();
                if self.completions.is_empty() {
                    self.completions = get_completions(&text);
                    self.completion_index = 0;
                }
                if !self.completions.is_empty() {
                    let idx = self.completion_index % self.completions.len();
                    self.input.set_text(&self.completions[idx]);
                    self.completion_index = (idx + 1) % self.completions.len();
                }
            }
            Action::ExecuteCommand(raw) => {
                let raw = raw.trim();
                let parts: Vec<&str> = raw.splitn(2, ' ').collect();
                let cmd = parts[0].to_lowercase();
                let arg = parts.get(1).map(|s| s.to_string());

                if !matches!(cmd.as_str(), "search" | "s") {
                    self.saved_search = None;
                }

                if arg.is_none() && has_visible_subcommands(&cmd) {
                    self.push_command_popup_back_target(&cmd);
                    self.input.set_text(&format!("{cmd} "));
                    self.mode = Mode::Command { selected: 0 };
                    self.completions.clear();
                    self.completion_index = 0;
                    return false;
                }

                match cmd.as_str() {
                    "search" | "s" => {
                        let query = arg.unwrap_or_default();
                        self.filter = query.clone();
                        self.input.set_text(&query);
                        self.mode = Mode::Searching;
                    }
                    "delete" | "d" => {
                        self.mode = Mode::MultiSelect {
                            cmd: MultiSelectCmd::Delete,
                            selected: HashSet::new(),
                            selected_categories: HashSet::new(),
                        };
                    }
                    "done" | "x" => {
                        self.pane = Pane::Items;
                        self.mode = Mode::MultiSelect {
                            cmd: MultiSelectCmd::ToggleDone,
                            selected: HashSet::new(),
                            selected_categories: HashSet::new(),
                        };
                    }
                    "select" => {
                        self.mode = Mode::MultiSelect {
                            cmd: MultiSelectCmd::PickAction,
                            selected: HashSet::new(),
                            selected_categories: HashSet::new(),
                        };
                    }
                    "clear" | "c" => {
                        self.data.clear_done();
                        self.mark_dirty();
                        self.mode = Mode::Normal;
                    }
                    "archive" => {
                        match arg.as_deref() {
                            Some("one") => {
                                if self.pane == Pane::Categories {
                                    self.set_current_category_archived(true);
                                } else {
                                    self.archive_single_visible_item(true);
                                }
                                self.mode = Mode::Normal;
                            }
                            Some("bulk") | Some("select") => {
                                self.mode = Mode::MultiSelect {
                                    cmd: MultiSelectCmd::Archive,
                                    selected: HashSet::new(),
                                    selected_categories: HashSet::new(),
                                };
                            }
                            Some("archived") => {
                                self.show_archived = !self.show_archived;
                                self.mode = Mode::Normal;
                            }
                            Some("done") => {
                                self.archive_done_items();
                                self.mode = Mode::Normal;
                            }
                            Some("all") => {
                                self.set_visible_archived(true);
                                self.mode = Mode::Normal;
                            }
                            Some(restore_arg)
                                if restore_arg == "restore"
                                    || restore_arg.starts_with("restore ") =>
                            {
                                let restore_all = restore_arg == "restore all";
                                let restore_bulk = restore_arg == "restore bulk"
                                    || restore_arg == "restore select";
                                if restore_all {
                                    self.set_visible_archived(false);
                                } else if restore_bulk {
                                    self.show_archived = true;
                                    self.mode = Mode::MultiSelect {
                                        cmd: MultiSelectCmd::RestoreArchive,
                                        selected: HashSet::new(),
                                        selected_categories: HashSet::new(),
                                    };
                                    self.clamp_selection();
                                    return false;
                                } else if self.pane == Pane::Categories {
                                    self.set_current_category_archived(false);
                                } else {
                                    self.archive_single_visible_item(false);
                                }
                                self.mode = Mode::Normal;
                            }
                            _ => {
                                self.push_command_popup_back_target(&cmd);
                                self.mode = Mode::ArchivePicker { selected: 0 };
                            }
                        }
                        self.clamp_selection();
                    }
                    "archived" => {
                        self.show_archived = !self.show_archived;
                        self.mode = Mode::Normal;
                        self.clamp_selection();
                    }
                    "unarchive" => {
                        match arg.as_deref() {
                            Some("all") => {
                                self.set_visible_archived(false);
                            }
                            _ => {
                                if self.pane == Pane::Categories {
                                    self.set_current_category_archived(false);
                                } else {
                                    self.archive_single_visible_item(false);
                                }
                            }
                        }
                        self.mode = Mode::Normal;
                        self.clamp_selection();
                    }
                    "due" => {
                        self.due_filter = match arg.as_deref() {
                            Some("today") => Some(DueFilter::Today),
                            Some("week") => Some(DueFilter::Week),
                            Some("overdue") => Some(DueFilter::Overdue),
                            Some("clear") | Some("all") | None => None,
                            _ => self.due_filter,
                        };
                        self.mode = Mode::Normal;
                        self.clamp_selection();
                    }
                    "rename" => {
                        if let Some(arg) = arg {
                            let parts: Vec<&str> = arg.split_whitespace().collect();
                            if parts.len() >= 2 && self.data.rename_category(parts[0], parts[1]) {
                                if self.category_filter.as_deref() == Some(parts[0]) {
                                    self.category_filter = Some(parts[1].to_string());
                                }
                                self.mark_dirty();
                            }
                            self.mode = Mode::Normal;
                        } else {
                            let rename_target = if self.pane == Pane::Categories {
                                let entries = self.category_entries();
                                if self.category_index > 0 {
                                    entries
                                        .get(self.category_index - 1)
                                        .map(|e| RenameTarget::Category(e.path.clone()))
                                } else {
                                    None
                                }
                            } else {
                                self.items()
                                    .get(self.selected_index())
                                    .map(|item| RenameTarget::Item(item.id))
                            };
                            if let Some(target) = rename_target {
                                self.input.clear();
                                match &target {
                                    RenameTarget::Item(id) => {
                                        if let Some(item) = self.data.get(*id) {
                                            self.input.insert_str(&item.text);
                                        }
                                    }
                                    RenameTarget::Category(name) => {
                                        self.input.insert_str(name);
                                    }
                                }
                                self.mode = Mode::RenameInput { target };
                            } else {
                                self.mode = Mode::Normal;
                            }
                        }
                        self.clamp_selection();
                    }
                    "filter" => match arg.as_deref() {
                        Some("due") => {
                            self.push_command_popup_back_target(&cmd);
                            self.mode = Mode::DueDateFilterPicker { selected: 0 };
                        }
                        Some("priority") => {
                            self.push_command_popup_back_target(&cmd);
                            self.mode = Mode::PriorityPicker { selected: 0 };
                        }
                        Some("category") => {
                            self.push_command_popup_back_target(&cmd);
                            self.mode = Mode::CategoryFilterPicker { selected: 0 };
                        }
                        Some("archived") => {
                            self.show_archived = !self.show_archived;
                            self.mode = Mode::Normal;
                            self.clamp_selection();
                        }
                        Some("clear") => {
                            self.filter.clear();
                            self.due_filter = None;
                            self.priority_filter = None;
                            self.category_filter = None;
                            self.category_index = 0;
                            self.show_archived = false;
                            self.sort_mode = SortMode::Default;
                            self.mode = Mode::Normal;
                            self.clamp_selection();
                        }
                        _ => {
                            self.push_command_popup_back_target(&cmd);
                            self.mode = Mode::FilterPicker { selected: 0 };
                        }
                    },
                    "reset" => {
                        self.filter.clear();
                        self.due_filter = None;
                        self.priority_filter = None;
                        self.category_filter = None;
                        self.category_index = 0;
                        self.show_archived = false;
                        self.sort_mode = SortMode::Default;
                        self.mode = Mode::Normal;
                        self.clamp_selection();
                    }
                    "move" | "m" => {
                        if self.pane == Pane::Categories {
                            if let Some(category) = self.selected_category_path() {
                                self.push_command_popup_back_target(&cmd);
                                self.input.clear();
                                self.mode = Mode::CategoryPicker {
                                    selected: 0,
                                    target: CategoryPickerTarget::MoveCategory(category),
                                };
                            } else {
                                self.mode = Mode::Normal;
                            }
                        } else {
                            self.push_command_popup_back_target(&cmd);
                            self.mode = Mode::CategoryPicker {
                                selected: 0,
                                target: CategoryPickerTarget::AssignItem,
                            };
                        }
                    }
                    "notes" | "n" => {
                        let id = if self.pane == Pane::Items {
                            self.items().get(self.selected_index()).map(|item| item.id)
                        } else {
                            None
                        };
                        match id {
                            Some(id) => self.open_notes(id),
                            None => self.mode = Mode::Normal,
                        }
                    }
                    "themes" => {
                        self.push_command_popup_back_target(&cmd);
                        self.mode = Mode::ThemePicker {
                            selected: theme_index_by_name(self.theme.name),
                            original_theme: self.theme.clone(),
                        };
                    }
                    "priorities" | "p" => {
                        self.push_command_popup_back_target(&cmd);
                        self.mode = Mode::PriorityPicker { selected: 0 };
                    }
                    "help" => {
                        self.mode = Mode::Help;
                    }
                    "keybindings" | "k" => {
                        self.mode = Mode::Keybindings;
                    }
                    "sidebar" => {
                        let return_pane = self.pane;
                        let original_width = self.sidebar_width;
                        self.pane = Pane::Categories;
                        self.mode = Mode::ResizeSidebar {
                            return_pane,
                            original_width,
                        };
                    }
                    "categories" | "cat" => {
                        self.push_command_popup_back_target(&cmd);
                        self.mode = Mode::CategoryFilterPicker { selected: 0 };
                    }
                    "sort" => match arg.as_deref() {
                        Some("priority") => {
                            self.sort_mode = SortMode::Priority;
                            self.mode = Mode::Normal;
                        }
                        Some("due") => {
                            self.sort_mode = SortMode::DueDate;
                            self.mode = Mode::Normal;
                        }
                        Some("default") => {
                            self.sort_mode = SortMode::Default;
                            self.mode = Mode::Normal;
                        }
                        _ => {
                            self.push_command_popup_back_target(&cmd);
                            self.mode = Mode::SortPicker {
                                selected: sort_index(self.sort_mode),
                                original_sort: self.sort_mode,
                            };
                        }
                    },
                    _ => {
                        self.mode = Mode::Normal;
                    }
                }
                self.completions.clear();
                self.completion_index = 0;
                if !matches!(self.mode, Mode::RenameInput { .. } | Mode::Searching) {
                    self.input.clear();
                }
                self.clamp_selection();
            }
            Action::ToggleMultiSelect(id) => {
                if let Mode::MultiSelect {
                    ref mut selected, ..
                } = &mut self.mode
                {
                    if !selected.insert(id) {
                        selected.remove(&id);
                    }
                }
            }
            Action::ToggleCategoryMultiSelect(name) => {
                if let Mode::MultiSelect {
                    ref mut selected_categories,
                    ..
                } = &mut self.mode
                {
                    if !selected_categories.insert(name.clone()) {
                        selected_categories.remove(&name);
                    }
                }
            }
            Action::SelectAllMultiSelect => {
                let Mode::MultiSelect { cmd, .. } = &self.mode else {
                    return false;
                };

                if *cmd == MultiSelectCmd::Delete {
                    let (ids, category_names) = if self.pane == Pane::Categories {
                        (
                            self.data.items().iter().map(|item| item.id).collect(),
                            self.categories(),
                        )
                    } else {
                        let ids: Vec<u64> = self.items().iter().map(|item| item.id).collect();
                        (ids, Vec::new())
                    };
                    if !ids.is_empty() || !category_names.is_empty() {
                        self.mode = self.build_confirm_delete_mode(ids, category_names);
                    }
                    return false;
                }

                let ids: HashSet<u64> = if self.pane == Pane::Categories {
                    HashSet::new()
                } else {
                    self.items().iter().map(|item| item.id).collect()
                };
                let selected_categories: HashSet<String> = if self.pane == Pane::Categories {
                    self.categories().into_iter().collect()
                } else {
                    HashSet::new()
                };

                if let Mode::MultiSelect {
                    selected,
                    selected_categories: categories,
                    ..
                } = &mut self.mode
                {
                    *selected = ids;
                    *categories = selected_categories;
                }
            }
            Action::ConfirmMultiSelect => {
                let mode = std::mem::replace(&mut self.mode, Mode::Normal);
                if let Mode::MultiSelect {
                    cmd,
                    selected,
                    selected_categories,
                } = mode
                {
                    match cmd {
                        MultiSelectCmd::PickAction => {
                            let ids: Vec<u64> = selected.into_iter().collect();
                            let category_names: Vec<String> =
                                selected_categories.into_iter().collect();
                            if ids.is_empty() && category_names.is_empty() {
                                self.mode = Mode::Normal;
                                return false;
                            }
                            self.mode = Mode::BulkActionPicker {
                                ids,
                                category_names,
                                selected: 0,
                            };
                            return false;
                        }
                        MultiSelectCmd::Delete => {
                            let ids: Vec<u64> = selected.into_iter().collect();
                            let category_names: Vec<String> =
                                selected_categories.into_iter().collect();
                            if ids.is_empty() && category_names.is_empty() {
                                self.mode = Mode::Normal;
                                return false;
                            }
                            self.mode = self.build_confirm_delete_mode(ids, category_names);
                            return false;
                        }
                        MultiSelectCmd::ToggleDone => {
                            for id in &selected {
                                self.data.toggle_done(*id);
                            }
                            self.mark_dirty();
                        }
                        MultiSelectCmd::Archive => {
                            let item_ids: Vec<u64> = selected.iter().copied().collect();
                            let category_names: Vec<String> =
                                selected_categories.iter().cloned().collect();
                            for id in &item_ids {
                                self.data.set_archived(*id, true);
                            }
                            for category in &category_names {
                                self.data.set_category_archived(category, true);
                            }
                            self.push_undo_archive(item_ids, category_names, false);
                            self.mark_dirty();
                        }
                        MultiSelectCmd::RestoreArchive => {
                            let item_ids: Vec<u64> = selected.iter().copied().collect();
                            let category_names: Vec<String> =
                                selected_categories.iter().cloned().collect();
                            for id in &item_ids {
                                self.data.set_archived(*id, false);
                            }
                            for category in &category_names {
                                self.data.set_category_archived(category, false);
                            }
                            self.push_undo_archive(item_ids, category_names, true);
                            self.mark_dirty();
                        }
                    }
                    if let Some(ref cat) = self.category_filter {
                        if !self
                            .categories()
                            .iter()
                            .any(|category| category_matches(Some(category), cat))
                        {
                            self.category_filter = None;
                            self.category_index = 0;
                        }
                    }
                    self.clamp_selection();
                }
            }
            Action::CancelMultiSelect => {
                self.mode = Mode::Normal;
            }
            Action::ThemeSelect(idx) => {
                let names = Theme::theme_names();
                if let Some(name) = names.get(idx) {
                    if let Some(new_theme) = Theme::by_name(name) {
                        self.theme = new_theme;
                        config::save_theme_name(name);
                    }
                }
                self.mode = Mode::Normal;
            }
            Action::CancelThemePicker => {
                if let Mode::ThemePicker { original_theme, .. } = &self.mode {
                    self.theme = original_theme.clone();
                }
                self.mode = Mode::Normal;
            }
            Action::ConfirmDeleteYes => {
                let mode = std::mem::replace(&mut self.mode, Mode::Normal);
                if let Mode::ConfirmDelete {
                    ids,
                    category_names,
                    ..
                } = mode
                {
                    let mut all_ids: Vec<u64> = ids;
                    let mut seen: HashSet<u64> = all_ids.iter().copied().collect();
                    for name in &category_names {
                        let cascaded: Vec<u64> = self
                            .data
                            .items()
                            .iter()
                            .filter(|item| category_matches(item.category.as_deref(), name))
                            .map(|item| item.id)
                            .collect();
                        for id in cascaded {
                            if seen.insert(id) {
                                all_ids.push(id);
                            }
                        }
                    }
                    let snapshot: Vec<TodoItem> = all_ids
                        .iter()
                        .filter_map(|id| self.data.get(*id).cloned())
                        .collect();
                    for id in all_ids {
                        self.data.delete(id);
                    }
                    for name in category_names {
                        self.data.remove_category(&name);
                    }
                    self.push_undo_delete(snapshot);
                    self.mark_dirty();
                }
                if let Some(ref cat) = self.category_filter {
                    if !self
                        .data
                        .categories()
                        .iter()
                        .any(|category| category_matches(Some(category), cat))
                    {
                        self.category_filter = None;
                        self.category_index = 0;
                    }
                }
                self.return_after_secondary_mode();
            }
            Action::ConfirmDeleteNo => {
                self.return_after_secondary_mode();
            }
            Action::ConfirmDeleteArchive => {
                let mode = std::mem::replace(&mut self.mode, Mode::Normal);
                if let Mode::ConfirmDelete {
                    ids,
                    category_names,
                    ..
                } = mode
                {
                    for id in &ids {
                        self.data.set_archived(*id, true);
                    }
                    for name in &category_names {
                        self.data.set_category_archived(name, true);
                    }
                    self.push_undo_archive(ids, category_names, false);
                    self.mark_dirty();
                }
                self.return_after_secondary_mode();
            }
            Action::PrioritySelect(idx) => {
                const PRIORITIES: [Option<Priority>; 5] = [
                    None,
                    Some(Priority::Urgent),
                    Some(Priority::High),
                    Some(Priority::Normal),
                    Some(Priority::Low),
                ];
                self.priority_filter = PRIORITIES[idx];
                self.mode = Mode::Normal;
                self.clamp_selection();
            }
            Action::CancelPriorityPicker => {
                self.mode = Mode::Normal;
            }
            Action::FilterSelect(idx) => {
                self.mode = Mode::Normal;
                match idx {
                    0 => {
                        self.show_archived = !self.show_archived;
                        self.clamp_selection();
                    }
                    1 => {
                        self.popup_back_stack
                            .push(PopupBackTarget::FilterPicker { selected: idx });
                        self.mode = Mode::CategoryFilterPicker { selected: 0 };
                    }
                    2 => {
                        self.popup_back_stack
                            .push(PopupBackTarget::FilterPicker { selected: idx });
                        self.mode = Mode::DueDateFilterPicker { selected: 0 };
                    }
                    3 => {
                        self.popup_back_stack
                            .push(PopupBackTarget::FilterPicker { selected: idx });
                        self.mode = Mode::PriorityPicker { selected: 0 };
                    }
                    4 => {
                        self.filter.clear();
                        self.due_filter = None;
                        self.priority_filter = None;
                        self.category_filter = None;
                        self.category_index = 0;
                        self.show_archived = false;
                        self.sort_mode = SortMode::Default;
                        self.clamp_selection();
                    }
                    _ => {}
                }
            }
            Action::ArchiveSelect(idx) => {
                let label = match archive_picker_labels().get(idx).copied() {
                    Some(label) => label,
                    None => return false,
                };
                match label {
                    "Archive one" => {
                        if self.pane == Pane::Categories {
                            self.set_current_category_archived(true);
                        } else {
                            self.archive_single_visible_item(true);
                        }
                    }
                    "Archive bulk" => {
                        self.mode = Mode::MultiSelect {
                            cmd: MultiSelectCmd::Archive,
                            selected: HashSet::new(),
                            selected_categories: HashSet::new(),
                        };
                        return false;
                    }
                    "Archive done" => {
                        self.archive_done_items();
                    }
                    "Archive all" => {
                        self.set_visible_archived(true);
                    }
                    "Archived view" => {
                        self.show_archived = !self.show_archived;
                    }
                    "Restore one" => {
                        if self.pane == Pane::Categories {
                            self.set_current_category_archived(false);
                        } else {
                            self.archive_single_visible_item(false);
                        }
                    }
                    "Restore bulk" => {
                        self.show_archived = true;
                        self.mode = Mode::MultiSelect {
                            cmd: MultiSelectCmd::RestoreArchive,
                            selected: HashSet::new(),
                            selected_categories: HashSet::new(),
                        };
                        return false;
                    }
                    "Restore all" => {
                        self.set_visible_archived(false);
                    }
                    _ => {}
                }
                self.mode = Mode::Normal;
                self.clamp_selection();
            }
            Action::CancelArchivePicker => {
                self.mode = Mode::Normal;
            }
            Action::CancelFilterPicker => {
                self.mode = Mode::Normal;
            }
            Action::DueDateFilterSelect(idx) => {
                const DUE_OPTIONS: [Option<DueFilter>; 4] = [
                    Some(DueFilter::Overdue),
                    Some(DueFilter::Week),
                    Some(DueFilter::Today),
                    None,
                ];
                self.due_filter = DUE_OPTIONS[idx];
                self.mode = Mode::Normal;
                self.clamp_selection();
            }
            Action::CancelDueDateFilterPicker => {
                self.mode = Mode::Normal;
            }
            Action::PopupBack => {
                let mode = std::mem::replace(&mut self.mode, Mode::Normal);
                self.mode = match mode {
                    Mode::CategoryFilterPicker { selected } => {
                        let parent_idx = self.parent_category_index_for(selected);
                        if parent_idx != selected {
                            Mode::CategoryFilterPicker {
                                selected: parent_idx,
                            }
                        } else if let Some(mode) = self.pop_popup_back_target_mode() {
                            mode
                        } else {
                            Mode::FilterPicker { selected: 2 }
                        }
                    }
                    Mode::DueDateFilterPicker { .. } => {
                        if let Some(mode) = self.pop_popup_back_target_mode() {
                            mode
                        } else {
                            Mode::FilterPicker { selected: 0 }
                        }
                    }
                    Mode::PriorityPicker { .. } => {
                        if let Some(mode) = self.pop_popup_back_target_mode() {
                            mode
                        } else {
                            Mode::FilterPicker { selected: 1 }
                        }
                    }
                    Mode::CategoryCreateChoice { parent, name, .. } => {
                        self.input.set_text(&name);
                        Mode::CategoryAdd { parent }
                    }
                    Mode::CategoryParentPicker { name, .. } => Mode::CategoryCreateChoice {
                        selected: 0,
                        parent: None,
                        name,
                    },
                    Mode::ThemePicker { original_theme, .. } => {
                        self.theme = original_theme;
                        if let Some(mode) = self.pop_popup_back_target_mode() {
                            mode
                        } else {
                            self.input.clear();
                            self.completions.clear();
                            self.completion_index = 0;
                            Mode::Command { selected: 0 }
                        }
                    }
                    Mode::SortPicker { original_sort, .. } => {
                        self.sort_mode = original_sort;
                        self.clamp_selection();
                        if let Some(mode) = self.pop_popup_back_target_mode() {
                            mode
                        } else {
                            self.input.clear();
                            self.completions.clear();
                            self.completion_index = 0;
                            Mode::Command { selected: 0 }
                        }
                    }
                    Mode::CategoryPicker { .. }
                    | Mode::ArchivePicker { .. }
                    | Mode::FilterPicker { .. } => {
                        if let Some(mode) = self.pop_popup_back_target_mode() {
                            mode
                        } else {
                            self.input.clear();
                            self.completions.clear();
                            self.completion_index = 0;
                            Mode::Command { selected: 0 }
                        }
                    }
                    Mode::BulkActionPicker { .. } => Mode::Normal,
                    other => other,
                };
            }
            Action::Paste => {
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    if let Ok(text) = cb.get_text() {
                        self.input.insert_str(&text);
                    }
                }
            }
            Action::TogglePin(id) => {
                self.data.toggle_pin(id);
                self.mark_dirty();
                let new_idx = self.items().iter().position(|i| i.id == id);
                if let Some(pos) = new_idx {
                    self.selected_index = pos;
                }
            }
            Action::SetDueDate => {
                let (edit_id, from_normal) = match &self.mode {
                    Mode::Editing { edit_id } => (*edit_id, false),
                    Mode::Normal => (self.items().get(self.selected_index()).map(|i| i.id), true),
                    Mode::Searching => {
                        self.saved_search = Some(self.input.text().to_string());
                        (self.items().get(self.selected_index()).map(|i| i.id), true)
                    }
                    _ => return false,
                };
                let saved_text = self.input.text().to_string();
                let selected = self.initial_due_date(edit_id);
                self.input.set_text(&selected.iso());
                self.mode = Mode::DueDateCalendar {
                    edit_id,
                    saved_text,
                    from_new: false,
                    from_normal,
                    selected,
                    prompt_focused: false,
                };
            }
            Action::SubmitDueDate => {
                let (edit_id, saved_text, from_new, from_normal, date) = match &self.mode {
                    Mode::DueDateCalendar {
                        edit_id,
                        saved_text,
                        from_new,
                        from_normal,
                        ..
                    } => (
                        *edit_id,
                        saved_text.clone(),
                        *from_new,
                        *from_normal,
                        Date::parse(self.input.text().trim()).map(Date::iso),
                    ),
                    _ => (None, String::new(), false, false, Some(String::new())),
                };
                let Some(date) = date else {
                    return false;
                };
                if !date.is_empty() {
                    if let Some(id) = edit_id {
                        self.data.set_due_date(id, Some(date));
                        self.mark_dirty();
                    } else {
                        self.pending_due_date = Some(date);
                    }
                }
                self.restore_after_due_date(edit_id, saved_text, from_new, from_normal);
            }
            Action::CancelDueDate => {
                let (edit_id, saved_text, from_new, from_normal) = match &self.mode {
                    Mode::DueDateCalendar {
                        edit_id,
                        saved_text,
                        from_new,
                        from_normal,
                        ..
                    } => (*edit_id, saved_text.clone(), *from_new, *from_normal),
                    _ => (None, String::new(), false, false),
                };
                self.restore_after_due_date(edit_id, saved_text, from_new, from_normal);
            }
            Action::CalendarMove(days) => {
                let mut next = None;
                if let Mode::DueDateCalendar { selected, .. } = &mut self.mode {
                    *selected = selected.add_days(days);
                    next = Some(selected.iso());
                }
                if let Some(date) = next {
                    self.input.set_text(&date);
                }
            }
            Action::CalendarMonth(months) => {
                let mut next = None;
                if let Mode::DueDateCalendar { selected, .. } = &mut self.mode {
                    *selected = selected.add_months(months);
                    next = Some(selected.iso());
                }
                if let Some(date) = next {
                    self.input.set_text(&date);
                }
            }
            Action::CalendarToday => {
                if let Mode::DueDateCalendar { selected, .. } = &mut self.mode {
                    *selected = crate::date::Date::today();
                    self.input.set_text(&selected.iso());
                }
            }
            Action::CalendarClear => {
                let (edit_id, saved_text, from_new, from_normal) = match &self.mode {
                    Mode::DueDateCalendar {
                        edit_id,
                        saved_text,
                        from_new,
                        from_normal,
                        ..
                    } => (*edit_id, saved_text.clone(), *from_new, *from_normal),
                    _ => (None, String::new(), false, false),
                };
                if let Some(id) = edit_id {
                    self.data.set_due_date(id, None);
                    self.mark_dirty();
                } else {
                    self.pending_due_date = None;
                }
                self.restore_after_due_date(edit_id, saved_text, from_new, from_normal);
            }
            Action::CalendarTextChanged => {
                if let Mode::DueDateCalendar {
                    selected,
                    prompt_focused,
                    ..
                } = &mut self.mode
                {
                    *prompt_focused = true;
                    if let Some(date) = Date::parse(self.input.text().trim()) {
                        *selected = date;
                    }
                }
            }
            Action::CalendarToggleFocus => {
                if let Mode::DueDateCalendar { prompt_focused, .. } = &mut self.mode {
                    *prompt_focused = !*prompt_focused;
                }
            }
            Action::SwitchCategoryFilter(direction, parent_only) => {
                self.switch_category_filter(direction, parent_only);
            }
            Action::SwitchPane => {
                self.pane = match self.pane {
                    Pane::Items => Pane::Categories,
                    Pane::Categories => Pane::Items,
                };
            }
            Action::OpenCategoryPicker => {
                self.mode = Mode::CategoryPicker {
                    selected: 0,
                    target: CategoryPickerTarget::AssignItem,
                };
            }
            Action::OpenCategoryMovePicker(category) => {
                self.push_command_popup_back_target("move");
                self.input.clear();
                self.mode = Mode::CategoryPicker {
                    selected: 0,
                    target: CategoryPickerTarget::MoveCategory(category),
                };
            }
            Action::CategorySelect(idx) => {
                let picker_target = match &self.mode {
                    Mode::CategoryPicker { target, .. } => Some(target.clone()),
                    _ => None,
                };
                if let Some(target) = picker_target {
                    let cats = self.categories();
                    match target {
                        CategoryPickerTarget::AssignItem => {
                            let items = self.items();
                            let id = items.get(self.selected_index()).map(|i| i.id);
                            if let Some(id) = id {
                                let category = if idx == 0 {
                                    None
                                } else {
                                    cats.get(idx - 1).cloned()
                                };
                                self.data.set_category(id, category);
                                self.mark_dirty();
                            }
                        }
                        CategoryPickerTarget::MoveCategory(old_category) => {
                            let root_cats = self.root_categories();
                            let destination = if idx == 0 {
                                None
                            } else {
                                root_cats.get(idx - 1).map(String::as_str)
                            };
                            self.move_category_to(&old_category, destination);
                        }
                        CategoryPickerTarget::AssignBulk(bulk_ids) => {
                            let category = if idx == 0 {
                                None
                            } else {
                                cats.get(idx - 1).cloned()
                            };
                            for id in bulk_ids {
                                self.data.set_category(id, category.clone());
                            }
                            self.mark_dirty();
                        }
                    }
                    self.return_after_secondary_mode();
                } else {
                    self.set_category_index(idx);
                    self.pane = Pane::Items;
                    self.mode = Mode::Normal;
                }
            }
            Action::CategoryFilterSelect(idx) => {
                self.set_category_index(idx);
                self.pane = Pane::Items;
                self.mode = Mode::Normal;
            }
            Action::CategoryFilterParent(idx) => {
                self.mode = Mode::CategoryFilterPicker {
                    selected: self.parent_category_index_for(idx),
                };
            }
            Action::AddCategory(name) => {
                if let Some(normalized) = normalize_category(&name) {
                    self.input.clear();
                    if self.data.categories().is_empty() {
                        self.finish_add_category(normalized);
                    } else {
                        self.mode = Mode::CategoryCreateChoice {
                            selected: 0,
                            parent: self.current_category_parent(),
                            name,
                        };
                    }
                } else {
                    self.input.clear();
                    self.mode = Mode::Normal;
                }
            }
            Action::DeleteCategory(name) => {
                self.mode = self.build_confirm_delete_mode(Vec::new(), vec![name]);
            }
            Action::CancelCategoryPicker => {
                self.return_after_secondary_mode();
            }
            Action::AddCategoryChoice(choice) => {
                let mode = std::mem::replace(&mut self.mode, Mode::Normal);
                if let Mode::CategoryCreateChoice {
                    selected,
                    parent,
                    name,
                } = mode
                {
                    let choice = if choice == usize::MAX {
                        selected
                    } else {
                        choice
                    };
                    match (choice, parent) {
                        (0, Some(parent)) => {
                            if let Some(name) = Self::category_from_input(Some(&parent), &name) {
                                self.finish_add_category(name);
                            } else {
                                self.mode = Mode::Normal;
                            }
                        }
                        (0, None) => {
                            let roots = self.root_categories();
                            if roots.is_empty() {
                                if let Some(name) = Self::category_from_input(None, &name) {
                                    self.finish_add_category(name);
                                } else {
                                    self.mode = Mode::Normal;
                                }
                            } else {
                                self.mode = Mode::CategoryParentPicker { selected: 0, name };
                            }
                        }
                        _ => {
                            if let Some(name) = Self::category_from_input(None, &name) {
                                self.finish_add_category(name);
                            } else {
                                self.mode = Mode::Normal;
                            }
                        }
                    }
                }
            }
            Action::SelectCategoryParent(idx) => {
                let roots = self.root_categories();
                let (idx, name) = if idx == usize::MAX {
                    match &self.mode {
                        Mode::CategoryParentPicker { selected, name } => (*selected, name.clone()),
                        _ => (idx, String::new()),
                    }
                } else {
                    let name = match &self.mode {
                        Mode::CategoryParentPicker { name, .. } => name.clone(),
                        _ => String::new(),
                    };
                    (idx, name)
                };
                if let Some(parent) = roots.get(idx) {
                    if let Some(name) = Self::category_from_input(Some(parent), &name) {
                        self.finish_add_category(name);
                    } else {
                        self.mode = Mode::Normal;
                    }
                } else {
                    self.mode = Mode::Normal;
                }
            }
            Action::CreateAndAssignCategory(name) => {
                let items = self.items();
                let id = items.get(self.selected_index()).map(|i| i.id);
                let Some(name) = normalize_category(&name) else {
                    self.return_after_secondary_mode();
                    return false;
                };
                if let Some(id) = id {
                    self.data.set_category(id, Some(name.clone()));
                    self.mark_dirty();
                }
                self.category_filter = Some(name.clone());
                let cats = self.data.category_entries();
                if let Some(pos) = cats.iter().position(|c| c.path == name) {
                    self.category_index = pos + 1;
                } else {
                    self.category_index = cats.len() + 1;
                }
                self.return_after_secondary_mode();
            }
            Action::SortSelect(idx) => {
                if let Some(mode) = sort_by_index(idx) {
                    self.sort_mode = mode;
                    self.clamp_selection();
                }
                self.mode = Mode::Normal;
            }
            Action::CancelSortPicker => {
                if let Mode::SortPicker { original_sort, .. } = &self.mode {
                    self.sort_mode = *original_sort;
                    self.clamp_selection();
                }
                self.mode = Mode::Normal;
            }
            Action::StartCategoryAddWithChar(c) => {
                self.input.clear();
                self.input.insert_char(c);
                self.mode = Mode::CategoryAdd { parent: None };
            }
            Action::CancelCategoryAdd => {
                self.input.clear();
                self.mode = Mode::Normal;
            }
            Action::Undo => {
                if let Some(entry) = self.undo_stack.pop() {
                    match entry {
                        UndoEntry::Delete(items) => {
                            for item in items {
                                self.data.restore_item(item);
                            }
                        }
                        UndoEntry::Archive {
                            item_ids,
                            category_names,
                            previous_archived,
                        } => {
                            for id in item_ids {
                                self.data.set_archived(id, previous_archived);
                            }
                            for name in category_names {
                                self.data.set_category_archived(&name, previous_archived);
                            }
                        }
                        UndoEntry::NoteDelete {
                            item_id,
                            index,
                            note,
                        } => {
                            self.data.restore_note(item_id, index, note);
                            if let Mode::Notes {
                                item_id: open_id,
                                selected,
                            } = &mut self.mode
                            {
                                if *open_id == item_id {
                                    *selected = index;
                                }
                            }
                        }
                    }
                    self.mark_dirty();
                    self.clamp_selection();
                }
            }
            Action::BulkActionSelect(idx) => {
                let single = match &self.mode {
                    Mode::BulkActionPicker { ids, category_names, .. } => {
                        ids.len() == 1 && category_names.is_empty()
                    }
                    _ => return false,
                };
                let labels = bulk_action_labels(single);
                let label = match labels.get(idx).copied() {
                    Some(label) => label,
                    None => return false,
                };
                let (ids, category_names) = match std::mem::replace(&mut self.mode, Mode::Normal) {
                    Mode::BulkActionPicker { ids, category_names, .. } => (ids, category_names),
                    _ => return false,
                };

                match label {
                    "Delete" => {
                        self.mode = self.build_confirm_delete_mode(ids, category_names);
                    }
                    "Archive" => {
                        for id in &ids {
                            self.data.set_archived(*id, true);
                        }
                        for name in &category_names {
                            self.data.set_category_archived(name, true);
                        }
                        self.push_undo_archive(ids, category_names, false);
                        self.mark_dirty();
                        self.return_after_secondary_mode();
                    }
                    "Toggle done" => {
                        for id in &ids {
                            self.data.toggle_done(*id);
                        }
                        self.mark_dirty();
                        self.return_after_secondary_mode();
                    }
                    "Assign category" => {
                        if ids.is_empty() {
                            return false;
                        }
                        self.input.clear();
                        self.mode = Mode::CategoryPicker {
                            selected: 0,
                            target: CategoryPickerTarget::AssignBulk(ids),
                        };
                    }
                    "Edit" => {
                        if !single {
                            return false;
                        }
                        let id = ids[0];
                        if let Some(item) = self.data.get(id) {
                            self.input.set_text(&item.text);
                            self.mode = Mode::Editing { edit_id: Some(id) };
                        }
                    }
                    "Notes" => {
                        if !single {
                            return false;
                        }
                        self.open_notes(ids[0]);
                    }
                    _ => {}
                }
            }
            Action::CancelBulkActionPicker => {
                self.return_after_secondary_mode();
            }
            Action::OpenNotes(id) => {
                self.open_notes(id);
            }
            Action::CloseNotes => {
                self.input.clear();
                self.return_after_secondary_mode();
            }
            Action::StartNoteAppend => {
                let Mode::Notes { item_id, .. } = &self.mode else {
                    return false;
                };
                let item_id = *item_id;
                self.input.clear();
                self.mode = Mode::NoteInput {
                    item_id,
                    edit_index: None,
                };
            }
            Action::StartNoteAppendWithChar(c) => {
                let Mode::Notes { item_id, .. } = &self.mode else {
                    return false;
                };
                let item_id = *item_id;
                self.input.clear();
                self.input.insert_char(c);
                self.mode = Mode::NoteInput {
                    item_id,
                    edit_index: None,
                };
            }
            Action::StartNoteEdit => {
                let Mode::Notes { item_id, selected } = &self.mode else {
                    return false;
                };
                let (item_id, selected) = (*item_id, *selected);
                let Some(text) = self
                    .data
                    .get(item_id)
                    .and_then(|item| item.notes.get(selected))
                    .map(|note| note.text.clone())
                else {
                    return false;
                };
                self.input.set_text(&text);
                self.mode = Mode::NoteInput {
                    item_id,
                    edit_index: Some(selected),
                };
            }
            Action::SubmitNote => {
                let Mode::NoteInput {
                    item_id,
                    edit_index,
                } = &self.mode
                else {
                    return false;
                };
                let (item_id, edit_index) = (*item_id, *edit_index);
                let text = self.input.text().trim().to_string();

                let selected = match edit_index {
                    Some(index) => {
                        if self.data.update_note(item_id, index, &text) {
                            self.mark_dirty();
                        }
                        index
                    }
                    None => {
                        if self.data.add_note(item_id, &text) {
                            self.mark_dirty();
                        }
                        self.data.last_note_index(item_id)
                    }
                };

                self.input.clear();
                self.mode = Mode::Notes { item_id, selected };
            }
            Action::CancelNote => {
                let Mode::NoteInput {
                    item_id,
                    edit_index,
                } = &self.mode
                else {
                    return false;
                };
                let (item_id, edit_index) = (*item_id, *edit_index);
                let selected = edit_index.unwrap_or_else(|| self.data.last_note_index(item_id));
                self.input.clear();
                self.mode = Mode::Notes { item_id, selected };
            }
            Action::DeleteNote => {
                let Mode::Notes { item_id, selected } = &self.mode else {
                    return false;
                };
                let (item_id, selected) = (*item_id, *selected);
                if let Some(note) = self.data.delete_note(item_id, selected) {
                    self.push_undo(UndoEntry::NoteDelete {
                        item_id,
                        index: selected,
                        note,
                    });
                    self.mark_dirty();
                    let next = selected.min(self.data.last_note_index(item_id));
                    self.mode = Mode::Notes {
                        item_id,
                        selected: next,
                    };
                }
            }
            Action::YankNote => {
                let Mode::Notes { item_id, selected } = &self.mode else {
                    return false;
                };
                let text = self
                    .data
                    .get(*item_id)
                    .and_then(|item| item.notes.get(*selected))
                    .map(|note| note.text.clone());
                if let Some(text) = text {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        let _ = cb.set_text(text);
                    }
                }
            }
            Action::EnterResizeSidebar => {
                let return_pane = self.pane;
                let original_width = self.sidebar_width;
                self.pane = Pane::Categories;
                self.mode = Mode::ResizeSidebar {
                    return_pane,
                    original_width,
                };
            }
            Action::ResizeSidebar(delta) => {
                let current = self.sidebar_width as i32;
                let next = (current + delta as i32)
                    .clamp(SIDEBAR_WIDTH_MIN as i32, SIDEBAR_WIDTH_MAX as i32)
                    as u16;
                self.sidebar_width = next;
            }
            Action::ResetSidebarWidth => {
                self.sidebar_width = SIDEBAR_WIDTH_DEFAULT;
            }
            Action::ConfirmResizeSidebar => {
                if let Mode::ResizeSidebar { return_pane, original_width } = &self.mode {
                    self.pane = *return_pane;
                    let changed = *original_width != self.sidebar_width;
                    self.mode = Mode::Normal;
                    if changed {
                        config::save_sidebar_width(self.sidebar_width);
                    }
                } else {
                    self.mode = Mode::Normal;
                }
            }
            Action::CancelResizeSidebar => {
                if let Mode::ResizeSidebar { return_pane, original_width } = &self.mode {
                    self.pane = *return_pane;
                    self.sidebar_width = *original_width;
                }
                self.mode = Mode::Normal;
            }
            Action::Quit => return true,
        }

        false
    }
}

fn theme_index_by_name(name: &str) -> usize {
    Theme::theme_names()
        .iter()
        .position(|theme_name| *theme_name == name)
        .unwrap_or(0)
}

fn theme_by_index(idx: usize) -> Option<Theme> {
    Theme::theme_names()
        .get(idx)
        .and_then(|name| Theme::by_name(name))
}

fn sort_by_index(idx: usize) -> Option<SortMode> {
    const SORTS: [SortMode; 3] = [SortMode::Default, SortMode::DueDate, SortMode::Priority];
    SORTS.get(idx).copied()
}

fn sort_index(mode: SortMode) -> usize {
    match mode {
        SortMode::Default => 0,
        SortMode::DueDate => 1,
        SortMode::Priority => 2,
    }
}

fn is_popup_list_mode(mode: &Mode) -> bool {
    matches!(
        mode,
        Mode::ThemePicker { .. }
            | Mode::PriorityPicker { .. }
            | Mode::CategoryPicker { .. }
            | Mode::CategoryFilterPicker { .. }
            | Mode::CategoryCreateChoice { .. }
            | Mode::CategoryParentPicker { .. }
            | Mode::SortPicker { .. }
            | Mode::ArchivePicker { .. }
            | Mode::BulkActionPicker { .. }
            | Mode::FilterPicker { .. }
            | Mode::DueDateFilterPicker { .. }
    )
}

pub(crate) fn bulk_action_labels(single_item: bool) -> Vec<&'static str> {
    let mut labels = vec!["Archive", "Assign category", "Delete", "Toggle done"];
    if single_item {
        labels.push("Edit");
        labels.push("Notes");
    }
    labels.sort_unstable();
    labels
}

pub(crate) fn archive_picker_labels() -> &'static [&'static str] {
    &[
        "Archive one",
        "Archive bulk",
        "Archive done",
        "Archive all",
        "Restore one",
        "Restore bulk",
        "Restore all",
        "Archived view",
    ]
}

#[cfg(test)]
mod tests;
