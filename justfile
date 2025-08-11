# Default recipe lists all available commands
default:
    @just --list

# === WORKSPACE COMMANDS ===

# Build and install CLI release version to global cargo directory
install:
    cargo install --path ksm-cli

# Build release version of all workspace members
build:
    cargo build --release --workspace

# Build release version of CLI only
build-cli:
    cargo build --release --package ksm

# Build release version of library only  
build-lib:
    cargo build --release --package kitty-lib

# Quick check of all workspace members
check:
    cargo check --workspace

# Quick check of CLI only
check-cli:
    cargo check --package ksm

# Quick check of library only
check-lib:
    cargo check --package kitty-lib

# Run all tests in workspace
test:
    cargo test --workspace

# Run CLI tests only
test-cli:
    cargo test --package ksm

# Run library tests only
test-lib:
    cargo test --package kitty-lib

# Clean build artifacts for entire workspace
clean:
    cargo clean

# === DEVELOPMENT ===

# Run linter on entire workspace
lint:
    #!/usr/bin/env bash
    cargo clippy --verbose --workspace --all-targets --all-features -- -D warnings

# Run linter on CLI only
lint-cli:
    cargo clippy --verbose --package ksm --all-targets --all-features -- -D warnings

# Run linter on library only
lint-lib:
    cargo clippy --verbose --package kitty-lib --all-targets --all-features -- -D warnings

# Format all code in workspace
fmt:
    cargo fmt --all

# Check formatting without making changes
fmt-check:
    cargo fmt --all --check

# Run comprehensive checks (format, lint, test)
ci: fmt-check lint test

# === CLI USAGE ===

# Run CLI with help
help:
    cargo run --package ksm -- --help

# Run CLI list command
list:
    cargo run --package ksm -- list

# Run CLI select command
select:
    cargo run --package ksm -- select

# Run CLI with specific key (usage: just key <key_name>)
key KEY:
    cargo run --package ksm -- key {{KEY}}
