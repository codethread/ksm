# ksm-cli Developer Guide

This README is for developers working on the ksm codebase.

## Project Structure

```
src/
├── cli.rs          # Command-line interface definitions
├── cmd/            # Command implementations
│   ├── mod.rs      # Module exports
│   ├── key.rs      # Key-based project switching
│   ├── list.rs     # List available sessions
│   └── select.rs   # Interactive project selection
├── config.rs       # Configuration loading and management
├── kitty.rs        # Kitty terminal integration
├── utils.rs        # Utility functions
├── lib.rs          # Library exports
├── app.rs          # Application logic
└── main.rs         # Application entry point

tests/
├── config_tests.rs      # Configuration tests
├── key_tests.rs         # Key command tests
├── kitty_mock_test.rs   # Kitty integration tests
└── utils_tests.rs       # Utility function tests
```

## Development Commands

Since this is part of a Rust workspace, use these commands for development:

```bash
# Run the application
cargo run -p ksm -- --help

# Run tests
cargo test

# Quick validation
cargo check

# Build release
cargo build --release -p ksm
```

## Dependencies

- `kitty-lib`: Internal library for Kitty terminal integration
- `clap`: Command-line argument parsing
- `serde`/`serde_json`: Configuration serialization
- `skim`: Fuzzy finder for interactive selection
- `glob`: Pattern matching for directory scanning
- `shellexpand`: Shell-style path expansion

