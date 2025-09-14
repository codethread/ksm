# ksm (kitty session manager)

A Rust based Kitty Session Manager to emulate tmux style project sessions within the Kitty terminal emulator.

## Overview

ksm allows you to quickly switch between project directories by creating and managing Kitty tabs with project-specific environments. It supports both keyed shortcuts and interactive project selection.

## Features

- **Key-based switching**: Jump to projects using predefined keys
- **Interactive selection**: Browse and select projects with fuzzy finding
- **Session management**: Automatically create or focus existing project tabs
- **Session-aware tab navigation**: Navigate between tabs within the current session context
- **Work/Personal contexts**: Separate project sets for different contexts
- **Project discovery**: Automatically find projects in configured directories

## Installation

```bash
# Install from source
cargo install --path ksm-cli

# Or use the justfile
just install

# Then
ksm --help
```

## Configuration

> [!INFO]
> The `KSM_WORK` env (when set to a truthy value) enables the work context for all commands.

Create a configuration file at `~/.local/data/sessions.json`:

```json
{
  "dirs": ["~/dev/projects/*", "~/work/*/projects", "~/personal/code", "~/code/**/*-project"],
  "base": [
    ["config", "~/.config"],
    ["dots", "~/dotfiles"]
  ],
  "personal": [
    ["blog", "~/personal/blog"],
    ["hobby", "~/personal/hobby-project"]
  ],
  "work": [
    ["api", "~/work/main-api"],
    ["frontend", "~/work/frontend-app"]
  ]
}
```

### Configuration Structure

- **`dirs`**: Array of directory patterns to scan for projects (supports glob patterns)
  - **`~/dev/projects/*`**: Match all direct subdirectories in `~/dev/projects` (e.g., `~/dev/projects/project1`, `~/dev/projects/project2`)
  - **`~/work/*/projects`**: Match directories named `projects` in any subdirectory of `~/work` (e.g., `~/work/team1/projects`, `~/work/team2/projects`)
  - **`~/personal/code`**: Literal directory path (no glob expansion)
  - **`~/code/**/\*-project`**: Match any directory ending in `-project`at any depth under`~/code` using recursive glob
- **`base`**: Key-value pairs available in both work and personal contexts
- **`personal`**: Key-value pairs available only in personal context
- **`work`**: Key-value pairs available only in work context

#### Glob Pattern Support

The `dirs` configuration supports standard glob patterns:

- **`*`**: Matches any number of characters within a directory name (non-recursive)
- **`**`\*\*: Matches any number of directories recursively
- **`?`**: Matches exactly one character
- **`[abc]`**: Matches any one character in the set
- **`[!abc]`**: Matches any one character not in the set

**Examples:**

- `~/projects/*` → `~/projects/web-app`, `~/projects/api-service`
- `~/code/**/*.git` → Any `.git` directory at any depth under `~/code`
- `~/work/team[12]/src` → `~/work/team1/src`, `~/work/team2/src`
- `~/dev/project-*` → `~/dev/project-alpha`, `~/dev/project-beta`

Each project entry is a `[key, path]` pair where:

- `key`: Short identifier for quick access
- `path`: Full or tilde-expanded path to the project directory

## Usage

### Basic Commands

```bash
# Jump to a project by key
ksm key <project-key>

# Interactive project selection
ksm select

# List all available projects
ksm list
```

### Session-Aware Tab Navigation

Navigate between tabs within your current session context:

```bash
# Navigate to next tab in current session
ksm next-tab

# Navigate to previous tab in current session
ksm prev-tab

# Navigate without wrap-around (stops at first/last tab)
ksm next-tab --no-wrap
ksm prev-tab --no-wrap
```

#### How Session Navigation Works

- **Session Context**: When you create a project session with `ksm key` or `ksm select`, tabs are automatically tagged with the session context
- **Session-Aware Navigation**: The `next-tab`/`prev-tab` commands only cycle through tabs belonging to your current session
- **Automatic Inheritance**: New tabs created from within a session automatically inherit the session context
- **Unnamed Sessions**: Tabs created outside of any session are grouped into an "unnamed" session
- **Wrap-Around**: By default, navigation wraps around (last tab → first tab), but can be disabled with `--no-wrap`

This allows you to efficiently navigate between tabs relevant to your current project without cycling through unrelated tabs.

## Development

This project uses a Rust workspace with two main packages:

- **`ksm-cli`**: The main CLI application
- **`kitty-lib`**: Rust abstraction over Kitty terminal apis

See the justfile for available development commands:

```bash
just --list     # Show all available commands
just ci         # Run comprehensive checks (format, lint, test)
just test       # Run all tests
just lint       # Run clippy linter
just fmt        # Format code
```

See individual package READMEs for more details:

- [ksm-cli/README.md](ksm-cli/README.md) - CLI development guide
- [kitty-lib/README.md](kitty-lib/README.md) - Library architecture and API
