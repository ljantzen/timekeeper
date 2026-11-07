# Command Mode

Press `:` to open the command bar. Use `Tab` / `↓` and `Shift-Tab` / `↑` to cycle through completions, `Enter` to execute, `Esc` to cancel.

| Command | Action |
|---------|--------|
| `theme <name>` | Switch colour theme (live preview while cycling) |
| `set date-format <preset>` | Change date display format |
| `set week-start <day>` | Set first day of the week (e.g. `mon`, `sun`) |
| `config-open` | Open `config.toml` in `$EDITOR` |
| `config-reload` | Reload config from disk |
| `config-write` | Persist current theme and display settings to config |
| `quit` | Quit |

Theme and display changes take effect immediately but are only written to disk with `:config-write`.
