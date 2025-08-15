# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

**Running the CLI:**

- Use `cargo run --package ksm` instead of building and running the binary directly
- Examples: `cargo run --package ksm -- key --help`, `cargo run --package ksm -- list`
- Or use justfile commands: `just help`, `just list`, `just key <keyname>`

**Development checks:**

- Use `cargo check --workspace` for quick validation (preferred over `cargo build`)
- Run all tests: `cargo test --workspace`
- Linting: `just lint` or `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Code coverage: `just coverage` (generates console output), `just coverage-html` (generates HTML report), `just coverage-open` (opens HTML report in browser)
- Comprehensive CI checks: `just ci` (runs format check, lint, and test)
- **IMPORTANT:** please run `just ci` and fix all issues before considering a job as complete

**Build commands:**

- Build entire workspace: `cargo build --release --workspace`
- Install CLI globally: `cargo install --path ksm-cli` or `just install`

## Architecture

This is a Rust workspace with two main packages:

**ksm-cli** (`ksm-cli/`): The main CLI application

- Entry point: `src/main.rs`
- Core modules: `app.rs`, `cli.rs`, `config.rs`, `kitty.rs`
- Commands: `src/cmd/` (key.rs, list.rs, select.rs)
- Uses clap for CLI parsing, skim for fuzzy selection

**kitty-lib** (`kitty-lib/`): Library for Kitty terminal integration

- `CommandExecutor` trait in `src/executor/` for abstracting Kitty kitten commands
- `KittyExecutor`: Production implementation that calls actual kitten commands
- `MockExecutor`: Test implementation with call tracking and configurable responses
- Command structs in `src/commands/`: `KittenLsCommand`, `KittenFocusTabCommand`, `KittenLaunchCommand`
- Shared types and utilities in `src/types.rs` and `src/utils.rs`

**Key architectural patterns:**

- The `App` struct wraps a `Kitty<KittyExecutor>` instance
- Commands are implemented as separate functions in `src/cmd/`
- Configuration is loaded from `~/.local/data/sessions.json`
- when updating apis, like config or cli params, we don't need to consider graceful migration by handling old behaviour - we can simply make breaking changes

**Testing strategy:**

- Unit tests use `MockExecutor` to simulate Kitty interactions
- Integration tests in `/tests/` directory
- CLI tests verify argument parsing and command routing

