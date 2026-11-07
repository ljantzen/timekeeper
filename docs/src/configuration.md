# Configuration

## Locations

- **Config file**: `~/.config/tmkpr/config.toml`
- **Database**: `~/.local/share/tmkpr/tmkpr.db`

## Override database path

Set via environment variable or command-line flag:

```bash
# Environment variable (works with all tools)
TMKPR_DB=/path/to/other.db tmkpr list
TMKPR_DB=/path/to/other.db tmkpr-ui

# Command-line flag (CLI and UI only)
tmkpr --db /path/to/other.db list
tmkpr-ui --db /path/to/other.db
```

> When testing or experimenting, always point `--db` / `TMKPR_DB` at a scratch file rather than your real database.

## Override theme

```bash
# Environment variable
TMKPR_THEME=catppuccin_mocha tmkpr-ui

# Command-line flag
tmkpr-ui --theme dracula
```

See [Themes](./configuration/themes.md) for the full list and how to define your own.

## Configuration file

Edit `~/.config/tmkpr/config.toml` to customize settings:

```toml
[display]
time_format = "24h"            # "24h" (default) or "12h"
date_format = "%Y-%m-%d %H:%M"
week_start = "mon"             # mon (default), tue, wed, thu, fri, sat, sun
color = true
theme = "catppuccin_mocha"     # see Themes chapter

[database]
path = "~/.local/share/tmkpr/tmkpr.db"

[pomodoro]
work_duration_minutes = 25
break_duration_minutes = 5
long_break_duration_minutes = 15
sessions_before_long_break = 4
notify_desktop = false
auto_start_break = false
```

Inspect the active configuration — including which file is in use and any `--db` override — with:

```bash
tmkpr config show
tmkpr config show --format json
```

### Time format

In `24h` mode (default), bare timestamps like `07:51` and `14:50` are accepted wherever a time is expected. In `12h` mode, use `9:30am` / `2:50pm` style:

```toml
[display]
time_format = "24h"   # "24h" or "12h"
```

## Pomodoro settings

The `[pomodoro]` section configures `tmkpr-pomodoro`'s timer durations and notifications — see [Pomodoro Configuration](./pomodoro/configuration.md) for the full reference.

## Further reading

- [Themes](./configuration/themes.md)
- [Obsidian Integration](./configuration/obsidian.md)
