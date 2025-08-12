# kitty-lib

A Rust library for programmatically interacting with Kitty terminal through its remote control protocol. This library provides a type-safe, testable interface for executing Kitty commands.

## Architecture

The library follows a command pattern with dependency injection to enable easy testing and maintainability.

### Core Components

1. **Command Structs** (`src/commands/`): Pure data holders that represent Kitty commands
   - `KittenLsCommand` - List windows and tabs
   - `KittenFocusTabCommand` - Focus a specific tab  
   - `KittenLaunchCommand` - Launch new tabs/windows

2. **CommandExecutor Trait** (`src/executor/mod.rs`): Abstraction for command execution
   - Enables dependency injection and testability
   - Allows different implementations for production vs testing

3. **Executors** (`src/executor/`):
   - `KittyExecutor` - Production implementation that calls actual `kitten` commands
   - `MockExecutor` - Test implementation with call tracking and configurable responses

4. **Types** (`src/types.rs`): Shared data structures for Kitty objects
5. **Utilities** (`src/utils.rs`): Helper functions for common operations

## Usage

### Basic Usage (Production)

```rust
use kitty_lib::executor::{KittyExecutor, CommandExecutor};
use kitty_lib::commands::KittenLsCommand;

let executor = KittyExecutor;
let command = KittenLsCommand::new("unix:/tmp/mykitty".to_string())
    .match_env("KITTY_SESSION_PROJECT", "my-project");

let output = executor.execute_ls_command(command)?;
```

### Testing with MockExecutor

```rust
use kitty_lib::executor::{MockExecutor, CommandExecutor};
use kitty_lib::commands::KittenLsCommand;
use std::process::{Output, ExitStatus};
use std::os::unix::process::ExitStatusExt;

let mock = MockExecutor::new();

// Setup expected response
let mock_output = Output {
    status: ExitStatus::from_raw(0),
    stdout: br#"[{"tabs": [{"id": 42}]}]"#.to_vec(),
    stderr: Vec::new(),
};
mock.expect_ls_response(Ok(mock_output));

let command = KittenLsCommand::new("unix:/tmp/mykitty".to_string());
let result = mock.execute_ls_command(command)?;

// Verify calls
assert_eq!(mock.ls_call_count(), 1);
let calls = mock.get_ls_calls();
assert_eq!(calls[0].socket, "unix:/tmp/mykitty");
```

## Command Reference

### KittenLsCommand

Lists windows and tabs, optionally filtered by environment variables.

```rust
let command = KittenLsCommand::new(socket)
    .match_env("KITTY_SESSION_PROJECT", "project-name");
```

**Builder Methods:**

- `match_env(env_var, value)` - Filter by environment variable

### KittenFocusTabCommand

Focuses a specific tab by ID.

```rust
let command = KittenFocusTabCommand::new(socket, tab_id);
```

### KittenLaunchCommand

Launches new tabs or windows with various options.

```rust
let command = KittenLaunchCommand::new(socket)
    .launch_type("tab")
    .cwd("/path/to/directory")
    .env("KEY", "value")
    .tab_title("My Tab");
```

**Builder Methods:**

- `launch_type(type)` - "tab" or "window"
- `cwd(path)` - Working directory
- `env(key, value)` - Environment variable
- `tab_title(title)` - Tab title

## MockExecutor Testing Utilities

The `MockExecutor` provides comprehensive testing capabilities:

### Response Configuration

```rust
mock.expect_ls_response(Ok(output));
mock.expect_focus_tab_response(Ok(ExitStatus::from_raw(0)));
mock.expect_launch_response(Ok(ExitStatus::from_raw(0)));
```

### Call Verification

```rust
// Check call counts
assert_eq!(mock.ls_call_count(), 1);
assert_eq!(mock.focus_tab_call_count(), 2);
assert_eq!(mock.launch_call_count(), 0);

// Inspect actual calls
let ls_calls = mock.get_ls_calls();
let focus_calls = mock.get_focus_tab_calls();
let launch_calls = mock.get_launch_calls();
```

## Integration with Higher-Level APIs

This library is designed to be used by higher-level APIs that provide more convenient interfaces:

```rust
use kitty_lib::executor::{CommandExecutor, KittyExecutor};
use kitty_lib::commands::KittenLsCommand;

pub struct Kitty<E: CommandExecutor> {
    socket: String,
    executor: E,
}

impl Kitty<KittyExecutor> {
    pub fn new() -> Self {
        Self {
            socket: get_kitty_socket(),
            executor: KittyExecutor,
        }
    }
}

impl<E: CommandExecutor> Kitty<E> {
    pub fn with_executor(executor: E) -> Self {
        Self {
            socket: get_kitty_socket(),
            executor,
        }
    }

    pub fn find_project_tab(&self, project: &str) -> Result<Option<Tab>> {
        let command = KittenLsCommand::new(self.socket.clone())
            .match_env("KITTY_SESSION_PROJECT", project);
        let output = self.executor.execute_ls_command(command)?;
        // Parse and return result...
    }
}
```

## Error Handling

All executor methods return `anyhow::Result<T>` for comprehensive error handling. The library handles:

- Command execution failures
- Invalid socket connections
- JSON parsing errors (for ls commands)
- System-level errors

## Requirements

- Kitty terminal with remote control enabled
- Unix-like system (uses Unix domain sockets)
- Rust 2021 edition or later

## Development

This library is part of a Rust workspace. Use the justfile at the workspace root for development:

```bash
# Library-specific commands
just test-lib                # Run library tests only  
just check-lib               # Quick validation of library only
just build-lib               # Build library release version
just lint-lib                # Run clippy on library only

# Workspace-wide commands
just ci                      # Run all checks (format, lint, test)
just test                    # Run all workspace tests
just fmt                     # Format all code
```

You can also use cargo directly:

```bash
# Run with cargo (from workspace root)
cargo test --package kitty-lib
cargo check --package kitty-lib
cargo build --package kitty-lib
```

## Dependencies

- `anyhow` - Error handling
- `log` - Logging  
- `std::process` - Command execution
- `std::cell::RefCell` - Interior mutability for mock state
