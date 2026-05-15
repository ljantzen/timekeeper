# tmkpr-ui

Terminal dashboard for tmkpr, built with [ratatui](https://github.com/ratatui/ratatui). See the [top-level README](../README.md) for storage and config details.

## Launch

```
tmkpr-ui
```

Override the database path:

```
tmkpr-ui --db /path/to/other.db
TMKPR_DB=/path/to/other.db tmkpr-ui
```

## Layout

The dashboard shows:

- **Active entry** — currently running timer with elapsed time (top)
- **Entry list** — the 50 most recent completed entries
- **Week report** — total time per project for the current ISO week (sidebar)
- **Status bar** — feedback messages and error hints (bottom)

## Keybindings

### Normal mode

| Key | Action |
|-----|--------|
| `j` / `↓` | Select next entry |
| `k` / `↑` | Select previous entry |
| `s` | Start tracking (opens form) |
| `x` | Stop active entry |
| `e` | Edit selected entry |
| `d` | Delete selected entry (prompts for confirmation) |
| `p` | Add a new project |
| `t` | Add a new task |
| `r` | Refresh data |
| `?` | Show help |
| `q` / `Esc` | Quit |
| `Ctrl-c` | Quit |

### Forms (start, edit, add project, add task)

| Key | Action |
|-----|--------|
| `Tab` | Accept highlighted completion (if any), then advance to next field |
| `Shift-Tab` | Move to previous field |
| `↓` | On completion fields: select next suggestion in dropdown |
| `↑` | On completion fields: select previous suggestion in dropdown |
| `Enter` | Accept highlighted completion and advance field; submit on the last field |
| `Esc` | Cancel |

Project and task fields show a dropdown of matching names as you type. Use `↓`/`↑` to highlight a suggestion, then `Tab` or `Enter` to accept it.

### Delete confirmation

| Key | Action |
|-----|--------|
| `y` / `Enter` | Confirm delete |
| Any other key | Cancel |
