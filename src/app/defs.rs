use std::collections::HashSet;

use crate::date::Date;
use crate::ui::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Default,
    Priority,
    DueDate,
}

/// How much of an item's note log is shown inline in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoteDisplay {
    /// Only the `📝 n` counter on the status row.
    Hidden,
    /// The newest entry, truncated to a single line.
    Latest,
    /// Every entry, wrapped. The default: notes exist to be read, and an item
    /// realistically carries a handful, not dozens.
    #[default]
    All,
}

impl NoteDisplay {
    /// Steps down in verbosity, so one press from the default shows less
    /// rather than nothing.
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Latest,
            Self::Latest => Self::Hidden,
            Self::Hidden => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::Latest => "latest",
            Self::All => "all",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "hidden" => Some(Self::Hidden),
            "latest" => Some(Self::Latest),
            "all" => Some(Self::All),
            _ => None,
        }
    }
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
    PickAction,
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
    BulkActionPicker {
        ids: Vec<u64>,
        category_names: Vec<String>,
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
    ResizeSidebar {
        return_pane: Pane,
        original_width: u16,
    },
    Notes {
        item_id: u64,
        selected: usize,
    },
    NoteInput {
        item_id: u64,
        edit_index: Option<usize>,
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
            | Self::BulkActionPicker { selected, .. }
            | Self::FilterPicker { selected }
            | Self::DueDateFilterPicker { selected }
            | Self::Notes { selected, .. } => Some(selected),
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
    AssignBulk(Vec<u64>),
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
    CyclePriority(u64),
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
    BulkActionSelect(usize),
    CancelBulkActionPicker,
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
    Undo,
    Quit,
    OpenNotes(u64),
    CloseNotes,
    CycleNoteDisplay,
    SetNoteDisplay(NoteDisplay),
    StartNoteAppend,
    StartNoteAppendWithChar(char),
    StartNoteEdit,
    SubmitNote,
    CancelNote,
    DeleteNote,
    YankNote,
    EnterResizeSidebar,
    ResizeSidebar(i16),
    ResetSidebarWidth,
    ConfirmResizeSidebar,
    CancelResizeSidebar,
}

pub(super) enum PopupBackTarget {
    Command { input: String, selected: usize },
    FilterPicker { selected: usize },
}
