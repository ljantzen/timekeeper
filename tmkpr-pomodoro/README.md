# tmkpr-pomodoro

Terminal-based Pomodoro timer integrated with the tmkpr time tracking database. Manage focused work sessions with built-in breaks, automatic logging, and notifications.

**See the [main README](../README.md) for installation, storage, and general configuration details.**

## When to use tmkpr-pomodoro

- **Focused work sessions** — structure your day with timed intervals
- **Break reminders** — automatic breaks to prevent burnout
- **Session logging** — automatically record completed Pomodoros to the database
- **Notifications** — audio and desktop alerts for transitions
- **Distraction-free** — stay in the terminal, avoid context switching

## Features

- **Pomodoro Timer**: 25-minute work sessions with 5-minute breaks (configurable)
- **Long Breaks**: Automatic longer breaks after N work sessions (default: 15 minutes after 4 sessions)
- **Project & Task Selection**: Choose from projects and tasks stored in your tmkpr database
- **Session Logging**: Automatically log completed work sessions to the database
- **Configurable Notifications**:
  - Audio file playback on session transitions (configurable per transition)
  - System desktop notifications (Linux, macOS, Windows)
  - Auto-clearing status messages
- **In-App Settings**: Edit all configuration without leaving the terminal
- **Session Progress**: Visual feedback on where you are in the work/break cycle

## Installation

### System dependencies

Audio playback requires platform audio libraries when building from source.

**Linux** — install ALSA development headers before building:

```bash
# Fedora / RHEL
sudo dnf install alsa-lib-devel

# Debian / Ubuntu
sudo apt install libasound2-dev
```

**macOS** — CoreAudio is used; no extra packages needed.

**Windows** — WASAPI is used; no extra packages needed.

### Building

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
| `Enter` | Start work session |
| `B` | Start break (short or long) |
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

1. **Work Session**: 25 minutes (started with `Enter`)
2. **Short Break**: 5 minutes
3. *(repeat 3 more times)*
4. **Long Break**: 15 minutes
5. *(cycle repeats)*

Example: Work → Break → Work → Break → Work → Break → Work → **Long Break** → *(repeat)*

### Break Transitions

When a work session completes:
- **With `auto_start_break = true`**: Break timer starts immediately and continues counting down
- **With `auto_start_break = false`**: Timer pauses, allowing you to set up before pressing Space to resume

You can also start a break manually at any time with `B` (when timer is stopped). This:
- Immediately starts a break (short or long, depending on cycle position)
- Increments the work session counter toward the next long break
- Useful for taking breaks without doing a work session first

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
notify_desktop = false            # System notifications (opt-in)
message_timeout_secs = 5          # Auto-clear status messages (0 = never)

# Audio (WAV, MP3, OGG, FLAC supported; omit to disable)
sound_work_to_break = ""          # Played when a work session ends
sound_break_to_work = ""          # Played when a break ends

# Break behavior
auto_start_break = false          # Auto-start break when work ends (true) or pause (false)
```

### Configuration Examples

**Quiet Mode** (no notifications):
```toml
[pomodoro]
notify_desktop = false
message_timeout_secs = 0
```

**Audio + Desktop Notifications**:
```toml
[pomodoro]
sound_work_to_break = "/home/user/sounds/gong.wav"
sound_break_to_work = "/home/user/sounds/bell.ogg"
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

**Pause Between Sessions** (for setup/review):
```toml
[pomodoro]
auto_start_break = false         # Pause when work ends, resume manually
message_timeout_secs = 3
```

**Continuous Flow** (minimal interruption):
```toml
[pomodoro]
auto_start_break = true          # Breaks start automatically
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

### Audio
- Plays an audio file when sessions transition
- Configure separate files for work→break and break→work transitions
- Supports WAV, MP3, OGG Vorbis, and FLAC
- Omit the setting (or leave it empty) to disable sound for that transition

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
- **rodio**: Audio playback (WAV, MP3, OGG, FLAC via symphonia)
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
