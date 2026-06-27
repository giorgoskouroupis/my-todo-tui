use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};

use crate::clip::Clipboard;
use crate::config;
use crate::data::{
    category_matches, normalize_category, CategoryEntry, Priority, TodoData, TodoItem,
};
use crate::date::Date;
use crate::keys;
use crate::ui::input::InputBuffer;
use crate::ui::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Default,
    Priority,
    DueDate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DueFilter {
    Today,
    Week,
    Overdue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Items,
    Categories,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiSelectCmd {
    Delete,
    ToggleDone,
    Archive,
    RestoreArchive,
}

pub enum Mode {
    Normal,
    Editing {
        edit_id: Option<u64>,
    },
    Command {
        selected: usize,
    },
    Searching,
    MultiSelect {
        cmd: MultiSelectCmd,
        selected: HashSet<u64>,
        selected_categories: HashSet<String>,
    },
    ThemePicker {
        selected: usize,
        original_theme: Theme,
    },
    PriorityPicker {
        selected: usize,
    },
    Help,
    Keybindings,
    ConfirmDelete {
        ids: Vec<u64>,
        texts: Vec<String>,
        category_names: Vec<String>,
    },
    CategoryPicker {
        selected: usize,
        target: CategoryPickerTarget,
    },
    CategoryFilterPicker {
        selected: usize,
    },
    CategoryCreateChoice {
        selected: usize,
        parent: Option<String>,
        name: String,
    },
    CategoryParentPicker {
        selected: usize,
        name: String,
    },
    CategoryAdd {
        parent: Option<String>,
    },
    SortPicker {
        selected: usize,
        original_sort: SortMode,
    },
    ArchivePicker {
        selected: usize,
    },
    RenameInput {
        target: RenameTarget,
    },
    FilterPicker {
        selected: usize,
    },
    DueDateFilterPicker {
        selected: usize,
    },
    DueDateCalendar {
        edit_id: Option<u64>,
        saved_text: String,
        from_new: bool,
        from_normal: bool,
        selected: Date,
        prompt_focused: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenameTarget {
    Item(u64),
    Category(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CategoryPickerTarget {
    AssignItem,
    MoveCategory(String),
}

pub enum Action {
    SelectPrev,
    SelectNext,
    StartNewItem,
    StartNewItemWithChar(char),
    EditItem(u64),
    SubmitEdit,
    CancelEdit,
    ToggleDone(u64),
    ToggleDoing(u64),
    DeleteItem(u64),
    CyclePriority(u64, bool),
    Reorder(u64, i32),
    ApplySearch,
    ClearSearch,
    StartCommand,
    ExecuteCommand(String),
    TabComplete,
    ToggleMultiSelect(u64),
    ToggleCategoryMultiSelect(String),
    SelectAllMultiSelect,
    ConfirmMultiSelect,
    CancelMultiSelect,
    ThemeSelect(usize),
    CancelThemePicker,
    ConfirmDeleteYes,
    ConfirmDeleteNo,
    ConfirmDeleteArchive,
    PrioritySelect(usize),
    CancelPriorityPicker,
    Copy,
    Paste,
    UndoDelete,
    TogglePin(u64),
    SetDueDate,
    SwitchPane,
    OpenCategoryPicker,
    CategorySelect(usize),
    CategoryFilterSelect(usize),
    CategoryFilterParent(usize),
    CategoryParent,
    AddCategory(String),
    AddCategoryChoice(usize),
    SelectCategoryParent(usize),
    CreateAndAssignCategory(String),
    DeleteCategory(String),
    CancelCategoryPicker,
    StartCategoryAddWithChar(char),
    CancelCategoryAdd,
    StartSortPicker,
    SortSelect(usize),
    CancelSortPicker,
    SubmitRename,
    CancelArchivePicker,
    ArchiveSelect(usize),
    CancelFilterPicker,
    FilterSelect(usize),
    CancelDueDateFilterPicker,
    DueDateFilterSelect(usize),
    PopupBack,
    SubmitDueDate,
    CancelDueDate,
    CalendarMove(i32),
    CalendarMonth(i32),
    CalendarToday,
    CalendarClear,
    CalendarTextChanged,
    CalendarToggleFocus,
    SwitchCategoryFilter(i32, bool),
    Quit,
}

enum PopupBackTarget {
    Command { input: String, selected: usize },
    FilterPicker { selected: usize },
}

pub struct App {
    pub data: TodoData,
    pub selected_index: usize,
    pub input: InputBuffer,
    pub mode: Mode,
    pub clip: Clipboard,
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
    category_selection_memory: HashMap<String, usize>,
    popup_back_stack: Vec<PopupBackTarget>,
}

impl App {
    pub fn new() -> Self {
        let theme = config::load_theme();
        let data = TodoData::load();
        Self {
            data,
            selected_index: 0,
            input: InputBuffer::new(),
            mode: Mode::Normal,
            clip: Clipboard::new(),
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
            category_selection_memory: HashMap::new(),
            popup_back_stack: Vec::new(),
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
            self.input.clear();
            self.mode = Mode::Normal;
            self.clamp_selection();
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

    fn parent_category_index(&self) -> usize {
        self.parent_category_index_for(self.category_index)
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

        if let Some(category) = self.selected_category_path() {
            self.data.set_category_archived(&category, archived);
        } else {
            let ids: Vec<u64> = self.items().iter().map(|item| item.id).collect();
            let categories = self.categories();
            for id in ids {
                self.data.set_archived(id, archived);
            }
            for category in categories {
                self.data.set_category_archived(&category, archived);
            }
        }
        self.mark_dirty();
        self.category_index = self.category_index.min(self.category_entries().len());
        self.clamp_selection();
    }

    fn set_visible_archived(&mut self, archived: bool) {
        let ids: Vec<u64> = self.items().iter().map(|item| item.id).collect();
        let categories = if self.pane == Pane::Categories {
            self.categories()
        } else {
            Vec::new()
        };

        for id in ids {
            self.data.set_archived(id, archived);
        }
        for category in categories {
            self.data.set_category_archived(&category, archived);
        }
        self.mark_dirty();
        self.category_index = self.category_index.min(self.category_entries().len());
        self.clamp_selection();
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
        if is_keybindings_shortcut(key) {
            return Some(Action::ExecuteCommand("keybindings".to_string()));
        }
        if key.code == KeyCode::Char('/')
            && !matches!(self.mode, Mode::Command { .. } | Mode::CategoryAdd { .. })
        {
            return Some(Action::StartCommand);
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(Action::Quit);
        }
        if key.code == KeyCode::Char('d')
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(&self.mode, Mode::Normal)
            && self.pane == Pane::Items
            && self.selected_index() < self.items().len()
        {
            return Some(Action::SetDueDate);
        }

        let is_pane_mode = matches!(self.mode, Mode::Normal | Mode::MultiSelect { .. });
        if is_pane_mode {
            match key.code {
                KeyCode::Left if self.pane == Pane::Items => return Some(Action::SwitchPane),
                KeyCode::Right if self.pane == Pane::Categories => return Some(Action::SwitchPane),
                KeyCode::Backspace if matches!(self.mode, Mode::Normal) => {
                    return Some(Action::CategoryParent);
                }
                KeyCode::PageUp => {
                    return Some(Action::SwitchCategoryFilter(
                        -1,
                        key.modifiers.contains(KeyModifiers::CONTROL),
                    ));
                }
                KeyCode::PageDown => {
                    return Some(Action::SwitchCategoryFilter(
                        1,
                        key.modifiers.contains(KeyModifiers::CONTROL),
                    ));
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
            Mode::Searching => keys::handle_search(key, &mut self.filter),
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
            Mode::Help | Mode::Keybindings => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => Some(Action::CancelEdit),
                _ => None,
            },
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
            Mode::FilterPicker { selected } => Some(Action::FilterSelect(*selected)),
            Mode::DueDateFilterPicker { selected } => Some(Action::DueDateFilterSelect(*selected)),
            _ => None,
        }
    }

    fn handle_action(&mut self, action: Action) -> bool {
        match action {
            Action::SelectPrev => {
                if let Mode::Command { ref mut selected } = &mut self.mode {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                } else if let Mode::ThemePicker {
                    ref mut selected, ..
                } = &mut self.mode
                {
                    if *selected > 0 {
                        *selected -= 1;
                        if let Some(theme) = theme_by_index(*selected) {
                            self.theme = theme;
                        }
                    }
                } else if let Mode::PriorityPicker { ref mut selected } = &mut self.mode {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                } else if let Mode::SortPicker {
                    ref mut selected, ..
                } = &mut self.mode
                {
                    if *selected > 0 {
                        *selected -= 1;
                        if let Some(mode) = sort_by_index(*selected) {
                            self.sort_mode = mode;
                        }
                    }
                } else if let Mode::ArchivePicker { ref mut selected } = &mut self.mode {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                } else if let Mode::FilterPicker { ref mut selected } = &mut self.mode {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                } else if let Mode::DueDateFilterPicker { ref mut selected } = &mut self.mode {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                } else if let Mode::CategoryPicker {
                    ref mut selected, ..
                } = &mut self.mode
                {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                } else if let Mode::CategoryFilterPicker { ref mut selected } = &mut self.mode {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                } else if let Mode::CategoryCreateChoice {
                    ref mut selected, ..
                } = &mut self.mode
                {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                } else if let Mode::CategoryParentPicker {
                    ref mut selected, ..
                } = &mut self.mode
                {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                } else if self.pane == Pane::Categories {
                    if self.category_index > 0 {
                        self.set_category_index(self.category_index - 1);
                    }
                } else if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            Action::SelectNext => {
                let visible_category_count = self.category_entries().len();
                let picker_category_count = self.categories().len();
                if let Mode::Command { ref mut selected } = &mut self.mode {
                    let max = get_filtered_commands(self.input.text())
                        .len()
                        .saturating_sub(1);
                    if *selected < max {
                        *selected += 1;
                    }
                } else if let Mode::ThemePicker {
                    ref mut selected, ..
                } = &mut self.mode
                {
                    let max = Theme::theme_names().len().saturating_sub(1);
                    if *selected < max {
                        *selected += 1;
                        if let Some(theme) = theme_by_index(*selected) {
                            self.theme = theme;
                        }
                    }
                } else if let Mode::PriorityPicker { ref mut selected } = &mut self.mode {
                    let max = 4usize;
                    if *selected < max {
                        *selected += 1;
                    }
                } else if let Mode::SortPicker {
                    ref mut selected, ..
                } = &mut self.mode
                {
                    let max = 2usize;
                    if *selected < max {
                        *selected += 1;
                        if let Some(mode) = sort_by_index(*selected) {
                            self.sort_mode = mode;
                        }
                    }
                } else if let Mode::ArchivePicker { ref mut selected } = &mut self.mode {
                    let max = 7usize;
                    if *selected < max {
                        *selected += 1;
                    }
                } else if let Mode::FilterPicker { ref mut selected } = &mut self.mode {
                    let max = 4usize;
                    if *selected < max {
                        *selected += 1;
                    }
                } else if let Mode::DueDateFilterPicker { ref mut selected } = &mut self.mode {
                    let max = 3usize;
                    if *selected < max {
                        *selected += 1;
                    }
                } else if let Mode::CategoryPicker {
                    ref mut selected, ..
                } = &mut self.mode
                {
                    if *selected < picker_category_count {
                        *selected += 1;
                    }
                } else if let Mode::CategoryFilterPicker { ref mut selected } = &mut self.mode {
                    let max = visible_category_count;
                    if *selected < max {
                        *selected += 1;
                    }
                } else if let Mode::CategoryCreateChoice {
                    ref mut selected, ..
                } = &mut self.mode
                {
                    if *selected < 1 {
                        *selected += 1;
                    }
                } else if matches!(self.mode, Mode::CategoryParentPicker { .. }) {
                    let max = self.root_categories().len().saturating_sub(1);
                    if let Mode::CategoryParentPicker {
                        ref mut selected, ..
                    } = &mut self.mode
                    {
                        if *selected < max {
                            *selected += 1;
                        }
                    }
                } else if self.pane == Pane::Categories {
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
                    self.input.clear();
                    self.mode = Mode::Normal;
                    self.clamp_selection();
                }
            }
            Action::CancelEdit => {
                self.input.clear();
                if let Mode::Editing { edit_id: None } = &self.mode {
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
                self.mode = Mode::Normal;
            }
            Action::ToggleDone(id) => {
                self.data.toggle_done(id);
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
            Action::ApplySearch => {
                self.mode = Mode::Normal;
                self.clamp_selection();
            }
            Action::ClearSearch => {
                self.filter.clear();
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
                        if let Some(query) = arg {
                            self.filter = query;
                            self.mode = Mode::Searching;
                        } else {
                            self.filter.clear();
                            self.mode = Mode::Searching;
                        }
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
                                    if let Some(id) =
                                        self.items().get(self.selected_index()).map(|item| item.id)
                                    {
                                        self.data.set_archived(id, true);
                                        self.mark_dirty();
                                    }
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
                                let ids: Vec<u64> = self
                                    .data
                                    .items()
                                    .iter()
                                    .filter(|item| item.done && !item.archived)
                                    .map(|item| item.id)
                                    .collect();
                                for id in ids {
                                    self.data.set_archived(id, true);
                                }
                                self.mark_dirty();
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
                                    if let Some(id) =
                                        self.items().get(self.selected_index()).map(|item| item.id)
                                    {
                                        self.data.set_archived(id, false);
                                    }
                                }
                                self.mark_dirty();
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
                                    if let Some(id) =
                                        self.items().get(self.selected_index()).map(|item| item.id)
                                    {
                                        self.data.set_archived(id, false);
                                        self.mark_dirty();
                                    }
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
                if !matches!(self.mode, Mode::RenameInput { .. }) {
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

                    let mut texts: Vec<String> = ids
                        .iter()
                        .filter_map(|id| self.data.get(*id).map(|item| item.text.clone()))
                        .collect();
                    texts.extend(
                        category_names
                            .iter()
                            .map(|category| format!("[category] {category}")),
                    );
                    if !ids.is_empty() || !category_names.is_empty() {
                        self.mode = Mode::ConfirmDelete {
                            ids,
                            texts,
                            category_names,
                        };
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
                        MultiSelectCmd::Delete => {
                            for id in &selected {
                                if let Some(item) = self.data.delete(*id) {
                                    self.clip.cut(item);
                                }
                            }
                            for category in &selected_categories {
                                let ids: Vec<u64> = self
                                    .data
                                    .items()
                                    .iter()
                                    .filter(|i| category_matches(i.category.as_deref(), category))
                                    .map(|i| i.id)
                                    .collect();
                                for id in ids {
                                    if let Some(item) = self.data.delete(id) {
                                        self.clip.cut(item);
                                    }
                                }
                                self.data.remove_category(category);
                            }
                            self.mark_dirty();
                        }
                        MultiSelectCmd::ToggleDone => {
                            for id in &selected {
                                self.data.toggle_done(*id);
                            }
                            self.mark_dirty();
                        }
                        MultiSelectCmd::Archive => {
                            for id in &selected {
                                self.data.set_archived(*id, true);
                            }
                            for category in &selected_categories {
                                self.data.set_category_archived(category, true);
                            }
                            self.mark_dirty();
                        }
                        MultiSelectCmd::RestoreArchive => {
                            for id in &selected {
                                self.data.set_archived(*id, false);
                            }
                            for category in &selected_categories {
                                self.data.set_category_archived(category, false);
                            }
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
                    for id in ids {
                        if let Some(item) = self.data.delete(id) {
                            self.clip.cut(item);
                        }
                    }
                    for name in category_names {
                        self.data.remove_category(&name);
                    }
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
                self.clamp_selection();
            }
            Action::ConfirmDeleteNo => {
                self.mode = Mode::Normal;
            }
            Action::ConfirmDeleteArchive => {
                let mode = std::mem::replace(&mut self.mode, Mode::Normal);
                if let Mode::ConfirmDelete {
                    ids,
                    category_names,
                    ..
                } = mode
                {
                    for id in ids {
                        self.data.set_archived(id, true);
                    }
                    for name in category_names {
                        self.data.set_category_archived(&name, true);
                    }
                    self.mark_dirty();
                }
                self.clamp_selection();
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
                match idx {
                    0 => {
                        self.mode = Mode::MultiSelect {
                            cmd: MultiSelectCmd::Archive,
                            selected: HashSet::new(),
                            selected_categories: HashSet::new(),
                        };
                        return false;
                    }
                    1 => {
                        let ids: Vec<u64> = self
                            .data
                            .items()
                            .iter()
                            .filter(|item| item.done && !item.archived)
                            .map(|item| item.id)
                            .collect();
                        for id in ids {
                            self.data.set_archived(id, true);
                        }
                        self.mark_dirty();
                    }
                    2 => {
                        if self.pane == Pane::Categories {
                            self.set_current_category_archived(true);
                        } else {
                            if let Some(id) =
                                self.items().get(self.selected_index()).map(|item| item.id)
                            {
                                self.data.set_archived(id, true);
                                self.mark_dirty();
                            }
                        }
                    }
                    3 => {
                        self.set_visible_archived(true);
                    }
                    4 => {
                        self.show_archived = !self.show_archived;
                    }
                    5 => {
                        self.show_archived = true;
                        self.mode = Mode::MultiSelect {
                            cmd: MultiSelectCmd::RestoreArchive,
                            selected: HashSet::new(),
                            selected_categories: HashSet::new(),
                        };
                        return false;
                    }
                    6 => {
                        if self.pane == Pane::Categories {
                            self.set_current_category_archived(false);
                        } else {
                            if let Some(id) =
                                self.items().get(self.selected_index()).map(|item| item.id)
                            {
                                self.data.set_archived(id, false);
                                self.mark_dirty();
                            }
                        }
                    }
                    7 => {
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
                    other => other,
                };
            }
            Action::Copy => {
                if let Some(text) = self.input.selected_text() {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        let _ = cb.set_text(text);
                    }
                }
            }
            Action::Paste => {
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    if let Ok(text) = cb.get_text() {
                        self.input.insert_str(&text);
                    }
                }
            }
            Action::UndoDelete => {
                if let Some(item) = self.clip.paste() {
                    self.data.restore(item);
                    self.mark_dirty();
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
                let today = Date::today();
                if let Mode::DueDateCalendar { selected, .. } = &mut self.mode {
                    *selected = today;
                }
                self.input.set_text(&today.iso());
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
                            let destination = if idx == 0 {
                                None
                            } else {
                                cats.get(idx - 1).map(String::as_str)
                            };
                            self.move_category_to(&old_category, destination);
                        }
                    }
                    self.mode = Mode::Normal;
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
            Action::CategoryParent => {
                let parent_idx = self.parent_category_index();
                if parent_idx != self.category_index {
                    self.set_category_index(parent_idx);
                    self.pane = Pane::Items;
                }
                self.mode = Mode::Normal;
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
                let ids: Vec<u64> = self
                    .data
                    .items()
                    .iter()
                    .filter(|i| category_matches(i.category.as_deref(), &name))
                    .map(|i| i.id)
                    .collect();
                let texts: Vec<String> = self
                    .data
                    .items()
                    .iter()
                    .filter(|i| category_matches(i.category.as_deref(), &name))
                    .map(|i| i.text.clone())
                    .collect();
                self.mode = Mode::ConfirmDelete {
                    ids,
                    texts,
                    category_names: vec![name],
                };
            }
            Action::CancelCategoryPicker => {
                self.mode = Mode::Normal;
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
                    self.input.clear();
                    self.mode = Mode::Normal;
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
                self.input.clear();
                self.mode = Mode::Normal;
            }
            Action::StartSortPicker => {
                self.mode = Mode::SortPicker {
                    selected: sort_index(self.sort_mode),
                    original_sort: self.sort_mode,
                };
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
            Action::Quit => return true,
        }

        false
    }
}

pub const COMMANDS: &[(&str, &str)] = &[
    ("archive", "Archive items/categories (done, all, restore)"),
    ("archived", "Toggle archived view"),
    ("clear", "Clear completed items"),
    ("c", "Alias for clear"),
    ("delete", "Bulk delete items"),
    ("done", "Bulk toggle done"),
    ("due", "Filter due dates"),
    ("d", "Alias for delete"),
    ("filter", "Filter items (due, priority, category, archived)"),
    ("help", "Show help"),
    ("keybindings", "Show keybindings"),
    ("k", "Alias for keybindings"),
    ("categories", "Filter by category"),
    ("cat", "Alias for categories"),
    ("move", "Move selected item/category"),
    ("m", "Alias for move"),
    ("priorities", "Filter by priority"),
    ("p", "Alias for priorities"),
    ("rename", "Rename category path"),
    ("reset", "Clear all filters and restore default view"),
    ("search", "Filter items by text"),
    ("s", "Alias for search"),
    ("sort", "Sort items (priority/due/default)"),
    ("themes", "List available themes"),
    ("unarchive", "Restore archived item/all visible"),
    ("x", "Alias for done"),
];

const VISIBLE_COMMANDS: &[(&str, &str)] = &[
    ("archive", "Archive or restore items/categories"),
    ("archive bulk", "Bulk-select items/categories to archive"),
    ("archive done", "Archive completed items"),
    ("archive one", "Archive selected item/category"),
    ("archive all", "Archive all visible items/categories"),
    ("archive archived", "Toggle archived view"),
    (
        "archive restore bulk",
        "Bulk-select archived items/categories to restore",
    ),
    ("archive restore", "Restore selected archived item/category"),
    (
        "archive restore all",
        "Restore all visible archived items/categories",
    ),
    ("clear", "Clear completed items"),
    ("delete", "Bulk delete items/categories"),
    ("done", "Bulk toggle done"),
    ("filter", "Filter items"),
    ("filter archived", "Toggle archived view"),
    ("filter category", "Open category filter picker"),
    ("filter due", "Open due-date filter picker"),
    ("filter priority", "Open priority filter picker"),
    ("filter clear", "Clear all filters"),
    ("help", "Show help"),
    ("keybindings", "Show keybindings"),
    ("move", "Move selected item/category"),
    ("rename", "Rename category path"),
    ("reset", "Clear all filters"),
    ("search", "Filter items by text"),
    ("sort", "Sort items"),
    ("sort default", "Restore default sort"),
    ("sort due", "Sort by due date"),
    ("sort priority", "Sort by priority"),
    ("themes", "List available themes"),
];

pub fn get_filtered_commands(prefix: &str) -> Vec<(&'static str, &'static str)> {
    let lower = prefix.trim().to_lowercase();
    let subcommand_context = contains_whitespace(prefix);

    VISIBLE_COMMANDS
        .iter()
        .filter(|(name, _)| {
            let is_subcommand = contains_whitespace(name);
            if lower.is_empty() {
                !is_subcommand
            } else if subcommand_context {
                is_subcommand && name.starts_with(&lower)
            } else {
                !is_subcommand && name.starts_with(&lower)
            }
        })
        .copied()
        .collect()
}

fn get_completions(prefix: &str) -> Vec<String> {
    get_filtered_commands(prefix)
        .into_iter()
        .map(|(name, _)| {
            if !contains_whitespace(name) && has_visible_subcommands(name) {
                format!("{name} ")
            } else {
                name.to_string()
            }
        })
        .collect()
}

fn resolve_command_input(input: &str, selected: usize) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        let matches = get_filtered_commands(input);
        if matches.is_empty() {
            return None;
        }
        let idx = selected.min(matches.len().saturating_sub(1));
        return Some(matches[idx].0.to_string());
    }

    let lower = trimmed.to_lowercase();
    let has_trailing_space = ends_with_whitespace(input);
    let first = lower.split_whitespace().next().unwrap_or_default();
    let has_args = lower.split_whitespace().nth(1).is_some();
    let exact_visible = VISIBLE_COMMANDS.iter().any(|(name, _)| *name == lower);

    if !has_trailing_space && (exact_visible || (has_args && is_known_command(first))) {
        return Some(trimmed.to_string());
    }

    let matches = get_filtered_commands(input);
    if !matches.is_empty() {
        let idx = selected.min(matches.len().saturating_sub(1));
        return Some(matches[idx].0.to_string());
    }

    if is_known_command(first) {
        return Some(trimmed.to_string());
    }

    None
}

fn is_known_command(name: &str) -> bool {
    COMMANDS
        .iter()
        .any(|(command, _)| command.eq_ignore_ascii_case(name))
}

fn has_visible_subcommands(command: &str) -> bool {
    let prefix = format!("{command} ");
    VISIBLE_COMMANDS
        .iter()
        .any(|(name, _)| name.starts_with(&prefix))
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

fn contains_whitespace(value: &str) -> bool {
    value.chars().any(char::is_whitespace)
}

fn ends_with_whitespace(value: &str) -> bool {
    value.chars().last().is_some_and(char::is_whitespace)
}

fn is_keybindings_shortcut(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('/') | KeyCode::Char('_'))
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
            | Mode::FilterPicker { .. }
            | Mode::DueDateFilterPicker { .. }
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Instant;

    use super::{
        get_completions, get_filtered_commands, resolve_command_input, theme_index_by_name, Action,
        App, CategoryPickerTarget, Mode, MultiSelectCmd, Pane, RenameTarget, SortMode,
    };
    use crate::clip::Clipboard;
    use crate::data::TodoData;
    use crate::ui::input::InputBuffer;
    use crate::ui::theme::Theme;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn test_app(data: TodoData) -> App {
        App {
            data,
            selected_index: 0,
            input: InputBuffer::new(),
            mode: Mode::Normal,
            clip: Clipboard::new(),
            filter: String::new(),
            priority_filter: None,
            due_filter: None,
            category_filter: None,
            show_archived: false,
            pane: Pane::Items,
            category_index: 0,
            sort_mode: SortMode::Default,
            theme: Theme::one_dark(),
            completions: Vec::new(),
            completion_index: 0,
            dirty: false,
            last_mutated: Instant::now(),
            pending_due_date: None,
            category_selection_memory: HashMap::new(),
            popup_back_stack: Vec::new(),
        }
    }

    #[test]
    fn category_from_input_attaches_child_to_parent() {
        assert_eq!(
            App::category_from_input(Some("Work"), "work2"),
            Some("Work/work2".to_string())
        );
    }

    #[test]
    fn category_from_input_keeps_manual_path() {
        assert_eq!(
            App::category_from_input(Some("Work"), "Personal/p1"),
            Some("Personal/p1".to_string())
        );
    }

    #[test]
    fn ctrl_a_delete_from_category_pane_removes_all_items_and_categories() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        data.set_category(id, Some("Work/work1".to_string()));
        data.add_category("Personal");
        let mut app = test_app(data);
        app.pane = Pane::Categories;
        app.mode = Mode::MultiSelect {
            cmd: MultiSelectCmd::Delete,
            selected: Default::default(),
            selected_categories: Default::default(),
        };

        app.handle_action(Action::SelectAllMultiSelect);
        assert!(matches!(app.mode, Mode::ConfirmDelete { .. }));
        app.handle_action(Action::ConfirmDeleteYes);

        assert!(app.data.items().is_empty());
        assert!(app.data.categories().is_empty());
    }

    #[test]
    fn ctrl_a_delete_from_items_pane_keeps_category_names() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        data.set_category(id, Some("Work/work1".to_string()));
        data.add_category("Work/work1");
        let mut app = test_app(data);
        app.pane = Pane::Items;
        app.category_filter = Some("Work".to_string());
        app.mode = Mode::MultiSelect {
            cmd: MultiSelectCmd::Delete,
            selected: Default::default(),
            selected_categories: Default::default(),
        };

        app.handle_action(Action::SelectAllMultiSelect);
        assert!(matches!(app.mode, Mode::ConfirmDelete { .. }));
        app.handle_action(Action::ConfirmDeleteYes);

        assert!(app.data.items().is_empty());
        assert!(app.data.categories().contains(&"Work/work1".to_string()));
    }

    #[test]
    fn confirm_delete_ignores_unmapped_keys() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        let mut app = test_app(data);
        app.mode = Mode::ConfirmDelete {
            ids: vec![id],
            texts: vec!["ship".to_string()],
            category_names: Vec::new(),
        };

        let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(action.is_none());
        assert!(matches!(app.mode, Mode::ConfirmDelete { .. }));
        assert!(app.data.get(id).is_some());
    }

    #[test]
    fn confirm_delete_n_cancels() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        let mut app = test_app(data);
        app.mode = Mode::ConfirmDelete {
            ids: vec![id],
            texts: vec!["ship".to_string()],
            category_names: Vec::new(),
        };

        if let Some(action) =
            app.dispatch_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE))
        {
            app.handle_action(action);
        }

        assert!(matches!(app.mode, Mode::Normal));
        assert!(app.data.get(id).is_some());
    }

    #[test]
    fn confirm_delete_esc_cancels() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        let mut app = test_app(data);
        app.mode = Mode::ConfirmDelete {
            ids: vec![id],
            texts: vec!["ship".to_string()],
            category_names: Vec::new(),
        };

        if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)) {
            app.handle_action(action);
        }

        assert!(matches!(app.mode, Mode::Normal));
        assert!(app.data.get(id).is_some());
    }

    #[test]
    fn confirm_delete_a_archives_instead_of_deleting() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        let mut app = test_app(data);
        app.mode = Mode::ConfirmDelete {
            ids: vec![id],
            texts: vec!["ship".to_string()],
            category_names: Vec::new(),
        };

        if let Some(action) =
            app.dispatch_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE))
        {
            app.handle_action(action);
        }

        let item = app.data.get(id).unwrap();
        assert!(matches!(app.mode, Mode::Normal));
        assert!(item.archived);
    }

    #[test]
    fn rename_command_targets_selected_item_in_items_pane() {
        let mut data = TodoData::new();
        let id = data.add("old item");
        let mut app = test_app(data);
        app.pane = Pane::Items;

        app.handle_action(Action::ExecuteCommand("rename".to_string()));
        assert!(matches!(
            app.mode,
            Mode::RenameInput {
                target: RenameTarget::Item(target_id)
            } if target_id == id
        ));
        assert_eq!(app.input.text(), "old item");

        app.input.set_text("new item");
        app.handle_action(Action::SubmitRename);
        assert_eq!(
            app.data.get(id).map(|item| item.text.as_str()),
            Some("new item")
        );
    }

    #[test]
    fn rename_command_targets_highlighted_category_in_category_pane() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        data.set_category(id, Some("Work/api".to_string()));
        let mut app = test_app(data);
        app.pane = Pane::Categories;
        app.category_index = 1;

        app.handle_action(Action::ExecuteCommand("rename".to_string()));
        assert!(matches!(
            app.mode,
            Mode::RenameInput {
                target: RenameTarget::Category(ref category)
            } if category == "Work"
        ));

        app.input.set_text("Office");
        app.handle_action(Action::SubmitRename);
        assert_eq!(
            app.data.get(id).and_then(|item| item.category.as_deref()),
            Some("Office/api")
        );
    }

    #[test]
    fn move_command_from_items_pane_targets_selected_item() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        data.add_category("Work");
        let mut app = test_app(data);
        app.pane = Pane::Items;

        app.handle_action(Action::ExecuteCommand("move".to_string()));
        assert!(matches!(
            app.mode,
            Mode::CategoryPicker {
                target: CategoryPickerTarget::AssignItem,
                ..
            }
        ));

        app.handle_action(Action::CategorySelect(1));
        assert_eq!(
            app.data.get(id).and_then(|item| item.category.as_deref()),
            Some("Work")
        );
    }

    #[test]
    fn move_command_from_category_pane_targets_highlighted_category() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        data.set_category(id, Some("Work/api".to_string()));
        data.add_category("Personal");
        let mut app = test_app(data);
        app.pane = Pane::Categories;
        app.category_index = app
            .category_entries()
            .iter()
            .position(|entry| entry.path == "Work" && entry.depth == 0)
            .map(|idx| idx + 1)
            .unwrap();

        app.handle_action(Action::ExecuteCommand("move".to_string()));
        assert!(matches!(
            app.mode,
            Mode::CategoryPicker {
                target: CategoryPickerTarget::MoveCategory(ref category),
                ..
            } if category == "Work"
        ));

        let destination_idx = app
            .categories()
            .iter()
            .position(|category| category == "Personal")
            .map(|idx| idx + 1)
            .unwrap();
        app.handle_action(Action::CategorySelect(destination_idx));

        assert_eq!(
            app.data.get(id).and_then(|item| item.category.as_deref()),
            Some("Personal/Work/api")
        );
        assert!(app
            .data
            .categories()
            .contains(&"Personal/Work/api".to_string()));
    }

    #[test]
    fn move_category_to_root_keeps_leaf_name() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        data.set_category(id, Some("Work/api".to_string()));
        let mut app = test_app(data);
        app.pane = Pane::Categories;
        app.category_index = app
            .category_entries()
            .iter()
            .position(|entry| entry.path == "Work/api")
            .map(|idx| idx + 1)
            .unwrap();

        app.handle_action(Action::ExecuteCommand("move".to_string()));
        app.handle_action(Action::CategorySelect(0));

        assert_eq!(
            app.data.get(id).and_then(|item| item.category.as_deref()),
            Some("api")
        );
        assert!(app.data.categories().contains(&"api".to_string()));
    }

    #[test]
    fn archive_one_archives_selected_item() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        let mut app = test_app(data);

        app.handle_action(Action::ExecuteCommand("archive one".to_string()));

        assert_eq!(app.data.get(id).map(|item| item.archived), Some(true));
    }

    #[test]
    fn archive_bulk_opens_bulk_archive_multiselect() {
        let data = TodoData::new();
        let mut app = test_app(data);

        app.handle_action(Action::ExecuteCommand("archive bulk".to_string()));

        assert!(matches!(
            app.mode,
            Mode::MultiSelect {
                cmd: MultiSelectCmd::Archive,
                ..
            }
        ));
    }

    #[test]
    fn archive_select_alias_still_opens_bulk_archive_multiselect() {
        let data = TodoData::new();
        let mut app = test_app(data);

        app.handle_action(Action::ExecuteCommand("archive select".to_string()));

        assert!(matches!(
            app.mode,
            Mode::MultiSelect {
                cmd: MultiSelectCmd::Archive,
                ..
            }
        ));
    }

    #[test]
    fn archive_restore_bulk_opens_restore_archive_multiselect() {
        let data = TodoData::new();
        let mut app = test_app(data);

        app.handle_action(Action::ExecuteCommand("archive restore bulk".to_string()));

        assert!(app.show_archived);
        assert!(matches!(
            app.mode,
            Mode::MultiSelect {
                cmd: MultiSelectCmd::RestoreArchive,
                ..
            }
        ));
    }

    #[test]
    fn restore_archive_multiselect_restores_selected_items() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        data.set_archived(id, true);
        let mut app = test_app(data);
        app.show_archived = true;
        app.mode = Mode::MultiSelect {
            cmd: MultiSelectCmd::RestoreArchive,
            selected: [id].into_iter().collect(),
            selected_categories: Default::default(),
        };

        app.handle_action(Action::ConfirmMultiSelect);

        assert_eq!(app.data.get(id).map(|item| item.archived), Some(false));
    }

    #[test]
    fn archive_one_from_category_pane_archives_category_branch() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        data.set_category(id, Some("Work/api".to_string()));
        data.add_category("Work/docs");
        let mut app = test_app(data);
        app.pane = Pane::Categories;
        app.category_index = app
            .category_entries()
            .iter()
            .position(|entry| entry.path == "Work" && entry.depth == 0)
            .map(|idx| idx + 1)
            .unwrap();

        app.handle_action(Action::ExecuteCommand("archive one".to_string()));

        assert_eq!(app.data.get(id).map(|item| item.archived), Some(true));
        assert!(app.data.categories().is_empty());
        assert!(app
            .data
            .categories_for_archived(true)
            .contains(&"Work/docs".to_string()));
    }

    #[test]
    fn archive_all_from_all_category_archives_items_and_categories() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        data.set_category(id, Some("Work/api".to_string()));
        data.add_category("Personal");
        let mut app = test_app(data);
        app.pane = Pane::Categories;
        app.category_index = 0;

        app.handle_action(Action::ExecuteCommand("archive all".to_string()));

        assert_eq!(app.data.get(id).map(|item| item.archived), Some(true));
        assert!(app.data.categories().is_empty());
        assert!(app
            .data
            .categories_for_archived(true)
            .contains(&"Personal".to_string()));
    }

    #[test]
    fn archive_bulk_from_category_pane_archives_selected_categories() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        data.set_category(id, Some("Work/api".to_string()));
        data.add_category("Personal");
        let mut app = test_app(data);
        app.pane = Pane::Categories;
        app.mode = Mode::MultiSelect {
            cmd: MultiSelectCmd::Archive,
            selected: Default::default(),
            selected_categories: ["Work".to_string()].into_iter().collect(),
        };

        app.handle_action(Action::ConfirmMultiSelect);

        assert_eq!(app.data.get(id).map(|item| item.archived), Some(true));
        assert!(!app.data.categories().contains(&"Work/api".to_string()));
        assert!(app.data.categories().contains(&"Personal".to_string()));
    }

    #[test]
    fn restore_archive_bulk_from_category_pane_restores_selected_categories() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        data.set_category(id, Some("Work/api".to_string()));
        data.add_category("Work/docs");
        data.set_category_archived("Work", true);
        let mut app = test_app(data);
        app.show_archived = true;
        app.pane = Pane::Categories;
        app.mode = Mode::MultiSelect {
            cmd: MultiSelectCmd::RestoreArchive,
            selected: Default::default(),
            selected_categories: ["Work".to_string()].into_iter().collect(),
        };

        app.handle_action(Action::ConfirmMultiSelect);

        assert_eq!(app.data.get(id).map(|item| item.archived), Some(false));
        assert!(app.data.categories().contains(&"Work/docs".to_string()));
        assert!(!app
            .data
            .categories_for_archived(true)
            .contains(&"Work/docs".to_string()));
    }

    #[test]
    fn filter_category_opens_filter_picker_not_assignment_picker() {
        let mut data = TodoData::new();
        data.add_category("Work");
        let mut app = test_app(data);

        app.handle_action(Action::ExecuteCommand("filter category".to_string()));

        assert!(matches!(app.mode, Mode::CategoryFilterPicker { .. }));
    }

    #[test]
    fn filter_picker_category_opens_filter_picker_not_assignment_picker() {
        let mut data = TodoData::new();
        data.add_category("Work");
        let mut app = test_app(data);

        app.handle_action(Action::FilterSelect(1));

        assert!(matches!(app.mode, Mode::CategoryFilterPicker { .. }));
    }

    #[test]
    fn category_filter_picker_filters_without_assigning_item() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        data.add_category("Work");
        let mut app = test_app(data);
        app.mode = Mode::CategoryFilterPicker { selected: 0 };

        app.handle_action(Action::CategoryFilterSelect(1));

        assert_eq!(app.category_filter.as_deref(), Some("Work"));
        assert_eq!(
            app.data.get(id).and_then(|item| item.category.as_deref()),
            None
        );
    }

    #[test]
    fn category_filter_picker_jumps_to_selected_category_and_shows_only_that_branch() {
        let mut data = TodoData::new();
        let work_id = data.add("work task");
        data.set_category(work_id, Some("Work/api".to_string()));
        let personal_id = data.add("personal task");
        data.set_category(personal_id, Some("Personal".to_string()));
        let mut app = test_app(data);
        let work_idx = app
            .data
            .category_entries()
            .iter()
            .position(|entry| entry.path == "Work")
            .map(|idx| idx + 1)
            .unwrap();

        app.handle_action(Action::CategoryFilterSelect(work_idx));

        let visible: Vec<String> = app.items().into_iter().map(|item| item.text).collect();
        assert_eq!(app.category_index, work_idx);
        assert_eq!(app.category_filter.as_deref(), Some("Work"));
        assert_eq!(visible, vec!["work task"]);
    }

    #[test]
    fn backspace_from_nested_category_filter_moves_to_parent() {
        let mut data = TodoData::new();
        data.add_category("Work/api");
        let mut app = test_app(data);
        let child_idx = app
            .data
            .category_entries()
            .iter()
            .position(|entry| entry.path == "Work/api")
            .map(|idx| idx + 1)
            .unwrap();

        app.handle_action(Action::CategoryFilterSelect(child_idx));
        if let Some(action) =
            app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
        {
            app.handle_action(action);
        }

        assert_eq!(app.category_filter.as_deref(), Some("Work"));
        assert_eq!(app.data.category_entries()[app.category_index - 1].depth, 0);
    }

    #[test]
    fn nested_category_back_restores_parent_item_selection() {
        let mut data = TodoData::new();
        let first_id = data.add("first work task");
        data.set_category(first_id, Some("Work".to_string()));
        let api_id = data.add("api task");
        data.set_category(api_id, Some("Work/api".to_string()));
        let second_id = data.add("second work task");
        data.set_category(second_id, Some("Work".to_string()));
        data.add_category("Work/api");
        let mut app = test_app(data);

        let entries = app.data.category_entries();
        let work_idx = entries
            .iter()
            .position(|entry| entry.path == "Work")
            .map(|idx| idx + 1)
            .unwrap();
        let api_idx = entries
            .iter()
            .position(|entry| entry.path == "Work/api")
            .map(|idx| idx + 1)
            .unwrap();

        app.handle_action(Action::CategoryFilterSelect(work_idx));
        app.selected_index = 2;
        app.handle_action(Action::CategoryFilterSelect(api_idx));

        assert_eq!(app.category_filter.as_deref(), Some("Work/api"));
        assert_eq!(app.selected_index(), 0);

        if let Some(action) =
            app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
        {
            app.handle_action(action);
        }

        assert_eq!(app.category_filter.as_deref(), Some("Work"));
        assert_eq!(app.selected_index(), 2);
    }

    #[test]
    fn sidebar_category_navigation_restores_saved_item_selection() {
        let mut data = TodoData::new();
        let first_id = data.add("first work task");
        data.set_category(first_id, Some("Work".to_string()));
        let api_id = data.add("api task");
        data.set_category(api_id, Some("Work/api".to_string()));
        data.add_category("Work/api");
        let mut app = test_app(data);
        app.pane = Pane::Categories;

        app.handle_action(Action::CategorySelect(1));
        app.pane = Pane::Categories;
        app.selected_index = 1;
        app.handle_action(Action::SelectNext);

        assert_eq!(app.category_filter.as_deref(), Some("Work/api"));
        assert_eq!(app.selected_index(), 0);

        app.handle_action(Action::SelectPrev);

        assert_eq!(app.category_filter.as_deref(), Some("Work"));
        assert_eq!(app.selected_index(), 1);
    }

    #[test]
    fn backspace_in_category_filter_popup_moves_step_by_step_to_command_list() {
        let mut data = TodoData::new();
        data.add_category("Work/api");
        let mut app = test_app(data);
        let child_idx = app
            .data
            .category_entries()
            .iter()
            .position(|entry| entry.path == "Work/api")
            .map(|idx| idx + 1)
            .unwrap();
        app.mode = Mode::CategoryFilterPicker {
            selected: child_idx,
        };

        if let Some(action) =
            app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
        {
            app.handle_action(action);
        }

        assert!(matches!(
            app.mode,
            Mode::CategoryFilterPicker { selected: 1 }
        ));

        if let Some(action) =
            app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
        {
            app.handle_action(action);
        }

        assert!(matches!(
            app.mode,
            Mode::CategoryFilterPicker { selected: 0 }
        ));

        if let Some(action) =
            app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
        {
            app.handle_action(action);
        }

        assert!(matches!(app.mode, Mode::FilterPicker { selected: 2 }));

        if let Some(action) =
            app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
        {
            app.handle_action(action);
        }

        assert_eq!(app.input.text(), "");
        assert!(matches!(app.mode, Mode::Command { selected: 0 }));
    }

    #[test]
    fn backspace_from_subcommand_context_restores_parent_command_selection() {
        let mut app = test_app(TodoData::new());
        let sort_idx = get_filtered_commands("")
            .iter()
            .position(|(name, _)| *name == "sort")
            .unwrap();
        app.mode = Mode::Command { selected: sort_idx };

        app.handle_action(Action::ExecuteCommand("sort".to_string()));

        assert_eq!(app.input.text(), "sort ");
        assert!(matches!(app.mode, Mode::Command { selected: 0 }));

        let action = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

        assert!(action.is_none());
        assert_eq!(app.input.text(), "");
        assert!(matches!(app.mode, Mode::Command { selected } if selected == sort_idx));
    }

    #[test]
    fn backspace_from_typed_subcommand_context_restores_matching_parent_command() {
        let mut app = test_app(TodoData::new());
        let sort_idx = get_filtered_commands("")
            .iter()
            .position(|(name, _)| *name == "sort")
            .unwrap();
        app.mode = Mode::Command { selected: 0 };
        app.input.insert_str("sort");

        app.handle_action(Action::ExecuteCommand("sort".to_string()));

        let action = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

        assert!(action.is_none());
        assert_eq!(app.input.text(), "");
        assert!(matches!(app.mode, Mode::Command { selected } if selected == sort_idx));
    }

    #[test]
    fn backspace_from_picker_restores_command_subcommand_selection() {
        let mut app = test_app(TodoData::new());
        let filter_idx = get_filtered_commands("")
            .iter()
            .position(|(name, _)| *name == "filter")
            .unwrap();
        let priority_idx = get_filtered_commands("filter ")
            .iter()
            .position(|(name, _)| *name == "filter priority")
            .unwrap();
        app.mode = Mode::Command {
            selected: filter_idx,
        };

        app.handle_action(Action::ExecuteCommand("filter".to_string()));
        if let Mode::Command { ref mut selected } = app.mode {
            *selected = priority_idx;
        }
        app.handle_action(Action::ExecuteCommand("filter priority".to_string()));

        assert!(matches!(app.mode, Mode::PriorityPicker { .. }));

        if let Some(action) =
            app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
        {
            app.handle_action(action);
        }

        assert_eq!(app.input.text(), "filter ");
        assert!(matches!(app.mode, Mode::Command { selected } if selected == priority_idx));

        let action = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

        assert!(action.is_none());
        assert_eq!(app.input.text(), "");
        assert!(matches!(app.mode, Mode::Command { selected } if selected == filter_idx));
    }

    #[test]
    fn left_arrow_in_category_filter_popup_moves_one_step_back() {
        let mut data = TodoData::new();
        data.add_category("Work/api");
        let mut app = test_app(data);
        let child_idx = app
            .data
            .category_entries()
            .iter()
            .position(|entry| entry.path == "Work/api")
            .map(|idx| idx + 1)
            .unwrap();
        app.mode = Mode::CategoryFilterPicker {
            selected: child_idx,
        };

        if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)) {
            app.handle_action(action);
        }

        assert!(matches!(
            app.mode,
            Mode::CategoryFilterPicker { selected: 1 }
        ));
    }

    #[test]
    fn left_arrow_in_root_popup_returns_to_initial_command_list() {
        let mut app = test_app(TodoData::new());
        app.mode = Mode::ArchivePicker { selected: 0 };

        if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)) {
            app.handle_action(action);
        }

        assert_eq!(app.input.text(), "");
        assert!(matches!(app.mode, Mode::Command { selected: 0 }));
    }

    #[test]
    fn right_arrow_in_popup_list_behaves_like_enter() {
        let mut app = test_app(TodoData::new());
        app.mode = Mode::SortPicker {
            selected: 1,
            original_sort: SortMode::Default,
        };

        if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)) {
            app.handle_action(action);
        }

        assert_eq!(app.sort_mode, SortMode::DueDate);
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn theme_picker_previews_highlighted_theme_and_cancel_restores_original() {
        let mut app = test_app(TodoData::new());
        app.theme = Theme::one_dark();
        app.mode = Mode::ThemePicker {
            selected: theme_index_by_name("one-dark"),
            original_theme: app.theme.clone(),
        };

        app.handle_action(Action::SelectPrev);

        assert_ne!(app.theme.name, "one-dark");
        app.handle_action(Action::CancelThemePicker);
        assert_eq!(app.theme.name, "one-dark");
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn sort_picker_previews_highlighted_sort_and_cancel_restores_original() {
        let mut app = test_app(TodoData::new());
        app.sort_mode = SortMode::Default;
        app.mode = Mode::SortPicker {
            selected: 0,
            original_sort: SortMode::Default,
        };

        app.handle_action(Action::SelectNext);

        assert_eq!(app.sort_mode, SortMode::DueDate);
        app.handle_action(Action::CancelSortPicker);
        assert_eq!(app.sort_mode, SortMode::Default);
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn right_arrow_in_command_popup_behaves_like_enter() {
        let mut app = test_app(TodoData::new());
        app.mode = Mode::Command { selected: 0 };
        app.input.insert_str("keyb");

        if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)) {
            app.handle_action(action);
        }

        assert!(matches!(app.mode, Mode::Keybindings));
    }

    #[test]
    fn left_arrow_in_command_popup_returns_to_initial_command_list() {
        let mut app = test_app(TodoData::new());
        app.mode = Mode::Command { selected: 0 };
        app.input.insert_str("keyb");

        let action = app.dispatch_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));

        assert!(action.is_none());
        assert_eq!(app.input.text(), "");
        assert!(matches!(app.mode, Mode::Command { selected: 0 }));
    }

    #[test]
    fn left_arrow_at_root_command_list_preserves_selection() {
        let mut app = test_app(TodoData::new());
        let sort_idx = get_filtered_commands("")
            .iter()
            .position(|(name, _)| *name == "sort")
            .unwrap();
        app.mode = Mode::Command { selected: sort_idx };

        let action = app.dispatch_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));

        assert!(action.is_none());
        assert_eq!(app.input.text(), "");
        assert!(matches!(app.mode, Mode::Command { selected } if selected == sort_idx));
    }

    #[test]
    fn backspace_at_initial_command_list_stops_there() {
        let mut app = test_app(TodoData::new());
        app.mode = Mode::Command { selected: 0 };

        let action = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

        assert!(action.is_none());
        assert_eq!(app.input.text(), "");
        assert!(matches!(app.mode, Mode::Command { selected: 0 }));
    }

    #[test]
    fn left_arrow_at_initial_command_list_stops_there() {
        let mut app = test_app(TodoData::new());
        app.mode = Mode::Command { selected: 0 };

        let action = app.dispatch_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));

        assert!(action.is_none());
        assert_eq!(app.input.text(), "");
        assert!(matches!(app.mode, Mode::Command { selected: 0 }));
        assert!(get_filtered_commands(app.input.text())
            .iter()
            .all(|(name, _)| !name.contains(' ')));
    }

    #[test]
    fn backspace_in_filter_subpicker_returns_to_filter_popup() {
        let mut app = test_app(TodoData::new());
        app.mode = Mode::DueDateFilterPicker { selected: 1 };

        if let Some(action) =
            app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
        {
            app.handle_action(action);
        }

        assert!(matches!(app.mode, Mode::FilterPicker { selected: 0 }));
    }

    #[test]
    fn backspace_at_initial_filter_popup_returns_to_initial_command_list() {
        let mut app = test_app(TodoData::new());
        app.mode = Mode::FilterPicker { selected: 0 };

        if let Some(action) =
            app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
        {
            app.handle_action(action);
        }

        assert_eq!(app.input.text(), "");
        assert!(matches!(app.mode, Mode::Command { selected: 0 }));
    }

    #[test]
    fn backspace_in_root_popup_returns_to_initial_command_list() {
        let mut app = test_app(TodoData::new());
        app.mode = Mode::ArchivePicker { selected: 0 };

        if let Some(action) =
            app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
        {
            app.handle_action(action);
        }

        assert_eq!(app.input.text(), "");
        assert!(matches!(app.mode, Mode::Command { selected: 0 }));
    }

    #[test]
    fn ctrl_backspace_deletes_previous_word_in_text_prompt() {
        let mut app = test_app(TodoData::new());
        app.mode = Mode::Editing { edit_id: None };
        app.input.insert_str("hello world");

        let action = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::CONTROL));

        assert!(action.is_none());
        assert_eq!(app.input.text(), "hello ");
    }

    #[test]
    fn empty_new_item_prompt_returns_to_normal_mode() {
        let mut app = test_app(TodoData::new());
        app.mode = Mode::Editing { edit_id: None };
        app.input.insert_str("a");

        if let Some(action) =
            app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
        {
            app.handle_action(action);
        }

        assert_eq!(app.input.text(), "");
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn reset_command_clears_all_filters() {
        let mut app = test_app(TodoData::new());
        app.filter = "ship".to_string();
        app.priority_filter = Some(crate::data::Priority::High);
        app.due_filter = Some(super::DueFilter::Today);
        app.category_filter = Some("Work".to_string());
        app.category_index = 1;
        app.show_archived = true;
        app.sort_mode = SortMode::Priority;

        app.handle_action(Action::ExecuteCommand("reset".to_string()));

        assert_eq!(app.filter, "");
        assert_eq!(app.priority_filter, None);
        assert_eq!(app.due_filter, None);
        assert_eq!(app.category_filter, None);
        assert_eq!(app.category_index, 0);
        assert!(!app.show_archived);
        assert_eq!(app.sort_mode, SortMode::Default);
    }

    #[test]
    fn terminal_ctrl_backspace_encoding_deletes_previous_word_in_text_prompt() {
        let mut app = test_app(TodoData::new());
        app.mode = Mode::Editing { edit_id: None };
        app.input.insert_str("hello world");

        let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));

        assert!(action.is_none());
        assert_eq!(app.input.text(), "hello ");
    }

    #[test]
    fn get_filtered_commands_matches_filter_command() {
        let results = get_filtered_commands("filter");
        assert!(!results.is_empty(), "filter should match");
        assert!(
            results.iter().any(|(n, _)| *n == "filter"),
            "should contain filter"
        );
    }

    #[test]
    fn get_filtered_commands_matches_archive_command() {
        let results = get_filtered_commands("archive");
        assert!(!results.is_empty(), "archive should match");
        assert!(
            results.iter().any(|(n, _)| *n == "archive"),
            "should contain archive"
        );
    }

    #[test]
    fn get_filtered_commands_shows_archive_subcommands_after_space() {
        let results = get_filtered_commands("archive ");
        let names: Vec<&str> = results.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            names,
            vec![
                "archive bulk",
                "archive done",
                "archive one",
                "archive all",
                "archive archived",
                "archive restore bulk",
                "archive restore",
                "archive restore all"
            ]
        );
    }

    #[test]
    fn tab_completion_enters_subcommand_context_for_parent_commands() {
        assert_eq!(get_completions("so"), vec!["sort ".to_string()]);
        assert_eq!(get_completions("filter"), vec!["filter ".to_string()]);
        assert_eq!(get_completions("archive"), vec!["archive ".to_string()]);
    }

    #[test]
    fn subcommand_context_filters_by_typed_suffix() {
        let sort_results: Vec<&str> = get_filtered_commands("sort du")
            .iter()
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(sort_results, vec!["sort due"]);

        let filter_results: Vec<&str> = get_filtered_commands("filter pr")
            .iter()
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(filter_results, vec!["filter priority"]);
    }

    #[test]
    fn parent_command_execution_enters_promptable_subcommand_context() {
        let mut app = test_app(TodoData::new());

        app.handle_action(Action::ExecuteCommand("sort".to_string()));

        assert_eq!(app.input.text(), "sort ");
        assert!(matches!(app.mode, Mode::Command { selected: 0 }));
    }

    #[test]
    fn resolve_command_input_executes_selected_partial_command() {
        assert_eq!(
            resolve_command_input("arch", 0),
            Some("archive".to_string())
        );
    }

    #[test]
    fn resolve_command_input_executes_selected_command_when_input_is_empty() {
        assert_eq!(resolve_command_input("", 1), Some("clear".to_string()));
    }

    #[test]
    fn resolve_command_input_preserves_argument_commands() {
        assert_eq!(
            resolve_command_input("search Work task", 0),
            Some("search Work task".to_string())
        );
        assert_eq!(
            resolve_command_input("rename Work Office", 0),
            Some("rename Work Office".to_string())
        );
    }

    #[test]
    fn resolve_command_input_executes_selected_subcommand_after_space() {
        assert_eq!(
            resolve_command_input("archive ", 1),
            Some("archive done".to_string())
        );
        assert_eq!(
            resolve_command_input("filter ", 4),
            Some("filter clear".to_string())
        );
    }

    #[test]
    fn get_filtered_commands_shows_regular_commands() {
        let results = get_filtered_commands("");
        assert!(
            results.iter().any(|(n, _)| *n == "help"),
            "should contain regular commands"
        );
    }

    #[test]
    fn get_filtered_commands_groups_archive_commands() {
        let results = get_filtered_commands("");
        let names: Vec<&str> = results.iter().map(|(name, _)| *name).collect();
        assert!(names.contains(&"archive"));
        assert!(!names.contains(&"unarchive"));
        assert!(!names.contains(&"archive archived"));
    }

    #[test]
    fn unarchive_is_hidden_alias_not_visible_parent_command() {
        assert!(get_filtered_commands("unarchive").is_empty());
        assert_eq!(
            resolve_command_input("unarchive", 0),
            Some("unarchive".to_string())
        );
    }

    #[test]
    fn get_filtered_commands_parent_list_contains_only_parent_commands() {
        let results = get_filtered_commands("");
        assert!(
            results.iter().all(|(name, _)| !name.contains(' ')),
            "parent command list should not contain subcommands"
        );
    }

    #[test]
    fn get_filtered_commands_keybindings_matches() {
        let results = get_filtered_commands("keybindings");
        assert_eq!(results.len(), 1, "keybindings should match exactly one");
        assert_eq!(results[0].0, "keybindings");
    }

    #[test]
    fn ctrl_slash_opens_keybindings() {
        let mut app = test_app(TodoData::new());
        let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::CONTROL));
        assert!(matches!(
            action,
            Some(Action::ExecuteCommand(command)) if command == "keybindings"
        ));
    }

    #[test]
    fn ctrl_underscore_opens_keybindings_for_terminal_ctrl_slash() {
        let mut app = test_app(TodoData::new());
        let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('_'), KeyModifiers::CONTROL));
        assert!(matches!(
            action,
            Some(Action::ExecuteCommand(command)) if command == "keybindings"
        ));
    }
}
