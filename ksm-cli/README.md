# ksm-cli

The main CLI application for the Kitty Session Manager. This package provides the command-line interface for managing Kitty terminal sessions and project switching.

## Architecture

### Core Components

- **`main.rs`**: Application entry point and CLI setup
- **`app.rs`**: Main application logic and coordination
- **`cli.rs`**: Command-line interface definitions using clap
- **`config.rs`**: Configuration file loading and session management
- **`kitty.rs`**: High-level Kitty terminal integration wrapper
- **`cmd/<name>.rs`**: individual cli commands

### Testing

- **Unit tests**: Embedded in source files using `#[cfg(test)]`
- **Integration tests**: Located in `tests/` directory
- **Mock-based testing**: Uses `kitty-lib`'s `MockExecutor` for testing Kitty interactions

## Development

This package is part of a Rust workspace. Use the justfile at the workspace root for development:

```bash
# Run the CLI
just help                    # Show CLI help

# Workspace-wide commands (affects both CLI and library)
just ci                      # Run all checks (format, lint, test)
just test                    # Run all workspace tests
just fmt                     # Format all code
```

You can also use cargo directly:

```bash
# Run with cargo (from workspace root)
cargo run --package ksm -- --help
cargo test --package ksm
cargo check --package ksm
```
