# ksm-cli

The main CLI application for the Kitty Session Manager. This package provides the command-line interface for managing Kitty terminal sessions and project switching.

## Architecture

### Core Components

- **`main.rs`**: Application entry point and CLI setup
- **`app.rs`**: Main application logic and coordination
- **`cli.rs`**: Command-line interface definitions using clap
- **`config.rs`**: Configuration file loading and session management
- **`kitty.rs`**: High-level Kitty terminal integration wrapper
- **`utils.rs`**: Shared utility functions

### Commands (`src/cmd/`)

Each command is implemented as a separate module:

- **`key.rs`**: Switch to projects using predefined keys
- **`list.rs`**: List all available sessions and projects  
- **`select.rs`**: Interactive project selection with fuzzy finding

### Testing

- **Unit tests**: Embedded in source files using `#[cfg(test)]`
- **Integration tests**: Located in `tests/` directory
- **Mock-based testing**: Uses `kitty-lib`'s `MockExecutor` for testing Kitty interactions

## Development

This package is part of a Rust workspace. Use the justfile at the workspace root for development:

```bash
# Run the CLI
just help                    # Show CLI help
just list                    # Run list command
just select                  # Run select command  
just key <keyname>           # Run key command

# Development commands
just test-cli                # Run CLI tests only
just check-cli               # Quick validation of CLI only
just build-cli               # Build CLI release version
just lint-cli                # Run clippy on CLI only

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

## Dependencies

- `kitty-lib`: Internal library for Kitty terminal integration
- `clap`: Command-line argument parsing
- `serde`/`serde_json`: Configuration serialization
- `skim`: Fuzzy finder for interactive selection
- `glob`: Pattern matching for directory scanning
- `shellexpand`: Shell-style path expansion

