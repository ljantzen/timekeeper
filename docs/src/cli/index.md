# Command-Line Interface (tmkpr)

`tmkpr` is the full-featured command-line interface for tmkpr time tracking. Use it for scripting, automation, integration with other tools, and fast keyboard-driven workflows.

## When to use tmkpr

- **Scripting & automation** — pipe commands, integrate with cron jobs, git hooks, etc.
- **Keyboard workflow** — quick commands without GUI overhead
- **Data export** — JSON, CSV, Markdown output formats
- **Structured queries** — precise filtering and reporting
- **CI/CD integration** — programmatic time logging
- **Remote access** — works over SSH without X11

## Quick start

```bash
tmkpr project add myproject
tmkpr task add coding -p myproject

tmkpr start -p myproject -t coding -n "working on feature X"
tmkpr status
tmkpr stop

tmkpr list
tmkpr report
```

The rest of this chapter covers each command group in detail:

- [Tracking Time](./tracking.md) — `start`, `stop`, `status`, `log`
- [Entries](./entries.md) — `list`, `report`, `edit`, `delete`, `merge`, `fill-gap`
- [Projects & Tasks](./projects-tasks.md)
- [Tags](./tags.md)
- [Comments](./comments.md)
- [Events](./events.md)
- [Import & Export](./import-export.md)
- [Output Formats](./output-formats.md)
- [Shell Completion](./shell-completion.md)
- [Launching Other Tools](./launching-other-tools.md)

Entry and comment IDs can be abbreviated to any unambiguous prefix (8+ characters) anywhere an `ID` is expected.
