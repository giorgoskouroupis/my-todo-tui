# todo-tui

A keyboard-driven terminal todo app built with Ratatui and crossterm.

![todo-tui — main view with items, categories, priorities, and due dates](media/main.png)

## Install With Nix / NixOS

### As a flake input in your NixOS or home-manager config (recommended)

Add to your `flake.nix` inputs:

```nix
inputs = {
  # ... your other inputs
  todo-tui = {
    url = "github:giorgoskouroupis/my-todo-tui/v0.9.0";
    inputs.nixpkgs.follows = "nixpkgs";
  };
};
```

Reference the package in your home-manager (or system) config:

```nix
home.packages = [
  inputs.todo-tui.packages.${pkgs.stdenv.hostPlatform.system}.default
];
```

Then `sudo nixos-rebuild switch --flake .` (or `home-manager switch --flake .`). `todo-tui` will be on your `PATH`.

### One-shot / quick try

```sh
nix run github:giorgoskouroupis/my-todo-tui                # try it without installing
nix profile install github:giorgoskouroupis/my-todo-tui    # install to user profile
```

### From a local clone (for hacking on the code)

```sh
git clone https://github.com/giorgoskouroupis/my-todo-tui
cd my-todo-tui
nix develop --command cargo build --release
./target/release/todo-tui
```

## Install On Debian/Ubuntu

Make sure Rust is installed. On Debian 13+ / Ubuntu 24.04+ you can install `rustup` straight from apt (recommended, keeps you on a current toolchain):

```sh
sudo apt install -y rustup git
rustup default stable
```

On older releases either grab `rustup` from [rustup.rs](https://rustup.rs), or use the apt-provided compiler:

```sh
sudo apt install -y cargo rustc git
```

Then clone and build:

```sh
git clone https://github.com/giorgoskouroupis/my-todo-tui
cd my-todo-tui
cargo build --release
```

Copy the binary somewhere on your `PATH`:

```sh
mkdir -p ~/.local/bin
cp target/release/todo-tui ~/.local/bin/
```

If `~/.local/bin` isn't on your `PATH`, add this line to your `~/.bashrc` (or `~/.zshrc`) and reopen your terminal:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

Now run `todo-tui`.

To update later:

```sh
cd my-todo-tui
git pull
cargo build --release
cp target/release/todo-tui ~/.local/bin/
```

### Build a `.deb` (optional)

```sh
sudo apt install -y dpkg-dev
cargo install cargo-deb
cargo deb
# sudo apt install ./target/debian/todo-tui_*.deb
```

## Core Controls

| Key | Action |
|---|---|
| `Up`/`Down` | Navigate |
| `Enter` | Toggle done |
| `Space` | Toggle doing |
| `Ctrl+E` | Edit selected item, or create one if the list is empty |
| `Delete` | Confirm-delete selected item |
| `Ctrl+P` | Cycle priority |
| `Ctrl+Up` / `Ctrl+Down` | Reorder item |
| `*` / `Ctrl+*` | Toggle pin |
| `Left` / `Right` / `Tab` | Switch between items and category sidebar |
| `PageUp` / `PageDown` | Switch category filter |
| `Ctrl+O` | Assign category to selected item |
| `Ctrl+D` | Open due-date calendar |
| `Ctrl+N` | Open the note log for the selected item |
| `Ctrl+L` | Cycle inline notes: all → newest → hidden |
| `Ctrl+B` | Resize sidebar (`←/→` ±1, `Shift+←/→` ±5, `r` reset, `Enter` save, `Esc` cancel) |
| `/` | Command mode |
| `Ctrl+Z` | Undo last delete or archive (up to 20 levels) |
| `Ctrl+K` | Show keybindings |
| `Ctrl+H` | Show help |
| `Ctrl+C` | Quit |

Any other printable character in the items pane starts a new todo prefilled with that character. In the sidebar it starts a new category.

Command mode supports `/search`, `/delete`, `/done`, `/select`, `/clear`, `/reset`, `/notes`, `/themes`, `/sort`, `/sidebar`, `/help`, and `/keybindings`, plus unified `/filter` subcommands, `/archive` subcommands, `/rename`, and `/move` to open the category picker. `/select` toggles into a generic bulk-select mode: `Space` toggles items/categories, `Ctrl+A` selects all, and `Enter` opens an action popup (`Delete`, `Archive`, `Toggle done`, `Assign category`, plus `Edit` when the selection is a single item) applied to the whole selection.

`/search` types-to-filter live. `↑`/`↓` navigate the filtered list. Item shortcuts work on the highlighted result while typing: `Ctrl+P` cycles priority, `Ctrl+↑/↓` reorder, `Ctrl+*` toggle pin, `Ctrl+D` opens the due-date calendar, `Ctrl+E` edits the item, `Ctrl+O` assigns a category. `Enter` opens the same action popup as `/select` but on the single highlighted item (or press `Tab` first to promote into bulk-select mode on the currently filtered list, where `Space` toggles multiple items). All these round-trip back to the search prompt when the target flow finishes, so the query you were building is never lost. `Esc` clears the filter.

`/themes` shows themes grouped by `Light` and `Dark`. Light: `catppuccin-latte`, `gruvbox-light`, `one-light`, `solarized-light`. Dark: `catppuccin-mocha`, `dracula`, `gruvbox`, `nord`, `one-dark`. Type a command prefix to filter the popup, press `Tab` to autocomplete the highlighted command or subcommand, and press `Enter` to execute it. `/filter category` filters the visible list; `/filter clear` and `/reset` clear filters and restore the default view. `Ctrl+O` assigns/moves the selected item. `/move` assigns the selected item in the items pane, or moves the highlighted category branch in the sidebar. `/archive one` archives the current item in the items pane or the highlighted category branch in the sidebar; from `All`, `/archive one` archives all visible items and categories. `/archive bulk` bulk-selects items or categories based on the active pane, `/archive restore bulk` restores archived items or categories, and `/archive archived` toggles archived view. `/archived` and `/unarchive` remain hidden aliases. `/rename` edits the selected item in the items pane or the highlighted category in the sidebar; `/rename old new` remains available for direct category-path renames. In `/delete`, `Ctrl+A` opens a confirmation popup for all current delete targets. In the items pane it deletes visible items only and keeps category names; in the category pane it deletes all items and all category names.

Categories can be flat (`Dog`) or nested with `/` (`Work/work2`). The sidebar groups nested categories under their parent; selecting the parent includes both direct parent items and nested subcategories. In the sidebar, type a category name directly and press `Enter`; if categories already exist, the placement popup asks whether to attach under the highlighted category or create at root. From `All`, attach opens a parent picker. Manual `Parent/child` entry still works as a shortcut. Global `All` shows full badges like `[Work|work2]`; a parent filter like `Work` shows shorter child badges like `[work2]`.

Due-date calendar keys: `Tab` switches focus between the calendar and the bottom date prompt. The popup opens near the selected item from Normal mode and near the input bar from editing/new-item flows; the rest of the UI is muted while the active calendar or prompt stays highlighted. In calendar focus, arrows move by day/week, `PageUp`/`PageDown` change month, `Ctrl+T` jumps to today, and `Delete` clears. In prompt focus, type digits or `-` to edit `YYYY-MM-DD`, and arrows move the cursor. `Enter` saves a valid date and `Esc` cancels. Due dates render muted by default, yellow within 10 days, and red within 3 days or overdue.

## Notes

Each item carries a note log: an append-only list of dated one-line entries, for recording what actually came out of a task. Mark "call the doctor for a rdv" done and you can still keep the result — `no answer, retry morning`, then `rdv 15/09 10h30, bring the x-ray`. New entries are appended rather than overwriting the previous one, so the history of a task stays readable.

`Ctrl+N` (or `/notes`, or the `Notes` entry in the `/search`/`/select` action popup on a single item) opens the log for the selected item. Inside it:

| Key | Action |
|---|---|
| `Enter` or any printable character | Append a new entry (the character prefills it) |
| `↑` / `↓` | Walk the entries |
| `Ctrl+E` | Edit the highlighted entry |
| `Delete` | Delete the highlighted entry |
| `Ctrl+Y` | Copy the highlighted entry to the system clipboard |
| `Ctrl+Z` | Undo the last deletion (restored at its original position) |
| `Esc` | Close |

Notes also show inline in the list, under the item text they belong to. `Ctrl+L` (or `/notes latest`, `/notes all`, `/notes hidden`) cycles between three modes:

```text
all (default)                       latest                              hidden
- ◆ 📅 2026-09-15  📝 3   [Work]     - ◆ 📅 2026-09-15  📝 3   [Work]     - ◆ 📅 2026-09-15  📝 3   [Work]
  call the doctor for a rdv           call the doctor for a rdv           call the doctor for a rdv
  │ 2026-09-15  rdv 10h30, bring      │ 2026-09-15  rdv 10h30, br…
  │             the x-ray
  │ 2026-09-07  line busy
  │ 2026-09-05  no answer
```

Entries list **newest first**, both inline and in the popup, so the current state of a task is the line nearest its title and the highlight opens on the entry you most likely want. The stored order in `todos.json` stays chronological — only the display is reversed.

`all` is the default and wraps every entry. `latest` keeps one line per item, cut at a word boundary with `…` when there is more to read. `hidden` leaves only the counter, for when the list gets dense. `Ctrl+L` steps down through them in that order, so one press from the default shows less rather than nothing. The choice is saved to `config.json` as `notes_inline`, and the header shows `[notes: latest]` / `[notes: hidden]` when you are not on the default. The three `/notes` arguments work when typed but are not listed in the command palette, so `/notes` + `Enter` still opens the log in one step.

Entries are stamped with the date they were written and capped at 500 characters. Items with notes show a `📝 n` counter on their status row. Marking an item done shows a one-off reminder in the prompt bar when it has no notes yet — the moment you finish something is usually when you have the outcome in hand. Notes ride along with the item everywhere: archive, delete + `Ctrl+Z`, category moves, and rename all preserve them, and existing `todos.json` files load unchanged (items without notes simply start empty).

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
