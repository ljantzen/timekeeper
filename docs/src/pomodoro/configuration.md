# Configuration

Settings are stored in `~/.config/tmkpr/config.toml`. You can edit them directly in the file or through the in-app settings screen (press `S`).

## Configuration options

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

# Cycle limit (0 = unlimited)
max_cycles = 0                    # Stop automatically after this many work sessions
```

## Configuration examples

**Quiet mode** (no notifications):

```toml
[pomodoro]
notify_desktop = false
message_timeout_secs = 0
```

**Audio + desktop notifications**:

```toml
[pomodoro]
sound_work_to_break = "/home/user/sounds/gong.wav"
sound_break_to_work = "/home/user/sounds/bell.ogg"
notify_desktop = true
message_timeout_secs = 3
```

**Short sessions** (for testing):

```toml
[pomodoro]
work_duration_minutes = 1
break_duration_minutes = 1
long_break_duration_minutes = 2
message_timeout_secs = 2
```

**Pause between sessions** (for setup/review):

```toml
[pomodoro]
auto_start_break = false         # Pause when work ends, resume manually
message_timeout_secs = 3
```

**Continuous flow** (minimal interruption):

```toml
[pomodoro]
auto_start_break = true          # Breaks start automatically
message_timeout_secs = 2
```
