use anyhow::Result;
use kitty_lib::{
    KittyCommandResult, KittyLaunchResponse, KittyOsWindow, KittyTab, KittyWindow, MockExecutor,
};
use ksm::kitty::Kitty;
use std::collections::HashMap;

#[test]
fn test_kitty_with_mock_executor() -> Result<()> {
    let mock_executor = MockExecutor::with_default_socket();

    // Setup mock response for ls command
    let mock_tab = KittyTab {
        id: 42,
        index: Some(0),
        title: "Test Tab".to_string(),
        windows: vec![KittyWindow {
            id: 1,
            title: "Test Window".to_string(),
            pid: 12345,
            cwd: "/tmp/test".to_string(),
            cmdline: vec!["zsh".to_string()],
            env: HashMap::new(),
            is_self: true,
            state: Some("active".to_string()),
            num: Some(0),
            recent: Some(0),
        }],
        state: Some("active".to_string()),
        recent: Some(0),
    };
    let mock_os_window = KittyOsWindow {
        id: 1,
        tabs: vec![mock_tab],
        title: Some("Test OS Window".to_string()),
        state: Some("active".to_string()),
    };
    mock_executor.expect_ls_response(Ok(vec![mock_os_window]));

    // Setup mock response for focus command
    mock_executor.expect_focus_tab_response(Ok(KittyCommandResult::success_empty()));

    let kitty = Kitty::with_executor(&mock_executor);

    // Test match_session_tab
    let result = kitty.match_session_tab("test-project")?;
    assert!(result.is_some());
    assert_eq!(result.unwrap().id, 42);

    // Test focus_tab
    kitty.focus_tab(42)?;

    // Verify calls were made
    assert_eq!(mock_executor.ls_call_count(), 1);
    assert_eq!(mock_executor.focus_tab_call_count(), 1);

    // Verify call details
    let ls_calls = mock_executor.get_ls_calls();
    assert_eq!(ls_calls.len(), 1);

    let focus_calls = mock_executor.get_focus_tab_calls();
    assert_eq!(focus_calls.len(), 1);
    assert_eq!(focus_calls[0].tab_id, 42);

    Ok(())
}

#[test]
fn test_kitty_mock_no_matching_tabs() -> Result<()> {
    let mock_executor = MockExecutor::with_default_socket();

    // Setup mock response for ls command with no matching tabs
    mock_executor.expect_ls_response(Ok(Vec::new()));

    let kitty = Kitty::with_executor(&mock_executor);

    // Test match_session_tab with no matches
    let result = kitty.match_session_tab("nonexistent-project")?;
    assert!(result.is_none());

    // Verify call was made
    assert_eq!(mock_executor.ls_call_count(), 1);

    Ok(())
}

#[test]
fn test_kitty_mock_create_session() -> Result<()> {
    let mock_executor = MockExecutor::with_default_socket();

    // Setup mock response for launch command
    mock_executor.expect_launch_response(Ok(KittyCommandResult::success(KittyLaunchResponse {
        tab_id: None,
        window_id: None,
    })));

    let kitty = Kitty::with_executor(&mock_executor);

    // Test create_session_tab_by_path
    kitty.create_session_tab_by_path("/tmp/test-project", "test-project")?;

    // Verify call was made
    assert_eq!(mock_executor.launch_call_count(), 1);

    // Verify call details
    let launch_calls = mock_executor.get_launch_calls();
    assert_eq!(launch_calls.len(), 1);
    assert_eq!(launch_calls[0].cwd, Some("/tmp/test-project".to_string()));
    assert_eq!(
        launch_calls[0].env,
        Some("KITTY_SESSION_PROJECT=test-project".to_string())
    );
    assert_eq!(
        launch_calls[0].tab_title,
        Some("📁 test-project".to_string())
    );

    Ok(())
}
