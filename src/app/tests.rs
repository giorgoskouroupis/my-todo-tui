use std::collections::HashMap;
use std::time::Instant;

use super::{
    get_completions, get_filtered_commands, resolve_command_input, theme_index_by_name, Action,
    App, CategoryPickerTarget, Mode, MultiSelectCmd, Pane, RenameTarget, SortMode,
};
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
        sidebar_width: super::SIDEBAR_WIDTH_DEFAULT,
        note_hint: false,
        category_selection_memory: HashMap::new(),
        popup_back_stack: Vec::new(),
        undo_stack: Vec::new(),
        saved_search: None,
    }
}

#[test]
fn ctrl_z_restores_last_deleted_item() {
    let mut data = TodoData::new();
    let keep = data.add("keep me");
    let victim = data.add("delete me");
    let mut app = test_app(data);

    app.handle_action(Action::DeleteItem(victim));
    app.handle_action(Action::ConfirmDeleteYes);
    assert_eq!(app.data.items().len(), 1);

    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert!(matches!(action, Some(Action::Undo)));
    app.handle_action(action.unwrap());

    assert_eq!(app.data.items().len(), 2);
    assert!(app.data.items().iter().any(|i| i.id == victim && i.text == "delete me"));
    assert!(app.data.items().iter().any(|i| i.id == keep));
}

#[test]
fn ctrl_z_no_op_when_undo_stack_empty() {
    let mut app = test_app(TodoData::new());
    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert!(action.is_none());
}

#[test]
fn ctrl_z_restores_bulk_multiselect_delete() {
    let mut data = TodoData::new();
    let a = data.add("a");
    let b = data.add("b");
    let c = data.add("c");
    let mut app = test_app(data);

    app.mode = Mode::MultiSelect {
        cmd: MultiSelectCmd::Delete,
        selected: [a, b, c].into_iter().collect(),
        selected_categories: Default::default(),
    };
    app.handle_action(Action::ConfirmMultiSelect);
    assert!(matches!(app.mode, Mode::ConfirmDelete { .. }));
    assert_eq!(app.data.items().len(), 3);

    app.handle_action(Action::ConfirmDeleteYes);
    assert!(app.data.items().is_empty());

    app.handle_action(Action::Undo);
    assert_eq!(app.data.items().len(), 3);
    for id in [a, b, c] {
        assert!(app.data.items().iter().any(|i| i.id == id));
    }
}

#[test]
fn bulk_delete_via_enter_routes_through_confirm_popup() {
    let mut data = TodoData::new();
    let a = data.add("keep");
    let b = data.add("victim");
    let mut app = test_app(data);
    app.mode = Mode::MultiSelect {
        cmd: MultiSelectCmd::Delete,
        selected: [b].into_iter().collect(),
        selected_categories: Default::default(),
    };

    app.handle_action(Action::ConfirmMultiSelect);
    assert!(matches!(app.mode, Mode::ConfirmDelete { .. }));
    assert!(app.data.get(a).is_some());
    assert!(app.data.get(b).is_some());

    app.handle_action(Action::ConfirmDeleteNo);
    assert!(matches!(app.mode, Mode::Normal));
    assert!(app.data.get(b).is_some());
}

#[test]
fn confirm_delete_yes_cascades_categories_to_items() {
    let mut data = TodoData::new();
    let child = data.add("child item");
    data.set_category(child, Some("Work".to_string()));
    data.add_category("Work");
    let mut app = test_app(data);

    app.mode = Mode::MultiSelect {
        cmd: MultiSelectCmd::Delete,
        selected: Default::default(),
        selected_categories: ["Work".to_string()].into_iter().collect(),
    };
    app.handle_action(Action::ConfirmMultiSelect);
    assert!(matches!(app.mode, Mode::ConfirmDelete { .. }));

    app.handle_action(Action::ConfirmDeleteYes);
    assert!(app.data.get(child).is_none());
    assert!(!app.data.categories().contains(&"Work".to_string()));

    app.handle_action(Action::Undo);
    assert!(app.data.get(child).is_some());
}

#[test]
fn ctrl_z_reverses_bulk_archive() {
    let mut data = TodoData::new();
    let a = data.add("a");
    let b = data.add("b");
    let mut app = test_app(data);

    app.mode = Mode::MultiSelect {
        cmd: MultiSelectCmd::Archive,
        selected: [a, b].into_iter().collect(),
        selected_categories: Default::default(),
    };
    app.handle_action(Action::ConfirmMultiSelect);
    assert!(app.data.items().iter().all(|i| i.archived));

    app.handle_action(Action::Undo);
    assert!(app.data.items().iter().all(|i| !i.archived));
}

#[test]
fn ctrl_z_reverses_confirm_delete_archive() {
    let mut data = TodoData::new();
    let id = data.add("victim");
    let mut app = test_app(data);

    app.handle_action(Action::DeleteItem(id));
    app.handle_action(Action::ConfirmDeleteArchive);
    assert!(app.data.get(id).unwrap().archived);

    app.handle_action(Action::Undo);
    assert!(!app.data.get(id).unwrap().archived);
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
            "archive one",
            "archive bulk",
            "archive done",
            "archive all",
            "archive restore",
            "archive restore bulk",
            "archive restore all",
            "archive archived"
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
        Some("archive bulk".to_string())
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
fn ctrl_k_opens_keybindings() {
    let mut app = test_app(TodoData::new());
    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
    assert!(matches!(
        action,
        Some(Action::ExecuteCommand(command)) if command == "keybindings"
    ));
}

#[test]
fn ctrl_h_opens_help_from_normal_mode() {
    let mut app = test_app(TodoData::new());
    app.mode = Mode::Normal;
    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));
    assert!(matches!(
        action,
        Some(Action::ExecuteCommand(command)) if command == "help"
    ));
}

#[test]
fn ctrl_h_deletes_word_back_in_editing_mode() {
    let mut app = test_app(TodoData::new());
    app.mode = Mode::Editing { edit_id: None };
    app.input.insert_str("hello world");
    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));
    assert!(action.is_none());
    assert_eq!(app.input.text(), "hello ");
}

#[test]
fn up_down_arrows_in_search_mode_move_selection_through_filtered_results() {
    let mut data = TodoData::new();
    let _ = data.add("first");
    let _ = data.add("second");
    let _ = data.add("third");
    let mut app = test_app(data);
    app.mode = Mode::Searching;
    app.filter = String::new();
    app.selected_index = 0;

    let down = app.dispatch_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert!(matches!(down, Some(Action::SelectNext)));
    app.handle_action(down.unwrap());
    assert_eq!(app.selected_index, 1);

    let up = app.dispatch_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert!(matches!(up, Some(Action::SelectPrev)));
    app.handle_action(up.unwrap());
    assert_eq!(app.selected_index, 0);
    assert!(matches!(app.mode, Mode::Searching));
}

#[test]
fn typing_in_search_mode_updates_filter_via_input_buffer() {
    let mut app = test_app(TodoData::new());
    app.handle_action(Action::ExecuteCommand("search".to_string()));
    assert!(matches!(app.mode, Mode::Searching));
    assert!(app.input.text().is_empty());

    app.dispatch_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
    app.dispatch_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
    app.dispatch_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));

    assert_eq!(app.input.text(), "foo");
    assert_eq!(app.filter, "foo");
}

#[test]
fn ctrl_backspace_in_search_mode_deletes_previous_word() {
    let mut app = test_app(TodoData::new());
    app.handle_action(Action::ExecuteCommand("search hello world".to_string()));
    assert!(matches!(app.mode, Mode::Searching));
    assert_eq!(app.input.text(), "hello world");

    app.dispatch_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::CONTROL));
    assert_eq!(app.input.text(), "hello ");
    assert_eq!(app.filter, "hello ");
}

#[test]
fn enter_in_search_mode_opens_bulk_action_picker_on_highlighted_item() {
    let mut data = TodoData::new();
    let a = data.add("alpha");
    let _ = data.add("beta");
    let mut app = test_app(data);
    app.mode = Mode::Searching;
    app.input.set_text("alp");
    app.filter = "alp".to_string();
    app.selected_index = 0;

    let action = app.dispatch_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(matches!(action, Some(Action::ApplySearch)));
    app.handle_action(action.unwrap());

    match &app.mode {
        Mode::BulkActionPicker { ids, category_names, .. } => {
            assert_eq!(ids, &vec![a]);
            assert!(category_names.is_empty());
        }
        other => panic!("expected BulkActionPicker, got {:?}", std::mem::discriminant(other)),
    }
    assert_eq!(app.filter, "alp");
    assert!(app.input.text().is_empty());
}

#[test]
fn ctrl_p_in_search_mode_cycles_priority_on_highlighted_item() {
    let mut data = TodoData::new();
    let id = data.add("foo");
    let mut app = test_app(data);
    let before = app.data.get(id).unwrap().priority;

    app.mode = Mode::Searching;
    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    assert!(matches!(action, Some(Action::CyclePriority(item_id, true)) if item_id == id));
    app.handle_action(action.unwrap());

    assert_ne!(app.data.get(id).unwrap().priority, before);
    assert!(matches!(app.mode, Mode::Searching));
}

#[test]
fn bulk_action_from_search_popup_returns_to_search_after_action() {
    let mut data = TodoData::new();
    let a = data.add("alpha");
    let _ = data.add("beta");
    let mut app = test_app(data);
    app.mode = Mode::Searching;
    app.input.set_text("alp");
    app.filter = "alp".to_string();
    app.selected_index = 0;

    app.handle_action(Action::ApplySearch);
    assert!(matches!(app.mode, Mode::BulkActionPicker { .. }));

    let archive_idx = crate::app::bulk_action_labels(true)
        .iter()
        .position(|l| *l == "Archive")
        .unwrap();
    app.handle_action(Action::BulkActionSelect(archive_idx));
    assert!(app.data.get(a).unwrap().archived);
    assert!(matches!(app.mode, Mode::Searching));
    assert_eq!(app.input.text(), "alp");
    assert_eq!(app.filter, "alp");
}

#[test]
fn cancel_bulk_action_popup_from_search_returns_to_search() {
    let mut data = TodoData::new();
    let _ = data.add("alpha");
    let mut app = test_app(data);
    app.mode = Mode::Searching;
    app.input.set_text("alp");
    app.filter = "alp".to_string();
    app.selected_index = 0;

    app.handle_action(Action::ApplySearch);
    assert!(matches!(app.mode, Mode::BulkActionPicker { .. }));

    app.handle_action(Action::CancelBulkActionPicker);
    assert!(matches!(app.mode, Mode::Searching));
    assert_eq!(app.input.text(), "alp");
}

#[test]
fn tab_in_search_mode_transitions_to_multiselect_with_filter_preserved() {
    let mut data = TodoData::new();
    let _ = data.add("alpha");
    let _ = data.add("beta");
    let mut app = test_app(data);
    app.mode = Mode::Searching;
    app.input.set_text("alp");
    app.filter = "alp".to_string();

    app.dispatch_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

    assert!(matches!(
        app.mode,
        Mode::MultiSelect { cmd: MultiSelectCmd::PickAction, .. }
    ));
    assert_eq!(app.filter, "alp");
    assert!(app.input.text().is_empty());
    assert_eq!(app.items().len(), 1);
}

#[test]
fn ctrl_e_in_search_mode_edits_and_restores_search_on_submit() {
    let mut data = TodoData::new();
    let id = data.add("orig");
    let mut app = test_app(data);
    app.mode = Mode::Searching;
    app.input.set_text("or");
    app.filter = "or".to_string();

    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    assert!(matches!(action, Some(Action::EditItem(item_id)) if item_id == id));
    app.handle_action(action.unwrap());

    assert!(matches!(app.mode, Mode::Editing { edit_id: Some(item_id) } if item_id == id));
    assert_eq!(app.input.text(), "orig");
    assert_eq!(app.saved_search.as_deref(), Some("or"));

    app.input.set_text("updated");
    app.handle_action(Action::SubmitEdit);

    assert_eq!(app.data.get(id).unwrap().text, "updated");
    assert!(matches!(app.mode, Mode::Searching));
    assert_eq!(app.input.text(), "or");
    assert_eq!(app.filter, "or");
    assert!(app.saved_search.is_none());
}

#[test]
fn ctrl_e_in_search_mode_restores_search_on_cancel() {
    let mut data = TodoData::new();
    let _ = data.add("orig");
    let mut app = test_app(data);
    app.mode = Mode::Searching;
    app.input.set_text("or");
    app.filter = "or".to_string();

    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    app.handle_action(action.unwrap());

    app.handle_action(Action::CancelEdit);

    assert!(matches!(app.mode, Mode::Searching));
    assert_eq!(app.input.text(), "or");
    assert!(app.saved_search.is_none());
}

#[test]
fn ctrl_o_in_search_mode_opens_category_picker_and_restores_search_on_pick() {
    let mut data = TodoData::new();
    let id = data.add("thing");
    data.add_category("Work");
    let mut app = test_app(data);
    app.mode = Mode::Searching;
    app.input.set_text("th");
    app.filter = "th".to_string();

    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL));
    assert!(matches!(action, Some(Action::OpenCategoryPicker)));
    app.handle_action(action.unwrap());

    assert!(matches!(
        app.mode,
        Mode::CategoryPicker { target: CategoryPickerTarget::AssignItem, .. }
    ));
    assert!(app.input.text().is_empty());
    assert_eq!(app.saved_search.as_deref(), Some("th"));

    let work_idx = app
        .categories()
        .iter()
        .position(|c| c == "Work")
        .map(|i| i + 1)
        .unwrap();
    app.handle_action(Action::CategorySelect(work_idx));

    assert_eq!(
        app.data.get(id).and_then(|item| item.category.as_deref()),
        Some("Work")
    );
    assert!(matches!(app.mode, Mode::Searching));
    assert_eq!(app.input.text(), "th");
    assert!(app.saved_search.is_none());
}

#[test]
fn ctrl_up_in_search_mode_reorders_highlighted_item() {
    let mut data = TodoData::new();
    let a = data.add("a");
    let b = data.add("b");
    let mut app = test_app(data);

    app.mode = Mode::Searching;
    app.selected_index = 1;

    let action = app.dispatch_key(KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL));
    assert!(matches!(action, Some(Action::Reorder(item_id, -1)) if item_id == b));
    app.handle_action(action.unwrap());

    let ids: Vec<u64> = app.items().iter().map(|item| item.id).collect();
    assert_eq!(ids, vec![b, a]);
    assert!(matches!(app.mode, Mode::Searching));
}

#[test]
fn slash_in_editing_mode_types_a_slash_instead_of_opening_command_palette() {
    let mut app = test_app(TodoData::new());
    app.mode = Mode::Editing { edit_id: None };
    app.input.insert_str("foo");

    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));

    assert!(action.is_none());
    assert!(matches!(app.mode, Mode::Editing { edit_id: None }));
    assert_eq!(app.input.text(), "foo/");
}

#[test]
fn up_and_down_arrows_jump_to_start_and_end_in_editing_prompt() {
    let mut app = test_app(TodoData::new());
    app.mode = Mode::Editing { edit_id: None };
    app.input.insert_str("hello");

    app.dispatch_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.input.cursor(), 0);

    app.dispatch_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(app.input.cursor(), "hello".len());
}

#[test]
fn select_command_opens_pick_action_multiselect() {
    let mut app = test_app(TodoData::new());

    app.handle_action(Action::ExecuteCommand("select".to_string()));

    assert!(matches!(
        app.mode,
        Mode::MultiSelect {
            cmd: MultiSelectCmd::PickAction,
            ..
        }
    ));
}

#[test]
fn confirm_pick_action_multiselect_opens_bulk_action_picker() {
    let mut data = TodoData::new();
    let a = data.add("a");
    let b = data.add("b");
    let mut app = test_app(data);
    app.mode = Mode::MultiSelect {
        cmd: MultiSelectCmd::PickAction,
        selected: [a, b].into_iter().collect(),
        selected_categories: Default::default(),
    };

    app.handle_action(Action::ConfirmMultiSelect);

    match &app.mode {
        Mode::BulkActionPicker { ids, category_names, .. } => {
            let mut sorted = ids.clone();
            sorted.sort();
            let mut expected = vec![a, b];
            expected.sort();
            assert_eq!(sorted, expected);
            assert!(category_names.is_empty());
        }
        other => panic!("expected BulkActionPicker, got {:?}", std::mem::discriminant(other)),
    }
}

#[test]
fn confirm_pick_action_multiselect_with_empty_selection_returns_to_normal() {
    let mut app = test_app(TodoData::new());
    app.mode = Mode::MultiSelect {
        cmd: MultiSelectCmd::PickAction,
        selected: Default::default(),
        selected_categories: Default::default(),
    };

    app.handle_action(Action::ConfirmMultiSelect);

    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn bulk_action_delete_opens_confirm_delete_popup() {
    let mut data = TodoData::new();
    let a = data.add("a");
    let mut app = test_app(data);
    let idx = crate::app::bulk_action_labels(true)
        .iter()
        .position(|l| *l == "Delete")
        .unwrap();
    app.mode = Mode::BulkActionPicker {
        ids: vec![a],
        category_names: Vec::new(),
        selected: idx,
    };

    app.handle_action(Action::BulkActionSelect(idx));

    assert!(matches!(app.mode, Mode::ConfirmDelete { .. }));
}

#[test]
fn bulk_action_archive_archives_and_records_undo() {
    let mut data = TodoData::new();
    let a = data.add("a");
    let b = data.add("b");
    let mut app = test_app(data);
    let idx = crate::app::bulk_action_labels(false)
        .iter()
        .position(|l| *l == "Archive")
        .unwrap();
    app.mode = Mode::BulkActionPicker {
        ids: vec![a, b],
        category_names: Vec::new(),
        selected: idx,
    };

    app.handle_action(Action::BulkActionSelect(idx));

    assert!(app.data.get(a).unwrap().archived);
    assert!(app.data.get(b).unwrap().archived);

    app.handle_action(Action::Undo);
    assert!(!app.data.get(a).unwrap().archived);
    assert!(!app.data.get(b).unwrap().archived);
}

#[test]
fn bulk_action_toggle_done_flips_done_flag() {
    let mut data = TodoData::new();
    let a = data.add("a");
    let mut app = test_app(data);
    let idx = crate::app::bulk_action_labels(true)
        .iter()
        .position(|l| *l == "Toggle done")
        .unwrap();
    app.mode = Mode::BulkActionPicker {
        ids: vec![a],
        category_names: Vec::new(),
        selected: idx,
    };

    app.handle_action(Action::BulkActionSelect(idx));

    assert!(app.data.get(a).unwrap().done);
}

#[test]
fn bulk_action_labels_include_edit_only_for_single_item() {
    let single = crate::app::bulk_action_labels(true);
    let multi = crate::app::bulk_action_labels(false);
    assert!(single.contains(&"Edit"));
    assert!(!multi.contains(&"Edit"));
    assert!(single.contains(&"Assign category"));
    assert!(multi.contains(&"Assign category"));
}

#[test]
fn bulk_action_edit_opens_editing_mode_on_single_item() {
    let mut data = TodoData::new();
    let a = data.add("hello");
    let mut app = test_app(data);
    let idx = crate::app::bulk_action_labels(true)
        .iter()
        .position(|l| *l == "Edit")
        .unwrap();
    app.mode = Mode::BulkActionPicker {
        ids: vec![a],
        category_names: Vec::new(),
        selected: idx,
    };

    app.handle_action(Action::BulkActionSelect(idx));

    assert!(matches!(app.mode, Mode::Editing { edit_id: Some(id) } if id == a));
    assert_eq!(app.input.text(), "hello");
}

#[test]
fn bulk_action_edit_is_no_op_with_multiple_ids() {
    let mut data = TodoData::new();
    let a = data.add("a");
    let b = data.add("b");
    let mut app = test_app(data);
    let out_of_range = crate::app::bulk_action_labels(false).len();
    app.mode = Mode::BulkActionPicker {
        ids: vec![a, b],
        category_names: Vec::new(),
        selected: 0,
    };

    app.handle_action(Action::BulkActionSelect(out_of_range));

    assert!(matches!(app.mode, Mode::BulkActionPicker { .. }));
}

#[test]
fn bulk_action_move_opens_category_picker_with_bulk_target() {
    let mut data = TodoData::new();
    let a = data.add("a");
    let b = data.add("b");
    data.add_category("Work");
    let mut app = test_app(data);
    let idx = crate::app::bulk_action_labels(false)
        .iter()
        .position(|l| *l == "Assign category")
        .unwrap();
    app.mode = Mode::BulkActionPicker {
        ids: vec![a, b],
        category_names: Vec::new(),
        selected: idx,
    };

    app.handle_action(Action::BulkActionSelect(idx));

    match &app.mode {
        Mode::CategoryPicker {
            target: CategoryPickerTarget::AssignBulk(ids),
            ..
        } => {
            let mut sorted = ids.clone();
            sorted.sort();
            let mut expected = vec![a, b];
            expected.sort();
            assert_eq!(sorted, expected);
        }
        _ => panic!("expected CategoryPicker with AssignBulk target"),
    }

    let work_idx = app
        .categories()
        .iter()
        .position(|c| c == "Work")
        .map(|i| i + 1)
        .unwrap();
    app.handle_action(Action::CategorySelect(work_idx));

    assert_eq!(
        app.data.get(a).and_then(|item| item.category.as_deref()),
        Some("Work")
    );
    assert_eq!(
        app.data.get(b).and_then(|item| item.category.as_deref()),
        Some("Work")
    );
}

#[test]
fn ctrl_z_after_archive_one_restores_the_archived_item_not_an_older_delete() {
    let mut data = TodoData::new();
    let older = data.add("older delete");
    let target = data.add("archive me");
    let mut app = test_app(data);

    app.handle_action(Action::DeleteItem(older));
    app.handle_action(Action::ConfirmDeleteYes);
    assert!(app.data.get(older).is_none());

    app.handle_action(Action::ExecuteCommand("archive one".to_string()));
    assert!(app.data.get(target).unwrap().archived);

    app.handle_action(Action::Undo);
    assert!(!app.data.get(target).unwrap().archived);
    assert!(app.data.get(older).is_none());
}

#[test]
fn ctrl_z_after_archive_done_restores_completed_items_to_unarchived() {
    let mut data = TodoData::new();
    let a = data.add("a");
    data.toggle_done(a);
    let mut app = test_app(data);

    app.handle_action(Action::ExecuteCommand("archive done".to_string()));
    assert!(app.data.get(a).unwrap().archived);

    app.handle_action(Action::Undo);
    assert!(!app.data.get(a).unwrap().archived);
}

#[test]
fn theme_registry_lists_light_group_first_and_every_entry_has_by_name() {
    let registry = Theme::theme_registry();
    let mut seen_dark = false;
    for (name, is_light) in registry {
        assert!(Theme::by_name(name).is_some(), "missing by_name for {name}");
        if !*is_light {
            seen_dark = true;
        } else {
            assert!(!seen_dark, "light theme {name} appears after a dark one");
        }
    }
    assert!(registry.iter().any(|(_, light)| *light), "no light themes");
    assert!(registry.iter().any(|(_, light)| !*light), "no dark themes");
}

#[test]
fn ctrl_z_after_archive_all_restores_all_visible_items() {
    let mut data = TodoData::new();
    let a = data.add("a");
    let b = data.add("b");
    let mut app = test_app(data);

    app.handle_action(Action::ExecuteCommand("archive all".to_string()));
    assert!(app.data.get(a).unwrap().archived);
    assert!(app.data.get(b).unwrap().archived);

    app.handle_action(Action::Undo);
    assert!(!app.data.get(a).unwrap().archived);
    assert!(!app.data.get(b).unwrap().archived);
}

#[test]
fn ctrl_n_opens_the_note_log_on_the_selected_item() {
    let mut data = TodoData::new();
    let id = data.add("call the doctor for a rdv");
    let mut app = test_app(data);

    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
    assert!(matches!(action, Some(Action::OpenNotes(target)) if target == id));
    app.handle_action(action.unwrap());
    assert!(matches!(app.mode, Mode::Notes { item_id, selected: 0 } if item_id == id));
}

#[test]
fn ctrl_n_is_inert_when_the_list_is_empty() {
    let mut app = test_app(TodoData::new());
    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
    assert!(action.is_none());
}

#[test]
fn typing_in_the_note_log_appends_a_timestamped_entry() {
    let mut data = TodoData::new();
    let id = data.add("call the doctor for a rdv");
    let mut app = test_app(data);

    app.handle_action(Action::OpenNotes(id));
    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    assert!(matches!(action, Some(Action::StartNoteAppendWithChar('r'))));
    app.handle_action(action.unwrap());
    assert_eq!(app.input.text(), "r");

    for c in "dv 15/09".chars() {
        app.dispatch_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    let action = app.dispatch_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(matches!(action, Some(Action::SubmitNote)));
    app.handle_action(action.unwrap());

    let notes = &app.data.get(id).unwrap().notes;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "rdv 15/09");
    assert_eq!(notes[0].created, crate::date::Date::today().iso());
    // Back to browsing, highlighting the entry just written.
    assert!(matches!(app.mode, Mode::Notes { selected: 0, .. }));
    assert!(app.input.is_empty());
}

#[test]
fn empty_note_submit_keeps_the_log_untouched() {
    let mut data = TodoData::new();
    let id = data.add("item");
    let mut app = test_app(data);

    app.handle_action(Action::OpenNotes(id));
    app.handle_action(Action::StartNoteAppendWithChar(' '));
    app.handle_action(Action::SubmitNote);

    assert!(app.data.get(id).unwrap().notes.is_empty());
    assert!(matches!(app.mode, Mode::Notes { .. }));
}

#[test]
fn editing_a_note_replaces_only_that_entry() {
    let mut data = TodoData::new();
    let id = data.add("item");
    data.add_note(id, "first");
    data.add_note(id, "second");
    let mut app = test_app(data);

    app.handle_action(Action::OpenNotes(id));
    // Opens on the newest entry.
    assert!(matches!(app.mode, Mode::Notes { selected: 1, .. }));
    app.handle_action(Action::SelectPrev);
    assert!(matches!(app.mode, Mode::Notes { selected: 0, .. }));

    app.handle_action(Action::StartNoteEdit);
    assert_eq!(app.input.text(), "first");
    app.input.set_text("first, corrected");
    app.handle_action(Action::SubmitNote);

    let notes = &app.data.get(id).unwrap().notes;
    assert_eq!(notes[0].text, "first, corrected");
    assert_eq!(notes[1].text, "second");
    assert!(matches!(app.mode, Mode::Notes { selected: 0, .. }));
}

#[test]
fn cancelling_a_note_edit_leaves_the_entry_alone() {
    let mut data = TodoData::new();
    let id = data.add("item");
    data.add_note(id, "untouched");
    let mut app = test_app(data);

    app.handle_action(Action::OpenNotes(id));
    app.handle_action(Action::StartNoteEdit);
    app.input.set_text("discarded");
    app.handle_action(Action::CancelNote);

    assert_eq!(app.data.get(id).unwrap().notes[0].text, "untouched");
    assert!(app.input.is_empty());
    assert!(matches!(app.mode, Mode::Notes { selected: 0, .. }));
}

#[test]
fn ctrl_z_restores_a_deleted_note_in_place() {
    let mut data = TodoData::new();
    let id = data.add("item");
    data.add_note(id, "first");
    data.add_note(id, "second");
    data.add_note(id, "third");
    let mut app = test_app(data);

    app.handle_action(Action::OpenNotes(id));
    app.handle_action(Action::SelectPrev);
    app.handle_action(Action::DeleteNote);
    let texts: Vec<&str> = app
        .data
        .get(id)
        .unwrap()
        .notes
        .iter()
        .map(|note| note.text.as_str())
        .collect();
    assert_eq!(texts, vec!["first", "third"]);

    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert!(matches!(action, Some(Action::Undo)));
    app.handle_action(action.unwrap());

    let texts: Vec<&str> = app
        .data
        .get(id)
        .unwrap()
        .notes
        .iter()
        .map(|note| note.text.as_str())
        .collect();
    assert_eq!(texts, vec!["first", "second", "third"]);
    assert!(matches!(app.mode, Mode::Notes { selected: 1, .. }));
}

#[test]
fn deleting_the_last_note_clamps_the_highlight() {
    let mut data = TodoData::new();
    let id = data.add("item");
    data.add_note(id, "only");
    let mut app = test_app(data);

    app.handle_action(Action::OpenNotes(id));
    app.handle_action(Action::DeleteNote);

    assert!(app.data.get(id).unwrap().notes.is_empty());
    assert!(matches!(app.mode, Mode::Notes { selected: 0, .. }));
}

#[test]
fn closing_the_note_log_returns_to_the_search_prompt() {
    let mut data = TodoData::new();
    let id = data.add("call the doctor");
    let mut app = test_app(data);

    app.handle_action(Action::ExecuteCommand("search doctor".to_string()));
    assert!(matches!(app.mode, Mode::Searching));

    let action = app.dispatch_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
    assert!(matches!(action, Some(Action::OpenNotes(target)) if target == id));
    app.handle_action(action.unwrap());
    app.handle_action(Action::CloseNotes);

    assert!(matches!(app.mode, Mode::Searching));
    assert_eq!(app.input.text(), "doctor");
    assert_eq!(app.filter, "doctor");
}

#[test]
fn notes_command_opens_the_log_for_the_selected_item() {
    let mut data = TodoData::new();
    data.add("first");
    let second = data.add("second");
    let mut app = test_app(data);
    app.selected_index = 1;

    app.handle_action(Action::ExecuteCommand("notes".to_string()));
    assert!(matches!(app.mode, Mode::Notes { item_id, .. } if item_id == second));
}

#[test]
fn notes_command_in_the_sidebar_is_a_no_op() {
    let mut data = TodoData::new();
    data.add("first");
    let mut app = test_app(data);
    app.pane = Pane::Categories;

    app.handle_action(Action::ExecuteCommand("notes".to_string()));
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn bulk_action_picker_offers_notes_for_a_single_item() {
    let mut data = TodoData::new();
    let id = data.add("item");
    let mut app = test_app(data);

    app.mode = Mode::BulkActionPicker {
        ids: vec![id],
        category_names: Vec::new(),
        selected: 0,
    };
    let labels = super::bulk_action_labels(true);
    let idx = labels.iter().position(|label| *label == "Notes").unwrap();
    app.handle_action(Action::BulkActionSelect(idx));

    assert!(matches!(app.mode, Mode::Notes { item_id, .. } if item_id == id));
    assert!(!super::bulk_action_labels(false).contains(&"Notes"));
}

#[test]
fn marking_an_item_done_hints_at_logging_the_outcome() {
    let mut data = TodoData::new();
    let id = data.add("call for a rdv");
    let mut app = test_app(data);

    app.handle_action(Action::ToggleDone(id));
    assert!(app.note_hint);

    // Any other action drops the hint, and it never returns for an item
    // that already carries a note.
    app.handle_action(Action::SelectNext);
    assert!(!app.note_hint);

    app.data.add_note(id, "rdv booked");
    app.handle_action(Action::ToggleDone(id));
    app.handle_action(Action::ToggleDone(id));
    assert!(!app.note_hint);
}
