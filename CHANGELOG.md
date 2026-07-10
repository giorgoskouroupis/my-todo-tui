# v0.7.4 — Correctness fixes and Nix packaging

## Correctness
- **Data-loss fix.** On startup, if `todos.json` fails to parse (partial write, corruption, incompatible field), the file is renamed to `todos.json.corrupt-<unix-ts>` instead of silently overwritten with an empty store.
- **`Date::today()` uses local time.** The daily calendar boundary was UTC; now the process caches the local tz offset from `date +%z` at startup and applies it, so overdue / "within N days" / week filters match the user's calendar.
- **`reorder` no longer wraps `u64`.** Item order swap is now a straight two-value swap; long reorder chains no longer risk wrapping order values around zero.
- **`rename_category` preserves manual order.** v0.7.2 explicitly removed alphabetical sorting of the category list, but `rename_category` and `move_category_branch` still called `.sort()` internally. Both now dedup while preserving first-seen order.
- **Consistent Ctrl+K gating.** Ctrl+K in text-input modes (Editing, RenameInput, CategoryPicker, DueDateCalendar, Searching) was blowing away the input to open the keybindings popup. Now gated with the same `is_text_mode` guard as Ctrl+H.

## Config
- `hex_to_color` accepts 3-char shorthand (`#abc` → `#aabbcc`) in addition to 6-char.

## Nix packaging
- `flake.nix` now exposes `packages.default` and `apps.default` via `rustPlatform.buildRustPackage`, so:
  ```sh
  nix build              # → ./result/bin/todo-tui
  nix run                # build + run
  nix profile install .  # install to user profile
  ```
  Consumer flakes can install via `inputs.todo-tui.packages.x86_64-linux.default`.
- `.gitignore` now covers `result` / `result-*` symlinks.

## Docs
- `README.md` keybindings table synced to actual code (was documenting removed bindings from v0.6.x: `Enter=edit`, `u=undo`, `s=sort`, `Alt+↑↓`, `Ctrl+/`, etc.).
- Removed `BUILD_DEB_INSTALL.md` (superseded by the Debian section in `README.md`).

---

# v0.7.3 — Keybindings popup polish

## Normal mode
- `Backspace` unbound (removed the `CategoryParent` action; category navigation is via `PageUp`/`PageDown` and `←`/`→`).
- `Ctrl+K` → keybindings popup (previously category-assign).
- `Ctrl+O` → category assign / move picker (took over `Ctrl+K`'s old job).

## Keybindings popup
- Two-column independent layout with grouped sections.
- Height bumped by one line to fit the new grouping.
- Confirm-delete section removed from the popup (still documented, just not in the popup).

---

# v0.7.2 — Keybinding modernization

Standalone letter shortcuts and `Alt+` combinations removed in favor of `Ctrl+` bindings. The confirm-delete popup is the only place standalone letters (`Y`/`N`/`A`) remain.

## Removed
- `d` (toggle doing) — `Space` already does it.
- `p` / `P` (cycle priority) — replaced by `Ctrl+P` / `Ctrl+Shift+P`.
- `Alt+P` (cycle priority backward) — replaced by `Ctrl+Shift+P`.
- `Alt+↑` / `Alt+↓` reorder — `Ctrl+↑` / `Ctrl+↓` kept.
- `Ctrl+/` (keybindings popup) — unreliable across terminals; replaced by `Ctrl+K`.
- `Ctrl+PageUp` / `Ctrl+PageDown` distinction — `PageUp`/`PageDown` now navigate only parent categories directly.

## Added / changed
- `Ctrl+↑` / `Ctrl+↓` in the sidebar reorders categories within their siblings.
- `Ctrl+K` (was category assign) → opens keybindings popup.
- `Ctrl+O` → category assign / move picker.
- Calendar "jump to today" moved from `t` → `Ctrl+T`.
- Subcategory reorder at the sibling edge now falls through to the move picker.
- Move / `Ctrl+O` picker for categories shows only root-level parents.

## Data
- Categories no longer sorted alphabetically — the stored vector order is respected, so manual reorder sticks across saves.

---

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
