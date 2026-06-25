set dotenv-load := false

TEST_DB := "/tmp/test-tmkpr.db"

# Show available recipes
default:
    @just --list

# Format all crates
fmt:
    cargo fmt --all

# Run clippy on all crates
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Run the full test suite
test:
    cargo test --all

# Run fmt + clippy + tests (pre-commit gate)
check: fmt clippy test

# Build all crates in debug mode (audio=false to disable)
build audio="true":
    cargo build --all {{ if audio == "true" { "--features tmkpr-pomodoro/audio" } else { "" } }}

# Build all crates in release mode (audio=false to disable)
build-release audio="true":
    cargo build --all --release {{ if audio == "true" { "--features tmkpr-pomodoro/audio" } else { "" } }}

# Run the TUI against the test database
ui *ARGS:
    cargo run -p tmkpr-ui -- --db {{ TEST_DB }} {{ ARGS }}

# Run the CLI against the test database
cli *ARGS:
    cargo run -p tmkpr-cli -- --db {{ TEST_DB }} {{ ARGS }}

# Run the Pomodoro timer against the test database (audio=false to disable)
pomo audio="true" *ARGS:
    cargo run -p tmkpr-pomodoro {{ if audio == "true" { "--features audio" } else { "" } }} -- --db {{ TEST_DB }} {{ ARGS }}

# Install all binaries (tmkpr, tmkpr-ui, tmkpr-pomodoro) to ~/.cargo/bin (audio=false to disable)
install audio="true":
    cargo install --path tmkpr-cli
    cargo install --path tmkpr-ui
    cargo install --path tmkpr-pomodoro {{ if audio == "true" { "--features audio" } else { "" } }}

# Bump patch version and trigger a GitHub release (pass VERSION to pin one)
release VERSION="":
    ./release.sh {{ VERSION }}

# Bump patch version and publish to crates.io after release
release-publish VERSION="":
    ./release.sh --publish {{ VERSION }}
