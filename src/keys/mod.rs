use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::Action;
use crate::data::TodoData;

pub fn handle_normal(key: KeyEvent, data: &TodoData, selected_index: usize) -> Option<Action> {
    let idx = selected_index;
    let has_selection = idx < data.items().len();
    let selected_id = has_selection.then(|| data.items()[idx].id);

    match key.code {
        KeyCode::Up | KeyCode::Char('k') if !key.modifiers.contains(KeyModifiers::ALT) => {
            Some(Action::SelectPrev)
        }
        KeyCode::Down | KeyCode::Char('j') if !key.modifiers.contains(KeyModifiers::ALT) => {
            Some(Action::SelectNext)
        }
        KeyCode::Enter => {
            if has_selection {
                Some(Action::EditItem(selected_id.unwrap()))
            } else {
                Some(Action::StartNewItem)
            }
        }
        KeyCode::Char(' ') => {
            selected_id.map(Action::ToggleDone)
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            selected_id.map(Action::DeleteItem)
        }
        KeyCode::Char('p') => {
            selected_id.map(|id| Action::CyclePriority(id, true))
        }
        KeyCode::Char('P') | KeyCode::Tab => {
            selected_id.map(|id| Action::CyclePriority(id, false))
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::ALT) => {
            selected_id.map(|id| Action::Reorder(id, -1))
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::ALT) => {
            selected_id.map(|id| Action::Reorder(id, 1))
        }
        KeyCode::Char('/') => Some(Action::StartCommand),
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
        _ => {
            if let KeyCode::Char(c) = key.code {
                if !key.modifiers.contains(KeyModifiers::ALT) && !key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Some(Action::StartNewItemWithChar(c));
                }
            }
            None
        }
    }
}

pub fn handle_command(key: KeyEvent, input: &mut crate::ui::input::InputBuffer) -> Option<Action> {
    match key.code {
        KeyCode::Enter => {
            let cmd = input.text().to_string();
            Some(Action::ExecuteCommand(cmd))
        }
        KeyCode::Esc => {
            input.clear();
            Some(Action::CancelEdit)
        }
        KeyCode::Backspace => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                input.delete_word_back();
            } else {
                input.backspace();
            }
            None
        }
        KeyCode::Delete => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                input.delete_word_forward();
            } else {
                input.delete();
            }
            None
        }
        KeyCode::Left => {
            input.move_left(key.modifiers.contains(KeyModifiers::SHIFT));
            None
        }
        KeyCode::Right => {
            input.move_right(key.modifiers.contains(KeyModifiers::SHIFT));
            None
        }
        KeyCode::Home => {
            input.move_home(key.modifiers.contains(KeyModifiers::SHIFT));
            None
        }
        KeyCode::End => {
            input.move_end(key.modifiers.contains(KeyModifiers::SHIFT));
            None
        }
        _ => {
            if key.modifiers == KeyModifiers::CONTROL {
                match key.code {
                    KeyCode::Left => input.move_word_left(false),
                    KeyCode::Right => input.move_word_right(false),
                    KeyCode::Char('a') => input.move_home(false),
                    KeyCode::Char('e') => input.move_end(false),
                    KeyCode::Char('w') => input.delete_word_back(),
                    _ => {}
                }
                return None;
            }

            if key.modifiers == KeyModifiers::SHIFT {
                match key.code {
                    KeyCode::Left => input.move_left(true),
                    KeyCode::Right => input.move_right(true),
                    KeyCode::Home => input.move_home(true),
                    KeyCode::End => input.move_end(true),
                    _ => {}
                }
                return None;
            }

            if key.modifiers.contains(KeyModifiers::ALT) {
                match key.code {
                    KeyCode::Left => input.move_word_left(false),
                    KeyCode::Right => input.move_word_right(false),
                    _ => {}
                }
                return None;
            }

            if let KeyCode::Char(c) = key.code {
                input.insert_char(c);
            }

            None
        }
    }
}

pub fn handle_multiselect(key: KeyEvent, data: &TodoData, selected_index: usize) -> Option<Action> {
    let idx = selected_index;
    let has_selection = idx < data.items().len();
    let selected_id = has_selection.then(|| data.items()[idx].id);

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Action::SelectPrev),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::SelectNext),
        KeyCode::Char(' ') if selected_id.is_some() => {
            Some(Action::ToggleMultiSelect(selected_id.unwrap()))
        }
        KeyCode::Char('x') if selected_id.is_some() => {
            Some(Action::ToggleMultiSelect(selected_id.unwrap()))
        }
        KeyCode::Enter => Some(Action::ConfirmMultiSelect),
        KeyCode::Esc => Some(Action::CancelMultiSelect),
        KeyCode::Char('q') => Some(Action::Quit),
        _ => None,
    }
}

pub fn handle_editing(key: KeyEvent, input: &mut crate::ui::input::InputBuffer) -> Option<Action> {
    match key.code {
        KeyCode::Enter => Some(Action::SubmitEdit),
        KeyCode::Esc => Some(Action::CancelEdit),
        KeyCode::Backspace => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                input.delete_word_back();
            } else {
                input.backspace();
            }
            None
        }
        KeyCode::Delete => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                input.delete_word_forward();
            } else {
                input.delete();
            }
            None
        }
        KeyCode::Left => {
            input.move_left(key.modifiers.contains(KeyModifiers::SHIFT));
            None
        }
        KeyCode::Right => {
            input.move_right(key.modifiers.contains(KeyModifiers::SHIFT));
            None
        }
        KeyCode::Home => {
            input.move_home(key.modifiers.contains(KeyModifiers::SHIFT));
            None
        }
        KeyCode::End => {
            input.move_end(key.modifiers.contains(KeyModifiers::SHIFT));
            None
        }
        _ => {
            if key.modifiers == KeyModifiers::CONTROL {
                match key.code {
                    KeyCode::Left => input.move_word_left(false),
                    KeyCode::Right => input.move_word_right(false),
                    KeyCode::Char('v') => {}
                    KeyCode::Char('c') => {}
                    KeyCode::Char('x') => {}
                    KeyCode::Char('a') => {
                        input.move_home(false);
                    }
                    KeyCode::Char('e') => {
                        input.move_end(false);
                    }
                    KeyCode::Char('w') => input.delete_word_back(),
                    _ => {}
                }
                return None;
            }

            if key.modifiers == KeyModifiers::SHIFT {
                match key.code {
                    KeyCode::Left => input.move_left(true),
                    KeyCode::Right => input.move_right(true),
                    KeyCode::Home => input.move_home(true),
                    KeyCode::End => input.move_end(true),
                    _ => {}
                }
                return None;
            }

            if key.modifiers.contains(KeyModifiers::ALT) {
                match key.code {
                    KeyCode::Left => input.move_word_left(false),
                    KeyCode::Right => input.move_word_right(false),
                    KeyCode::Char('v') => {}
                    _ => {}
                }
                return None;
            }

            if let KeyCode::Char(c) = key.code {
                input.insert_char(c);
            }

            None
        }
    }
}

pub fn handle_theme_picker(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Action::SelectPrev),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::SelectNext),
        KeyCode::Esc => Some(Action::CancelThemePicker),
        KeyCode::Char('q') => Some(Action::Quit),
        _ => None,
    }
}

pub fn handle_search(key: KeyEvent, query: &mut String) -> Option<Action> {
    match key.code {
        KeyCode::Enter => Some(Action::ApplySearch),
        KeyCode::Esc => Some(Action::ClearSearch),
        KeyCode::Backspace => {
            query.pop();
            None
        }
        KeyCode::Delete => {
            query.pop();
            None
        }
        _ => {
            if let KeyCode::Char(c) = key.code {
                if !key.modifiers.contains(KeyModifiers::CONTROL) && !key.modifiers.contains(KeyModifiers::ALT) {
                    query.push(c);
                }
            }
            None
        }
    }
}
