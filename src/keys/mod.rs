use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::Action;
use crate::data::{CategoryEntry, TodoItem};

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
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                input.move_word_left();
            } else {
                input.move_left();
            }
            true
        }
        KeyCode::Right => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                input.move_word_right();
            } else {
                input.move_right();
            }
            true
        }
        KeyCode::Home => {
            input.move_home();
            true
        }
        KeyCode::End => {
            input.move_end();
            true
        }
        KeyCode::Char('h') if key.modifiers == KeyModifiers::CONTROL => {
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

fn insert_calendar_digit(input: &mut crate::ui::input::InputBuffer, digit: char) {
    let mut digits: String = input
        .text()
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    if digits.len() >= 8 {
        digits.clear();
    }
    digits.push(digit);

    let mut formatted = String::new();
    for (idx, ch) in digits.chars().take(8).enumerate() {
        if idx == 4 || idx == 6 {
            formatted.push('-');
        }
        formatted.push(ch);
    }
    input.set_text(&formatted);
}

pub fn handle_normal(key: KeyEvent, items: &[TodoItem], selected_index: usize) -> Option<Action> {
    let idx = selected_index;
    let has_selection = idx < items.len();
    let selected_id = has_selection.then(|| items[idx].id);

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Up => {
                return selected_id.map(|id| Action::Reorder(id, -1));
            }
            KeyCode::Down => {
                return selected_id.map(|id| Action::Reorder(id, 1));
            }
            KeyCode::Char('e') => {
                return if has_selection {
                    Some(Action::EditItem(selected_id.unwrap()))
                } else {
                    Some(Action::StartNewItem)
                };
            }
            KeyCode::Char('p') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                return selected_id.map(|id| Action::CyclePriority(id, true));
            }
            KeyCode::Char('P') | KeyCode::Char('p') => {
                return selected_id.map(|id| Action::CyclePriority(id, false));
            }
            KeyCode::Char('*') => {
                return selected_id.map(Action::TogglePin);
            }
            _ => {}
        }
    }
    match key.code {
        KeyCode::Up if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::SelectPrev)
        }
        KeyCode::Down if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::SelectNext)
        }
        KeyCode::Enter => selected_id.map(Action::ToggleDone),
        KeyCode::Char(' ') => selected_id.map(Action::ToggleDoing),
        KeyCode::Delete => selected_id.map(Action::DeleteItem),
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
        KeyCode::Up => Some(Action::SelectPrev),
        KeyCode::Down => Some(Action::SelectNext),
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::SelectAllMultiSelect)
        }
        KeyCode::Char(' ') if selected_id.is_some() => {
            Some(Action::ToggleMultiSelect(selected_id.unwrap()))
        }
        KeyCode::Enter => Some(Action::ConfirmMultiSelect),
        KeyCode::Esc => Some(Action::CancelMultiSelect),
        _ => None,
    }
}

pub fn handle_category_multiselect(
    key: KeyEvent,
    categories: &[CategoryEntry],
    category_index: usize,
) -> Option<Action> {
    let selected_category = || {
        if category_index > 0 && category_index <= categories.len() {
            categories
                .get(category_index - 1)
                .map(|entry| entry.path.clone())
        } else {
            None
        }
    };

    match key.code {
        KeyCode::Up => Some(Action::SelectPrev),
        KeyCode::Down => Some(Action::SelectNext),
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::SelectAllMultiSelect)
        }
        KeyCode::Char(' ') => selected_category().map(Action::ToggleCategoryMultiSelect),
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
            if key.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) {
                if let KeyCode::Char('V') = key.code {
                    return Some(Action::Paste);
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
        KeyCode::Esc | KeyCode::Char('N') | KeyCode::Char('n') => Some(Action::ConfirmDeleteNo),
        KeyCode::Char('A') | KeyCode::Char('a') => Some(Action::ConfirmDeleteArchive),
        _ => None,
    }
}

pub fn handle_theme_picker(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up => Some(Action::SelectPrev),
        KeyCode::Down => Some(Action::SelectNext),
        KeyCode::Esc => Some(Action::CancelThemePicker),
        _ => None,
    }
}

pub fn handle_priority_picker(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up => Some(Action::SelectPrev),
        KeyCode::Down => Some(Action::SelectNext),
        KeyCode::Esc => Some(Action::CancelPriorityPicker),
        _ => None,
    }
}

pub fn handle_archive_picker(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up => Some(Action::SelectPrev),
        KeyCode::Down => Some(Action::SelectNext),
        KeyCode::Esc => Some(Action::CancelArchivePicker),
        _ => None,
    }
}

pub fn handle_filter_picker(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up => Some(Action::SelectPrev),
        KeyCode::Down => Some(Action::SelectNext),
        KeyCode::Esc => Some(Action::CancelFilterPicker),
        _ => None,
    }
}

pub fn handle_due_date_filter_picker(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up => Some(Action::SelectPrev),
        KeyCode::Down => Some(Action::SelectNext),
        KeyCode::Esc => Some(Action::CancelDueDateFilterPicker),
        _ => None,
    }
}

pub fn handle_sidebar(
    key: KeyEvent,
    categories: &[CategoryEntry],
    category_index: usize,
) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Up | KeyCode::Down
                if category_index > 0 && category_index <= categories.len() =>
            {
                let name = categories[category_index - 1].path.clone();
                if name.contains('/') {
                    let dir = if matches!(key.code, KeyCode::Up) { -1 } else { 1 };
                    return Some(Action::ReorderCategory(name, dir));
                }
                return Some(Action::OpenCategoryMovePicker(name));
            }
            KeyCode::Char('k')
                if category_index > 0 && category_index <= categories.len() =>
            {
                let name = categories[category_index - 1].path.clone();
                return Some(Action::OpenCategoryMovePicker(name));
            }
            _ => {}
        }
    }
    match key.code {
        KeyCode::Up => Some(Action::SelectPrev),
        KeyCode::Down => Some(Action::SelectNext),
        KeyCode::Enter => {
            if category_index == 0 || category_index <= categories.len() {
                Some(Action::CategorySelect(category_index))
            } else {
                Some(Action::CategorySelect(0))
            }
        }
        KeyCode::Delete => {
            if category_index > 0 && category_index <= categories.len() {
                let name = categories[category_index - 1].path.clone();
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
        KeyCode::Up => Some(Action::SelectPrev),
        KeyCode::Down => Some(Action::SelectNext),
        KeyCode::Esc => Some(Action::CancelCategoryPicker),
        _ => None,
    }
}

pub fn handle_category_create_choice(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up => Some(Action::SelectPrev),
        KeyCode::Down => Some(Action::SelectNext),
        KeyCode::Char('1') => Some(Action::AddCategoryChoice(0)),
        KeyCode::Char('2') => Some(Action::AddCategoryChoice(1)),
        KeyCode::Enter => Some(Action::AddCategoryChoice(usize::MAX)),
        KeyCode::Esc => Some(Action::CancelCategoryAdd),
        _ => None,
    }
}

pub fn handle_category_parent_picker(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up => Some(Action::SelectPrev),
        KeyCode::Down => Some(Action::SelectNext),
        KeyCode::Enter => Some(Action::SelectCategoryParent(usize::MAX)),
        KeyCode::Esc => Some(Action::CancelCategoryAdd),
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
        KeyCode::Up => Some(Action::SelectPrev),
        KeyCode::Down => Some(Action::SelectNext),
        KeyCode::Esc => Some(Action::CancelSortPicker),
        _ => None,
    }
}

pub fn handle_due_date_calendar(
    key: KeyEvent,
    input: &mut crate::ui::input::InputBuffer,
    prompt_focused: bool,
) -> Option<Action> {
    if key.code == KeyCode::Tab {
        return Some(Action::CalendarToggleFocus);
    }

    if prompt_focused {
        return match key.code {
            KeyCode::Enter => Some(Action::SubmitDueDate),
            KeyCode::Esc => Some(Action::CancelDueDate),
            KeyCode::Char(c)
                if (c.is_ascii_digit() || c == '-')
                    && !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                input.insert_char(c);
                Some(Action::CalendarTextChanged)
            }
            KeyCode::Char(_) => None,
            _ => {
                if handle_text_input_key(key, input) {
                    Some(Action::CalendarTextChanged)
                } else {
                    None
                }
            }
        };
    }

    match key.code {
        KeyCode::Left => Some(Action::CalendarMove(-1)),
        KeyCode::Right => Some(Action::CalendarMove(1)),
        KeyCode::Up => Some(Action::CalendarMove(-7)),
        KeyCode::Down => Some(Action::CalendarMove(7)),
        KeyCode::PageUp => Some(Action::CalendarMonth(-1)),
        KeyCode::PageDown => Some(Action::CalendarMonth(1)),
        KeyCode::Char('t') if key.modifiers == KeyModifiers::CONTROL => Some(Action::CalendarToday),
        KeyCode::Delete => Some(Action::CalendarClear),
        KeyCode::Enter => Some(Action::SubmitDueDate),
        KeyCode::Esc => Some(Action::CancelDueDate),
        KeyCode::Char(c)
            if c.is_ascii_digit()
                && !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT) =>
        {
            insert_calendar_digit(input, c);
            Some(Action::CalendarTextChanged)
        }
        KeyCode::Char('-')
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT) =>
        {
            input.clear();
            input.insert_char('-');
            Some(Action::CalendarTextChanged)
        }
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
