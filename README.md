# tmkpr — Rust Time Tracking Suite

A comprehensive, fast, and offline time tracker written in Rust. Track time against projects and tasks with a focus on simplicity and cross-platform compatibility. All data is stored locally in SQLite, giving you full control over your time-tracking information.

## Features

- **Project & Task Management**: Organize your work with projects and flexible task hierarchies
- **Time Entry Logging**: Track work with start/stop timers or manually log entries
- **Entry Editing**: Modify any time entry retroactively with natural language time input
- **Reporting**: View summaries by project, daily reports, weekly breakdowns, and ISO week reports
- **Comments**: Attach free-form notes to any entry for context
- **Tagging**: Organize entries with custom tags
- **Data Portability**: Export to JSON, CSV, or Markdown formats
- **Fast & Responsive**: Built in Rust for speed and reliability
- **Offline-First**: Works completely offline, syncs to local SQLite database
- **No Cloud Required**: Your time data stays on your machine

## Interfaces

Choose the interface that best fits your workflow:

### **[tmkpr-cli](tmkpr-cli/README.md)** — Command-line tool

Full-featured CLI for time tracking. Perfect for integration with scripts, cron jobs, and text-based workflows.

**Highlights:**
- Complete command coverage for all operations
- Natural language time input ("2 hours ago", "yesterday 9am")
- Multiple output formats (table, JSON, CSV, Markdown)
- Shell completion (Bash, Zsh, Fish)
- Context-aware task handoff without time gaps
- Scripting-friendly with structured output

**Quick example:**
```bash
tmkpr project add myproject
tmkpr task add coding -p myproject
tmkpr start -p myproject -t coding -n "feature development"
tmkpr stop
tmkpr report --wweek
```

### **[tmkpr-ui](tmkpr-ui/README.md)** — Terminal dashboard

Full-featured terminal UI dashboard built with [ratatui](https://github.com/ratatui/ratatui). Great for interactive session management and real-time visibility.

**Highlights:**
- Live timer display for active entries
- Sortable/filterable entry list
- Week report sidebar
- Project and task management
- Entry editing and deletion
- Intuitive vim-style keybindings
- Quick forms with autocomplete

**Launch:**
```bash
tmkpr-ui
```

### **[tmkpr-pomodoro](tmkpr-pomodoro/README.md)** — Pomodoro timer

Integrated Pomodoro timer that automatically logs sessions to the database. Ideal for focused work sessions with built-in breaks.

**Highlights:**
- 25-minute work sessions with 5-minute breaks (configurable)
- Automatic long breaks after N sessions
- Project and task selection from database
- Audio and desktop notifications
- Pause/resume capability
- Configurable cycle settings
- In-app settings editor

**Launch:**
```bash
tmkpr-pomodoro
```

## Installation

### From Source

```bash
# Install all three tools
cargo install --path tmkpr-cli
cargo install --path tmkpr-ui
cargo install --path tmkpr-pomodoro

# Or install individually
cargo install --path tmkpr-cli
cargo install --path tmkpr-ui
cargo install --path tmkpr-pomodoro
```

### Building

```bash
# Build all
cargo build --release

# Build specific tool
cargo build -p tmkpr-cli --release
cargo build -p tmkpr-ui --release
cargo build -p tmkpr-pomodoro --release
```

## Storage and Configuration

### Locations

- **Config file**: `~/.config/tmkpr/config.toml`
- **Database**: `~/.local/share/tmkpr/tmkpr.db`

### Override database path

Set via environment variable or command-line flag:

```bash
# Environment variable (works with all tools)
TMKPR_DB=/path/to/other.db tmkpr list
TMKPR_DB=/path/to/other.db tmkpr-ui

# Command-line flag (CLI and UI only)
tmkpr --db /path/to/other.db list
tmkpr-ui --db /path/to/other.db
```

### Configuration File

Edit `~/.config/tmkpr/config.toml` to customize settings:

```toml
[display]
time_format = "24h"        # "24h" (default) or "12h"
date_format = "%Y-%m-%d %H:%M"
color = true

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

See individual tool READMEs for complete configuration options.

## Quick Start

### 1. Create a project
```bash
tmkpr project add "Client Work"
```

### 2. Add tasks to the project
```bash
tmkpr task add "Feature Development" -p "Client Work"
tmkpr task add "Bug Fixes" -p "Client Work"
```

### 3. Start tracking
```bash
# Start timing
tmkpr start -p "Client Work" -t "Feature Development"

# Do some work...

# Stop timing
tmkpr stop

# Or log directly
tmkpr log -s "9:00am" -e "11:30am" -p "Client Work" -t "Feature Development" -n "implemented auth flow"
```

### 4. View your work
```bash
# Today's entries
tmkpr list

# This week's report
tmkpr report --wweek

# All entries for a project
tmkpr list -p "Client Work"
```

### 5. Or use the interactive dashboard
```bash
# Launch the terminal UI
tmkpr-ui
```

### 6. Or work with Pomodoro sessions
```bash
# Launch the Pomodoro timer
tmkpr-pomodoro
```

## Project Structure

- **tmkpr-lib** — Shared library providing database, configuration, and core data types
- **tmkpr-cli** — Command-line interface for all operations
- **tmkpr-ui** — Terminal dashboard UI
- **tmkpr-pomodoro** — Pomodoro timer with database integration

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed documentation on the tmkpr-ui codebase, maintainability guide, and refactoring roadmap.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on contributing to this project.

## License

MIT
