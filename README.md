# tmkpr

A time tracker written in Rust. Tracks time against projects and tasks, stored locally in SQLite.

Three interfaces are available:

- **[tmkpr-cli](tmkpr-cli/README.md)** — `tmkpr`, a full-featured command-line tool
- **[tmkpr-ui](tmkpr-ui/README.md)** — `tmkpr-ui`, a terminal dashboard built with ratatui
- **[tmkpr-pomodoro](tmkpr-pomodoro/README.md)** — `tmkpr-pomodoro`, a Pomodoro timer that logs sessions to the database

All three share the same database and config.

## Install

```
cargo install --path tmkpr-cli
cargo install --path tmkpr-ui
cargo install --path tmkpr-pomodoro
```

## Storage and config

Config file: `~/.config/tmkpr/config.toml`  
Database: `~/.local/share/tmkpr/tmkpr.db`

Override the database path at runtime:

```
TMKPR_DB=/path/to/other.db tmkpr list
tmkpr --db /path/to/other.db list
```

The same `--db` / `TMKPR_DB` override works for `tmkpr-ui`.

Relevant display options (CLI):

```toml
[display]
time_format = "24h"   # "24h" (default) or "12h"
date_format = "%Y-%m-%d %H:%M"
color = true
```
