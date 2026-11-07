# Tracking Time

```text
tmkpr start [-p PROJECT] [-t TASK] [-n NOTE] [-s TIME] [-f] [--tags t1,t2]
tmkpr stop  [-e TIME]
tmkpr status
tmkpr log   [-s START] [-e END | -d DURATION] [-p PROJECT] [-t TASK] [-n NOTE] [--tags t1,t2]
```

`log` (alias: `record`) adds a completed entry directly without a start/stop cycle. `--end` defaults to now if omitted. Use `--duration` instead of `--end` to specify how long the entry lasted (`1h30m`, `90m`, `1:30:00`); `--end` and `--duration` are mutually exclusive. If `--start` is omitted, tmkpr prompts with the end time of the last entry logged today; if there is no such entry, you must provide `--start` explicitly.

Pass `--start continue` (or `--start cont`) to start exactly where the last entry ended, with no prompt.

## Handing off between tasks

If a session is already running when you run `start`, you will be prompted to stop it first. When `--start` is provided, the active entry is stopped at that time and the new entry starts at the same time — so there is no gap and no overlap:

```bash
tmkpr start -p projectB -n "context switch" --start "30 minutes ago"
# active entry stopped 30 min ago; new entry started 30 min ago
```

Use `-f` / `--force` to skip the confirmation prompt. It is an error to pass a `--start` time earlier than the active entry's start time.

## Natural language time

All time flags accept natural language or ISO 8601:

```bash
tmkpr start -p myproject --start "2 hours ago"
tmkpr stop --end "5 minutes ago"
tmkpr log -s "yesterday 9am" -e "yesterday 5pm" -p myproject -n "deep work"
tmkpr log -s "9am" -d 2h -p myproject -n "morning session"
```
