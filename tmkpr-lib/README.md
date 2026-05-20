# tmkpr-lib

Shared library providing core functionality for the tmkpr time tracking suite. Provides database access, configuration management, and common data types used by all tmkpr interfaces.

## Purpose

`tmkpr-lib` is the foundation of all tmkpr tools:
- **Database layer** — SQLite database abstraction for time entries, projects, and tasks
- **Configuration** — Centralized config file handling
- **Data types** — Shared models and enums used across all interfaces
- **Storage trait** — Common interface for persistence

## Architecture

### Core Modules

- **`storage.rs`** — `Storage` trait defining database operations
- **`db/`** — SQLite database implementation, migrations, and queries
- **`config.rs`** — Configuration file loading and management
- **`model/`** — Shared data types (Entry, Project, Task, Comment, etc.)

## Usage

This library is used by:
- `tmkpr-cli` — Command-line interface
- `tmkpr-ui` — Terminal dashboard
- `tmkpr-pomodoro` — Pomodoro timer

All three tools share the same database schema and configuration format, ensuring data consistency and portability.

## Database Schema

The tmkpr database includes tables for:
- **projects** — Project definitions
- **tasks** — Tasks within projects
- **entries** — Time entries (work sessions)
- **comments** — Notes attached to entries
- **tags** — Custom tags for categorization

See database migrations in `src/db/` for complete schema details.

## Configuration

Configuration is read from `~/.config/tmkpr/config.toml`. The library provides:
- Type-safe config deserialization
- Default values
- Environment variable overrides (e.g., `TMKPR_DB`)

## For Developers

When adding new features:
1. Update the database schema (add migration if needed)
2. Update data models in `src/model/`
3. Implement storage methods in `src/db/`
4. Update the `Storage` trait if needed
5. Implement in each interface crate (cli, ui, pomodoro)

## Dependencies

- `rusqlite` — SQLite database access
- `chrono` — Date/time handling
- `serde` — Serialization/deserialization
- `dirs` — Config/data directory paths
- `toml` — Configuration file parsing
