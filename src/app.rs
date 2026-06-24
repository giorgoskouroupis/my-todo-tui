use std::collections::HashSet;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};

use crate::clip::Clipboard;
use crate::config;
use crate::data::{TodoData, TodoItem, Priority};
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
pub enum Pane {
    Items,
    Categories,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiSelectCmd {
    Delete,
    ToggleDone,
}

pub enum Mode {
    Normal,
    Editing { edit_id: Option<u64> },
    Command { selected: usize },
    Searching,
    MultiSelect { cmd: MultiSelectCmd, selected: HashSet<u64> },
    ThemePicker { selected: usize },
    PriorityPicker { selected: usize },
    Help,
    Keybindings,
    ConfirmDelete { ids: Vec<u64>, texts: Vec<String> },
    CategoryPicker { selected: usize },
    CategoryAdd,
    SortPicker { selected: usize },
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
    ConfirmMultiSelect,
    CancelMultiSelect,
    ThemeSelect(usize),
    CancelThemePicker,
    ConfirmDeleteYes,
    ConfirmDeleteNo,
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
    AddCategory(String),
    CreateAndAssignCategory(String),
    DeleteCategory(String),
    CancelCategoryPicker,
    StartCategoryAdd,
    StartCategoryAddWithChar(char),
    CancelCategoryAdd,
    StartSortPicker,
    SortSelect(usize),
    CancelSortPicker,
    Quit,
}

pub struct App {
    pub data: TodoData,
    pub selected_index: usize,
    pub input: InputBuffer,
    pub mode: Mode,
    pub clip: Clipboard,
    pub filter: String,
    pub priority_filter: Option<Priority>,
    pub category_filter: Option<String>,
    pub pane: Pane,
    pub category_index: usize,
    pub sort_mode: SortMode,
    pub theme: Theme,
    pub completions: Vec<String>,
    pub completion_index: usize,
    pub dirty: bool,
    pub last_mutated: Instant,
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
            category_filter: None,
            pane: Pane::Items,
            category_index: 0,
            sort_mode: SortMode::Default,
            theme,
            completions: Vec::new(),
            completion_index: 0,
            dirty: false,
            last_mutated: Instant::now(),
        }
    }

    fn mark_dirty(&mut self) {
        if !self.dirty {
            self.dirty = true;
            self.last_mutated = Instant::now();
        }
    }

    fn active_category(&self) -> Option<String> {
        let cats = self.data.categories();
        if self.category_index == 0 || self.category_index > cats.len() {
            return None;
        }
        cats.get(self.category_index - 1).cloned()
    }

    pub fn items(&self) -> Vec<TodoItem> {
        let items = self.data.items().to_vec();
        let items: Vec<TodoItem> = match self.priority_filter {
            Some(p) => items.into_iter().filter(|i| i.priority == p).collect(),
            None => items,
        };
        let items: Vec<TodoItem> = match self.active_category() {
            Some(cat) => items.into_iter().filter(|i| i.category.as_deref() == Some(cat.as_str())).collect(),
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
                    let due = i.due_date.as_deref().unwrap_or("9999-99-99").to_string();
                    (!i.pinned, due)
                });
            }
        }
        items
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index.min(
            self.items().len().saturating_sub(1),
        )
    }

    pub fn pending_count(&self) -> usize {
        self.data.pending_count()
    }

    fn clamp_selection(&mut self) {
        let max = self.items().len().saturating_sub(1);
        self.selected_index = self.selected_index.min(max);
    }

    pub fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
        let mut app = Self::new();

        terminal.hide_cursor()?;

        loop {
            terminal.draw(|f| {
                crate::ui::render(
                    f,
                    &app.items(),
                    app.selected_index(),
                    &app.input,
                    &app.mode,
                    app.pending_count(),
                    &app.filter,
                    &app.priority_filter,
                    &app.pane,
                    app.category_index,
                    &app.data.categories(),
                    &app.theme,
                    &app.sort_mode,
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
        if key.code == KeyCode::Char('/') && !matches!(self.mode, Mode::Command { .. } | Mode::CategoryAdd) {
            return Some(Action::StartCommand);
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(Action::Quit);
        }

        let is_pane_mode = matches!(self.mode, Mode::Normal | Mode::MultiSelect { .. });
        if is_pane_mode {
            match key.code {
                KeyCode::Left if self.pane == Pane::Items => return Some(Action::SwitchPane),
                KeyCode::Right if self.pane == Pane::Categories => return Some(Action::SwitchPane),
                _ => {}
            }
        }

        match &self.mode {
            Mode::Normal => {
                if self.pane == Pane::Categories {
                    keys::handle_sidebar(key, &self.data, self.category_index)
                } else {
                    let items = self.items();
                    keys::handle_normal(key, &items, self.selected_index)
                }
            }
            Mode::Editing { .. } => {
                keys::handle_editing(key, &mut self.input)
            }
            Mode::Searching => {
                keys::handle_search(key, &mut self.filter)
            }
            Mode::Command { selected } => {
                if let KeyCode::Enter = key.code {
                    let filtered = get_filtered_commands(self.input.text());
                    if let Some((cmd, _)) = filtered.get(*selected) {
                        return Some(Action::ExecuteCommand(cmd.to_string()));
                    }
                    return None;
                }

                let action = keys::handle_command(key, &mut self.input);
                if action.is_some() {
                    self.completions.clear();
                    self.completion_index = 0;
                } else if let KeyCode::Tab = key.code {
                    return Some(Action::TabComplete);
                } else if matches!(key.code, KeyCode::Up | KeyCode::Char('k'))
                    && !key.modifiers.is_empty()
                {
                    return action;
                } else if matches!(key.code, KeyCode::Up | KeyCode::Char('k')) {
                    return Some(Action::SelectPrev);
                } else if matches!(key.code, KeyCode::Down | KeyCode::Char('j')) {
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
                let items = self.items();
                keys::handle_multiselect(key, &items, self.selected_index)
            }
            Mode::ThemePicker { selected } => {
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
            Mode::CategoryPicker { selected } => {
                let nav_action = keys::handle_category_picker(key);
                if nav_action.is_some() {
                    return nav_action;
                }
                match key.code {
                    KeyCode::Enter => {
                        let text = self.input.text().to_string();
                        if text.is_empty() {
                            return Some(Action::CategorySelect(*selected));
                        } else {
                            return Some(Action::CreateAndAssignCategory(text));
                        }
                    }
                    KeyCode::Esc => {
                        self.input.clear();
                        return Some(Action::CancelCategoryPicker);
                    }
                    _ => {
                        if let KeyCode::Char(c) = key.code {
                            if !key.modifiers.contains(KeyModifiers::CONTROL)
                                && !key.modifiers.contains(KeyModifiers::ALT)
                            {
                                self.input.insert_char(c);
                            }
                        }
                        None
                    }
                }
            }
            Mode::CategoryAdd => {
                keys::handle_category_add(key, &mut self.input)
            }
            Mode::SortPicker { selected } => {
                let action = keys::handle_sort_picker(key);
                if action.is_none() && key.code == KeyCode::Enter {
                    return Some(Action::SortSelect(*selected));
                }
                action
            }
            Mode::ConfirmDelete { .. } => keys::handle_confirm_delete(key),
            Mode::Help | Mode::Keybindings => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => Some(Action::CancelEdit),
                _ => None,
            },
        }
    }

    fn handle_action(&mut self, action: Action) -> bool {
        match action {
            Action::SelectPrev => {
                if let Mode::Command { ref mut selected } = &mut self.mode {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                } else if let Mode::ThemePicker { ref mut selected } = &mut self.mode {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                } else if let Mode::PriorityPicker { ref mut selected } = &mut self.mode {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                } else if let Mode::SortPicker { ref mut selected } = &mut self.mode {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                } else if self.pane == Pane::Categories {
                    if self.category_index > 0 {
                        self.category_index -= 1;
                        let cats = self.data.categories();
                        self.category_filter = if self.category_index == 0 {
                            None
                        } else {
                            cats.get(self.category_index - 1).cloned()
                        };
                        self.clamp_selection();
                    }
                } else if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            Action::SelectNext => {
                if let Mode::Command { ref mut selected } = &mut self.mode {
                    let max = get_filtered_commands(self.input.text()).len().saturating_sub(1);
                    if *selected < max {
                        *selected += 1;
                    }
                } else if let Mode::ThemePicker { ref mut selected } = &mut self.mode {
                    let max = Theme::theme_names().len().saturating_sub(1);
                    if *selected < max {
                        *selected += 1;
                    }
                } else if let Mode::PriorityPicker { ref mut selected } = &mut self.mode {
                    let max = 4usize;
                    if *selected < max {
                        *selected += 1;
                    }
                } else if let Mode::SortPicker { ref mut selected } = &mut self.mode {
                    let max = 2usize;
                    if *selected < max {
                        *selected += 1;
                    }
                } else if self.pane == Pane::Categories {
                    let cats = self.data.categories();
                    if self.category_index < cats.len() {
                        self.category_index += 1;
                        self.category_filter = cats.get(self.category_index - 1).cloned();
                        self.clamp_selection();
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
                    self.mode = Mode::Editing {
                        edit_id: Some(id),
                    };
                }
            }
            Action::SubmitEdit => {
                match &self.mode {
                    Mode::Editing { edit_id } => {
                        let text = self.input.text().to_string();
                        let text = text.trim().to_string();

                        if !text.is_empty() && text.len() <= 500 {
                            match edit_id {
                                None => {
                                    let id = self.data.add(&text);
                                    if let Some(cat) = &self.category_filter {
                                        self.data.set_category(id, Some(cat.clone()));
                                    }
                                }
                                Some(id) => {
                                    self.data.update_text(*id, &text);
                                }
                            }
                            self.mark_dirty();
                        }
                    }
                    _ => {}
                }
                self.input.clear();
                self.mode = Mode::Normal;
                self.clamp_selection();
            }
            Action::CancelEdit => {
                self.input.clear();
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
                    };
                }
            }
            Action::CyclePriority(id, forward) => {
                self.data.cycle_priority(id, forward);
                self.mark_dirty();
            }
            Action::Reorder(id, direction) => {
                if self.data.reorder(id, direction) {
                    if direction < 0 && self.selected_index > 0 {
                        self.selected_index -= 1;
                    } else if direction > 0 && self.selected_index < self.items().len().saturating_sub(1) {
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
                        };
                    }
                    "done" | "x" => {
                        self.mode = Mode::MultiSelect {
                            cmd: MultiSelectCmd::ToggleDone,
                            selected: HashSet::new(),
                        };
                    }
                    "clear" | "c" => {
                        self.data.clear_done();
                        self.mark_dirty();
                        self.mode = Mode::Normal;
                    }
                    "themes" => {
                        self.mode = Mode::ThemePicker { selected: 0 };
                    }
                    "priorities" | "p" => {
                        self.mode = Mode::PriorityPicker { selected: 0 };
                    }
                    "help" => {
                        self.mode = Mode::Help;
                    }
                    "keybindings" | "k" => {
                        self.mode = Mode::Keybindings;
                    }
                    "categories" | "cat" => {
                        self.mode = Mode::CategoryPicker { selected: 0 };
                    }
                    "sort" => {
                        match arg.as_deref() {
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
                                self.mode = Mode::SortPicker { selected: 0 };
                            }
                        }
                    }
                    _ => {
                        self.mode = Mode::Normal;
                    }
                }
                self.completions.clear();
                self.completion_index = 0;
                self.input.clear();
                self.clamp_selection();
            }
            Action::ToggleMultiSelect(id) => {
                if let Mode::MultiSelect { ref mut selected, .. } = &mut self.mode {
                    if !selected.insert(id) {
                        selected.remove(&id);
                    }
                }
            }
            Action::ConfirmMultiSelect => {
                let mode = std::mem::replace(&mut self.mode, Mode::Normal);
                if let Mode::MultiSelect { cmd, selected } = mode {
                    match cmd {
                        MultiSelectCmd::Delete => {
                            for id in &selected {
                                if let Some(item) = self.data.delete(*id) {
                                    self.clip.cut(item);
                                }
                            }
                            self.mark_dirty();
                        }
                        MultiSelectCmd::ToggleDone => {
                            for id in &selected {
                                self.data.toggle_done(*id);
                            }
                            self.mark_dirty();
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
                self.mode = Mode::Normal;
            }
            Action::ConfirmDeleteYes => {
                let mode = std::mem::replace(&mut self.mode, Mode::Normal);
                if let Mode::ConfirmDelete { ids, .. } = mode {
                    for id in ids {
                        if let Some(item) = self.data.delete(id) {
                            self.clip.cut(item);
                        }
                    }
                    self.mark_dirty();
                }
                self.clamp_selection();
            }
            Action::ConfirmDeleteNo => {
                self.mode = Mode::Normal;
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
            }
            Action::SetDueDate => {
                if let Mode::Editing { edit_id: Some(id) } = &self.mode {
                    if let Some(item) = self.data.get(*id) {
                        let new_date = if item.due_date.is_some() {
                            None
                        } else {
                            let secs = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();
                            let mut days = secs / 86400;
                            let mut y = 1970u64;
                            loop {
                                let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
                                let yd = if leap { 366 } else { 365 };
                                if days < yd { break; }
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
                                if days < md { break; }
                                days -= md;
                                m += 1;
                            }
                            let d = days + 1;
                            Some(format!("{:04}-{:02}-{:02}", y, m, d))
                        };
                        self.data.set_due_date(*id, new_date);
                        self.mark_dirty();
                    }
                }
            }
            Action::SwitchPane => {
                self.pane = match self.pane {
                    Pane::Items => Pane::Categories,
                    Pane::Categories => Pane::Items,
                };
            }
            Action::OpenCategoryPicker => {
                self.mode = Mode::CategoryPicker { selected: 0 };
            }
            Action::CategorySelect(idx) => {
                let cats = self.data.categories();
                let opened_from_picker = matches!(self.mode, Mode::CategoryPicker { .. });
                if opened_from_picker {
                    let items = self.items();
                    let id = items.get(self.selected_index()).map(|i| i.id);
                    if let Some(id) = id {
                        let category = if idx == 0 { None } else { cats.get(idx - 1).cloned() };
                        self.data.set_category(id, category);
                        self.mark_dirty();
                    }
                    self.mode = Mode::Normal;
                } else {
                    if idx == 0 {
                        self.category_filter = None;
                        self.category_index = 0;
                    } else if let Some(cat) = cats.get(idx - 1) {
                        self.category_filter = Some(cat.clone());
                        self.category_index = idx;
                    }
                    self.pane = Pane::Items;
                    self.mode = Mode::Normal;
                    self.clamp_selection();
                }
            }
            Action::AddCategory(name) => {
                if !name.is_empty() {
                    self.category_filter = Some(name.clone());
                    let cats = self.data.categories();
                    if let Some(pos) = cats.iter().position(|c| c == &name) {
                        self.category_index = pos + 1;
                    } else {
                        self.category_index = cats.len() + 1;
                    }
                    self.pane = Pane::Categories;
                    self.clamp_selection();
                }
                self.input.clear();
                self.mode = Mode::Normal;
            }
            Action::DeleteCategory(name) => {
                self.data.delete_category(&name);
                self.category_filter = None;
                self.category_index = 0;
                self.mark_dirty();
                self.mode = Mode::Normal;
                self.clamp_selection();
            }
            Action::CancelCategoryPicker => {
                self.mode = Mode::Normal;
            }
            Action::StartCategoryAdd => {
                self.input.clear();
                self.mode = Mode::CategoryAdd;
            }
            Action::CreateAndAssignCategory(name) => {
                let items = self.items();
                let id = items.get(self.selected_index()).map(|i| i.id);
                if let Some(id) = id {
                    self.data.set_category(id, Some(name.clone()));
                    self.mark_dirty();
                }
                self.input.clear();
                self.mode = Mode::Normal;
            }
            Action::StartSortPicker => {
                self.mode = Mode::SortPicker { selected: 0 };
            }
            Action::SortSelect(idx) => {
                const SORTS: [SortMode; 3] = [SortMode::Priority, SortMode::DueDate, SortMode::Default];
                if let Some(&mode) = SORTS.get(idx) {
                    self.sort_mode = mode;
                }
                self.mode = Mode::Normal;
            }
            Action::CancelSortPicker => {
                self.mode = Mode::Normal;
            }
            Action::StartCategoryAddWithChar(c) => {
                self.input.clear();
                self.input.insert_char(c);
                self.mode = Mode::CategoryAdd;
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
    ("clear", "Clear completed items"),
    ("c", "Alias for clear"),
    ("delete", "Bulk delete items"),
    ("done", "Bulk toggle done"),
    ("d", "Alias for delete"),
    ("help", "Show help"),
    ("keybindings", "Show keybindings"),
    ("categories", "Filter by category"),
    ("cat", "Alias for categories"),
    ("priorities", "Filter by priority"),
    ("p", "Alias for priorities"),
    ("search", "Filter items by text"),
    ("s", "Alias for search"),
    ("sort", "Sort items (priority/due/default)"),
    ("themes", "List available themes"),
    ("x", "Alias for done"),
];

pub fn get_filtered_commands(prefix: &str) -> Vec<(&'static str, &'static str)> {
    let lower = prefix.to_lowercase();
    get_command_list()
        .into_iter()
        .filter(|(name, _)| name.starts_with(&lower))
        .collect()
}

pub fn get_command_list() -> Vec<(&'static str, &'static str)> {
    COMMANDS
        .iter()
        .filter(|(name, _)| !matches!(*name, "s" | "d" | "x" | "c" | "p" | "k" | "cat"))
        .copied()
        .collect()
}

fn get_completions(prefix: &str) -> Vec<String> {
    let lower = prefix.to_lowercase();
    let mut matches: Vec<&str> = COMMANDS
        .iter()
        .filter_map(|(name, _)| {
            if name.starts_with(&lower) {
                Some(*name)
            } else {
                None
            }
        })
        .collect();
    matches.sort();
    matches.dedup();
    matches.into_iter().map(|s| s.to_string()).collect()
}
