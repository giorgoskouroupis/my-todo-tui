use std::collections::HashSet;

use crate::date::Date;
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

impl Mode {
    pub fn selected_mut(&mut self) -> Option<&mut usize> {
        match self {
            Self::Command { selected }
            | Self::ThemePicker { selected, .. }
            | Self::PriorityPicker { selected }
            | Self::CategoryPicker { selected, .. }
            | Self::CategoryFilterPicker { selected }
            | Self::CategoryCreateChoice { selected, .. }
            | Self::CategoryParentPicker { selected, .. }
            | Self::SortPicker { selected, .. }
            | Self::ArchivePicker { selected }
            | Self::FilterPicker { selected }
            | Self::DueDateFilterPicker { selected } => Some(selected),
            _ => None,
        }
    }
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
    ReorderCategory(String, i32),
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
    Paste,
    TogglePin(u64),
    SetDueDate,
    SwitchPane,
    OpenCategoryPicker,
    OpenCategoryMovePicker(String),
    CategorySelect(usize),
    CategoryFilterSelect(usize),
    CategoryFilterParent(usize),


    AddCategory(String),
    AddCategoryChoice(usize),
    SelectCategoryParent(usize),
    CreateAndAssignCategory(String),
    DeleteCategory(String),
    CancelCategoryPicker,
    StartCategoryAddWithChar(char),
    CancelCategoryAdd,
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

pub(super) enum PopupBackTarget {
    Command { input: String, selected: usize },
    FilterPicker { selected: usize },
}
