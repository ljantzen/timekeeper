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

### Override database path

```bash
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
