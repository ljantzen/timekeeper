# Release Notes

## v1.3.2 — 2026-06-26

### tmkpr-ui
- Project color fields now accept CSS named colors (e.g. `red`, `coral`, `steelblue`) in addition to hex codes. Autocomplete lists all standard CSS color names.
- Status bar messages auto-clear after a configurable timeout (default 5 seconds). Set `status_timeout_secs` under `[display]` in `config.toml`, or adjust it in the Settings dialog. Set to `0` to disable.
- Changing the project in a start/edit modal now resets the task field. Switching back to the original project restores the original task.
- Audio feature enabled by default in release binaries and `just build`. Build without audio using `just build audio=false` or `cargo build --no-default-features`.

---

## v1.3.1 — 2026-06-25

### tmkpr-pomodoro
- Audio support is now an optional feature flag (`--features audio`) rather than always required. Binaries without audio fall back to a terminal bell. This removes the hard dependency on `libasound2-dev` / `alsa-lib-devel` for users who don't need audio.

---

## v1.3.0 — 2026-06-24

### tmkpr-cli
- `tmkpr tag list` — list all tags used across entries.
- `tmkpr tag edit --add-tag`/`--remove-tag` — add or remove tags from an entry.
- `tmkpr event list` — list point-in-time events.
- `tmkpr report --tag` — filter the report by tag.
- `tmkpr log --duration` — specify entry duration directly instead of an end time.
- `tmkpr merge --prev` — merge the selected entry with the previous one.
- `tmkpr config show` — print the resolved configuration.
- `tmkpr comment add` now accepts `--entry` to target a specific entry.

### tmkpr-ui
- Week navigation replaced the fixed "this week" filter: press `P`/`N` to move backward/forward through weeks.
- Comment fields are now multi-line.
- Event list rows are aligned with activity rows.

---

## v1.2.0 — 2026-06-19

### tmkpr-ui
- Point-in-time events: press `v` to add an event (a zero-duration marker). Events are visually distinct from activity entries.
- Press `E` to edit the currently active entry inline.
- TUI event operations (add, edit, delete) are logged to Obsidian.
- TUI settings dialog shows the app version and build commit.
- Fixed an off-by-one in the date filter's upper bound that could include entries from the day after.
- Fixed Up/Down inconsistency in form fields.

### tmkpr-cli
- `tmkpr event` subcommand with `add`, `edit`, and `delete` operations.
- Event operations are logged to Obsidian.

### General
- `justfile` added for common dev targets (`build`, `test`, `ui`, `pomo`, `install`).

---

## v1.1.10 — 2026-06-05

### tmkpr-ui
- Settings dialog: theme, date format, week start, and Obsidian configuration are now editable in-app (`:` command mode → `set`, or the new `S` key shortcut).
- Toggle fields (checkbox-style booleans) in forms.
- Manual time entry: create a past activity by specifying start and end times. Optionally snap start/end to the nearest existing entry boundary.
- Help dialog is now scrollable.
- `hide_completed_tasks` setting is persisted across sessions.
- Task queue in pomodoro: queue the next task while the current one is running.

---

## v1.1.9 — 2026-06-04

### tmkpr-pomodoro
- Full project and task management (add, edit, delete) inside the pomodoro client.
- Toggle visibility of completed tasks.

### General
- Obsidian logging: activity, task, and project operations write structured action-log entries to your vault.
- Block cursor shown when editing text fields.

---

## v1.1.8 — 2026-06-02

- Dependency bumps: rusqlite 0.39 → 0.40, rodio 0.17 → 0.22.

---

## v1.1.7 — 2026-05-27

### tmkpr-ui
- Weekly summary panel: navigate weeks with `<`/`>`.
- Current active task is included in the weekly summary.

---

## v1.1.6 — 2026-05-24

### tmkpr-cli
- `tmkpr ui` and `tmkpr pomodoro` subcommands launch the sibling TUI tools.

---

## v1.1.5 — 2026-05-24

### tmkpr-ui
- New themes: `high_contrast`, `light`, `cobalt`, `matrix`.

### tmkpr-cli
- `tmkpr import` / `tmkpr export` — import and export entries as CSV or JSON. `import` accepts stdin.

---

## v1.1.4 — 2026-05-23

- Release script fix only.

---

## v1.1.3 — 2026-05-23

### tmkpr-ui
- Helix-style command mode (`:` key): live theme picker, `:set` to change settings, `:config-write` to persist, tab-complete for command names and theme names.
- Colour themes for the TUI: `default`, `rose_pine`, `catppuccin_*`, `nord`, `gruvbox_dark`, `monokai`, `dracula`, `tokyonight`, `onedark`, `solarized_dark`, `github_dark`, `kanagawa`, `everforest`, `ayu_dark`. Custom themes configurable in `config.toml`.
- Filter to hide completed tasks by default (configurable).
- Inline comment editing in the entries list.
- Task autocomplete filtered by the selected project.
- Theme colors applied consistently to borders, selection, backgrounds, and timestamps.

### tmkpr-pomodoro
- Colour theme support matching tmkpr-ui themes.

---

## v1.1.1 — 2026-05-20

### General
- Task completion: mark tasks done via CLI (`tmkpr task complete`), TUI, or pomodoro. Completed tasks are blocked from new entries.

### tmkpr-pomodoro
- Inline new-task creation inside the pomodoro session.
- Completed sessions list shown in the UI.
- Project colours applied to the timer block.
- Active project and task displayed in the timer block.
- Configurable audio file playback on session transitions (replaces terminal bell).
- `max_cycles` setting to stop after a fixed number of pomodoro cycles.

---

## v1.1.0 — 2026-05-19

### tmkpr-pomodoro (new)
- Standalone TUI pomodoro timer that integrates with the tmkpr database.
- Configurable work/break/long-break durations and session counts.
- In-TUI settings dialog.
- Desktop notifications on session transitions.
- Auto-start break option.
- Manual break start with `B`.

### tmkpr-ui
- Gap-fill keybindings: `g` fills the gap before the selected entry, `G` fills all gaps for the day.
- `m` merges two consecutive entries.
- `S` (shift) starts a new entry copying fields from the selected entry.
- Entries list filter and sort settings are remembered across sessions.
- Comment indicator shown on entries that have attached comments.
- Filtering by project and date in the entries list.
- Sorting and filtering in the task and project management lists.
- Auto-create task if it doesn't exist when starting an entry.
- `C` shows existing comments for the active entry before adding a new one.
- Project hex colors shown in the entry list.

### tmkpr-cli
- `tmkpr merge` — merge two consecutive entries.
- `tmkpr fill-gaps` — fill gaps in the day's timeline.

---

## v1.0.3 — 2026-05-16

- CI/release script fixes.

## v1.0.2 — 2026-05-16

- CI/release script fixes.

## v1.0.1 — 2026-05-16

Initial public release.

### tmkpr-cli
- `start`, `stop`, `log`, `list`, `report`, `comment`, `project`, `task` commands.
- Shell completions for project and task names.
- `--format json/csv` output for `list`.
- `--gaps` / `--min-gap` to list or hide short gaps.
- Week report (`--wweek`, `--year`).
- Tags support: `--tag` on `log` and `list`.
- Markdown output format for `list` and `report`.
- UTF-8 styled output.

### tmkpr-ui
- Terminal UI with entries list, project/task management, and weekly summary.
- Start/stop/edit entries, add comments, attach tags.
- Entry filtering by project and date range.
