# Timer Cycle & Notifications

## Timer cycle

The timer follows this pattern (with default durations):

1. **Work session**: 25 minutes (started with `Enter`)
2. **Short break**: 5 minutes
3. *(repeat 3 more times)*
4. **Long break**: 15 minutes
5. *(cycle repeats)*

Example: Work → Break → Work → Break → Work → Break → Work → **Long Break** → *(repeat)*

### Break transitions

When a work session completes:

- **With `auto_start_break = true`**: break timer starts immediately and continues counting down
- **With `auto_start_break = false`**: timer pauses, allowing you to set up before pressing `Space` to resume

You can also start a break manually at any time with `B` (when the timer is stopped). This:

- Immediately starts a break (short or long, depending on cycle position)
- Increments the work session counter toward the next long break
- Is useful for taking breaks without doing a work session first

All durations and cycle length are configurable in [settings](./configuration.md).

## Notifications

### Audio

- Requires the `audio` feature flag at build time (`--features audio`)
- Plays an audio file when sessions transition
- Configure separate files for work→break and break→work transitions
- Supports WAV, MP3, OGG Vorbis, and FLAC
- Omit the setting (or leave it empty) to disable sound for that transition
- Without the `audio` feature, the terminal bell is emitted on every transition instead

### Desktop notifications

- Shows a system notification when a session ends/starts
- Requires a notification daemon (Linux) or native notification support (macOS/Windows)
- Disabled by default (opt-in with `notify_desktop = true`)
- Fails gracefully if the notification daemon is unavailable

### Status messages

- Displayed in the Status bar when state changes
- Auto-clear after a configurable timeout (default 5 seconds)
- Set `message_timeout_secs = 0` to keep messages visible
