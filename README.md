# tmkpr

A command-line time tracker written in Rust. Tracks time against projects and tasks, stored locally in SQLite.

## Install

```
cargo install --path tmkpr-cli
```

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
tmkpr start [-p PROJECT] [-t TASK] [-n NOTE] [-s TIME] [--tags t1,t2]
tmkpr stop  [-e TIME]
tmkpr status
tmkpr log   [-s START] [-e END] [-p PROJECT] [-t TASK] [-n NOTE] [--tags t1,t2]
```

`log` (alias: `record`) adds a completed entry directly without a start/stop cycle. `--end` defaults to now if omitted. If `--start` is omitted, tmkpr will suggest the end time of the last entry logged today.

If a session is already running when you run `start`, you will be prompted to stop it first. Answering `n` aborts without making any changes.

All time flags accept natural language or ISO 8601:

```
tmkpr start -p myproject --start "2 hours ago"
tmkpr stop --end "5 minutes ago"
tmkpr log -s "yesterday 9am" -e "yesterday 5pm" -p myproject -n "deep work"
```

### Entries

```
tmkpr list [--from TIME] [--until TIME] [-p PROJECT] [-t TASK] [-l LIMIT] [--tag TAG]
tmkpr report [--from TIME] [--until TIME] [-p PROJECT]
tmkpr edit <ID> [-p PROJECT] [-t TASK] [-n NOTE] [--start TIME] [--end TIME] [--tags t1,t2]
tmkpr delete <ID> [-y]
```

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
tmkpr task add <NAME> -p PROJECT [-d DESCRIPTION]
tmkpr task list -p PROJECT [--archived]
tmkpr task edit <NAME|ID> -p PROJECT [--name NAME] [-d DESCRIPTION] [--move-to PROJECT]
tmkpr task delete <NAME|ID> -p PROJECT [--hard]
```

`-p` identifies the project the task currently belongs to. Use `--move-to` to reassign it to a different project.

### Shell completion

```
tmkpr completion bash   >> ~/.bashrc
tmkpr completion zsh    >> ~/.zshrc
tmkpr completion fish   > ~/.config/fish/completions/tmkpr.fish
```

## Configuration

Config file: `~/.config/tmkpr/config.toml`  
Database: `~/.local/share/tmkpr/tmkpr.db`

Override the database path at runtime:

```
TMKPR_DB=/path/to/other.db tmkpr list
tmkpr --db /path/to/other.db list
```

Relevant display options:

```toml
[display]
time_format = "24h"   # "24h" (default) or "12h"
date_format = "%Y-%m-%d %H:%M"
color = true
```

In `24h` mode, bare timestamps like `07:51` and `14:50` are accepted wherever a time is expected. In `12h` mode, use `9:30am` / `2:50pm` style.

## Output format

All list-producing commands accept `--format table` (default), `--format json`, or `--format csv`.

```
tmkpr list --format json
tmkpr report --format csv
tmkpr project list --format json
tmkpr task list -p myproject --format csv
```

`project list --format json` includes a `tasks` array on each project object, making it suitable for scripting:

```bash
tmkpr project list --format json \
  | jq -r '.[] | . as $p | .tasks[] | "\($p.name) / \(.name)"'
```

CSV output uses raw seconds for duration fields to keep it machine-readable.
