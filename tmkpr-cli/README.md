# tmkpr-cli

Full-featured command-line interface for tmkpr time tracking. Use the CLI for scripting, automation, integration with other tools, and fast keyboard-driven workflows.

**See the [main README](../README.md) for installation, storage, and general configuration details.**

## When to use tmkpr-cli

- **Scripting & automation** — pipe commands, integrate with cron jobs, git hooks, etc.
- **Keyboard workflow** — quick commands without GUI overhead
- **Data export** — JSON, CSV, Markdown output formats
- **Structured queries** — precise filtering and reporting
- **CI/CD integration** — programmatic time logging
- **Remote access** — works over SSH without X11

## Quick start

```
tmkpr project add myproject
tmkpr task add coding -p myproject

tmkpr start -p myproject -t coding -n "working on feature X"
tmkpr status
tmkpr stop

tmkpr list
tmkpr report
```

## Commands

### Tracking

```
tmkpr start [-p PROJECT] [-t TASK] [-n NOTE] [-s TIME] [-f] [--tags t1,t2]
tmkpr stop  [-e TIME]
tmkpr status
tmkpr log   [-s START] [-e END | -d DURATION] [-p PROJECT] [-t TASK] [-n NOTE] [--tags t1,t2]
```

`log` (alias: `record`) adds a completed entry directly without a start/stop cycle. `--end` defaults to now if omitted. Use `--duration` instead of `--end` to specify how long the entry lasted (`1h30m`, `90m`, `1:30:00`); `--end` and `--duration` are mutually exclusive. If `--start` is omitted, tmkpr prompts with the end time of the last entry logged today; if there is no such entry, you must provide `--start` explicitly.

Pass `--start continue` (or `--start cont`) to start exactly where the last entry ended, with no prompt.

**Handing off between tasks:** if a session is already running when you run `start`, you will be prompted to stop it first. When `--start` is provided, the active entry is stopped at that time and the new entry starts at the same time — so there is no gap and no overlap:

```
tmkpr start -p projectB -n "context switch" --start "30 minutes ago"
# active entry stopped 30 min ago; new entry started 30 min ago
```

Use `-f` / `--force` to skip the confirmation prompt. It is an error to pass a `--start` time that is earlier than the active entry's start time.

All time flags accept natural language or ISO 8601:

```
tmkpr start -p myproject --start "2 hours ago"
tmkpr stop --end "5 minutes ago"
tmkpr log -s "yesterday 9am" -e "yesterday 5pm" -p myproject -n "deep work"
tmkpr log -s "9am" -d 2h -p myproject -n "morning session"
```

### Entries

```
tmkpr list     [--from TIME] [--until TIME] [-p PROJECT] [-t TASK] [-l LIMIT] [--tag TAG] [--gaps] [--min-gap MINUTES]
tmkpr report   [--from TIME] [--until TIME] [-p PROJECT] [--tag TAG]
tmkpr report   --week [N] [--year YEAR] [--tag TAG]
tmkpr report   --wweek [N] [--year YEAR] [--tag TAG]
tmkpr edit     <ID> [-p PROJECT] [-t TASK] [-n NOTE] [--start TIME] [--end TIME] [--tags t1,t2]
tmkpr delete   <ID> [-y]
tmkpr merge    <ID>
tmkpr fill-gap [ID]
```

`--week` shows a full 7-day ISO week (Mon–Sun). `--wweek` shows only Mon–Fri. Both accept an optional week number and a `--year` flag:

```
tmkpr report --week           # current full week
tmkpr report --wweek          # current working week (Mon–Fri)
tmkpr report --wweek 12       # working week 12 of the current year
tmkpr report --week 12 --year 2025
```

`list` with no `--from`/`--until` shows today's entries only. Pass `--from` to widen the range:

```
tmkpr list --from "last monday"
tmkpr list --from "2025-01-01"
```

`merge` (alias: `m`) joins an entry into the chronologically next entry that shares its project and task.

`fill-gap` (alias: `fg`) extends an entry's start and/or end times to abut adjacent entries on the same day. Omit `ID` to operate on the active entry.

Entry IDs can be abbreviated to any unambiguous prefix (8+ chars).

### Projects

```
tmkpr project add <NAME> [-d DESCRIPTION] [--color HEX]
tmkpr project list [--archived]
tmkpr project edit <NAME|ID> [--name NAME] [-d DESCRIPTION] [--color HEX]
tmkpr project delete <NAME|ID> [--hard]
```

Projects are listed with a numeric ID. Both the name and numeric ID are accepted wherever `NAME|ID` appears.

Deleting a project archives it by default. Use `--hard` to permanently remove it. Pass `-` to `--description` or `--color` to clear those fields.

### Tasks

```
tmkpr task add        <NAME> -p PROJECT [-d DESCRIPTION]
tmkpr task list       -p PROJECT [--archived]
tmkpr task edit       <NAME|ID> -p PROJECT [--name NAME] [-d DESCRIPTION] [--move-to PROJECT]
tmkpr task delete     <NAME|ID> -p PROJECT [--hard]
tmkpr task done       <NAME|ID> -p PROJECT
tmkpr task reactivate <NAME|ID> -p PROJECT
```

`-p` identifies the project the task currently belongs to. Use `--move-to` to reassign it to a different project.

`task delete` archives by default; use `--hard` to permanently remove. `task done` marks a task completed; `task reactivate` reverses that.

### Comments

Add free-form notes to any entry.

```
tmkpr comment add [-e ENTRY-ID] <TEXT...>
tmkpr comment list              # comments on active entry
tmkpr comment list <ENTRY-ID>   # comments on a specific entry
tmkpr comment edit <ID> <TEXT...>
tmkpr comment delete <ID> [-y]
```

`comment add` targets the active entry by default; use `-e` to add to any finished entry:

```
tmkpr comment add just deployed the fix
tmkpr comment add -e abc12345 actually deployed to staging
```

Aliases: `c` for the subcommand, `a` / `ls` / `e` / `d` for the actions. Comment IDs can be abbreviated to any unambiguous prefix (8+ chars).

### Events

Point-in-time events are entries with no duration — useful for marking moments (deploys, meetings, etc.).

```
tmkpr event add    [--at TIME] [-p PROJECT] [-t TASK] [-n NOTE] [--tags t1,t2]
tmkpr event list   [--from TIME] [--until TIME] [-p PROJECT] [--tag TAG] [-l LIMIT]
tmkpr event edit   <ID> [--at TIME] [-p PROJECT] [-t TASK] [-n NOTE] [--tags t1,t2]
tmkpr event delete <ID> [-y]
```

`--at` accepts natural language or ISO 8601 and defaults to now. Events also appear in `tmkpr list` alongside regular entries.

### Import / Export

```
tmkpr import [FILE] [--skip-errors] [--dry-run]
tmkpr export [FILE] [-p PROJECT] [-t TASK] [--from TIME] [--until TIME] [--tag TAG] [--no-active]
```

**Import** reads projects, tasks, and time entries from a CSV or JSON file and creates any missing projects and tasks automatically.

Supported CSV columns (case-insensitive, spaces/hyphens treated as underscores):

| Column | Notes |
|--------|-------|
| `start` | Combined start datetime (required — or split into `start_date` + `start_time`) |
| `start_date`, `start_time` | Alternative split form |
| `end` / `end_date` / `end_time` | End datetime; omit for an active entry |
| `duration` | Duration instead of end time (`1:30:00`, `1h30m`, `90m`) |
| `project` | Project name — created if it doesn't exist |
| `task` | Task name within the project — created if it doesn't exist |
| `note` / `description` / `comment` | Free-text note |
| `tags` | Comma-separated tags |

```bash
tmkpr import entries.csv               # import from file
tmkpr import entries.json              # JSON file (auto-detected by extension)
tmkpr import --dry-run entries.csv     # preview without writing
tmkpr import --skip-errors entries.csv # continue past bad rows
tmkpr import -                         # read CSV from stdin
tmkpr -f json import -                 # read JSON from stdin
```

**Export** writes entries to CSV (default) or JSON. Columns: `project`, `task`, `start`, `end`, `note`, `tags`. Datetimes are written in local time so files round-trip cleanly through `import`.

```bash
tmkpr export                           # all entries to stdout (CSV)
tmkpr export entries.csv               # all entries to file
tmkpr export entries.json              # JSON (auto-detected by extension)
tmkpr -f json export                   # JSON to stdout
tmkpr export --from 2024-01-01 out.csv # date range
tmkpr export -p "My Project" out.csv   # filter by project
tmkpr export | tmkpr import -          # round-trip: export then re-import
```

### Launching other tmkpr tools

`tmkpr ui` and `tmkpr pomodoro` launch the sibling terminal apps without you needing to remember their binary names. All flags are forwarded, including `--db`:

```bash
tmkpr ui                        # launch tmkpr-ui
tmkpr u                         # alias
tmkpr ui --theme dracula        # pass flags through

tmkpr pomodoro                  # launch tmkpr-pomodoro
tmkpr pomo                      # alias
tmkpr p25                       # alias

tmkpr --db /path/to/other.db ui         # --db is forwarded automatically
tmkpr --db /path/to/other.db pomodoro
```

### Shell completion

Dynamic completion (recommended) — includes project and task name suggestions:

```bash
# bash — add to ~/.bashrc
source <(COMPLETE=bash tmkpr)

# zsh — add to ~/.zshrc
source <(COMPLETE=zsh tmkpr)

# fish — add to ~/.config/fish/completions/tmkpr.fish
COMPLETE=fish tmkpr | source
```

Static completion (flags and subcommands only):

```bash
tmkpr completion bash   >> ~/.bashrc
tmkpr completion zsh    >> ~/.zshrc
tmkpr completion fish   > ~/.config/fish/completions/tmkpr.fish
```

## Output format

All list-producing commands accept `--format table` (default), `--format json`, `--format csv`, or `--format markdown`.

```
tmkpr list --format json
tmkpr list --format markdown
tmkpr report --format csv
tmkpr report --format markdown
tmkpr project list --format json
tmkpr task list -p myproject --format csv
```

`project list --format json` includes a `tasks` array on each project object, making it suitable for scripting:

```bash
tmkpr project list --format json \
  | jq -r '.[] | . as $p | .tasks[] | "\($p.name) / \(.name)"'
```

CSV output uses raw seconds for duration fields to keep it machine-readable.

## Time format

In `24h` mode (default), bare timestamps like `07:51` and `14:50` are accepted wherever a time is expected. In `12h` mode, use `9:30am` / `2:50pm` style. Set in `~/.config/tmkpr/config.toml`:

```toml
[display]
time_format = "24h"   # "24h" or "12h"
```
