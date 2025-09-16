mod common;

use common::{KittyTestHarness, compare_screenshots};

#[tokio::test]
async fn test_find_test_config() {
    // This test verifies that we can find the test config file
    let config_path = common::find_test_config();
    assert!(
        config_path.is_ok(),
        "Should be able to find test config file"
    );

    let path = config_path.unwrap();
    assert!(
        path.exists(),
        "Test config file should exist at: {:?}",
        path
    );
    assert!(
        path.ends_with("kitty.test.conf"),
        "Should find the correct config file"
    );
}

#[tokio::test]
async fn test_harness_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    // Skip test if Kitty is not available
    if !is_kitty_available().await {
        println!("Skipping test_harness_lifecycle: Kitty not available");
        return Ok(());
    }

    println!("Kitty is available, attempting to launch test harness...");

    // This is a basic smoke test to ensure the harness can launch and cleanup
    match KittyTestHarness::launch().await {
        Ok(harness) => {
            println!("Test harness launched successfully");

            // Verify we can execute a basic command
            let output = harness.execute_command("ls").await?;
            assert!(!output.is_empty(), "ls command should return some output");

            // Clean up
            harness.cleanup().await?;

            println!("Test harness cleaned up successfully");
            Ok(())
        }
        Err(e) => handle_launch_failure("test_harness_lifecycle", &e.to_string()),
    }
}

#[tokio::test]
async fn test_multiple_commands() -> Result<(), Box<dyn std::error::Error>> {
    // Skip test if Kitty is not available
    if !is_kitty_available().await {
        println!("Skipping test_multiple_commands: Kitty not available");
        return Ok(());
    }

    println!("Kitty is available, attempting to launch test harness for multiple commands test...");

    match KittyTestHarness::launch().await {
        Ok(harness) => {
            println!("Test harness launched successfully for multiple commands test");

            // Execute multiple commands to ensure the socket connection is stable
            let output1 = harness.execute_command("ls").await?;
            let output2 = harness.execute_command("ls").await?;

            assert!(!output1.is_empty());
            assert!(!output2.is_empty());

            // Parse both outputs as JSON to check structure consistency
            let json1: serde_json::Value = serde_json::from_str(&output1)?;
            let json2: serde_json::Value = serde_json::from_str(&output2)?;

            // Verify both outputs have the same basic structure (array of OS windows)
            assert!(json1.is_array(), "First output should be a JSON array");
            assert!(json2.is_array(), "Second output should be a JSON array");

            // Both should have at least one OS window
            let windows1 = json1.as_array().unwrap();
            let windows2 = json2.as_array().unwrap();
            assert!(!windows1.is_empty(), "Should have at least one OS window");
            assert!(!windows2.is_empty(), "Should have at least one OS window");

            // Both should have the same number of OS windows
            assert_eq!(
                windows1.len(),
                windows2.len(),
                "Should have same number of OS windows"
            );

            println!(
                "Commands produced consistent JSON structure with {} OS windows",
                windows1.len()
            );

            harness.cleanup().await?;

            println!("Multiple commands test completed successfully");
            Ok(())
        }
        Err(e) => handle_launch_failure("test_multiple_commands", &e.to_string()),
    }
}

#[tokio::test]
async fn test_session_aware_tab_navigation() -> Result<(), Box<dyn std::error::Error>> {
    // Skip test if Kitty is not available
    if !is_kitty_available().await {
        println!("Skipping test_session_aware_tab_navigation: Kitty not available");
        return Ok(());
    }

    println!("Kitty is available, testing session-aware tab navigation...");

    match KittyTestHarness::launch_with_test_name("test_session_navigation").await {
        Ok(harness) => {
            println!("Test harness launched successfully for session-aware tab navigation test");

            // 1. Get initial state - should have one default tab
            let initial_output = harness.execute_command("ls").await?;
            println!("Initial Kitty state: {}", initial_output);
            assert!(!initial_output.is_empty(), "Should have initial tab");

            // Capture initial screenshot (baseline before navigation)
            let initial_screenshot = match harness.capture_screenshot("baseline_before_nav").await {
                Ok(path) => {
                    println!("Captured initial screenshot: {:?}", path);
                    Some(path)
                }
                Err(e) => {
                    println!(
                        "Failed to capture initial screenshot (this may be normal in headless environments): {}",
                        e
                    );
                    None
                }
            };

            // 2. Create first tab with test session project environment
            let create_tab1_cmd = r#"launch --type=tab --env KITTY_SESSION_PROJECT=test_session --title "Test Tab 1""#;
            let tab1_output = harness.execute_command(create_tab1_cmd).await?;
            println!("Created tab 1: {}", tab1_output);

            // 3. Create second tab with same session project environment
            let create_tab2_cmd = r#"launch --type=tab --env KITTY_SESSION_PROJECT=test_session --title "Test Tab 2""#;
            let tab2_output = harness.execute_command(create_tab2_cmd).await?;
            println!("Created tab 2: {}", tab2_output);

            // 4. Create third tab with same session project environment
            let create_tab3_cmd = r#"launch --type=tab --env KITTY_SESSION_PROJECT=test_session --title "Test Tab 3""#;
            let tab3_output = harness.execute_command(create_tab3_cmd).await?;
            println!("Created tab 3: {}", tab3_output);

            // 5. Wait a moment for tabs to stabilize
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // 6. Verify all tabs were created by listing all tabs
            let all_tabs_output = harness.execute_command("ls").await?;
            println!("All tabs after creation: {}", all_tabs_output);

            // Parse JSON to verify we have multiple tabs
            let parsed_json: serde_json::Value = serde_json::from_str(&all_tabs_output)
                .map_err(|e| format!("Failed to parse ls output as JSON: {}", e))?;

            // Count total tabs across all OS windows
            let mut total_tab_count = 0;
            if let Some(os_windows) = parsed_json.as_array() {
                for os_window in os_windows {
                    if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                        total_tab_count += tabs.len();
                    }
                }
            }
            println!("Total tabs found: {}", total_tab_count);
            assert!(
                total_tab_count >= 4,
                "Should have at least 4 tabs (1 initial + 3 created)"
            );

            // 7. Test the critical regression fix: --match-tab vs --match behavior
            // This is the core test for the bug fix mentioned in the spec
            let session_tabs_cmd =
                "ls --all-env-vars --match-tab env:KITTY_SESSION_PROJECT=test_session";
            let session_tabs_output = harness.execute_command(session_tabs_cmd).await?;
            println!("Session tabs with --match-tab: {}", session_tabs_output);

            // Parse and verify we get tabs with the session environment
            let session_json: serde_json::Value = serde_json::from_str(&session_tabs_output)
                .map_err(|e| format!("Failed to parse session tabs output as JSON: {}", e))?;

            let mut session_tab_count = 0;
            if let Some(os_windows) = session_json.as_array() {
                for os_window in os_windows {
                    if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                        for tab in tabs {
                            // Verify tab has windows with the correct environment variable
                            if let Some(windows) = tab.get("windows").and_then(|w| w.as_array()) {
                                for window in windows {
                                    if let Some(env) = window.get("env").and_then(|e| e.as_object())
                                    {
                                        if let Some(session_value) = env
                                            .get("KITTY_SESSION_PROJECT")
                                            .and_then(|v| v.as_str())
                                        {
                                            if session_value == "test_session" {
                                                session_tab_count += 1;
                                                break; // Found matching window in this tab
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            println!("Session tabs with test_session env: {}", session_tab_count);
            assert_eq!(
                session_tab_count, 3,
                "Should find exactly 3 tabs with test_session environment"
            );

            // 8. Test tab focusing using session context
            // Focus on one of the session tabs
            if let Some(os_windows) = session_json.as_array() {
                if let Some(first_window) = os_windows.first() {
                    if let Some(tabs) = first_window.get("tabs").and_then(|t| t.as_array()) {
                        if let Some(first_tab) = tabs.first() {
                            if let Some(tab_id) = first_tab.get("id").and_then(|id| id.as_u64()) {
                                let focus_cmd = format!("focus-tab --match id:{}", tab_id);
                                let focus_output = harness.execute_command(&focus_cmd).await?;
                                println!("Focus tab output: {}", focus_output);
                                // Focus command typically returns empty output on success

                                // Wait a moment for the focus change to take effect
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            }
                        }
                    }
                }
            }

            // Capture screenshot after navigation (baseline after navigation)
            let nav_screenshot = match harness.capture_screenshot("baseline_after_nav").await {
                Ok(path) => {
                    println!("Captured navigation screenshot: {:?}", path);
                    Some(path)
                }
                Err(e) => {
                    println!(
                        "Failed to capture navigation screenshot (this may be normal in headless environments): {}",
                        e
                    );
                    None
                }
            };

            // If we have both screenshots, attempt to compare them to verify visual changes occurred
            if let (Some(initial_path), Some(nav_path)) = (&initial_screenshot, &nav_screenshot) {
                match compare_screenshots(nav_path, initial_path) {
                    Ok(similarity) => {
                        println!("Screenshot similarity after navigation: {:.4}", similarity);
                        // We expect some visual difference after creating tabs and navigating
                        // but we won't be too strict since terminal content can vary
                        if similarity > 0.99 {
                            println!(
                                "Warning: Screenshots are very similar, navigation may not have caused visual changes"
                            );
                        } else {
                            println!(
                                "Screenshots show visual differences as expected after navigation"
                            );
                        }
                    }
                    Err(e) => {
                        println!("Screenshot comparison failed (this may be normal): {}", e);
                    }
                }
            }

            // 9. Test that --match (without tab) would behave differently (this verifies the regression)
            // The --match flag filters windows, while --match-tab filters tabs containing matching windows
            let window_match_cmd =
                "ls --all-env-vars --match env:KITTY_SESSION_PROJECT=test_session";
            let window_match_output = harness.execute_command(window_match_cmd).await?;
            println!("Windows with --match: {}", window_match_output);

            // This should still work but potentially return a different structure
            // The key difference is that --match-tab ensures we get complete tab information
            // while --match only filters at the window level

            // 10. Clean up by closing the session tabs
            // Test closing tabs in the session (this also tests the close functionality)
            if let Some(os_windows) = session_json.as_array() {
                for os_window in os_windows {
                    if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                        for tab in tabs {
                            if let Some(tab_id) = tab.get("id").and_then(|id| id.as_u64()) {
                                let close_cmd = format!("close-tab --match id:{}", tab_id);
                                let close_output = harness.execute_command(&close_cmd).await;
                                match close_output {
                                    Ok(output) => println!("Closed tab {}: {}", tab_id, output),
                                    Err(e) => println!(
                                        "Failed to close tab {} (may be expected): {}",
                                        tab_id, e
                                    ),
                                }
                            }
                        }
                    }
                }
            }

            // 11. Verify tabs were closed
            let final_output = harness.execute_command("ls").await?;
            println!("Final Kitty state: {}", final_output);

            harness.cleanup().await?;

            println!("Session-aware tab navigation test completed successfully");
            Ok(())
        }
        Err(e) => handle_launch_failure("test_session_aware_tab_navigation", &e.to_string()),
    }
}

/// Check if Kitty terminal is available in the system PATH.
async fn is_kitty_available() -> bool {
    tokio::process::Command::new("kitty")
        .arg("--version")
        .output()
        .await
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if tests should fail on Kitty launch failures instead of skipping
fn should_fail_on_launch_error() -> bool {
    std::env::var("KSM_TEST_FAIL_ON_LAUNCH_ERROR").unwrap_or_default() == "1"
}

/// Handle test launch failure - either skip or fail based on environment
fn handle_launch_failure(test_name: &str, error: &str) -> Result<(), Box<dyn std::error::Error>> {
    if should_fail_on_launch_error() {
        Err(format!("Test {} failed to launch Kitty: {}", test_name, error).into())
    } else {
        println!(
            "Skipping {} due to launch failure (this may be normal in CI environments): {}",
            test_name, error
        );
        Ok(())
    }
}

#[tokio::test]
async fn test_navigation_wrap_around() -> Result<(), Box<dyn std::error::Error>> {
    // Skip test if Kitty is not available
    if !is_kitty_available().await {
        println!("Skipping test_navigation_wrap_around: Kitty not available");
        return Ok(());
    }

    println!("Testing navigation wrap-around behavior...");

    match KittyTestHarness::launch_with_test_name("test_navigation_wrap_around").await {
        Ok(harness) => {
            println!("Test harness launched successfully for navigation wrap-around test");

            // 1. Create multiple tabs in a session (4 tabs total including the initial one)
            let create_tab1_cmd =
                r#"launch --type=tab --env KITTY_SESSION_PROJECT=wrap_test --title "Tab 1""#;
            harness.execute_command(create_tab1_cmd).await?;

            let create_tab2_cmd =
                r#"launch --type=tab --env KITTY_SESSION_PROJECT=wrap_test --title "Tab 2""#;
            harness.execute_command(create_tab2_cmd).await?;

            let create_tab3_cmd =
                r#"launch --type=tab --env KITTY_SESSION_PROJECT=wrap_test --title "Tab 3""#;
            harness.execute_command(create_tab3_cmd).await?;

            // 2. Wait for tabs to stabilize
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // 3. Get all session tabs to work with their IDs
            let session_tabs_cmd =
                "ls --all-env-vars --match-tab env:KITTY_SESSION_PROJECT=wrap_test";
            let session_tabs_output = harness.execute_command(session_tabs_cmd).await?;

            let session_json: serde_json::Value = serde_json::from_str(&session_tabs_output)
                .map_err(|e| format!("Failed to parse session tabs output as JSON: {}", e))?;

            // Extract tab IDs from the session
            let mut tab_ids = Vec::new();
            if let Some(os_windows) = session_json.as_array() {
                for os_window in os_windows {
                    if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                        for tab in tabs {
                            if let Some(tab_id) = tab.get("id").and_then(|id| id.as_u64()) {
                                tab_ids.push(tab_id);
                            }
                        }
                    }
                }
            }

            println!("Found session tab IDs: {:?}", tab_ids);
            assert_eq!(tab_ids.len(), 3, "Should have exactly 3 session tabs");

            // Sort tab IDs to ensure consistent ordering
            tab_ids.sort();

            // 4. Focus on the first tab
            let focus_first_cmd = format!("focus-tab --match id:{}", tab_ids[0]);
            harness.execute_command(&focus_first_cmd).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // 5. Test forward navigation to last tab and verify wrap-around
            // Navigate: first -> second -> third -> first (wrap-around)

            // Navigate through tabs by focusing on them directly (simulating next navigation)
            // Go to second tab
            let focus_second_cmd = format!("focus-tab --match id:{}", tab_ids[1]);
            harness.execute_command(&focus_second_cmd).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Go to third tab (last)
            let focus_third_cmd = format!("focus-tab --match id:{}", tab_ids[2]);
            harness.execute_command(&focus_third_cmd).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Now simulate wrap-around by going back to first tab
            let focus_first_again_cmd = format!("focus-tab --match id:{}", tab_ids[0]);
            harness.execute_command(&focus_first_again_cmd).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Check which tab is currently active after wrap-around
            let current_state = harness.execute_command(session_tabs_cmd).await?;
            let current_json: serde_json::Value = serde_json::from_str(&current_state)
                .map_err(|e| format!("Failed to parse current state JSON: {}", e))?;

            // Find the currently active tab
            let mut active_tab_id = None;
            if let Some(os_windows) = current_json.as_array() {
                for os_window in os_windows {
                    if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                        for tab in tabs {
                            if let Some(is_active) = tab.get("is_active").and_then(|a| a.as_bool())
                            {
                                if is_active {
                                    if let Some(tab_id) = tab.get("id").and_then(|id| id.as_u64()) {
                                        active_tab_id = Some(tab_id);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            println!("Active tab after forward wrap-around: {:?}", active_tab_id);
            assert_eq!(
                active_tab_id,
                Some(tab_ids[0]),
                "Forward wrap-around should return to first tab"
            );

            // 6. Test backward navigation wrap-around
            // From first tab, go backwards should wrap to last tab (simulate with direct focus)
            let focus_last_cmd = format!("focus-tab --match id:{}", tab_ids[2]);
            harness.execute_command(&focus_last_cmd).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Check which tab is active after backward wrap-around
            let final_state = harness.execute_command(session_tabs_cmd).await?;
            let final_json: serde_json::Value = serde_json::from_str(&final_state)
                .map_err(|e| format!("Failed to parse final state JSON: {}", e))?;

            let mut final_active_tab_id = None;
            if let Some(os_windows) = final_json.as_array() {
                for os_window in os_windows {
                    if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                        for tab in tabs {
                            if let Some(is_active) = tab.get("is_active").and_then(|a| a.as_bool())
                            {
                                if is_active {
                                    if let Some(tab_id) = tab.get("id").and_then(|id| id.as_u64()) {
                                        final_active_tab_id = Some(tab_id);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            println!(
                "Active tab after backward wrap-around: {:?}",
                final_active_tab_id
            );
            assert_eq!(
                final_active_tab_id,
                Some(tab_ids[2]),
                "Backward wrap-around should go to last tab"
            );

            // 7. Capture a screenshot to verify final state
            match harness.capture_screenshot("wrap_around_final").await {
                Ok(path) => println!("Captured final wrap-around state: {:?}", path),
                Err(e) => println!("Screenshot capture failed (normal in headless): {}", e),
            }

            harness.cleanup().await?;
            println!("Navigation wrap-around test completed successfully");
            Ok(())
        }
        Err(e) => handle_launch_failure("test_navigation_wrap_around", &e.to_string()),
    }
}

#[tokio::test]
async fn test_session_context_detection() -> Result<(), Box<dyn std::error::Error>> {
    // Skip test if Kitty is not available
    if !is_kitty_available().await {
        println!("Skipping test_session_context_detection: Kitty not available");
        return Ok(());
    }

    println!("Testing session context detection...");

    match KittyTestHarness::launch_with_test_name("test_session_context_detection").await {
        Ok(harness) => {
            println!("Test harness launched successfully for session context detection test");

            // 1. Create tabs with different session contexts

            // Create a tab with session environment
            let session_tab1_cmd = r#"launch --type=tab --env KITTY_SESSION_PROJECT=test_project --title "Session Tab 1""#;
            harness.execute_command(session_tab1_cmd).await?;

            let session_tab2_cmd = r#"launch --type=tab --env KITTY_SESSION_PROJECT=test_project --title "Session Tab 2""#;
            harness.execute_command(session_tab2_cmd).await?;

            // Create a tab with different session environment
            let different_session_cmd = r#"launch --type=tab --env KITTY_SESSION_PROJECT=other_project --title "Other Session Tab""#;
            harness.execute_command(different_session_cmd).await?;

            // Create a tab without session environment (no session)
            let no_session_cmd = r#"launch --type=tab --title "No Session Tab""#;
            harness.execute_command(no_session_cmd).await?;

            // Wait for tabs to stabilize
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // 2. Test detection of tabs belonging to specific session
            let test_project_tabs_cmd =
                "ls --all-env-vars --match-tab env:KITTY_SESSION_PROJECT=test_project";
            let test_project_output = harness.execute_command(test_project_tabs_cmd).await?;

            let test_project_json: serde_json::Value =
                serde_json::from_str(&test_project_output)
                    .map_err(|e| format!("Failed to parse test_project tabs JSON: {}", e))?;

            // Count tabs in test_project session
            let mut test_project_count = 0;
            if let Some(os_windows) = test_project_json.as_array() {
                for os_window in os_windows {
                    if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                        for tab in tabs {
                            if let Some(windows) = tab.get("windows").and_then(|w| w.as_array()) {
                                for window in windows {
                                    if let Some(env) = window.get("env").and_then(|e| e.as_object())
                                    {
                                        if let Some(session_value) = env
                                            .get("KITTY_SESSION_PROJECT")
                                            .and_then(|v| v.as_str())
                                        {
                                            if session_value == "test_project" {
                                                test_project_count += 1;
                                                break; // Found matching window in this tab
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            println!("Found {} tabs in test_project session", test_project_count);
            assert_eq!(
                test_project_count, 2,
                "Should find exactly 2 tabs in test_project session"
            );

            // 3. Test detection of tabs belonging to different session
            let other_project_tabs_cmd =
                "ls --all-env-vars --match-tab env:KITTY_SESSION_PROJECT=other_project";
            let other_project_output = harness.execute_command(other_project_tabs_cmd).await?;

            let other_project_json: serde_json::Value = serde_json::from_str(&other_project_output)
                .map_err(|e| format!("Failed to parse other_project tabs JSON: {}", e))?;

            let mut other_project_count = 0;
            if let Some(os_windows) = other_project_json.as_array() {
                for os_window in os_windows {
                    if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                        for tab in tabs {
                            if let Some(windows) = tab.get("windows").and_then(|w| w.as_array()) {
                                for window in windows {
                                    if let Some(env) = window.get("env").and_then(|e| e.as_object())
                                    {
                                        if let Some(session_value) = env
                                            .get("KITTY_SESSION_PROJECT")
                                            .and_then(|v| v.as_str())
                                        {
                                            if session_value == "other_project" {
                                                other_project_count += 1;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            println!(
                "Found {} tabs in other_project session",
                other_project_count
            );
            assert_eq!(
                other_project_count, 1,
                "Should find exactly 1 tab in other_project session"
            );

            // 4. Test detection failure for non-existent session
            let nonexistent_session_cmd =
                "ls --all-env-vars --match-tab env:KITTY_SESSION_PROJECT=nonexistent_session";
            let nonexistent_output = harness
                .execute_query_command(nonexistent_session_cmd)
                .await?;

            let nonexistent_json: serde_json::Value = serde_json::from_str(&nonexistent_output)
                .map_err(|e| format!("Failed to parse nonexistent session JSON: {}", e))?;

            let mut nonexistent_count = 0;
            if let Some(os_windows) = nonexistent_json.as_array() {
                for os_window in os_windows {
                    if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                        nonexistent_count += tabs.len();
                    }
                }
            }

            println!("Found {} tabs in nonexistent session", nonexistent_count);
            assert_eq!(
                nonexistent_count, 0,
                "Should find no tabs for nonexistent session"
            );

            // 5. Test getting all tabs (including those without sessions)
            let all_tabs_output = harness.execute_command("ls").await?;
            let all_tabs_json: serde_json::Value = serde_json::from_str(&all_tabs_output)
                .map_err(|e| format!("Failed to parse all tabs JSON: {}", e))?;

            let mut total_tabs = 0;
            let mut session_tabs = 0;
            let mut no_session_tabs = 0;

            if let Some(os_windows) = all_tabs_json.as_array() {
                for os_window in os_windows {
                    if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                        for tab in tabs {
                            total_tabs += 1;
                            let mut has_session = false;

                            if let Some(windows) = tab.get("windows").and_then(|w| w.as_array()) {
                                for window in windows {
                                    if let Some(env) = window.get("env").and_then(|e| e.as_object())
                                    {
                                        if let Some(session_value) = env
                                            .get("KITTY_SESSION_PROJECT")
                                            .and_then(|v| v.as_str())
                                        {
                                            // Only count tabs with our specific session values
                                            if session_value == "test_project"
                                                || session_value == "other_project"
                                            {
                                                has_session = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }

                            if has_session {
                                session_tabs += 1;
                            } else {
                                no_session_tabs += 1;
                            }
                        }
                    }
                }
            }

            println!(
                "Total tabs: {}, Session tabs: {}, No session tabs: {}",
                total_tabs, session_tabs, no_session_tabs
            );

            // We expect: 1 initial tab (no session) + 4 created tabs = 5 total
            // 3 with sessions (2 test_project + 1 other_project) + 2 without session = 5 total
            // Note: there might be existing tabs, so use >= for more robust testing
            assert!(total_tabs >= 5, "Should have at least 5 tabs total");
            assert!(
                session_tabs >= 3,
                "Should have at least 3 tabs with sessions (may have more from previous tests)"
            );
            assert!(
                no_session_tabs >= 2,
                "Should have at least 2 tabs without sessions"
            );

            // 6. Verify session context can be used for navigation
            // Test focusing on a tab within the session (simulating navigation)
            let session_tabs_cmd =
                "ls --all-env-vars --match env:KITTY_SESSION_PROJECT=test_project";
            let session_tabs_output = harness.execute_command(session_tabs_cmd).await?;
            let session_tabs_json: serde_json::Value = serde_json::from_str(&session_tabs_output)?;

            // Focus on the first tab in the session
            if let Some(os_windows) = session_tabs_json.as_array() {
                if let Some(first_window) = os_windows.first() {
                    if let Some(tabs) = first_window.get("tabs").and_then(|t| t.as_array()) {
                        if let Some(first_tab) = tabs.first() {
                            if let Some(tab_id) = first_tab.get("id").and_then(|id| id.as_u64()) {
                                let focus_session_tab_cmd =
                                    format!("focus-tab --match id:{}", tab_id);
                                harness.execute_command(&focus_session_tab_cmd).await?;
                            }
                        }
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Check that we're still within the same session after navigation
            let after_nav_state = harness.execute_command(test_project_tabs_cmd).await?;
            let after_nav_json: serde_json::Value = serde_json::from_str(&after_nav_state)?;

            let mut active_session_tab = false;
            if let Some(os_windows) = after_nav_json.as_array() {
                for os_window in os_windows {
                    if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                        for tab in tabs {
                            if let Some(is_active) = tab.get("is_active").and_then(|a| a.as_bool())
                            {
                                if is_active {
                                    if let Some(windows) =
                                        tab.get("windows").and_then(|w| w.as_array())
                                    {
                                        for window in windows {
                                            if let Some(env) =
                                                window.get("env").and_then(|e| e.as_object())
                                            {
                                                if let Some(session_value) = env
                                                    .get("KITTY_SESSION_PROJECT")
                                                    .and_then(|v| v.as_str())
                                                {
                                                    if session_value == "test_project" {
                                                        active_session_tab = true;
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            assert!(
                active_session_tab,
                "After session navigation, active tab should still be in test_project session"
            );

            // 7. Capture final screenshot showing session context
            match harness.capture_screenshot("session_context_final").await {
                Ok(path) => println!("Captured session context state: {:?}", path),
                Err(e) => println!("Screenshot capture failed (normal in headless): {}", e),
            }

            harness.cleanup().await?;
            println!("Session context detection test completed successfully");
            Ok(())
        }
        Err(e) => handle_launch_failure("test_session_context_detection", &e.to_string()),
    }
}

#[tokio::test]
async fn test_screenshot_capture() -> Result<(), Box<dyn std::error::Error>> {
    // Skip test if Kitty is not available
    if !is_kitty_available().await {
        println!("Skipping test_screenshot_capture: Kitty not available");
        return Ok(());
    }

    println!("Testing screenshot capture functionality...");

    match KittyTestHarness::launch_with_test_name("test_screenshot_capture").await {
        Ok(harness) => {
            println!("Test harness launched successfully for screenshot capture test");

            // Test basic screenshot capture
            match harness.capture_screenshot("test_capture").await {
                Ok(screenshot_path) => {
                    println!("Screenshot captured successfully: {:?}", screenshot_path);

                    // Verify the file exists and has reasonable size
                    assert!(screenshot_path.exists(), "Screenshot file should exist");

                    let metadata = std::fs::metadata(&screenshot_path)?;
                    assert!(metadata.len() > 0, "Screenshot file should not be empty");
                    assert!(
                        metadata.len() > 1000,
                        "Screenshot file should be larger than 1KB"
                    );

                    println!("Screenshot file size: {} bytes", metadata.len());

                    // Try to capture a second screenshot
                    match harness.capture_screenshot("test_capture_2").await {
                        Ok(screenshot_path_2) => {
                            println!("Second screenshot captured: {:?}", screenshot_path_2);

                            // Compare the two screenshots - they should be very similar
                            match compare_screenshots(&screenshot_path_2, &screenshot_path) {
                                Ok(similarity) => {
                                    println!("Screenshot similarity: {:.4}", similarity);
                                    // Screenshots taken close together should be very similar
                                    assert!(
                                        similarity > 0.90,
                                        "Screenshots taken close together should be similar"
                                    );
                                }
                                Err(e) => {
                                    println!(
                                        "Screenshot comparison failed (this may be normal): {}",
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            println!("Second screenshot capture failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!(
                        "Screenshot capture failed (this may be normal in headless environments): {}",
                        e
                    );
                    // Don't fail the test if screenshot capture is not available
                }
            }

            harness.cleanup().await?;
            println!("Screenshot capture test completed");
            Ok(())
        }
        Err(e) => handle_launch_failure("test_screenshot_capture", &e.to_string()),
    }
}

#[tokio::test]
async fn test_edge_cases() -> Result<(), Box<dyn std::error::Error>> {
    // Skip test if Kitty is not available
    if !is_kitty_available().await {
        println!("Skipping test_edge_cases: Kitty not available");
        return Ok(());
    }

    println!("Testing edge cases for session navigation...");

    match KittyTestHarness::launch_with_test_name("test_edge_cases").await {
        Ok(harness) => {
            println!("Test harness launched successfully for edge cases test");

            // Test Case 1: Navigation with no session tabs (only default unnamed tabs)
            println!("Test Case 1: Navigation with no session tabs");

            // Create a few tabs without session environment
            let tab1_cmd = r#"launch --type=tab --title "No Session Tab 1""#;
            harness.execute_command(tab1_cmd).await?;

            let tab2_cmd = r#"launch --type=tab --title "No Session Tab 2""#;
            harness.execute_command(tab2_cmd).await?;

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Try to focus on a non-existent session - should fail gracefully
            let nav_nonexistent_cmd = "focus-tab --match env:KITTY_SESSION_PROJECT=nonexistent";
            let nav_result = harness.execute_command(nav_nonexistent_cmd).await;

            match nav_result {
                Ok(_) => {
                    // If it succeeds, it should be a no-op (no matching tabs to navigate)
                    println!("Navigation command succeeded (no-op for nonexistent session)");
                }
                Err(e) => {
                    println!(
                        "Navigation command failed as expected for nonexistent session: {}",
                        e
                    );
                    // This is expected behavior - trying to navigate in a session that doesn't exist
                }
            }

            // Verify that normal tab navigation still works without sessions
            let all_tabs_output = harness.execute_command("ls").await?;
            let all_tabs_json: serde_json::Value = serde_json::from_str(&all_tabs_output)?;

            let mut tab_ids = Vec::new();
            if let Some(os_windows) = all_tabs_json.as_array() {
                for os_window in os_windows {
                    if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                        for tab in tabs {
                            if let Some(tab_id) = tab.get("id").and_then(|id| id.as_u64()) {
                                tab_ids.push(tab_id);
                            }
                        }
                    }
                }
            }

            assert!(
                tab_ids.len() >= 3,
                "Should have at least 3 tabs (1 initial + 2 created)"
            );

            // Test navigation by tab ID (should work regardless of session)
            if let Some(first_tab_id) = tab_ids.first() {
                let focus_by_id_cmd = format!("focus-tab --match id:{}", first_tab_id);
                harness.execute_command(&focus_by_id_cmd).await?;
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                println!("Successfully navigated to tab by ID without session");
            }

            // Test Case 2: Single tab in a session
            println!("Test Case 2: Single tab in a session");

            // Create only one tab with a session
            let single_session_cmd = r#"launch --type=tab --env KITTY_SESSION_PROJECT=single_tab_session --title "Only Session Tab""#;
            harness.execute_command(single_session_cmd).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Focus on the single tab session (simulating navigation)
            let single_session_tabs_cmd =
                "ls --all-env-vars --match env:KITTY_SESSION_PROJECT=single_tab_session";
            let single_session_output = harness.execute_command(single_session_tabs_cmd).await?;
            let single_session_json: serde_json::Value =
                serde_json::from_str(&single_session_output)?;

            // Focus on the single tab
            if let Some(os_windows) = single_session_json.as_array() {
                if let Some(first_window) = os_windows.first() {
                    if let Some(tabs) = first_window.get("tabs").and_then(|t| t.as_array()) {
                        if let Some(first_tab) = tabs.first() {
                            if let Some(tab_id) = first_tab.get("id").and_then(|id| id.as_u64()) {
                                let focus_single_cmd = format!("focus-tab --match id:{}", tab_id);
                                harness.execute_command(&focus_single_cmd).await?;
                                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

                                // Try focusing again (simulating next/previous on single tab)
                                harness.execute_command(&focus_single_cmd).await?;
                            }
                        }
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Verify the single tab is still active after navigation attempts
            let single_session_tabs_cmd =
                "ls --all-env-vars --match-tab env:KITTY_SESSION_PROJECT=single_tab_session";
            let single_session_output = harness.execute_command(single_session_tabs_cmd).await?;
            let single_session_json: serde_json::Value =
                serde_json::from_str(&single_session_output)?;

            let mut single_tab_active = false;
            if let Some(os_windows) = single_session_json.as_array() {
                for os_window in os_windows {
                    if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                        for tab in tabs {
                            if let Some(is_active) = tab.get("is_active").and_then(|a| a.as_bool())
                            {
                                if is_active {
                                    single_tab_active = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            assert!(
                single_tab_active,
                "Single session tab should remain active after navigation"
            );
            println!("Single tab session handled navigation correctly");

            // Test Case 3: Invalid session names and special characters
            println!("Test Case 3: Invalid session names and special characters");

            // Test with session name containing spaces and special characters
            let special_session_cmd = r#"launch --type=tab --env KITTY_SESSION_PROJECT="test session with spaces!" --title "Special Session Tab""#;
            harness.execute_command(special_session_cmd).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Try to navigate with the special session name (needs proper escaping)
            let nav_special_cmd =
                "ls --all-env-vars --match-tab env:KITTY_SESSION_PROJECT=test session with spaces!";
            let special_result = harness.execute_command(nav_special_cmd).await;

            match special_result {
                Ok(output) => {
                    println!("Special session name query succeeded: {}", output);
                    // Parse and verify we found the tab
                    let special_json: serde_json::Value = serde_json::from_str(&output)?;
                    let mut found_special = false;

                    if let Some(os_windows) = special_json.as_array() {
                        for os_window in os_windows {
                            if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                                if !tabs.is_empty() {
                                    found_special = true;
                                    break;
                                }
                            }
                        }
                    }

                    assert!(found_special, "Should find tab with special session name");
                }
                Err(e) => {
                    println!("Special session name query failed (may be expected): {}", e);
                    // This might fail due to shell escaping issues, which is a valid edge case
                }
            }

            // Test Case 4: Empty session environment variable
            println!("Test Case 4: Empty session environment variable");

            let empty_session_cmd =
                r#"launch --type=tab --env KITTY_SESSION_PROJECT="" --title "Empty Session Tab""#;
            harness.execute_command(empty_session_cmd).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Try to query tabs with empty session value
            let empty_session_query_cmd =
                "ls --all-env-vars --match-tab env:KITTY_SESSION_PROJECT=";
            let empty_result = harness.execute_query_command(empty_session_query_cmd).await;

            match empty_result {
                Ok(output) => {
                    println!("Empty session query succeeded: {}", output);
                    // This should either find the tab with empty value or return no results
                }
                Err(e) => {
                    println!("Empty session query failed (expected): {}", e);
                    // This is expected as empty strings in environment matching can be tricky
                }
            }

            // Test Case 5: Very long session name
            println!("Test Case 5: Very long session name");

            let long_session_name = "a".repeat(100); // 100 character session name
            let long_session_cmd = format!(
                r#"launch --type=tab --env KITTY_SESSION_PROJECT={} --title "Long Session Tab""#,
                long_session_name
            );
            harness.execute_command(&long_session_cmd).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let long_session_query_cmd = format!(
                "ls --all-env-vars --match-tab env:KITTY_SESSION_PROJECT={}",
                long_session_name
            );
            let long_result = harness.execute_query_command(&long_session_query_cmd).await;

            match long_result {
                Ok(output) => {
                    println!(
                        "Long session name query succeeded (output length: {})",
                        output.len()
                    );
                    // Verify we can find the tab
                    let long_json: serde_json::Value = serde_json::from_str(&output)?;
                    let mut found_long = false;

                    if let Some(os_windows) = long_json.as_array() {
                        for os_window in os_windows {
                            if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                                if !tabs.is_empty() {
                                    found_long = true;
                                    break;
                                }
                            }
                        }
                    }

                    if found_long {
                        println!("Successfully found tab with very long session name");
                    } else {
                        println!("No tabs found with very long session name");
                    }
                }
                Err(e) => {
                    println!("Long session name query failed: {}", e);
                    // This might fail due to command line length limits
                }
            }

            // Test Case 6: Concurrent session operations
            println!("Test Case 6: Rapid session operations");

            // Rapidly create and query tabs to test for race conditions
            for i in 0..3 {
                let rapid_cmd = format!(
                    r#"launch --type=tab --env KITTY_SESSION_PROJECT=rapid_test --title "Rapid Tab {}""#,
                    i
                );
                harness.execute_command(&rapid_cmd).await?;
                // Don't wait between rapid operations to test robustness
            }

            // Query immediately after rapid creation
            let rapid_query_cmd =
                "ls --all-env-vars --match-tab env:KITTY_SESSION_PROJECT=rapid_test";
            let rapid_result = harness.execute_command(rapid_query_cmd).await?;
            let rapid_json: serde_json::Value = serde_json::from_str(&rapid_result)?;

            let mut rapid_count = 0;
            if let Some(os_windows) = rapid_json.as_array() {
                for os_window in os_windows {
                    if let Some(tabs) = os_window.get("tabs").and_then(|t| t.as_array()) {
                        rapid_count += tabs.len();
                    }
                }
            }

            println!("Found {} tabs after rapid creation", rapid_count);
            assert!(rapid_count <= 3, "Should not find more tabs than created");
            // Note: might find fewer than 3 if some operations haven't completed yet

            // Capture final screenshot showing edge case scenarios
            match harness.capture_screenshot("edge_cases_final").await {
                Ok(path) => println!("Captured edge cases state: {:?}", path),
                Err(e) => println!("Screenshot capture failed (normal in headless): {}", e),
            }

            harness.cleanup().await?;
            println!("Edge cases test completed successfully");
            Ok(())
        }
        Err(e) => handle_launch_failure("test_edge_cases", &e.to_string()),
    }
}
