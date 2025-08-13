# Default recipe lists all available commands
default:
    @just --list

# === WORKSPACE COMMANDS ===

# Build and install CLI release version to global cargo directory
install:
    cargo install --path ksm-cli

# Quick check of all workspace members

[group('project-wide')]
[group('validate')]
check:
    cargo check --workspace

# Quick check of CLI only
[group('validate')]
check-cli:
    cargo check --package ksm

# Quick check of library only
[group('validate')]
check-lib:
    cargo check --package kitty-lib

# Run all tests in workspace
[group('validate')]
[group('project-wide')]
test:
    cargo test --workspace

# Run CLI tests only
[group('validate')]
test-cli:
    cargo test --package ksm

# Run library tests only
[group('validate')]
test-lib:
    cargo test --package kitty-lib

# Clean build artifacts for entire workspace
clean:
    cargo clean

# === DEVELOPMENT ===

# Run linter on entire workspace
[group('project-wide')]
[group('lint')]
lint:
    #!/usr/bin/env bash
    cargo clippy --verbose --workspace --all-targets --all-features -- -D warnings

# Run linter on CLI only
[group('lint')]
lint-cli:
    cargo clippy --verbose --package ksm --all-targets --all-features -- -D warnings

# Run linter on library only
[group('lint')]
lint-lib:
    cargo clippy --verbose --package kitty-lib --all-targets --all-features -- -D warnings

# Format all code in workspace
[group('project-wide')]
[group('validate')]
fmt:
    cargo fmt --all

# Check formatting without making changes
[group('validate')]
fmt-check:
    cargo fmt --all --check

# Run comprehensive checks (format, lint, test)
[group('project-wide')]
[group('validate')]
ci: fmt-check lint test

# === CLI USAGE ===

# Run CLI with help
[group('ksm cmds')]
help:
    cargo run --package ksm -- --help

# Run CLI list command
[group('ksm cmds')]
list:
    cargo run --package ksm -- list

# Run CLI select command
[group('ksm cmds')]
select:
    cargo run --package ksm -- select

# Run CLI with specific key (usage: just key <key_name>)
[group('ksm cmds')]
key KEY:
    cargo run --package ksm -- key {{KEY}}
