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
    ("notes", "Notes on the selected item"),
    ("n", "Alias for notes"),
    // `/notes hidden|latest|all` set the inline display; kept out of
    // VISIBLE_COMMANDS so `/notes` + Enter opens the log in one step, the way
    // `/due today` works.
    ("priorities", "Filter by priority"),
    ("p", "Alias for priorities"),
    ("rename", "Rename category path"),
    ("reset", "Clear all filters and restore default view"),
    ("sidebar", "Resize sidebar width (Left/Right, Enter save, Esc cancel)"),
    ("search", "Filter items by text"),
    ("s", "Alias for search"),
    ("select", "Bulk-select items and choose an action"),
    ("sort", "Sort items (priority/due/default)"),
    ("themes", "List available themes"),
    ("unarchive", "Restore archived item/all visible"),
    ("x", "Alias for done"),
];

const VISIBLE_COMMANDS: &[(&str, &str)] = &[
    ("archive", "Archive or restore items/categories"),
    ("archive one", "Archive selected item/category"),
    ("archive bulk", "Bulk-select items/categories to archive"),
    ("archive done", "Archive completed items"),
    ("archive all", "Archive all visible items/categories"),
    ("archive restore", "Restore selected archived item/category"),
    (
        "archive restore bulk",
        "Bulk-select archived items/categories to restore",
    ),
    (
        "archive restore all",
        "Restore all visible archived items/categories",
    ),
    ("archive archived", "Toggle archived view"),
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
    ("notes", "Notes on the selected item"),
    ("rename", "Rename category path"),
    ("reset", "Clear all filters"),
    ("sidebar", "Resize sidebar width"),
    ("search", "Filter items by text"),
    ("select", "Bulk-select items and choose an action"),
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

pub fn get_completions(prefix: &str) -> Vec<String> {
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

pub fn resolve_command_input(input: &str, selected: usize) -> Option<String> {
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

pub fn has_visible_subcommands(command: &str) -> bool {
    let prefix = format!("{command} ");
    VISIBLE_COMMANDS
        .iter()
        .any(|(name, _)| name.starts_with(&prefix))
}

pub fn contains_whitespace(value: &str) -> bool {
    value.chars().any(char::is_whitespace)
}

fn ends_with_whitespace(value: &str) -> bool {
    value.chars().last().is_some_and(char::is_whitespace)
}
