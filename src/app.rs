use crossterm::event::{Event, KeyEvent};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};

use crate::clip::Clipboard;
use crate::data::{TodoData, TodoItem};
use crate::keys;
use crate::ui::input::InputBuffer;

pub enum Mode {
    Normal,
    Editing { edit_id: Option<u64> },
    Searching,
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
    DeleteItem(u64),
    CyclePriority(u64, bool),
    Reorder(u64, i32),
    StartSearch,
    ApplySearch,
    ClearSearch,
    Quit,
}

pub struct App {
    data: TodoData,
    selected_index: usize,
    input: InputBuffer,
    mode: Mode,
    clip: Clipboard,
    filter: String,
}

impl App {
    pub fn new() -> Self {
        let data = TodoData::load();
        Self {
            data,
            selected_index: 0,
            input: InputBuffer::new(),
            mode: Mode::Normal,
            clip: Clipboard::new(),
            filter: String::new(),
        }
    }

    pub fn items(&self) -> Vec<TodoItem> {
        let items = self.data.items().to_vec();
        if self.filter.is_empty() {
            items
        } else {
            let q = self.filter.to_lowercase();
            items
                .into_iter()
                .filter(|i| i.text.to_lowercase().contains(&q))
                .collect()
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index.min(
            self.items().len().saturating_sub(1),
        )
    }

    pub fn input(&self) -> &InputBuffer {
        &self.input
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
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
                )
            })?;

            let event = crossterm::event::read()?;
            let action = match event {
                Event::Key(key) => app.dispatch_key(key),
                _ => None,
            };

            if let Some(action) = action {
                let should_quit = app.handle_action(action);
                if should_quit {
                    break;
                }
            }
        }

        Ok(())
    }

    fn dispatch_key(&mut self, key: KeyEvent) -> Option<Action> {
        match &self.mode {
            Mode::Normal => {
                keys::handle_normal(key, &self.data, self.selected_index)
            }
            Mode::Editing { .. } => {
                keys::handle_editing(key, &mut self.input)
            }
            Mode::Searching => {
                keys::handle_search(key, &mut self.filter)
            }
        }
    }

    fn handle_action(&mut self, action: Action) -> bool {
        match action {
            Action::SelectPrev => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            Action::SelectNext => {
                let max = self.items().len().saturating_sub(1);
                if self.selected_index < max {
                    self.selected_index += 1;
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
                                    self.data.add(&text);
                                }
                                Some(id) => {
                                    self.data.update_text(*id, &text);
                                }
                            }
                            self.data.save();
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
                self.data.save();
            }
            Action::DeleteItem(id) => {
                if let Some(item) = self.data.delete(id) {
                    self.clip.cut(item);
                    self.data.save();
                }
                self.clamp_selection();
            }
            Action::CyclePriority(id, forward) => {
                self.data.cycle_priority(id, forward);
                self.data.save();
            }
            Action::Reorder(id, direction) => {
                if self.data.reorder(id, direction) {
                    // Move selection with the item
                    if direction < 0 && self.selected_index > 0 {
                        self.selected_index -= 1;
                    } else if direction > 0 && self.selected_index < self.items().len().saturating_sub(1) {
                        self.selected_index += 1;
                    }
                    self.data.save();
                }
                self.clamp_selection();
            }
            Action::StartSearch => {
                self.filter.clear();
                self.mode = Mode::Searching;
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
            Action::Quit => return true,
        }

        false
    }
}
