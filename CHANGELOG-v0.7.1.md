# v0.7.1 — Keybinding & packaging changes

## Keybinding changes

### Remove entirely
| Key | Was | Reason |
|---|---|---|
| `u` | Undo delete | Removed internal clipboard (`clip.rs`), undo data, and `UndoDelete` action |
| `s` | Open sort picker | Use `/sort` command instead |
| `Ctrl+/` | Keybindings popup | Moved to `Ctrl+/` (was duplicate) |
| `Ctrl+Shift+C` | Copy selection | Removed text selection + clipboard copy |
| `Ctrl+D` in editing | Due date calendar | Only available from Normal mode now |
| `Ctrl+A` | Select all in text input | Removed selection support |
| `Ctrl+E` | Move to end of line | Removed selection support |
| `Ctrl+W` | Delete word left | `Ctrl+Backspace` already does this |
| `Ctrl+H` in text input | (was help) | Now handled globally |
| `Alt+arrows` | Word jump | `Ctrl+arrows` still works |
| `Alt+Backspace`/`Alt+Del` | Word delete | `Ctrl+Backspace`/`Ctrl+Delete` still works |
| `Shift+arrows/Home/End` | Text selection | Removed |
| `Ctrl+_` | Keybindings fallback | Conflicted with Alacritty zoom |
| `ctrl + h/H` (original) | (was search text) | Changed to help popup |

### New / Change
| Key | Action | Notes |
|---|---|---|
| `Ctrl+/` | Keybindings popup | Single binding, no fallback |
| `Ctrl+H` | Help popup | Caught globally before mode-specific handlers |
| `Enter` | Toggle done | Was `Space`, swapped |
| `Space` | Toggle doing | Was `Enter`, swapped |
| `Ctrl+E` | Edit item / create new | New binding |
| `Ctrl+P` | Cycle priority forward | New binding |
| `Alt+P` | Cycle priority backward | Changed from `Ctrl+Shift+P` (unreliable in some terminals) |
| `Ctrl+↑`/`↓` | Reorder item | New binding |
| `Ctrl+*` | Toggle pin | New binding |
| `Ctrl+T` | Jump to today in calendar | New binding |
| `Ctrl+Backspace` | Delete word left (text editing) | Unchanged, still works |
| `Ctrl+Delete` | Delete word right (text editing) | Unchanged, still works |
| `Ctrl+Shift+V` | Paste | Unchanged |
| `Ctrl+Backspace` | Clear due date in calendar | New |


### Keybindings popup content
- Global: `Ctrl+C` quit, `Ctrl+/` keybindings, `Ctrl+H` help, `/` command
- Items: `↑/↓` navigate, `Enter` toggle done, `Space` toggle doing, `Ctrl+E` edit, printable key new item, `Delete` delete, `Backspace` parent filter, `Ctrl+P`/`Alt+P` priority cycle, `Ctrl+↑/↓` reorder, `Ctrl+*` pin, `Ctrl+K` category, `Ctrl+D` due date, `←/→` switch pane, `PgUp/PgDn`/`Ctrl+PgUp/PgDn` filter
- Text input: printable insert, `Enter` submit, `Esc` cancel, `←/→ Home/End` cursor, `Ctrl+←/→` word jump, `
- All other sections kept the same

### Help popup content
- Removed `/<alias>` and `/search <q>` lines
- Shows `/command`, `/help`, `/keybindings`, `/filter`, `/delete`, `/done`, `/clear`, `/archive`, `/move`, `/rename`, `/themes`
- Shows config and data file paths
