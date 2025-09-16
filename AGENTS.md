# ksm (kitty session manager) Agents file

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

**Running the CLI:**

- Use `cargo run --package ksm` instead of building and running the binary directly
- Examples: `cargo run --package ksm -- key --help`, `cargo run --package ksm -- list`, `cargo run --package ksm -- next-tab`
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
- Commands: `src/cmd/` (key.rs, list.rs, select.rs, next_tab.rs, prev_tab.rs, rename_tab.rs)
- Session management: `src/session.rs` (session context detection via tab titles and env vars, last-active tab tracking)
- Uses clap for CLI parsing, skim for fuzzy selection

**kitty-lib** (`kitty-lib/`): Library for Kitty terminal integration

- `CommandExecutor` trait in `src/executor/` for abstracting Kitty kitten commands
- `KittyExecutor`: Production implementation that calls actual kitten commands
- `MockExecutor`: Test implementation with call tracking and configurable responses
- Command structs in `src/commands/`: `KittenLsCommand`, `KittenFocusTabCommand`, `KittenLaunchCommand`, `KittenNavigateTabCommand`, `KittenSetTabTitleCommand`
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

## Session Identification

KSM uses a robust dual-approach for session identification:

1. **Primary: Tab Titles** - Sessions are marked with `session:<name>` prefix in tab titles
2. **Fallback: Environment Variables** - Uses `KITTY_SESSION_PROJECT` for backward compatibility

**Recommended Kitty Configuration:**

Add to `~/.config/kitty/kitty.conf` to preserve session titles:

```bash
# Disable automatic title updates
shell_integration no-title
```

**Commands:**

- `ksm rename-tab "Description"` - Rename tab while preserving session marker
  - In session: Sets title to `session:name - Description`
  - No session: Sets title to `Description`

## Keybinding Philosophy

KSM does NOT implement or manage keybindings directly. Instead:

- KSM exposes CLI commands that wrap Kitty terminal functionality
- Users are expected to configure their own keybindings in their Kitty config
- This approach provides maximum flexibility and avoids conflicts with existing user configurations

Example Kitty configuration to bind KSM commands:

```bash
# In ~/.config/kitty/kitty.conf
map ctrl+shift+] kitten @ launch --type=overlay ksm next-tab
map ctrl+shift+[ kitten @ launch --type=overlay ksm prev-tab
map ctrl+shift+t kitten @ launch --type=overlay ksm new-tab
```
