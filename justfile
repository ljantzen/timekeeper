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

# Build all crates in debug mode
build:
    cargo build --all

# Build all crates in release mode
build-release:
    cargo build --all --release

# Run the TUI against the test database
ui *ARGS:
    cargo run -p tmkpr-ui -- --db {{ TEST_DB }} {{ ARGS }}

# Run the CLI against the test database
cli *ARGS:
    cargo run -p tmkpr-cli -- --db {{ TEST_DB }} {{ ARGS }}

# Run the Pomodoro timer against the test database
pomo *ARGS:
    cargo run -p tmkpr-pomodoro -- --db {{ TEST_DB }} {{ ARGS }}

# Install all binaries (tmkpr, tmkpr-ui, tmkpr-pomodoro) to ~/.cargo/bin
install:
    cargo install --path tmkpr-cli
    cargo install --path tmkpr-ui
    cargo install --path tmkpr-pomodoro

# Bump patch version and trigger a GitHub release (pass VERSION to pin one)
release VERSION="":
    ./release.sh {{ VERSION }}

# Bump patch version and publish to crates.io after release
release-publish VERSION="":
    ./release.sh --publish {{ VERSION }}
