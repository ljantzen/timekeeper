# tmkpr-ui

Interactive terminal dashboard for tmkpr time tracking, built with [ratatui](https://github.com/ratatui/ratatui). Perfect for real-time visibility, interactive session management, and an intuitive keyboard-driven workflow.

**See the [main README](../README.md) for installation, storage, and general configuration details.**

## When to use tmkpr-ui

- **Interactive tracking** — start/stop timers with visual feedback
- **Quick overview** — see active timers, week totals, and recent entries at a glance
- **Real-time management** — pause, resume, edit, and delete entries on the fly
- **Visual organization** — manage projects and tasks interactively
- **Desktop workflow** — full-featured UI without leaving the terminal

## Launch

```bash
tmkpr-ui
```

### Overrides

```bash
# Database
tmkpr-ui --db /path/to/other.db
TMKPR_DB=/path/to/other.db tmkpr-ui

# Theme
tmkpr-ui --theme catppuccin_mocha
TMKPR_THEME=dracula tmkpr-ui
```

## Layout

The dashboard shows:

- **Active entry** — currently running timer with elapsed time (top); includes running task in week total
- **Entry list** — the 50 most recent completed entries
- **Week report** — total time per day and project for the displayed ISO week (sidebar); navigate with `<` and `>`
- **Status bar** — feedback messages and error hints (bottom)

## Keybindings

### Normal mode

| Key | Action |
|-----|--------|
| `j` / `↓` | Select next entry |
| `k` / `↑` | Select previous entry |
| `s` | Start tracking (opens form) |
| `S` | Start tracking pre-filled from selected entry |
| `n` | Add a manual entry (opens form) |
| `x` | Stop active entry |
| `e` | Edit selected entry |
| `d` | Delete selected entry (prompts for confirmation) |
| `c` | Open comments for selected entry |
| `C` | Open comments for active entry |
| `m` | Merge selected entry with the next one |
| `g` | Fill time gaps around the selected entry |
| `G` | Fill time gaps around the active entry |
| `f` | Open filter |
| `T` | Filter to today |
| `Y` | Filter to yesterday |
| `W` | Filter to this week |
| `o` | Cycle entry sort order |
| `<` | View previous week |
| `>` | View next week |
| `p` | Manage projects |
| `t` | Manage tasks |
| `r` | Refresh data |
| `:` | Enter command mode |
| `i` | Open settings dialog |
| `?` | Show help |
| `q` / `Esc` | Quit |
| `Ctrl-c` | Quit |

### Manage projects

| Key | Action |
|-----|--------|
| `j` / `↓` | Select next project |
| `k` / `↑` | Select previous project |
| `a` | Add a new project |
| `e` | Edit selected project |
| `d` | Delete selected project |
| `s` | Cycle sort order |
| `f` | Filter projects |
| `q` / `Esc` | Return to main view |

### Manage tasks

| Key | Action |
|-----|--------|
| `j` / `↓` | Select next task |
| `k` / `↑` | Select previous task |
| `a` | Add a new task |
| `e` | Edit selected task |
| `d` | Delete selected task |
| `c` | Toggle task complete / reactivate |
| `s` | Cycle sort order |
| `f` | Filter tasks |
| `q` / `Esc` | Return to main view |

### Comments panel

| Key | Action |
|-----|--------|
| `j` / `↓` | Select next comment |
| `k` / `↑` | Select previous comment |
| `a` | Add a new comment |
| `e` | Edit selected comment |
| `d` | Delete selected comment |
| `q` / `Esc` | Return to main view |

### Forms (start, edit, filter, add project, add task)

| Key | Action |
|-----|--------|
| `Tab` | Accept highlighted completion (if any), then advance to next field |
| `Shift-Tab` | Move to previous field |
| `↓` | On completion fields: select next suggestion in dropdown |
| `↑` | On completion fields: select previous suggestion in dropdown |
| `Enter` | Accept highlighted completion and advance field; submit on the last field |
| `Space` | On toggle fields: flip `[✓] On` / `[ ] Off` |
| `Esc` | Cancel |

Project and task fields show a dropdown of matching names as you type. Use `↓`/`↑` to highlight a suggestion, then `Tab` or `Enter` to accept it.

### Manual entry form

Press `n` to add a past or off-session entry (only available when no timer is active).

| Field | Notes |
|-------|-------|
| Project | Autocompleted from existing projects; creates a new project on submit if not found |
| Task | Autocompleted from tasks in the selected project; creates a new task on submit if not found |
| Note | Free-text description |
| Start | `YYYY-MM-DD HH:MM` or `HH:MM` (see datetime parsing below) |
| End | Same format as Start; leave blank to create an active (running) entry |
| Tags | Comma-separated list |
| Snap to existing activities | Toggle with `Space` — when on, the start and end times are snapped to the nearest boundary of an existing entry |

#### Datetime field input mode

Date/time fields use **overwrite mode**: typing a character replaces the character at the cursor instead of inserting.
- More intuitive for correcting dates and times
- Separators (`-`, `:`, space) are automatically skipped when navigating

#### Smart datetime parsing

When you submit a manual entry or edit the start/end times, the app intelligently handles datetime pairs:

| Scenario | Example | Result |
|----------|---------|--------|
| Both times, no date | `22:00` to `02:00` | Today 22:00 to tomorrow 02:00 (midnight crossing) |
| One has date, one doesn't | `14:00` to `2025-05-10 17:30` | Both on 2025-05-10 |
| Same date, end before start | `2025-05-10 23:00` to `01:00` | Today 23:00 to tomorrow 01:00 |
| Different dates, end < start | `2025-05-10 17:00` to `2025-05-09` | Error: end time before start time |

When only the start time is provided (no end time), the entry becomes active (running).

### Filter dialogs

Accessible via `f` from the main view, the project list, or the task list. Each opens a small form scoped to its context:

- **Entry filter** — project name and date range (e.g. `today`, `this week`, `2024-06-01..2024-06-30`)
- **Project filter** — show or hide archived projects
- **Task filter** — filter by project, show/hide archived and completed tasks

Submit with `Enter` on the last field; cancel with `Esc`.

### Help dialog

Press `?` to open the help reference. The dialog is scrollable when content exceeds the visible area.

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `Page Down` | Scroll down 10 lines |
| `Page Up` | Scroll up 10 lines |
| Any other key | Close |

### Delete confirmation

| Key | Action |
|-----|--------|
| `y` / `Enter` | Confirm delete |
| Any other key | Cancel |

## Command mode

Press `:` to open the command bar. Use `Tab` / `↓` and `Shift-Tab` / `↑` to cycle through completions, `Enter` to execute, `Esc` to cancel.

| Command | Action |
|---------|--------|
| `theme <name>` | Switch colour theme (live preview while cycling) |
| `set date-format <preset>` | Change date display format |
| `set week-start <day>` | Set first day of the week (e.g. `mon`, `sun`) |
| `config-open` | Open `config.toml` in `$EDITOR` |
| `config-reload` | Reload config from disk |
| `config-write` | Persist current theme and display settings to config |
| `quit` | Quit |

Theme and display changes take effect immediately but are only written to disk with `:config-write`.

## Settings dialog

Press `i` to open the interactive settings dialog. Changes are staged in the dialog and written to disk when you press `s` or `Enter`.

| Key | Action |
|-----|--------|
| `j` / `↓` | Next setting |
| `k` / `↑` | Previous setting |
| `←` / `→` | Cycle value (theme, date format, week start) |
| `Space` / `←` / `→` | Toggle boolean (Obsidian enabled) |
| `Enter` | On text fields: start inline editing; on other rows: save and close |
| `s` | Save and close |
| `Esc` | Cancel (restores any live theme preview) |

### Settings covered

| Setting | Description |
|---------|-------------|
| Theme | Cycles through all built-in and custom themes with live preview |
| Date format | `YYYY-MM-DD HH:MM`, `DD-MM-YYYY HH:MM`, or `MM-DD-YYYY HH:MM` |
| Week start | First day of the week shown in the sidebar report |
| Obsidian enabled | Toggle Obsidian logging on/off |
| Vault directory | Absolute path to your Obsidian vault |
| Activity category | Obsidian heading under which time entries are logged |
| Comment category | Obsidian heading under which comments are logged |
