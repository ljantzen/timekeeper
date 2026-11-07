# Obsidian Integration

Automatically log your time tracking activities to your Obsidian vault. This feature is entirely optional and works with all three tools: CLI, TUI, and Pomodoro timer.

## Setup

1. Edit `~/.config/tmkpr/config.toml` and add the `[obsidian]` section:

```toml
[obsidian]
enabled = true
vault_dir = "/path/to/your/obsidian/vault"
activity_category = "work"      # optional: category for time entries
comment_category = "notes"      # optional: category for comments
```

2. Create the vault directory if it doesn't exist and add an `obsidian-logging.yaml` config to your vault (see the [obsidian-logging documentation](https://github.com/ljantzen/obsidian-logging) for details).

## What gets logged

| Event | Format | Example |
|-------|--------|---------|
| Activity Started | `[STARTED] project / task (duration)` | `[STARTED] Backend / Feature Dev (active)` |
| Activity Stopped | `[STOPPED] project / task (duration)` | `[STOPPED] Backend / Bug Fixes (2h 30m)` |
| Activity Edited | `[EDITED] project / task (duration)` | `[EDITED] Frontend / Review (1h 15m)` |
| Activity Merged | `[MERGED] project / task (duration)` | `[MERGED] Backend / Testing (45m)` |
| Activity Deleted | `[DELETED] project / task (duration)` | `[DELETED] Support / Call (30m)` |
| Comment Added | Comment entry | `Comment: Fixed the bug` |
| Project Created | `[CREATED] project_name` | `[CREATED] Backend API` |
| Project Updated | `[UPDATED] project_name` | `[UPDATED] Backend API` |
| Project Deleted | `[DELETED] project_name` | `[DELETED] Archived Project` |
| Task Created | `[CREATED] project / task` | `[CREATED] Backend / Feature Dev` |
| Task Updated | `[UPDATED] project / task` | `[UPDATED] Backend / Feature Dev` |
| Task Completed | `[COMPLETED] project / task` | `[COMPLETED] Backend / Bug Fixes` |
| Task Deleted | `[DELETED] project / task` | `[DELETED] Support / Call` |

### Action types

Activities use these action prefixes:

- `[STARTED]` — activity started (active timer)
- `[STOPPED]` — activity stopped (completed with duration)
- `[EDITED]` — activity edited (retroactive logging or reactivation)
- `[COMPLETED]` — activity marked complete
- `[MERGED]` — multiple activities merged into one
- `[DELETED]` — activity deleted

Tasks and projects use similar action types:

- `[CREATED]` — new task or project created
- `[UPDATED]` — task or project name/description changed
- `[COMPLETED]` — task marked complete
- `[DELETED]` — task or project deleted

## Categories

- If `activity_category` is not specified, activities are logged to your vault's default section.
- If `comment_category` is not specified, comments are logged to your vault's default section.
- Project and task operations are always logged to the default section.

## Behavior

- Logging is **completely optional** — disable by setting `enabled = false` or omitting the `[obsidian]` section entirely.
- If the vault directory doesn't exist, logging silently fails with no error messages.
- All logging is **non-blocking** and doesn't affect performance.
- Works with all three tools: CLI, TUI, and Pomodoro timer.
- Activity logs include project and task names when available.
