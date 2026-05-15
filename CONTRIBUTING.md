# Contributing

## Prerequisites

- Rust toolchain (stable) — install via [rustup](https://rustup.rs)

## Build

```
cargo build --workspace
```

## Test

Tests live in `tmkpr-lib` and use an in-memory SQLite database — no setup required.

```
cargo test --workspace
```

## Project structure

| Crate | Binary | Purpose |
|-------|--------|---------|
| `tmkpr-lib` | — | Core models, storage, services, and NLP time parsing |
| `tmkpr-cli` | `tmkpr` | Command-line interface |
| `tmkpr-ui` | `tmkpr-ui` | Ratatui terminal dashboard |

New features that affect both interfaces (e.g. a new entry field) belong in `tmkpr-lib`. CLI and UI changes should only touch their respective crates.

## Releasing

Releases are managed by `release.sh`. It bumps the version, commits, pushes a tag, waits for GitHub Actions to build and publish the release, then publishes to crates.io.

```
./release.sh           # auto-increment patch version
./release.sh 1.2.0     # specific version
```

The working tree must be clean before running the script.
