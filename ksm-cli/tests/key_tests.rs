use ksm::app::App;
use ksm::cmd::key::{cmd_key_with_projects, resolve_project_path};
use ksm::config::Config;
use std::fs;

fn create_test_config() -> Config {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Create a unique temporary directory for each test to avoid conflicts
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("ksm_test_key_tests_{}", timestamp));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let config_content = r#"{
        "search": {
            "dirs": [],
            "vsc": [],
            "cmd": []
        },
        "projects": {
            "*": {},
            "personal": {},
            "work": {}
        }
    }"#;

    let config_file = temp_dir.join("test_config.json");
    fs::write(&config_file, config_content).unwrap();

    Config::load_from_path(Some(config_file)).unwrap()
}

#[test]
fn test_resolve_project_path_found() {
    let projects = vec![
        ("P1".to_string(), "~/test/project1".to_string()),
        ("P2".to_string(), "/absolute/path/project2".to_string()),
    ];

    let result = resolve_project_path("P1", &projects).unwrap();
    // Should expand tilde - exact path depends on system, but should contain "test/project1"
    assert!(result.contains("test/project1"));
}

#[test]
fn test_resolve_project_path_not_found() {
    let projects = vec![("P1".to_string(), "~/test/project1".to_string())];

    let result = resolve_project_path("P99", &projects);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No project found for key: P99")
    );
}

#[test]
fn test_resolve_project_path_empty_projects() {
    let projects = vec![];

    let result = resolve_project_path("P1", &projects);
    assert!(result.is_err());
}

#[test]
fn test_resolve_project_path_absolute_path() {
    let projects = vec![("P2".to_string(), "/absolute/path/project2".to_string())];

    let result = resolve_project_path("P2", &projects).unwrap();
    assert_eq!(result, "/absolute/path/project2");
}

// For testing cmd_key_with_projects with print_path=true, we need to capture stdout
// This is more complex since we're using println! directly
// We'll test the logic by checking the early return behavior
#[test]
fn test_cmd_key_with_print_path_returns_early() {
    let projects = vec![("P1".to_string(), "/test/path".to_string())];

    // When print_path is true, the function should return Ok(()) without calling kitty functions
    // Since we can't easily mock kitty functions, we test that it doesn't panic or error
    let config = create_test_config();
    let app = App::new(config);
    let result = cmd_key_with_projects(&app, "P1", true, &projects);
    assert!(result.is_ok());
}

#[test]
fn test_cmd_key_with_print_path_invalid_key() {
    let projects = vec![("P1".to_string(), "/test/path".to_string())];

    // Should return error for invalid key even with print_path=true
    let config = create_test_config();
    let app = App::new(config);
    let result = cmd_key_with_projects(&app, "P99", true, &projects);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No project found for key: P99")
    );
}

#[test]
fn test_cmd_key_with_projects_multiple_keys() {
    let projects = vec![
        ("config".to_string(), "~/.config".to_string()),
        ("dev".to_string(), "~/dev".to_string()),
        ("work".to_string(), "/work/project".to_string()),
    ];

    // Test finding the right key
    let result = resolve_project_path("work", &projects).unwrap();
    assert_eq!(result, "/work/project");

    let result = resolve_project_path("dev", &projects).unwrap();
    assert!(result.contains("dev"));
}
