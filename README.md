# todo-tui

A keyboard-driven terminal todo app built with Ratatui and crossterm.

## Build On Debian/Ubuntu

```sh
sudo apt update
sudo apt install -y cargo rustc

cargo test
cargo build --release
./target/release/todo-tui
```

To install into your user-local prefix:

```sh
cargo install --path . --root ~/.local
~/.local/bin/todo-tui
```

To build a Debian package:

```sh
sudo apt install -y dpkg-dev
cargo install cargo-deb
cargo deb
# sudo apt install ./target/debian/todo-tui_*.deb
```

## Build With Nix/NixOS

```sh
nix develop --command cargo test
nix develop --command cargo build --release
nix develop --command cargo run
```

To install with Cargo from inside the dev shell:

```sh
nix develop --command cargo install --path . --root ~/.local
todo-tui
```

## Core Controls

| Key | Action |
|---|---|
| `Up`/`Down` | Navigate |
| `Enter` | Edit selected item, or create one if the list is empty |
| `Space` | Toggle done |
| `d` | Toggle doing |
| `Delete` | Confirm-delete selected item |
| `p` / `P` | Cycle priority forward/backward |
| `Alt+Up` / `Alt+Down` | Reorder item |
| `u` | Undo last delete |
| `*` | Toggle pin |
| `s` | Sort picker |
| `Left` / `Right` | Switch between items and category sidebar |
| `PageUp` / `PageDown` | Switch category filter from either pane |
| `Ctrl+PageUp` / `Ctrl+PageDown` | Switch only top-level category filters |
| `Ctrl+K` | Assign category |
| `Ctrl+D` | Open due-date calendar |
| `/` | Command mode |
| `Ctrl+/` | Show keybindings |
| `Ctrl+C` | Quit |

Command mode supports `/search`, `/delete`, `/done`, `/clear`, `/reset`, `/themes`, `/sort`, `/help`, and `/keybindings`. v0.6 also adds unified `/filter` subcommands, `/archive` subcommands, `/rename`, and `/move` to open the category picker. Type a command prefix to filter the popup, press `Tab` to autocomplete the highlighted command or subcommand, and press `Enter` to execute it. `/filter category` filters the visible list; `/filter clear` and `/reset` clear filters and restore the default view. `Ctrl+K` assigns/moves the selected item. `/move` assigns the selected item in the items pane, or moves the highlighted category branch in the sidebar. `/archive one` archives the current item in the items pane or the highlighted category branch in the sidebar; from `All`, `/archive one` archives all visible items and categories. `/archive bulk` bulk-selects items or categories based on the active pane, `/archive restore bulk` restores archived items or categories, and `/archive archived` toggles archived view. `/archived` and `/unarchive` remain hidden aliases. `/rename` edits the selected item in the items pane or the highlighted category in the sidebar; `/rename old new` remains available for direct category-path renames. In `/delete`, `Ctrl+A` opens a confirmation popup for all current delete targets. In the items pane it deletes visible items only and keeps category names; in the category pane it deletes all items and all category names.

Categories can be flat (`Dog`) or nested with `/` (`Work/work2`). The sidebar groups nested categories under their parent; selecting the parent includes both direct parent items and nested subcategories. In the sidebar, type a category name directly and press `Enter`; if categories already exist, the placement popup asks whether to attach under the highlighted category or create at root. From `All`, attach opens a parent picker. Manual `Parent/child` entry still works as a shortcut. Global `All` shows full badges like `[Work|work2]`; a parent filter like `Work` shows shorter child badges like `[work2]`.

Due-date calendar keys: `Tab` switches focus between the calendar and the bottom date prompt. The popup opens near the selected item from Normal mode and near the input bar from editing/new-item flows; the rest of the UI is muted while the active calendar or prompt stays highlighted. In calendar focus, arrows move by day/week, `PageUp`/`PageDown` change month, `t` jumps to today, and `Delete` clears. In prompt focus, type digits or `-` to edit `YYYY-MM-DD`, and arrows move the cursor. `Enter` saves a valid date and `Esc` cancels. Due dates render muted by default, yellow within 10 days, and red within 3 days or overdue.

## Storage

Todos are stored as JSON at:

```text
$XDG_STATE_HOME/todo-tui/todos.json
```

If `XDG_STATE_HOME` is unset, the app uses:

```text
$HOME/.local/state/todo-tui/todos.json
```

Theme configuration is stored at:

```text
$XDG_CONFIG_HOME/todo-tui/config.json
```

## Dependencies

The runtime dependency set is intentionally small:

- `ratatui` for TUI layout/rendering
- `crossterm` for terminal control and input events
- `serde` and `serde_json` for local JSON storage/config
- `arboard` with default features disabled for text-only system clipboard support
