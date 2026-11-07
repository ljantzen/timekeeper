# Import & Export

```text
tmkpr import [FILE] [--skip-errors] [--dry-run]
tmkpr export [FILE] [-p PROJECT] [-t TASK] [--from TIME] [--until TIME] [--tag TAG] [--no-active]
```

## Import

Reads projects, tasks, and time entries from a CSV or JSON file and creates any missing projects and tasks automatically.

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

## Export

Writes entries to CSV (default) or JSON. Columns: `project`, `task`, `start`, `end`, `note`, `tags`. Datetimes are written in local time so files round-trip cleanly through `import`.

```bash
tmkpr export                           # all entries to stdout (CSV)
tmkpr export entries.csv               # all entries to file
tmkpr export entries.json              # JSON (auto-detected by extension)
tmkpr -f json export                   # JSON to stdout
tmkpr export --from 2024-01-01 out.csv # date range
tmkpr export -p "My Project" out.csv   # filter by project
tmkpr export | tmkpr import -          # round-trip: export then re-import
```
