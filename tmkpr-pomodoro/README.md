# tmkpr-pomodoro

A terminal-based Pomodoro timer integrated with the tmkpr time tracking database. Manage work sessions, breaks, and automatically log completed sessions.

## Features

- **Pomodoro Timer**: 25-minute work sessions with 5-minute breaks (configurable)
- **Long Breaks**: Automatic longer breaks after N work sessions (default: 15 minutes after 4 sessions)
- **Project & Task Selection**: Choose from projects and tasks stored in your tmkpr database
- **Session Logging**: Automatically log completed work sessions to the database
- **Configurable Notifications**:
  - Terminal bell (audible alert)
  - System desktop notifications (Linux, macOS, Windows)
  - Auto-clearing status messages
- **In-App Settings**: Edit all configuration without leaving the terminal
- **Session Progress**: Visual feedback on where you are in the work/break cycle

## Installation

The pomodoro module is part of the tmkpr workspace. Build it with:

```bash
cargo build -p tmkpr-pomodoro --release
```

The binary will be at `target/release/tmkpr-pomodoro`.

## Usage

### Starting the Timer

```bash
tmkpr-pomodoro
```

### Keyboard Controls

#### Main Screen

| Key | Action |
|-----|--------|
| `↑` / `↓` | Select project |
| `←` / `→` | Select task |
| `Enter` | Start timer |
| `Space` | Pause / Resume |
| `L` | Log current session to database |
| `R` | Reset timer |
| `S` | Open settings |
| `Q` / `Esc` | Quit (only when stopped) |

#### Settings Screen

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate between settings |
| `←` / `→` | Adjust value |
| `Enter` | Save and return to main |
| `Esc` | Cancel and discard changes |

## Timer Cycle

The timer follows this pattern (with default durations):

1. **Work Session**: 25 minutes
2. **Short Break**: 5 minutes
3. *(repeat 3 more times)*
4. **Long Break**: 15 minutes
5. *(cycle repeats)*

Example: Work → Break → Work → Break → Work → Break → Work → **Long Break** → *(repeat)*

All durations and cycle length are configurable in settings.

## Configuration

Settings are stored in `~/.config/tmkpr/config.toml`. You can edit them directly in the file or through the in-app settings screen (press `S`).

### Configuration Options

```toml
[pomodoro]
# Timer durations (in minutes)
work_duration_minutes = 25
break_duration_minutes = 5
long_break_duration_minutes = 15

# Number of work sessions before a long break
sessions_before_long_break = 4

# Notifications
notify_bell = true                # Audible alert when sessions transition
notify_desktop = false            # System notifications (opt-in)
message_timeout_secs = 5          # Auto-clear status messages (0 = never)
```

### Configuration Examples

**Quiet Mode** (no notifications):
```toml
[pomodoro]
notify_bell = false
notify_desktop = false
message_timeout_secs = 0
```

**Aggressive Notifications**:
```toml
[pomodoro]
notify_bell = true
notify_desktop = true
message_timeout_secs = 3
```

**Short Sessions** (for testing):
```toml
[pomodoro]
work_duration_minutes = 1
break_duration_minutes = 1
long_break_duration_minutes = 2
message_timeout_secs = 2
```

## UI Layout

```
┌─ Timer ─────────────────────────────────┐
│           12:34 / 25:00 [RUNNING]       │
│                Session 2 of 4           │
└─────────────────────────────────────────┘
┌─ Projects ──────┬─ Tasks ──────────────┐
│  ▶ Project A    │  ▶ Task 1            │
│    Project B    │    Task 2            │
│    Project C    │    Task 3            │
└─────────────────┴──────────────────────┘
┌─ Status ────────────────────────────────┐
│  Timer running...                        │
└─────────────────────────────────────────┘
┌─ Help ──────────────────────────────────┐
│  ↑↓: Select project  |  ←→: Select task │
│  Enter: Start   Space: Pause   S: Sett  │
└─────────────────────────────────────────┘
```

## Database Integration

The timer integrates with the tmkpr SQLite database:

- **Reads from**: Projects and tasks (via Storage trait)
- **Writes to**: Time entries (when logging sessions with `L` key)
- **Database path**: Configured in `~/.config/tmkpr/config.toml` (database section)

## Notifications

### Terminal Bell
- Emits a BEL character (`\x07`) when sessions transition
- Works in any terminal, enabled by default
- Toggle with `notify_bell` setting

### Desktop Notifications
- Shows system notification when session ends/starts
- Requires a notification daemon (Linux) or native notification support (macOS/Windows)
- Disabled by default (opt-in with `notify_desktop = true`)
- Fails gracefully if notification daemon unavailable

### Status Messages
- Display in the Status bar when state changes
- Auto-clear after configurable timeout (default 5 seconds)
- Set `message_timeout_secs = 0` to keep messages visible

## Development

### Building

```bash
cargo build -p tmkpr-pomodoro
```

### Testing

```bash
cargo test -p tmkpr-pomodoro
```

### Running with Debug Output

```bash
RUST_LOG=debug cargo run -p tmkpr-pomodoro
```

## Architecture

- **`src/main.rs`**: Event loop, terminal setup, key dispatch
- **`src/app.rs`**: Timer state machine, project/task navigation, session logic
- **`src/ui.rs`**: Ratatui-based terminal UI rendering

### Dependencies

- **ratatui**: Terminal UI framework
- **crossterm**: Terminal event handling
- **chrono**: Time handling
- **notify-rust**: Cross-platform desktop notifications
- **tmkpr-lib**: Shared database and configuration

## Troubleshooting

### Settings Not Saving

- Ensure `~/.config/tmkpr/` directory is writable
- Check for permission errors in terminal output

### Desktop Notifications Not Showing

- On Linux: Install a notification daemon (e.g., `notification-daemon`, `dunst`)
- On macOS: System notifications should work automatically
- On Windows: Requires Windows 10 or later

### Timer Not Advancing

- Ensure the timer is running (should show `[RUNNING]` in the timer display)
- Press `Space` to resume if paused

### Can't Open Settings

- Settings screen is only available when timer is stopped
- Press `R` to reset the timer first

## License

MIT

## See Also

- [tmkpr-lib](../tmkpr-lib/) - Shared library for time tracking
- [tmkpr-cli](../tmkpr-cli/) - Command-line interface
- [tmkpr-ui](../tmkpr-ui/) - Full-featured terminal UI
