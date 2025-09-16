use std::env;

/// The source of session identification
#[derive(Debug, Clone, PartialEq)]
pub enum SessionSource {
    /// Session detected from tab title with session: prefix
    TabTitle,
    /// Session detected from KITTY_SESSION_PROJECT environment variable
    Environment,
    /// Default unnamed session (no session context found)
    Default,
}

/// The environment variable used to identify the session/project context
pub const KITTY_SESSION_PROJECT_ENV: &str = "KITTY_SESSION_PROJECT";

/// The default session name for tabs created outside of any specific session
pub const UNNAMED_SESSION: &str = "unnamed";

/// Represents the current session context detected from tab titles or environment variables
#[derive(Debug, Clone, PartialEq)]
pub struct SessionContext {
    /// The session/project name, or "unnamed" if no session context is found
    pub session_name: String,
    /// Whether this session was explicitly set (true) or is the default unnamed session (false)
    pub is_explicit: bool,
    /// The source of the session identification
    pub source: SessionSource,
}

impl SessionContext {
    /// Detects the current session context from tab title or environment variables
    pub fn detect() -> Self {
        // Try tab title first
        if let Some(session_name) = Self::detect_from_tab_title() {
            return Self {
                session_name,
                is_explicit: true,
                source: SessionSource::TabTitle,
            };
        }

        // Fall back to environment variable
        match env::var(KITTY_SESSION_PROJECT_ENV) {
            Ok(session_name) if !session_name.is_empty() => Self {
                session_name,
                is_explicit: true,
                source: SessionSource::Environment,
            },
            _ => Self {
                session_name: UNNAMED_SESSION.to_string(),
                is_explicit: false,
                source: SessionSource::Default,
            },
        }
    }

    /// Creates a new explicit session context with the given name
    pub fn new(session_name: impl Into<String>) -> Self {
        Self {
            session_name: session_name.into(),
            is_explicit: true,
            source: SessionSource::Environment, // Default to environment source for manually created contexts
        }
    }

    /// Creates the default unnamed session context
    pub fn unnamed() -> Self {
        Self {
            session_name: UNNAMED_SESSION.to_string(),
            is_explicit: false,
            source: SessionSource::Default,
        }
    }

    /// Returns true if this is the unnamed default session
    pub fn is_unnamed(&self) -> bool {
        self.session_name == UNNAMED_SESSION
    }

    /// Returns the session name
    pub fn name(&self) -> &str {
        &self.session_name
    }

    /// Detects session from the current tab title by parsing the session: prefix
    fn detect_from_tab_title() -> Option<String> {
        use kitty_lib::commands::ls::KittenLsCommand;
        use kitty_lib::executor::{CommandExecutor, KittyExecutor};

        // Create a KittyExecutor to get the current tab info
        let executor = KittyExecutor::new();

        // Get current tab info
        let ls_result = executor.ls(KittenLsCommand::new()).ok()?;

        // Find the active tab across all OS windows
        for os_window in ls_result {
            for tab in os_window.tabs {
                if tab.is_active {
                    // Parse the tab title for session: prefix
                    if let Some(session_name) = Self::parse_session_from_title(&tab.title) {
                        return Some(session_name);
                    }
                }
            }
        }

        None
    }

    /// Parses a session name from a tab title with the format "session:<name>[ - description]"
    pub fn parse_session_from_title(title: &str) -> Option<String> {
        if let Some(after_prefix) = title.strip_prefix("session:") {
            // Split on " - " to handle optional description
            let session_name = after_prefix.split(" - ").next()?;

            if !session_name.is_empty() {
                Some(session_name.to_string())
            } else {
                None
            }
        } else {
            None
        }
    }
}

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Last active tab tracking for sessions
static LAST_ACTIVE_TABS: OnceLock<RwLock<HashMap<String, u32>>> = OnceLock::new();

fn get_last_active_tabs() -> &'static RwLock<HashMap<String, u32>> {
    LAST_ACTIVE_TABS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Utilities for working with session contexts
pub struct SessionUtils;

impl SessionUtils {
    /// Gets the current session context from environment variables
    pub fn current_session() -> SessionContext {
        SessionContext::detect()
    }

    /// Checks if we're currently in a session context (explicit session set)
    pub fn in_session() -> bool {
        SessionContext::detect().is_explicit
    }

    /// Gets the session name for use in Kitty commands, or None if unnamed
    pub fn session_name_for_kitty() -> Option<String> {
        let context = SessionContext::detect();
        if context.is_explicit {
            Some(context.session_name)
        } else {
            None
        }
    }

    /// Record the last active tab for a session
    pub fn set_last_active_tab(session_name: &str, tab_id: u32) {
        if let Ok(mut tabs) = get_last_active_tabs().write() {
            tabs.insert(session_name.to_string(), tab_id);
            log::debug!(
                "Set last active tab for session '{}' to {}",
                session_name,
                tab_id
            );
        }
    }

    /// Get the last active tab for a session
    pub fn get_last_active_tab(session_name: &str) -> Option<u32> {
        get_last_active_tabs()
            .read()
            .ok()
            .and_then(|tabs| tabs.get(session_name).copied())
    }

    /// Clear the last active tab tracking for a session (useful when session is deleted)
    pub fn clear_last_active_tab(session_name: &str) {
        if let Ok(mut tabs) = get_last_active_tabs().write() {
            tabs.remove(session_name);
            log::debug!(
                "Cleared last active tab tracking for session '{}'",
                session_name
            );
        }
    }

    /// Get all tracked session names
    pub fn get_tracked_sessions() -> Vec<String> {
        get_last_active_tabs()
            .read()
            .map(|tabs| tabs.keys().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as StdHashMap;
    use std::env;
    use std::sync::{Arc, Mutex};

    /// Thread-safe environment variable mock for testing
    #[allow(dead_code)]
    struct TestEnv {
        vars: Arc<Mutex<StdHashMap<String, String>>>,
    }

    #[allow(dead_code)]
    impl TestEnv {
        fn new() -> Self {
            Self {
                vars: Arc::new(Mutex::new(StdHashMap::new())),
            }
        }

        fn set_var(&self, key: &str, value: &str) {
            let mut vars = self.vars.lock().unwrap();
            vars.insert(key.to_string(), value.to_string());
        }

        fn remove_var(&self, key: &str) {
            let mut vars = self.vars.lock().unwrap();
            vars.remove(key);
        }

        fn get_var(&self, key: &str) -> Option<String> {
            let vars = self.vars.lock().unwrap();
            vars.get(key).cloned()
        }
    }

    /// Create a SessionContext with a custom environment mock
    #[allow(dead_code)]
    impl SessionContext {
        fn detect_with_env(test_env: &TestEnv) -> Self {
            match test_env.get_var(KITTY_SESSION_PROJECT_ENV) {
                Some(session_name) if !session_name.is_empty() => Self {
                    session_name,
                    is_explicit: true,
                    source: SessionSource::Environment,
                },
                _ => Self {
                    session_name: UNNAMED_SESSION.to_string(),
                    is_explicit: false,
                    source: SessionSource::Default,
                },
            }
        }
    }

    #[test]
    fn test_session_context_detect_with_explicit_session() {
        let test_env = TestEnv::new();
        test_env.set_var(KITTY_SESSION_PROJECT_ENV, "test-project");

        let context = SessionContext::detect_with_env(&test_env);
        assert_eq!(context.session_name, "test-project");
        assert!(context.is_explicit);
        assert!(!context.is_unnamed());
    }

    #[test]
    fn test_session_context_detect_without_session() {
        let test_env = TestEnv::new();
        // Don't set any environment variables

        let context = SessionContext::detect_with_env(&test_env);
        assert_eq!(context.session_name, UNNAMED_SESSION);
        assert!(!context.is_explicit);
        assert!(context.is_unnamed());
    }

    #[test]
    fn test_session_context_detect_with_empty_session() {
        let test_env = TestEnv::new();
        test_env.set_var(KITTY_SESSION_PROJECT_ENV, "");

        let context = SessionContext::detect_with_env(&test_env);
        assert_eq!(context.session_name, UNNAMED_SESSION);
        assert!(!context.is_explicit);
        assert!(context.is_unnamed());
    }

    #[test]
    fn test_session_context_new() {
        let context = SessionContext::new("my-project");
        assert_eq!(context.session_name, "my-project");
        assert!(context.is_explicit);
        assert!(!context.is_unnamed());
        assert_eq!(context.name(), "my-project");
    }

    #[test]
    fn test_session_context_unnamed() {
        let context = SessionContext::unnamed();
        assert_eq!(context.session_name, UNNAMED_SESSION);
        assert!(!context.is_explicit);
        assert!(context.is_unnamed());
        assert_eq!(context.name(), UNNAMED_SESSION);
    }

    #[test]
    fn test_session_utils_current_session() {
        // Note: This test uses actual environment variables since SessionUtils doesn't have dependency injection yet
        // In a production refactor, we'd want to make SessionUtils testable with dependency injection too
        let original_value = env::var(KITTY_SESSION_PROJECT_ENV).ok();

        // Test with explicit session
        unsafe { env::set_var(KITTY_SESSION_PROJECT_ENV, "test-session") };
        let context = SessionUtils::current_session();
        assert_eq!(context.session_name, "test-session");
        assert!(context.is_explicit);

        // Test without session
        unsafe { env::remove_var(KITTY_SESSION_PROJECT_ENV) };
        let context = SessionUtils::current_session();
        assert_eq!(context.session_name, UNNAMED_SESSION);
        assert!(!context.is_explicit);

        // Restore original value if it existed
        match original_value {
            Some(val) => unsafe { env::set_var(KITTY_SESSION_PROJECT_ENV, val) },
            None => unsafe { env::remove_var(KITTY_SESSION_PROJECT_ENV) },
        }
    }

    #[test]
    fn test_session_utils_in_session() {
        let original_value = env::var(KITTY_SESSION_PROJECT_ENV).ok();

        // Test with explicit session
        unsafe { env::set_var(KITTY_SESSION_PROJECT_ENV, "test-session") };
        assert!(SessionUtils::in_session());

        // Test without session
        unsafe { env::remove_var(KITTY_SESSION_PROJECT_ENV) };
        assert!(!SessionUtils::in_session());

        // Restore original value if it existed
        match original_value {
            Some(val) => unsafe { env::set_var(KITTY_SESSION_PROJECT_ENV, val) },
            None => unsafe { env::remove_var(KITTY_SESSION_PROJECT_ENV) },
        }
    }

    #[test]
    fn test_session_utils_session_name_for_kitty() {
        let original_value = env::var(KITTY_SESSION_PROJECT_ENV).ok();

        // Test with explicit session
        unsafe { env::set_var(KITTY_SESSION_PROJECT_ENV, "test-session") };
        assert_eq!(
            SessionUtils::session_name_for_kitty(),
            Some("test-session".to_string())
        );

        // Test without session
        unsafe { env::remove_var(KITTY_SESSION_PROJECT_ENV) };
        assert_eq!(SessionUtils::session_name_for_kitty(), None);

        // Restore original value if it existed
        match original_value {
            Some(val) => unsafe { env::set_var(KITTY_SESSION_PROJECT_ENV, val) },
            None => unsafe { env::remove_var(KITTY_SESSION_PROJECT_ENV) },
        }
    }

    #[test]
    fn test_last_active_tab_tracking() {
        // Test setting and getting last active tabs
        SessionUtils::set_last_active_tab("test-session-1", 100);
        assert_eq!(
            SessionUtils::get_last_active_tab("test-session-1"),
            Some(100)
        );

        // Test updating existing session
        SessionUtils::set_last_active_tab("test-session-1", 200);
        assert_eq!(
            SessionUtils::get_last_active_tab("test-session-1"),
            Some(200)
        );

        // Test different session
        SessionUtils::set_last_active_tab("test-session-2", 300);
        assert_eq!(
            SessionUtils::get_last_active_tab("test-session-2"),
            Some(300)
        );
        assert_eq!(
            SessionUtils::get_last_active_tab("test-session-1"),
            Some(200)
        ); // Should not affect

        // Test non-existent session
        assert_eq!(SessionUtils::get_last_active_tab("non-existent"), None);

        // Test clearing specific session
        SessionUtils::clear_last_active_tab("test-session-1");
        assert_eq!(SessionUtils::get_last_active_tab("test-session-1"), None);
        assert_eq!(
            SessionUtils::get_last_active_tab("test-session-2"),
            Some(300)
        ); // Should not affect

        // Test getting tracked sessions
        SessionUtils::set_last_active_tab("tracked-1", 1);
        SessionUtils::set_last_active_tab("tracked-2", 2);
        SessionUtils::set_last_active_tab("tracked-3", 3);

        let tracked = SessionUtils::get_tracked_sessions();
        assert!(tracked.contains(&"tracked-1".to_string()));
        assert!(tracked.contains(&"tracked-2".to_string()));
        assert!(tracked.contains(&"tracked-3".to_string()));
        assert!(tracked.contains(&"test-session-2".to_string())); // From previous test

        // Clear all for cleanup
        SessionUtils::clear_last_active_tab("tracked-1");
        SessionUtils::clear_last_active_tab("tracked-2");
        SessionUtils::clear_last_active_tab("tracked-3");
        SessionUtils::clear_last_active_tab("test-session-2");
    }

    #[test]
    fn test_last_active_tab_thread_safety() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::thread;

        let counter = Arc::new(AtomicU32::new(0));
        let mut handles = vec![];

        // Spawn multiple threads to test concurrent access
        for i in 0..10 {
            let counter_clone = Arc::clone(&counter);
            let handle = thread::spawn(move || {
                let session_name = format!("thread-session-{}", i);
                let tab_id = counter_clone.fetch_add(1, Ordering::SeqCst);

                // Set last active tab
                SessionUtils::set_last_active_tab(&session_name, tab_id);

                // Verify it was set correctly
                assert_eq!(
                    SessionUtils::get_last_active_tab(&session_name),
                    Some(tab_id)
                );

                // Clear it
                SessionUtils::clear_last_active_tab(&session_name);
                assert_eq!(SessionUtils::get_last_active_tab(&session_name), None);
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }
}
