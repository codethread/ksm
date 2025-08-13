use ksm::config::{Config, get_all_directories_from_path};
use std::fs;
use std::path::PathBuf;

#[test]
fn test_config_keyed_projects_personal() {
    // Create a test config file
    let temp_dir = std::env::temp_dir().join("ksm_test_config_personal");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let config_content = r#"{
        "dirs": [],
        "base": [["P0", "~/base"]],
        "personal": [["P1", "~/personal"]],
        "work": [["P2", "~/work"]]
    }"#;

    let config_file = temp_dir.join("test_config.json");
    fs::write(&config_file, config_content).unwrap();

    let config = Config::load_from_path(Some(config_file)).unwrap();
    let projects = config.keyed_projects(false);

    assert_eq!(projects.len(), 2);
    assert!(projects.contains(&("P0".to_string(), "~/base".to_string())));
    assert!(projects.contains(&("P1".to_string(), "~/personal".to_string())));
    assert!(!projects.contains(&("P2".to_string(), "~/work".to_string())));

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_config_keyed_projects_work() {
    // Create a test config file
    let temp_dir = std::env::temp_dir().join("ksm_test_config_work");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let config_content = r#"{
        "dirs": [],
        "base": [["P0", "~/base"]],
        "personal": [["P1", "~/personal"]],
        "work": [["P2", "~/work"]]
    }"#;

    let config_file = temp_dir.join("test_config.json");
    fs::write(&config_file, config_content).unwrap();

    let config = Config::load_from_path(Some(config_file)).unwrap();
    let projects = config.keyed_projects(true);

    assert_eq!(projects.len(), 2);
    assert!(projects.contains(&("P0".to_string(), "~/base".to_string())));
    assert!(projects.contains(&("P2".to_string(), "~/work".to_string())));
    assert!(!projects.contains(&("P1".to_string(), "~/personal".to_string())));

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_config_expanded_directories() {
    // Create a temporary directory structure for testing
    let temp_dir = std::env::temp_dir().join("ksm_test_config_expanded");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    // Create test directories
    fs::create_dir_all(temp_dir.join("project1")).unwrap();
    fs::create_dir_all(temp_dir.join("project2")).unwrap();

    let config_content = format!(
        r#"{{
            "dirs": ["{}/project*"],
            "base": [],
            "personal": [],
            "work": []
        }}"#,
        temp_dir.display()
    );

    let config_file = temp_dir.join("test_config.json");
    fs::write(&config_file, config_content).unwrap();

    let config = Config::load_from_path(Some(config_file)).unwrap();
    let directories = config.expanded_directories().unwrap();

    assert_eq!(directories.len(), 2);
    let dir_names: Vec<String> = directories
        .iter()
        .map(|p| {
            PathBuf::from(p)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect();

    assert!(dir_names.contains(&"project1".to_string()));
    assert!(dir_names.contains(&"project2".to_string()));

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_dirs_mixed_glob_and_regular_patterns() {
    // Create a temporary directory structure for testing
    let temp_dir = std::env::temp_dir().join("ksm_test_mixed");
    let _ = fs::remove_dir_all(&temp_dir); // Clean up if exists
    fs::create_dir_all(&temp_dir).unwrap();

    // Create test directories for glob patterns
    let test_dirs = ["glob_project1", "glob_project2", "subdir"];
    for dir in &test_dirs {
        fs::create_dir_all(temp_dir.join(dir)).unwrap();
    }

    let subdir_path = temp_dir.join("subdir");
    fs::create_dir_all(subdir_path.join("nested1")).unwrap();
    fs::create_dir_all(subdir_path.join("nested2")).unwrap();

    // Create test directories for regular paths
    fs::create_dir_all(temp_dir.join("regular_project1")).unwrap();
    fs::create_dir_all(temp_dir.join("regular_project2")).unwrap();

    // Create a test config file with mixed glob and regular paths
    let config_content = format!(
        r#"{{
            "dirs": [
                "{}/glob_*",
                "{}/regular_project1",
                "{}/regular_project2",
                "{}/subdir/*"
            ],
            "base": [],
            "personal": [],
            "work": []
        }}"#,
        temp_dir.display(),
        temp_dir.display(),
        temp_dir.display(),
        temp_dir.display()
    );

    let config_file = temp_dir.join("test_config.json");
    fs::write(&config_file, config_content).unwrap();

    // Test mixed pattern expansion
    let result = get_all_directories_from_path(Some(config_file)).unwrap();

    // Should find: glob_project1, glob_project2, regular_project1, regular_project2, nested1, nested2
    assert_eq!(result.len(), 6);

    // Convert to strings for easier comparison
    let result_strings: Vec<String> = result
        .iter()
        .map(|p| {
            PathBuf::from(p)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect();

    // Check glob patterns matched
    assert!(result_strings.contains(&"glob_project1".to_string()));
    assert!(result_strings.contains(&"glob_project2".to_string()));
    assert!(result_strings.contains(&"nested1".to_string()));
    assert!(result_strings.contains(&"nested2".to_string()));

    // Check regular paths matched
    assert!(result_strings.contains(&"regular_project1".to_string()));
    assert!(result_strings.contains(&"regular_project2".to_string()));

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}
