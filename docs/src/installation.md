# Installation

## GitHub releases

Download the latest binary for your OS from the [GitHub Releases](https://github.com/ljantzen/timekeeper/releases) page. Release binaries are built with audio support enabled for `tmkpr-pomodoro`. If you don't want audio support, build from source instead.

## From source

```bash
# Install all three tools (with audio)
cargo install --path tmkpr-cli
cargo install --path tmkpr-ui
cargo install --path tmkpr-pomodoro --features audio
```

## Building

```bash
# Build all (with audio)
cargo build --release --features tmkpr-pomodoro/audio

# Build a specific tool
cargo build -p tmkpr-cli --release
cargo build -p tmkpr-ui --release
cargo build -p tmkpr-pomodoro --release --features audio
```

### Building without audio

Linux requires `libasound2-dev` (Debian/Ubuntu) or `alsa-lib-devel` (Fedora/RHEL) to compile `tmkpr-pomodoro` with audio. If you'd rather skip that dependency, omit `--features audio`:

```bash
# Build without audio
cargo build --release

# Install without audio
cargo install --path tmkpr-pomodoro
```

Without the `audio` feature, `tmkpr-pomodoro` falls back to the terminal bell on phase transitions. macOS and Windows need no extra packages for audio (CoreAudio / WASAPI are used directly).

## Project structure

| Crate | Role |
|-------|------|
| `tmkpr-lib` | Shared library: database, configuration, core data types |
| `tmkpr-cli` | Command-line interface for all operations |
| `tmkpr-ui` | Terminal dashboard UI |
| `tmkpr-pomodoro` | Pomodoro timer with database integration |
