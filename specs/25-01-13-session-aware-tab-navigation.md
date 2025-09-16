# Session-Aware Tab Navigation

## Value statement

Enable users to create and navigate tabs within Kitty sessions with session-aware context. When users create new tabs from within an existing session, those tabs should be associated with the parent session and allow for session-scoped tab navigation. This improves workflow organization by allowing users to cycle through only the tabs relevant to their current session context, rather than all tabs in the terminal.

## Features Implemented

### 1. Automatic session-aware tab creation

- [x] 1.1 When creating a new tab from within an existing session, automatically inherit the parent session's `KITTY_SESSION_PROJECT` environment variable
- [x] 1.2 Create an 'unnamed' default session for all tabs created outside of existing sessions
- [x] 1.3 Ensure newly created tabs appear with appropriate session identification in tab titles

### 2. Session tab navigation commands

- [x] 2.1 Implement `next-tab` command to cycle to the next tab within the current session
- [x] 2.2 Implement `prev-tab` command to cycle to the previous tab within the current session
- [x] 2.3 Navigation should wrap around by default (last tab cycles to first, first tab cycles to last)
- [x] 2.4 Add `--no-wrap` option to prevent wrap-around behavior
- [x] 2.5 Commands should be no-op if user is not within a session or session has only one tab

### 3. Session context detection

- [x] 3.1 Add ability to detect current session context from environment variables
- [x] 3.2 List all tabs belonging to the current session
- [x] 3.3 Identify the currently active tab within a session

### 4. Enhanced session switching

- [x] 4.1 When switching between sessions, focus on the last active tab from the target session
- [x] 4.2 Track and maintain last active tab state per session
- [x] 4.3 Fall back to first tab if no last active tab is available

### 5. CLI command integration

- [x] 5.1 Add `next-tab` subcommand to CLI
- [x] 5.2 Add `prev-tab` subcommand to CLI
- [x] 5.3 Add `--no-wrap` option support to navigation commands
- [x] 5.4 Add help documentation for new session navigation commands

### 6. Session-aware tab creation

- [x] 6.1 Add `new-tab` command to create tabs within current session context
- [x] 6.2 Support optional working directory specification for new tabs
- [x] 6.3 Support optional tab title specification
- [x] 6.4 Automatic session inheritance when creating tabs from within existing sessions
- [x] 6.5 Create tabs in 'unnamed' session when no session context exists

### 7. Session lifecycle management

- [x] 7.1 Add `close-all-session-tabs` command to close all tabs in current session
- [x] 7.2 Support optional session name parameter to close specific session tabs
- [x] 7.3 Confirm before closing multiple tabs (with --force option to skip)
- [x] 7.4 Handle edge case of closing the last tab in a session

## Existing updates

### 1. Extend Kitty integration

- [x] 1.1 Add methods to `Kitty` struct for session-aware tab operations
- [x] 1.2 Extend `KittenLaunchCommand` to support session-aware tab creation
- [x] 1.3 Add session filtering to tab listing operations

### 2. Update configuration system

- [x] 2.1 Add configuration options for session tab navigation behavior (wrap/no-wrap default)
- [x] 2.2 Support keybinding configuration for new navigation commands
- [x] 2.3 Add configuration for default 'unnamed' session behavior

## Technical Requirements

### Environment Variable Strategy

- Use existing `KITTY_SESSION_PROJECT` as the primary session identifier
- Maintain backward compatibility with existing session creation logic
- Ensure environment variable propagation when creating tabs from within sessions

### Tab Navigation Logic

- Use `kitty @ ls --match-tab env:KITTY_SESSION_PROJECT=<session>` for accurate tab matching
- Implement circular navigation (wrap-around) behavior by default with `--no-wrap` option
- Handle edge cases: single tab, no session context, invalid session
- Parse JSON output from `kitty @ ls` to identify tab order and active states

### Session State Management

- Track last active tab per session for enhanced session switching
- Use Kitty's built-in tab ordering and focus mechanisms
- Persist session state across terminal restarts where possible

### Command Structure

- Follow existing CLI patterns from `src/cmd/` modules
- Integrate with existing `App` and `Kitty` abstractions
- Provide meaningful error messages for invalid operations

### Tab Creation Logic

- Use `KittenLaunchCommand` with session inheritance for `new-tab` command
- Detect current session context and automatically inherit `KITTY_SESSION_PROJECT`
- Support `--cwd <path>` and `--title <title>` options for tab customization
- Default to current working directory and auto-generated title if not specified
- Create in 'unnamed' session when run outside of any session context

### Session Lifecycle Management

- Use `kitty @ close-tab` with session filtering for closing session tabs
- Query current session tabs using `kitty @ ls --match-tab env:KITTY_SESSION_PROJECT=<session>`
- Implement confirmation prompt with tab count: "Close 5 tabs in session 'myproject'? (y/N)"
- Support `--force` flag to skip confirmation prompt
- Handle graceful degradation when closing last tab in session

## Implementation Architecture

### Core Components

1. **Session Context Detection**: New module to detect and validate current session context
2. **Session Tab Navigation**: New module for tab navigation logic within sessions
3. **Enhanced Kitty Integration**: Extensions to existing `Kitty` struct for session-aware operations
4. **CLI Commands**: New command modules following existing patterns
5. **In-Memory Testing Layout**: Enhanced kitty-lib with in-memory tab/session simulation for robust testing

### Integration Points

- `ksm-cli/src/kitty.rs`: Extend `Kitty` struct with session navigation and lifecycle methods
- `kitty-lib/src/commands/`: Add session-aware tab listing, navigation, creation, and closing commands
- `ksm-cli/src/cmd/`: Add new command modules (next_tab.rs, prev_tab.rs, new_tab.rs, close_all_session_tabs.rs)
- `ksm-cli/src/cli.rs`: Register new subcommands in CLI structure
- `kitty-lib/src/commands/close_tab.rs`: New command for closing tabs by session context

## Acceptance Criteria

### Core Navigation (✅ Complete)

- [x] User can create new tabs from within a session that inherit session context
- [x] User can navigate between tabs using `next-tab` and `prev-tab` commands
- [x] Navigation only cycles through tabs belonging to the current session
- [x] Session switching focuses on the last active tab from the target session
- [x] Commands gracefully handle edge cases (no session, single tab, etc.)
- [x] Existing session functionality remains unchanged and compatible
- [x] New commands integrate seamlessly with existing CLI structure and patterns

### Enhanced Tab Management (✅ Complete)

- [x] User can create new tabs with `new-tab` command that inherit current session context
- [x] User can specify working directory and title when creating new tabs
- [x] New tabs created outside sessions are automatically assigned to 'unnamed' session
- [x] User can close all tabs in current session with `close-all-session-tabs` command
- [x] User receives confirmation prompt before closing multiple tabs (unless --force used)
- [x] Commands handle edge cases gracefully (closing last tab, empty sessions, etc.)

## Tech debt created

- None created - implementation successfully extends existing patterns without breaking changes
- All new features integrate seamlessly with existing codebase architecture

## Post-Implementation Issues & Fixes

### Regression Found (2025-01-14)

**Issue**: The `prev-tab` and `next-tab` commands were failing with "No tabs found in session" error even when tabs existed in the session.

**Root Cause**: The implementation was using `--match=env:KITTY_SESSION_PROJECT=<session>` instead of `--match-tab env:KITTY_SESSION_PROJECT=<session>` in the `kitty @ ls` command. The `--match` flag filters windows that have the environment variable, while `--match-tab` filters tabs containing windows with that environment variable. Additionally, after using `--match-tab`, the code was redundantly filtering the already-filtered results.

**Fix Applied**:

1. Added `use_tab_match` field to `KittenLsCommand` to specify whether to use `--match` or `--match-tab`
2. Created `match_tab_env()` method for tab-level filtering operations
3. Updated `KittyExecutor::navigate_tab()` to use `--match-tab` for session filtering
4. Removed redundant filtering logic since `--match-tab` already returns the correct tabs
5. Updated all session tab operations to use `match_tab_env()` instead of `match_env()`

### Regression Found (2025-01-16)

**Issue**: Navigation commands (`prev-tab` and `next-tab`) were not working - tabs were not changing when running the commands.

**Root Cause**: The implementation was checking `tab.state == "active"` to find the currently active tab, but in Kitty's actual JSON output, the `state` field is `null`. The correct field to check is `tab.is_active` which is a boolean.

**Fix Applied**:

1. Added `is_active` and `is_focused` boolean fields to the `KittyTab` struct to match Kitty's actual JSON schema
2. Updated `KittyExecutor::navigate_tab()` to check `t.is_active` instead of `t.state.as_ref().is_some_and(|s| s == "active")`
3. Updated `MockExecutor` to properly set the `is_active` field when tabs change
4. Verified navigation works correctly: tabs cycle sequentially in creation order (Tab 1 → Tab 2 → Tab 3) with proper wrap-around

### Prevention Strategies for Future Planning

1. **Explicit API Documentation**: When planning features that interact with external tools (like Kitty), document the exact API calls and flags that will be used. In this case, explicitly stating "use `--match-tab` for tab filtering" would have prevented the issue.

2. **Integration Test Requirements**: Specify integration tests that exercise the actual Kitty commands, not just mock responses. This would have caught the `--match` vs `--match-tab` discrepancy early.

3. **Command Output Validation**: Include sample command outputs in the spec to validate assumptions. For example:

   ```bash
   # Expected command and output structure
   kitty @ ls --match-tab env:KITTY_SESSION_PROJECT=myproject
   # Returns: OS windows containing tabs that have the environment variable
   ```

4. **Edge Case Enumeration**: Explicitly list edge cases in the spec with their expected behavior:
   - Single tab in session → navigation should be no-op
   - Multiple tabs in session → should cycle correctly
   - No tabs matching session → should return appropriate error

5. **API Abstraction Layer Design**: Consider designing a clear abstraction between the business logic and external tool APIs. The `KittenLsCommand` abstraction was good, but it should have been more explicit about its filtering behavior from the start.

6. **Mock Behavior Alignment**: Ensure mock implementations closely mirror actual tool behavior. The `MockExecutor` should validate that the correct match type is being used for the intended operation.
