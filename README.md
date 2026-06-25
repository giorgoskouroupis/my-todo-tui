# todo-tui

A keyboard-driven terminal todo app built with Ratatui and crossterm.

## Build

```sh
cargo build
cargo run
cargo test
```

With Nix:

```sh
nix develop --command cargo build
nix develop --command cargo run
nix develop --command cargo test
```

## Install

```sh
cargo install --path .
todo
```

## Core Controls

| Key | Action |
|---|---|
| `j`/`k` or `Up`/`Down` | Navigate |
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
| `Ctrl+K` | Assign category |
| `Ctrl+D` | Open due-date prompt |
| `/` | Command mode |
| `Ctrl+C` | Quit |

Command mode supports `/search`, `/delete`, `/done`, `/clear`, `/themes`, `/priorities`, `/categories`, `/sort`, `/help`, and `/keybindings`.

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

