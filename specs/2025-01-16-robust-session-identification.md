# Robust Session Identification via Tab Titles

## Context and Problem Statement

Currently, KSM uses environment variables (`KITTY_SESSION_PROJECT`) to mark and identify sessions in Kitty. This approach has a critical limitation: environment variables are set at the window level, not the tab level. When the initial window in a tab is closed, the environment variable is lost, causing session matching to fail.

## Value Statement

Implementing tab title-based session identification will:

- Provide persistent session marking that survives window closures
- Maintain session context across all windows within a tab
- Enable visible session identification in the tab bar
- Support backward compatibility with existing environment variable approach

## Stakeholders

- **Users**: KSM CLI users managing multiple project sessions
- **Maintainers**: KSM development team
- **Dependencies**: Kitty terminal integration

## Technical Architecture

### Current Architecture (Environment Variables)

```
Tab
├── Window 1 (env: KITTY_SESSION_PROJECT=myproject)
├── Window 2 (inherits env from Window 1)
└── Window 3 (inherits env from Window 1)

Problem: Closing Window 1 loses the environment variable
```

### Proposed Architecture (Tab Titles)

```
Tab (title: "session:myproject - Development")
├── Window 1
├── Window 2
└── Window 3

Benefit: Title persists regardless of window state
```

### Migration Strategy

1. **Phase 1**: Add tab title support alongside existing env vars
2. **Phase 2**: Make tab titles primary, env vars fallback
3. **Phase 3**: Deprecate env var approach (future release)

## Functional Requirements

### 1. Tab Title Session Marking

- [ ] 1.1 Implement tab title setting with session prefix pattern `session:<name>`
- [ ] 1.2 Support optional descriptive suffix (e.g., `session:myproject - Development`)
- [ ] 1.3 Preserve human-readable tab titles while embedding session metadata

### 2. Session Detection

- [ ] 2.1 Detect sessions from tab titles using `session:` prefix
- [ ] 2.2 Fall back to `KITTY_SESSION_PROJECT` env var for backward compatibility
- [ ] 2.3 Return consistent `SessionContext` regardless of detection method

### 3. Tab Matching

- [ ] 3.1 Update `KittenLsCommand` to support `--match-tab title:` patterns
- [ ] 3.2 Implement regex matching for session-prefixed titles
- [ ] 3.3 Maintain existing env var matching as fallback

### 4. Session Creation

- [ ] 4.1 Set tab titles when creating new session tabs
- [ ] 4.2 Preserve session context when launching new windows within tabs
- [ ] 4.3 Support custom tab title suffixes for user context

### 5. Tab Management

- [x] 5.1 Implement `rename-tab` command to preserve session markers during tab renaming
- [x] 5.2 Automatically format titles as `session:<name> - <description>` when session context exists
- [x] 5.3 Support plain title formatting when no session context is detected

#### Implementation Summary

The `rename-tab` command has been successfully implemented with the following features:

- **Command Integration**: Added `ksm rename-tab <description>` command to CLI
- **Session-Aware Formatting**: Automatically detects current session context and formats titles appropriately:
  - In session: `session:project-name - User's Custom Name`
  - No session: `User's Custom Name`
- **Robust Implementation**: Uses existing `SessionContext::detect()` for session detection
- **Error Handling**: Proper error propagation and user feedback
- **Testing**: Comprehensive test suite covering session/non-session scenarios, edge cases, and error conditions

The command preserves session markers during tab renaming and provides a seamless user experience for managing tab titles across session contexts.

**Recommended Kitty Configuration:**

To prevent automatic title changes from interfering with session-aware titles, users should configure Kitty with:

```ini
# In kitty.conf
shell_integration no-title
```

This prevents shell integration from automatically overwriting tab titles with directory names or command names, ensuring session markers persist across directory changes and command execution.

### 6. Migration Support

- [ ] 6.1 Detect tabs using old env var approach
- [ ] 6.2 Optionally migrate existing sessions to tab title approach
- [ ] 6.3 Provide clear migration path in documentation

## Non-Functional Requirements

### Performance

- [ ] Session detection must complete in < 50ms
- [ ] Tab title updates must be immediate (< 10ms)

### Compatibility

- [ ] Must work with Kitty 0.35.0+
- [ ] Maintain backward compatibility with existing sessions
- [ ] Support both matching approaches simultaneously

### User Experience

- [ ] Session names visible in tab bar
- [ ] Seamless transition for existing users
- [ ] Clear error messages for unsupported Kitty versions

## Interface Definitions

### New Command Extensions

```rust
// kitty-lib/src/commands/kitten_set_tab_title.rs
pub struct KittenSetTabTitleCommand {
    title: String,
    match_pattern: Option<String>,
}

impl KittenSetTabTitleCommand {
    pub fn new(title: String) -> Self;
    pub fn with_match(mut self, pattern: &str) -> Self;
    pub fn execute(&self) -> Result<(), KittyError>;
}
```

### Enhanced Session Detection

```rust
// ksm-cli/src/session.rs
pub enum SessionSource {
    TabTitle,
    Environment,
    Default,
}

pub struct SessionContext {
    pub session_name: String,
    pub is_explicit: bool,
    pub source: SessionSource,  // New field
}

impl SessionContext {
    pub fn detect() -> Self {
        // Try tab title first
        if let Some(name) = Self::detect_from_tab_title() {
            return SessionContext {
                session_name: name,
                is_explicit: true,
                source: SessionSource::TabTitle,
            };
        }

        // Fall back to environment variable
        if let Ok(name) = env::var(KITTY_SESSION_PROJECT_ENV) {
            if !name.is_empty() {
                return SessionContext {
                    session_name: name,
                    is_explicit: true,
                    source: SessionSource::Environment,
                };
            }
        }

        // Default session
        SessionContext {
            session_name: "unnamed".to_string(),
            is_explicit: false,
            source: SessionSource::Default,
        }
    }

    fn detect_from_tab_title() -> Option<String>;
}
```

### Tab Title Patterns

```
Format: session:<name>[ - <description>]

Examples:
- session:myproject
- session:myproject - Development
- session:backend-api - Testing Environment
- session:frontend
```

## Acceptance Criteria

### Core Functionality

- [ ] Can set tab titles with session markers
- [ ] Can detect sessions from tab titles
- [ ] Can match tabs by title patterns
- [ ] Backward compatibility maintained

### Testing Requirements

- [ ] Unit tests for title parsing logic
- [ ] Integration tests for session detection
- [ ] Tests for migration scenarios
- [ ] Tests for fallback behavior

### Documentation

- [ ] Update CLAUDE.md with new approach
- [ ] Add migration guide for users
- [ ] Document tab title format

## Implementation Notes

### Key Considerations

1. **Title Format**: Use `session:` prefix for machine parsing, allow flexible suffix for human readability
2. **Regex Patterns**: Use `^session:([^\\s-]+)` to extract session name
3. **Fallback Chain**: Tab title → Environment variable → Default
4. **Error Handling**: Gracefully handle missing titles, invalid patterns

### Title Preservation

To prevent automatic title changes from interfering with session-aware titles, users should configure Kitty:

```ini
# In kitty.conf
shell_integration no-title
```

This prevents shell integration from automatically overwriting tab titles with directory names or command names, ensuring session markers persist across directory changes and command execution.

### Configuration Recommendations

- **Required**: `shell_integration no-title` - Prevents automatic title overwriting
- **Optional**: Configure custom tab title format in shell prompt to complement session titles
- **Optional**: Use terminal multiplexer (tmux/screen) awareness to enhance title management

### Potential Edge Cases

1. **Existing Custom Titles**: Users may have manually set tab titles
2. **Special Characters**: Session names with regex metacharacters
3. **Title Length Limits**: Kitty may have limits on title length
4. **Multi-window Tabs**: Ensure all windows see consistent session

### Implementation Order

1. Add `KittenSetTabTitleCommand` to kitty-lib
2. Extend `KittenLsCommand` with title matching
3. Update `SessionContext::detect()` with tab title detection
4. Modify session creation to set tab titles
5. Add tests and documentation
6. Deploy with backward compatibility

## Kitty Configuration for Title Preservation

### The Automatic Title Change Problem

Kitty can automatically change tab titles through:

1. **Shell Integration**: Updates titles with current directory/command
2. **OSC Sequences**: Programs like vim, ssh can override titles
3. **Shell Functions**: precmd/preexec hooks may update titles

### Recommended Kitty Configuration

Add to `~/.config/kitty/kitty.conf`:

```bash
# Disable automatic title updates to preserve session markers
shell_integration no-title

# Optional: Keep other shell integration features
# shell_integration no-title no-cursor

# Simple tab title template (preserves our format)
tab_title_template "{title}"
```

### Shell Configuration

For Zsh users, add to `~/.zshrc`:

```bash
# Prevent automatic title changes
export DISABLE_AUTO_TITLE="true"
```

### Using the rename-tab Command

KSM provides a `rename-tab` command that preserves session markers:

```bash
# In a session named "myproject"
ksm rename-tab "Development Environment"
# Sets title to: "session:myproject - Development Environment"

# Not in a session
ksm rename-tab "Personal Tasks"
# Sets title to: "Personal Tasks"
```

## Technical Debt Tracking

- [ ] Future: Remove environment variable approach after deprecation period
- [ ] Future: Add session metadata beyond just name (creation time, project path)
- [ ] Future: Consider persistent session storage for cross-terminal session sharing
- [ ] Future: Add periodic title health check to restore overwritten session markers

## Alternative Approaches Considered

### 1. User Variables (`var:`)

- **Pros**: Semantic, supports complex metadata
- **Cons**: Still window-level, requires managing all windows

### 2. Hybrid (Title + User Variables)

- **Pros**: Most robust, supports rich metadata
- **Cons**: More complex implementation, overkill for current needs

### 3. Custom Kitty Extension

- **Pros**: Could provide perfect solution
- **Cons**: Requires Kitty modification, not portable

## Decision

Use **tab title-based identification** as the primary approach because:

1. Simplest implementation with biggest impact
2. Visible to users (improves UX)
3. Truly tab-level persistence
4. Easy migration path
5. No Kitty modifications required
