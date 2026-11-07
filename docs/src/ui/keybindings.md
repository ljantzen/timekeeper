# Layout & Keybindings

## Normal mode

| Key | Action |
|-----|--------|
| `j` / `↓` | Select next entry |
| `k` / `↑` | Select previous entry |
| `s` | Start tracking (opens form) |
| `S` | Start tracking pre-filled from selected entry |
| `n` | Add a manual entry (opens form) |
| `v` | Add a point-in-time event (defaults to now) |
| `x` | Stop active entry |
| `e` | Edit selected entry |
| `E` | Edit active (currently running) entry |
| `d` | Delete selected entry (prompts for confirmation) |
| `c` | Open comments for selected entry |
| `C` | Open comments for active entry |
| `m` | Merge selected entry with the next one |
| `g` | Fill time gaps around the selected entry |
| `G` | Fill time gaps around the active entry |
| `f` | Open filter |
| `T` | Filter to today |
| `Y` | Filter to yesterday |
| `P` | Filter to previous week |
| `N` | Filter to next week |
| `o` | Cycle entry sort order |
| `<` | View previous week in week report |
| `>` | View next week in week report |
| `p` | Manage projects |
| `t` | Manage tasks |
| `r` | Refresh data |
| `:` | Enter command mode |
| `i` | Open settings dialog |
| `?` | Show help |
| `q` / `Esc` | Quit |
| `Ctrl-c` | Quit |

## Manage projects

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

## Manage tasks

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

## Comments panel

| Key | Action |
|-----|--------|
| `j` / `↓` | Select next comment |
| `k` / `↑` | Select previous comment |
| `a` | Add a new comment |
| `e` | Edit selected comment |
| `d` | Delete selected comment |
| `q` / `Esc` | Return to main view |

## Forms (start, edit, filter, add project, add task)

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
| Start | `YYYY-MM-DD HH:MM` or `HH:MM` (today is assumed when only a time is given) |
| End | Same format as Start; leave blank to create an active (running) entry |
| Tags | Comma-separated list |
| Snap to existing activities | Toggle with `Space` — when on, the start and end times are snapped to the nearest boundary of an existing entry |

### Filter dialogs

Accessible via `f` from the main view, the project list, or the task list. Each opens a small form scoped to its context:

- **Entry filter** — project name and date range (e.g. `today`, `this week`, `2024-06-01..2024-06-30`)
- **Project filter** — show or hide archived projects
- **Task filter** — filter by project, show/hide archived and completed tasks

Submit with `Enter` on the last field; cancel with `Esc`.

## Help dialog

Press `?` to open the help reference. The dialog is scrollable when content exceeds the visible area.

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `Page Down` | Scroll down 10 lines |
| `Page Up` | Scroll up 10 lines |
| Any other key | Close |

## Delete confirmation

| Key | Action |
|-----|--------|
| `y` / `Enter` | Confirm delete |
| Any other key | Cancel |
