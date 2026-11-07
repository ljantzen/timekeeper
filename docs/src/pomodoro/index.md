# Pomodoro Timer (tmkpr-pomodoro)

Terminal-based Pomodoro timer integrated with the tmkpr time tracking database. Manage focused work sessions with built-in breaks, automatic logging, and notifications.

## When to use tmkpr-pomodoro

- **Focused work sessions** — structure your day with timed intervals
- **Break reminders** — automatic breaks to prevent burnout
- **Session logging** — automatically record completed Pomodoros to the database
- **Notifications** — audio and desktop alerts for transitions
- **Distraction-free** — stay in the terminal, avoid context switching

## Features

- **Pomodoro timer**: 25-minute work sessions with 5-minute breaks (configurable)
- **Long breaks**: automatic longer breaks after N work sessions (default: 15 minutes after 4 sessions)
- **Project & task management**: select from, create, edit, and delete projects/tasks stored in your tmkpr database
- **Task filtering**: hide or show completed tasks with a single keypress
- **Session logging**: automatically log completed work sessions to the database
- **Configurable notifications**: audio file playback and/or system desktop notifications on transitions
- **In-app settings**: edit all configuration without leaving the terminal
- **Session progress**: visual feedback on where you are in the work/break cycle

## Starting the timer

```bash
tmkpr-pomodoro
# or via the CLI
tmkpr pomodoro
tmkpr pomo      # alias
tmkpr p25       # alias

# Override database
TMKPR_DB=/path/to/other.db tmkpr-pomodoro

# Override theme
tmkpr-pomodoro --theme nord
TMKPR_THEME=catppuccin_mocha tmkpr-pomodoro
```

## UI layout

```text
┌─ Timer ─────────────────────────────────────────────────────┐
│                12:34 / 25:00 [RUNNING]                      │
│                     Session 2 of 4                           │
└─────────────────────────────────────────────────────────────┘
┌─ Projects ──────────────┬─ Tasks ─────────────┬─ Completed ─┐
│  ▶ Project A            │  ▶ Task 1           │ #1 10:00    │
│    Project B            │    Task 2           │ #2  5:45    │
│    Project C            │    Task 3 ✓         │             │
└─────────────────────────┴─────────────────────┴─────────────┘
┌─ Status ────────────────────────────────────────────────────┐
│  Timer running...                                            │
└─────────────────────────────────────────────────────────────┘
┌─ Help ──────────────────────────────────────────────────────┐
│  ↑↓: Project  |  ←→: Task  |  Enter: Work  |  B: Break      │
│  N: New task  |  P: New project  |  E: Edit  |  D: Delete    │
│  H: Hide completed  |  C: Complete  |  S: Settings  |  Q: Quit
└─────────────────────────────────────────────────────────────┘
```

**Completed task filtering**: the Tasks column title changes when filters are active — `Tasks` (all visible) vs. `Tasks (completed hidden)` (only incomplete tasks shown).

## Database integration

The timer integrates with the tmkpr SQLite database:

- **Reads from**: projects and tasks
- **Writes to**: time entries (when logging sessions with the `L` key)
- **Database path**: configured in `~/.config/tmkpr/config.toml` (`[database]` section), or overridden with `--db` / `TMKPR_DB`

Continue to [Keyboard Controls](./keyboard-controls.md) and [Timer Cycle & Notifications](./timer-cycle.md), or jump to [Configuration](./configuration.md) for the full `[pomodoro]` settings reference.
