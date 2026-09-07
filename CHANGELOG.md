# v0.9.0 — Notes

## New
- **Per-item note log (`Ctrl+N`, `/notes`).** Every item now carries an append-only list of dated one-line entries, for recording what came out of a task rather than just that it happened. Mark "call the doctor for a rdv" done and you can still keep the result — `no answer, retry morning`, then `rdv 15/09 10h30, bring the x-ray`. Entries are appended, never overwritten, so the history of a task stays readable.
  - Inside the popup: `Enter` (or any printable character, which prefills it) appends an entry, `↑`/`↓` walk the log, `Ctrl+E` edits the highlighted entry, `Delete` removes it, `Ctrl+Y` copies it to the system clipboard, `Esc` closes.
  - `Ctrl+Z` undoes a note deletion and restores the entry **at its original position** in the log. It works from inside the notes popup, so the undo is visible where you made the mistake.
  - Reachable three ways: `Ctrl+N` on the selected item (also while `/search`ing — the query round-trips back on close, like `Ctrl+E`/`Ctrl+O` do), the `/notes` command, or the new `Notes` entry in the `/search` / `/select` action popup when the selection is a single item.
  - Items with notes show a `📝 n` counter on their status row, right-aligned badge layout unaffected.
- **Notes show inline in the list, with a three-way toggle.** `Ctrl+L` (or `/notes all|latest|hidden`) steps down from every entry wrapped (the default) to just the newest on one line (cut at a word boundary with `…`) to counter-only for a dense list, so one press shows less rather than nothing. Saved to `config.json` as `notes_inline`; the header shows `[notes: all]` / `[notes: hidden]` when you're off the default. The `/notes` arguments are typed-only and stay out of the command palette, so `/notes` + `Enter` still opens the log in one step rather than a submenu — unlike `/archive` or `/filter`, opening the log dominates the other actions, so it keeps the short path. Entries list newest first, inline and in the popup, so a task's current state is the line nearest its title; `todos.json` keeps them chronological and only the display is reversed, through a single index flip (`note_index_flip`) rather than reversing the stored log. Inline notes sit under the item text, indented behind a `│` gutter — going down rather than into a third column, which would have cost horizontal space *and* still grown the row whenever an item had more notes than wrapped text lines.
  - The due-date calendar's popup anchor accounts for the extra rows, so it still opens next to the right item when inline notes are on. Row-height counting lives next to the row rendering (`note_row_count` beside `note_lines`) so the two can't drift.
  - Entries are stamped with the date they were written (no new dependency — reuses the existing `Date` code) and capped at 500 characters.
- **A nudge at the moment you have the outcome.** Marking an item done shows a one-off `Ctrl+N to log what came out of it` reminder in the prompt bar, and only when that item has no notes yet. It disappears on the next keystroke — no popup, nothing to dismiss.

## Fixes
- **Dropped `Ctrl+Shift+P` (cycle priority backward).** It never worked: the app runs with legacy key encoding (`PopKeyboardEnhancementFlags`, never pushed), where `Ctrl+Shift+<letter>` is the same byte as `Ctrl+<letter>`, so the guard on `SHIFT` in `handle_normal` could not fire and the chord always cycled forward. Priority now cycles forward only, on `Ctrl+P`, and `Priority::prev` is gone. No new keys depend on `Ctrl+Shift+<letter>` — the inline-notes toggle is `Ctrl+L` for the same reason.
- **Keybindings popup listed the wrong key for "Assign cat" and sidebar "Move".** Both showed `Ctrl+K` (which opens the keybindings popup itself); the actual binding is `Ctrl+O`.

## Storage
- `todos.json` gains an optional `notes` array per item. Existing files load unchanged — items without the field simply start with an empty log. Notes survive delete + `Ctrl+Z`, archive/restore, category moves, and renames, since all of those already round-trip whole items.

---

# v0.8.2 — Fix test build

## Fixes
- **Test suite compiles again.** `App` gained a `sidebar_width` field in v0.8.1 but the `test_app` helper in `src/app/tests.rs` was not updated, so `cargo test` (and `nix build`, which runs the check phase) failed. Runtime binary was unaffected.

---

# v0.8.1 — Resizable sidebar, Tab pane switch, popup wrapping

## New
- **`/sidebar` and `Ctrl+B` resize the sidebar.** Enters a resize mode: `←/→` nudges the separator by 1 column, `Shift+←/→` by 5, `r` resets to the default (22), `Enter` saves the new width to `config.json`, `Esc` cancels and restores the original width. Both borders on either side of the separator highlight while resizing.
- **`Tab` switches between the Categories and Items panes**, mirroring `←/→` in Normal and multi-select modes.

## Fixes
- **Popups now fit on narrow terminals.** The Help popup auto-sizes to its widest line and wraps content that still overflows; the Keybindings popup drops from two columns to one when there isn't room; the Confirm-delete header wraps too. Tested down to ~40-col widths.

---



## New
- **Generic bulk-select mode (`/select`).** Toggle any mix of items and categories with `Space`, hit `Ctrl+A` to select all, then `Enter` opens an action popup with `Delete`, `Archive`, `Toggle done`, `Assign category` (and `Edit` when the selection is a single item). The delete action reuses the same confirmation popup as single delete, so single vs. bulk delete look and behave the same.
  - `Assign category` opens the category picker with the whole selection as target; picking a category (or `None`) reassigns every selected item at once. Existing single-`/move` behaviour is unchanged.
- **`/select` alongside the shortcut commands.** `/delete`, `/done`, `/archive bulk` still work as one-shot shortcuts; `/select` is the discoverable "pick first, then act" entry point.

## Fixes
- **`Ctrl+Z` now reverses every archive path**, not just the delete-popup Archive and `/archive bulk` multi-select. Single `/archive one`, `/archive done`, `/archive all`, `/archive restore …`, and the `Archive` menu picker all push undo entries now, so `Ctrl+Z` restores the most recent action regardless of whether it was a delete or an archive.
- **`/` in text input no longer hijacks the command palette.** Typing `/` mid-word during Editing/Rename/Search/CategoryAdd/CategoryPicker/DueDateCalendar prompts inserts a literal `/` instead of swallowing the buffer and switching to command mode. `/` still opens the command palette from Normal mode.
- **Category badge no longer clips long items.** The `[cat|sub]` badge moved from the end of the wrapped text line to the item's status/marker row and is now **right-aligned** — badges land in a predictable column instead of shifting with the number of pin/due-date icons. When the row is too narrow to fit a right-anchored badge, it falls back to trailing after the due date so nothing ever gets pushed off-screen.
- **Uniform delete confirmation.** Every delete path now flows through the same `ConfirmDelete` popup — single `Delete`, `/delete` + `Enter`, `/delete` + `Ctrl+A`, sidebar category delete, and `/select` → Delete action. Previously `/delete` + `Enter` deleted without confirmation. The popup handler now also cascades deletion of a category to every item inside it (previously items got orphaned with a category label pointing at a removed category), and `Ctrl+Z` restores both categories and their items in one step.

## UI
- **Wrapped item text is justified.** Multi-line item text now expands the inter-word gaps so each wrapped segment fills the full text column (Word-style justify). The final wrap line stays left-aligned so short trailing text doesn't spread across the screen; single-line items are unaffected.
- **`Up` / `Down` in item text prompts jump to line start / end** (mirrors `Home` / `End`). Applies to the new-item, edit-item, and rename prompts.
- **`/search` now shares the standard text-prompt behavior** — Ctrl+←/→ word jump, Home/End, Ctrl+Backspace/Delete word delete, all the usual editing keys just work. The search prompt routes through the same `InputBuffer` + `handle_text_input_key` helper as every other text mode, so typing in Search feels identical to typing an item.
- **`Up` / `Down` in `/search` navigate the filtered list** while you keep typing (fzf-style).
- **`Enter` in `/search` opens the bulk-action popup on the highlighted result** (single-item). Pick `Delete`, `Archive`, `Toggle done`, `Assign category`, or `Edit`. All popup outcomes (including inner Confirm-Delete Yes/No/Archive and the Assign-category picker) round-trip back to the search prompt with the query restored — so search + act + keep searching is a single fluid flow. `Esc` still clears the filter and returns to Normal.
- **Item shortcuts work while searching.** `Ctrl+P` / `Ctrl+Shift+P` cycle priority, `Ctrl+↑/↓` reorder, `Ctrl+*` toggle pin, and `Ctrl+D` opens the due-date calendar — all on the currently highlighted result. `Ctrl+E` (edit item) and `Ctrl+O` (assign category) also work: the search query is stashed on the way in and restored automatically on submit/cancel, so you land back in Search where you left off.
- **`Tab` in `/search` promotes to bulk-select.** Instead of overloading `Ctrl+Space` on top of the typing prompt, `Tab` commits the filter and enters `/select`'s multi-select mode on the currently-filtered list. `Space` naturally becomes toggle (no typing to conflict with), `Ctrl+A` selects all filtered results, `Enter` opens the action popup. Cleanly reuses the existing `/select` semantics — no new hot-fix bindings.
- **Bulk-action popup surfaces `Edit` and renames `Move to category` → `Assign category`.** The Assign action now mirrors `Ctrl+O`'s label; `Edit` appears when the selection is exactly one item (mirrors `Ctrl+E`) and opens the edit prompt on that item. Popup entries are now sorted **alphabetically** so `Edit` no longer sticks at the end — the order is stable regardless of what's shown. Dispatch is label-based so future reorders can't drift.
- **Archive picker groups now read `one → bulk → done → all`** within each of the Archive and Restore groups (was `bulk → done → one → all`). `Archived view` moves to the bottom (state toggle, not an action). Same reorder applied to the `/archive` command-palette subcommand list so both surfaces agree.
- **Popup widths auto-fit their content.** The `/archive` command completions were clipping the longer descriptions (e.g. `Bulk-select archived items/categories to restore` — 48 chars) at the fixed 62-col width. Both the command-palette popup and the menu popups (`Archive` / `Filter` / `Priority` / `Sort` / `Due Filter` / `Bulk action` / `Themes` / `Assign Category` / `Filter Category`) now compute their width from the widest label (and title) they need to show, subject to a per-popup minimum and the terminal width. No more clipped hint text.
- **Symmetric item pane margins.** The text column left 4 cols of dead space on the right and only 2 on the left; now it's 2/2, and the right-aligned category badge sits 2 cols from the pane's right edge.
- Multi-select hint bar now shows `Bulk select: … | Space toggle …, Ctrl+A all, Enter choose action, Esc cancel` when in the new `/select` mode.
- **Search prompt shows an inline hint** when the input is empty: `Search: [type to filter, ↑/↓ nav, Tab bulk-select, Enter action, Esc clear]`. The `Search:` label stays in the accent color and the bracketed hint is warning-colored, matching the styling of other empty prompts. Hint hides itself the moment you start typing.
- **Light themes.** Added `one-light`, `catppuccin-latte`, `solarized-light`, and `gruvbox-light`. The `/themes` picker now groups entries under `Light` / `Dark` headers so both categories are visible at a glance.

---

# v0.7.4 — Correctness fixes and Nix packaging

## New
- **Common-sense undo (`Ctrl+Z`).** The delete-undo removed in v0.7.1 is back, and it now also reverses archive/unarchive actions. Every one of these state changes pushes a snapshot onto a 20-entry undo stack:
  - Confirmed delete (single item, bulk multi-select via `/delete`, `Ctrl+A` all).
  - Archive from the delete popup (`A` key).
  - Bulk archive / bulk restore via `/archive` multi-select.

  `Ctrl+Z` from Normal mode pops the top of the stack. Delete restore reinserts items with their original IDs, category, and order; archive undo flips the archived flag back to its previous state for items and categories.

## UI
- Multi-select hint bar now says `Space toggle items` (was `select items`), so the toggle key is discoverable without opening the keybindings popup.

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
