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

    if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE)) {
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

    if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)) {
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
    if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)) {
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

    if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)) {
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

    if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)) {
        app.handle_action(action);
    }

    assert!(matches!(
        app.mode,
        Mode::CategoryFilterPicker { selected: 1 }
    ));

    if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)) {
        app.handle_action(action);
    }

    assert!(matches!(
        app.mode,
        Mode::CategoryFilterPicker { selected: 0 }
    ));

    if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)) {
        app.handle_action(action);
    }

    assert!(matches!(app.mode, Mode::FilterPicker { selected: 2 }));

    if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)) {
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

    if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)) {
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

    if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)) {
        app.handle_action(action);
    }

    assert!(matches!(app.mode, Mode::FilterPicker { selected: 0 }));
}

#[test]
fn backspace_at_initial_filter_popup_returns_to_initial_command_list() {
    let mut app = test_app(TodoData::new());
    app.mode = Mode::FilterPicker { selected: 0 };

    if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)) {
        app.handle_action(action);
    }

    assert_eq!(app.input.text(), "");
    assert!(matches!(app.mode, Mode::Command { selected: 0 }));
}

#[test]
fn backspace_in_root_popup_returns_to_initial_command_list() {
    let mut app = test_app(TodoData::new());
    app.mode = Mode::ArchivePicker { selected: 0 };

    if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)) {
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

    if let Some(action) = app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)) {
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
