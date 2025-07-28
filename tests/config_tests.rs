use ksm::config::{get_keyed_projects_from_config, get_all_directories_from_path, SessionConfig};
use std::fs;
use std::path::PathBuf;

#[test]
fn test_get_keyed_projects_personal() {
    let config = SessionConfig {
        dirs: vec![],
        dirs_special: None,
        base: vec![("P0".to_string(), "~/base".to_string())],
        personal: vec![("P1".to_string(), "~/personal".to_string())],
        work: vec![("P2".to_string(), "~/work".to_string())],
    };

    let projects = get_keyed_projects_from_config(&config, false);
    assert_eq!(projects.len(), 2);
    assert!(projects.contains(&("P0".to_string(), "~/base".to_string())));
    assert!(projects.contains(&("P1".to_string(), "~/personal".to_string())));
    assert!(!projects.contains(&("P2".to_string(), "~/work".to_string())));
}

#[test]
fn test_get_keyed_projects_work() {
    let config = SessionConfig {
        dirs: vec![],
        dirs_special: None,
        base: vec![("P0".to_string(), "~/base".to_string())],
        personal: vec![("P1".to_string(), "~/personal".to_string())],
        work: vec![("P2".to_string(), "~/work".to_string())],
    };

    let projects = get_keyed_projects_from_config(&config, true);
    assert_eq!(projects.len(), 2);
    assert!(projects.contains(&("P0".to_string(), "~/base".to_string())));
    assert!(projects.contains(&("P2".to_string(), "~/work".to_string())));
    assert!(!projects.contains(&("P1".to_string(), "~/personal".to_string())));
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
    let result = get_all_directories_from_path(Some(config_file), false).unwrap();
    
    // Should find: glob_project1, glob_project2, regular_project1, regular_project2, nested1, nested2
    assert_eq!(result.len(), 6);
    
    // Convert to strings for easier comparison
    let result_strings: Vec<String> = result.iter()
        .map(|p| PathBuf::from(p).file_name().unwrap().to_string_lossy().to_string())
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
