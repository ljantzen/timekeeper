# Terminal Dashboard (tmkpr-ui)

Interactive terminal dashboard for tmkpr time tracking, built with [ratatui](https://github.com/ratatui/ratatui). Perfect for real-time visibility, interactive session management, and an intuitive keyboard-driven workflow.

## When to use tmkpr-ui

- **Interactive tracking** — start/stop timers with visual feedback
- **Quick overview** — see active timers, week totals, and recent entries at a glance
- **Real-time management** — pause, resume, edit, and delete entries on the fly
- **Visual organization** — manage projects and tasks interactively
- **Desktop workflow** — full-featured UI without leaving the terminal

## Launch

```bash
tmkpr-ui
# or via the CLI
tmkpr ui
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

- **Active entry** — currently running timer with elapsed time (top); includes the running task in the week total
- **Entry list** — the 50 most recent completed entries
- **Week report** — total time per day and project for the displayed ISO week (sidebar); navigate with `<` and `>`
- **Status bar** — feedback messages and error hints (bottom)

Continue to [Layout & Keybindings](./keybindings.md) for the full key reference, [Command Mode](./command-mode.md) for the `:` command bar, and [Settings Dialog](./settings.md) for the in-app settings editor.
