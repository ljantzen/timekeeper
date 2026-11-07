# Introduction

tmkpr is a fast, offline-first time tracking suite written in Rust. It tracks time against **projects** and **tasks**, stores everything locally in SQLite, and gives you three interfaces to work with the same data:

| Tool | Best for |
|------|----------|
| [`tmkpr`](./cli/index.md) (CLI) | Scripting, automation, cron jobs, keyboard-driven workflows, SSH sessions |
| [`tmkpr-ui`](./ui/index.md) | Interactive, real-time session management with a terminal dashboard |
| [`tmkpr-pomodoro`](./pomodoro/index.md) | Structured, focused work in 25/5 minute cycles with automatic logging |

All three tools share the same SQLite database, so you can start a timer in `tmkpr-ui`, stop it from the CLI, and see it show up in a Pomodoro session log — whichever interface fits the moment.

## Key features

- **Project & task management** with flexible hierarchies
- **Start/stop timers** or manual entry logging, with natural language time input ("2 hours ago", "yesterday 9am")
- **Retroactive editing** of any entry
- **Reporting** — daily, weekly (ISO week), and working-week (Mon–Fri) summaries
- **Tags and comments** for organizing and annotating entries
- **Point-in-time events** for marking moments (deploys, meetings) with no duration
- **CSV/JSON import and export** for data portability
- **Optional Obsidian integration** to mirror activity into your notes vault
- **23 built-in colour themes** plus custom theme support
- **Offline-first** — no cloud, no network dependency, full control over your data

## How this manual is organized

- [Installation](./installation.md) and [Quick Start](./quick-start.md) get you tracking time in a few minutes.
- [Configuration](./configuration.md) covers the config file, themes, and Obsidian logging — shared by all three tools.
- The CLI, terminal dashboard, and Pomodoro timer each get their own chapter with the full command/keybinding reference.
- [Troubleshooting & FAQ](./troubleshooting.md) collects common issues and fixes.

> This manual documents the tools as of the `main` branch. For a condensed version, each tool also ships its own `README.md` in the repository (`tmkpr-cli/README.md`, `tmkpr-ui/README.md`, `tmkpr-pomodoro/README.md`).
