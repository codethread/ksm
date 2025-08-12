# kitty-lib

A Rust library for programmatically interacting with Kitty terminal through its remote control protocol. This library provides a type-safe, testable interface for executing Kitty commands.

## Architecture

The library follows a command pattern with dependency injection to enable easy testing and maintainability.

### Core Components

1. **Command Structs**: Pure data holders that represent Kitty commands

   - `KittenLsCommand` - List windows and tabs
   - `KittenFocusTabCommand` - Focus a specific tab
   - `KittenLaunchCommand` - Launch new tabs/windows

2. **CommandExecutor Trait**: Abstraction for command execution

   - Enables dependency injection
   - Allows different implementations for production vs testing

3. **Executors**:
   - `KittyExecutor` - Production implementation that calls actual `kitten` commands
   - `MockExecutor` - Test implementation for unit testing

## Usage

### Basic Usage (Production)

```rust
use kitty_lib::{KittyExecutor, KittenLsCommand, CommandExecutor};

let executor = KittyExecutor;
let command = KittenLsCommand::new("unix:/tmp/mykitty".to_string())
    .match_env("KITTY_SESSION_PROJECT", "my-project");

let output = executor.execute_ls_command(command)?;
```

### Testing with MockExecutor

```rust
use kitty_lib::{MockExecutor, KittenLsCommand, CommandExecutor};
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
use kitty_lib::{CommandExecutor, KittyExecutor};

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

## Dependencies

- `anyhow` - Error handling
- `log` - Logging
- `std::process` - Command execution
- `std::cell::RefCell` - Interior mutability for mock state
