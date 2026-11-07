# Troubleshooting & FAQ

## Build fails: "Package alsa was not found in the pkg-config search path"

This occurs on Linux when building `tmkpr-pomodoro` with `--features audio` and the ALSA development headers are not installed. Install them before building:

```bash
# Debian / Ubuntu
sudo apt install libasound2-dev

# Fedora / RHEL
sudo dnf install alsa-lib-devel
```

Alternatively, build without the audio feature — the terminal bell is used instead:

```bash
cargo build -p tmkpr-pomodoro --release
```

## Settings not saving

- Ensure `~/.config/tmkpr/` is writable.
- Check for permission errors in terminal output.

## Desktop notifications not showing

- On Linux: install a notification daemon (e.g. `notification-daemon`, `dunst`).
- On macOS: system notifications should work automatically.
- On Windows: requires Windows 10 or later.

## Timer not advancing (tmkpr-pomodoro)

- Ensure the timer is running (should show `[RUNNING]` in the timer display).
- Press `Space` to resume if paused.

## Can't open settings (tmkpr-pomodoro)

- The settings screen is only available when the timer is stopped.
- Press `R` to reset the timer first.

## Where is my data stored?

- **Config file**: `~/.config/tmkpr/config.toml`
- **Database**: `~/.local/share/tmkpr/tmkpr.db`

See [Configuration](./configuration.md) for how to override either location.

## I want to try tmkpr without touching my real data

Point every command at a scratch database with `--db` or `TMKPR_DB`:

```bash
TMKPR_DB=/tmp/test-tmkpr.db tmkpr project add demo
TMKPR_DB=/tmp/test-tmkpr.db tmkpr-ui
```

## How do I check my active configuration?

```bash
tmkpr config show
tmkpr config show --format json
```

This prints the resolved configuration and the path to the config file in use, including any `--db` override.
