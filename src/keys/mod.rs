use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::Action;
use crate::data::{TodoData, TodoItem};

fn handle_text_input_key(key: KeyEvent, input: &mut crate::ui::input::InputBuffer) -> bool {
    match key.code {
        KeyCode::Backspace => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                input.delete_word_back();
            } else {
                input.backspace();
            }
            true
        }
        KeyCode::Delete => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                input.delete_word_forward();
            } else {
                input.delete();
            }
            true
        }
        KeyCode::Left => {
            if key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::ALT)
            {
                input.move_word_left(false);
            } else {
                input.move_left(key.modifiers.contains(KeyModifiers::SHIFT));
            }
            true
        }
        KeyCode::Right => {
            if key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::ALT)
            {
                input.move_word_right(false);
            } else {
                input.move_right(key.modifiers.contains(KeyModifiers::SHIFT));
            }
            true
        }
        KeyCode::Home => {
            input.move_home(key.modifiers.contains(KeyModifiers::SHIFT));
            true
        }
        KeyCode::End => {
            input.move_end(key.modifiers.contains(KeyModifiers::SHIFT));
            true
        }
        KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => {
            input.move_home(false);
            true
        }
        KeyCode::Char('e') if key.modifiers == KeyModifiers::CONTROL => {
            input.move_end(false);
            true
        }
        KeyCode::Char('w') if key.modifiers == KeyModifiers::CONTROL => {
            input.delete_word_back();
            true
        }
        KeyCode::Char(c)
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT) =>
        {
            input.insert_char(c);
            true
        }
        _ => false,
    }
}

pub fn handle_normal(key: KeyEvent, items: &[TodoItem], selected_index: usize) -> Option<Action> {
    let idx = selected_index;
    let has_selection = idx < items.len();
    let selected_id = has_selection.then(|| items[idx].id);

    match key.code {
        KeyCode::Up | KeyCode::Char('k')
            if !key.modifiers.contains(KeyModifiers::ALT)
                && !key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            Some(Action::SelectPrev)
        }
        KeyCode::Down | KeyCode::Char('j')
            if !key.modifiers.contains(KeyModifiers::ALT)
                && !key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            Some(Action::SelectNext)
        }
        KeyCode::Enter => {
            if has_selection {
                Some(Action::EditItem(selected_id.unwrap()))
            } else {
                Some(Action::StartNewItem)
            }
        }
        KeyCode::Char(' ') => selected_id.map(Action::ToggleDone),
        KeyCode::Char('d') if key.modifiers.is_empty() => selected_id.map(Action::ToggleDoing),
        KeyCode::Delete => selected_id.map(Action::DeleteItem),
        KeyCode::Char('p') => selected_id.map(|id| Action::CyclePriority(id, true)),
        KeyCode::Char('P') => selected_id.map(|id| Action::CyclePriority(id, false)),
        KeyCode::Up if key.modifiers.contains(KeyModifiers::ALT) => {
            selected_id.map(|id| Action::Reorder(id, -1))
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::ALT) => {
            selected_id.map(|id| Action::Reorder(id, 1))
        }
        KeyCode::Char('u') => Some(Action::UndoDelete),
        KeyCode::Char('z') if key.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) => {
            Some(Action::UndoDelete)
        }
        KeyCode::Char('Z') if key.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) => {
            Some(Action::UndoDelete)
        }
        KeyCode::Char('s') => Some(Action::StartSortPicker),
        KeyCode::Char('*') => selected_id.map(Action::TogglePin),
        KeyCode::Char('k') if key.modifiers == KeyModifiers::CONTROL => {
            Some(Action::OpenCategoryPicker)
        }
        KeyCode::Char('/') => Some(Action::StartCommand),
        KeyCode::Esc => None,
        _ => {
            if let KeyCode::Char(c) = key.code {
                if !key.modifiers.contains(KeyModifiers::ALT)
                    && !key.modifiers.contains(KeyModifiers::CONTROL)
                {
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
        _ => {
            handle_text_input_key(key, input);
            None
        }
    }
}

pub fn handle_multiselect(
    key: KeyEvent,
    items: &[TodoItem],
    selected_index: usize,
) -> Option<Action> {
    let idx = selected_index;
    let has_selection = idx < items.len();
    let selected_id = has_selection.then(|| items[idx].id);

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
        _ => None,
    }
}

pub fn handle_editing(key: KeyEvent, input: &mut crate::ui::input::InputBuffer) -> Option<Action> {
    match key.code {
        KeyCode::Enter => Some(Action::SubmitEdit),
        KeyCode::Esc => Some(Action::CancelEdit),
        _ => {
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('d') {
                return Some(Action::SetDueDate);
            }

            if key.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) {
                match key.code {
                    KeyCode::Char('C') => return Some(Action::Copy),
                    KeyCode::Char('V') => return Some(Action::Paste),
                    _ => {}
                }
            }

            handle_text_input_key(key, input);
            None
        }
    }
}

pub fn handle_confirm_delete(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Enter | KeyCode::Char('Y') | KeyCode::Char('y') => Some(Action::ConfirmDeleteYes),
        _ => Some(Action::ConfirmDeleteNo),
    }
}

pub fn handle_theme_picker(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Action::SelectPrev),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::SelectNext),
        KeyCode::Esc => Some(Action::CancelThemePicker),
        _ => None,
    }
}

pub fn handle_priority_picker(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Action::SelectPrev),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::SelectNext),
        KeyCode::Esc => Some(Action::CancelPriorityPicker),
        _ => None,
    }
}

pub fn handle_sidebar(key: KeyEvent, data: &TodoData, category_index: usize) -> Option<Action> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Action::SelectPrev),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::SelectNext),
        KeyCode::Enter => {
            let cats = data.categories();
            let cats = cats.iter().map(|s| s.as_str()).collect::<Vec<_>>();
            if category_index == 0 || category_index <= cats.len() {
                Some(Action::CategorySelect(category_index))
            } else {
                Some(Action::CategorySelect(0))
            }
        }
        KeyCode::Char('a') => Some(Action::StartCategoryAdd),
        KeyCode::Delete => {
            let cats = data.categories();
            if category_index > 0 && category_index <= cats.len() {
                let name = cats[category_index - 1].clone();
                Some(Action::DeleteCategory(name))
            } else {
                None
            }
        }
        KeyCode::Esc => None,
        _ => {
            if let KeyCode::Char(c) = key.code {
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                {
                    return Some(Action::StartCategoryAddWithChar(c));
                }
            }
            None
        }
    }
}

pub fn handle_category_picker(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Action::SelectPrev),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::SelectNext),
        KeyCode::Esc => Some(Action::CancelCategoryPicker),
        _ => None,
    }
}

pub fn handle_category_add(
    key: KeyEvent,
    input: &mut crate::ui::input::InputBuffer,
) -> Option<Action> {
    match key.code {
        KeyCode::Enter => {
            let text = input.text().to_string();
            Some(Action::AddCategory(text))
        }
        KeyCode::Esc => {
            input.clear();
            Some(Action::CancelCategoryAdd)
        }
        _ => {
            handle_text_input_key(key, input);
            None
        }
    }
}

pub fn handle_sort_picker(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Action::SelectPrev),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::SelectNext),
        KeyCode::Char('p') => Some(Action::SortSelect(0)),
        KeyCode::Char('d') => Some(Action::SortSelect(1)),
        KeyCode::Char('n') => Some(Action::SortSelect(2)),
        KeyCode::Esc => Some(Action::CancelSortPicker),
        _ => None,
    }
}

pub fn handle_due_date_input(
    key: KeyEvent,
    input: &mut crate::ui::input::InputBuffer,
) -> Option<Action> {
    match key.code {
        KeyCode::Enter => Some(Action::SubmitDueDate),
        KeyCode::Esc => {
            input.clear();
            Some(Action::CancelDueDate)
        }
        _ => {
            handle_text_input_key(key, input);
            None
        }
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
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                {
                    query.push(c);
                }
            }
            None
        }
    }
}
