use assert_fs::TempDir;
use assert_fs::prelude::*;
use ksm::config::{Config, get_all_directories_from_path};
use std::path::PathBuf;

#[test]
fn test_config_keyed_projects_personal() {
    let temp = TempDir::new().unwrap();

    temp.child("test_config.json")
        .write_str(
            r#"{
            "search": {
                "dirs": [],
                "vsc": [],
                "cmd": []
            },
            "projects": {
                "*": {
                    "P0": "~/base"
                },
                "personal": {
                    "P1": "~/personal"
                },
                "work": {
                    "P2": "~/work"
                }
            }
        }"#,
        )
        .unwrap();

    let config = Config::load_from_path(Some(temp.path().join("test_config.json"))).unwrap();
    let projects = config.keyed_projects(false);

    assert_eq!(projects.len(), 2);
    assert!(projects.contains(&("P0".to_string(), "~/base".to_string())));
    assert!(projects.contains(&("P1".to_string(), "~/personal".to_string())));
    assert!(!projects.contains(&("P2".to_string(), "~/work".to_string())));
}

#[test]
fn test_config_keyed_projects_work() {
    let temp = TempDir::new().unwrap();

    temp.child("test_config.json")
        .write_str(
            r#"{
            "search": {
                "dirs": [],
                "vsc": [],
                "cmd": []
            },
            "projects": {
                "*": {
                    "P0": "~/base"
                },
                "personal": {
                    "P1": "~/personal"
                },
                "work": {
                    "P2": "~/work"
                }
            }
        }"#,
        )
        .unwrap();

    let config = Config::load_from_path(Some(temp.path().join("test_config.json"))).unwrap();
    let projects = config.keyed_projects(true);

    assert_eq!(projects.len(), 2);
    assert!(projects.contains(&("P0".to_string(), "~/base".to_string())));
    assert!(projects.contains(&("P2".to_string(), "~/work".to_string())));
    assert!(!projects.contains(&("P1".to_string(), "~/personal".to_string())));
}

#[test]
fn test_config_expanded_directories() {
    let temp = TempDir::new().unwrap();

    // Create test directories
    temp.child("project1").create_dir_all().unwrap();
    temp.child("project2/subdir").create_dir_all().unwrap();
    temp.child("non-project").create_dir_all().unwrap();

    temp.child("test_config.json")
        .write_str(&format!(
            r#"{{
                "search": {{
                    "dirs": ["{}/project*"],
                    "vsc": [],
                    "cmd": []
                }},
                "projects": {{
                    "*": {{}},
                    "personal": {{}},
                    "work": {{}}
                }}
            }}"#,
            temp.path().display()
        ))
        .unwrap();

    let config = Config::load_from_path(Some(temp.path().join("test_config.json"))).unwrap();
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
}

#[test]
fn test_dirs_mixed_glob_and_regular_patterns() {
    let temp = TempDir::new().unwrap();

    // Create directory structure more readably
    for dir in &[
        "glob_project1",
        "glob_project2",
        "regular_project1",
        "regular_project2",
    ] {
        temp.child(dir).create_dir_all().unwrap();
    }

    temp.child("subdir/nested1").create_dir_all().unwrap();
    temp.child("subdir/nested2").create_dir_all().unwrap();

    let config_content = format!(
        r#"{{
            "search": {{
                "dirs": [
                    "{}/glob_*",
                    "{}/regular_project1",
                    "{}/regular_project2",
                    "{}/subdir/*"
                ],
                "vsc": [],
                "cmd": []
            }},
            "projects": {{
                "*": {{}},
                "personal": {{}},
                "work": {{}}
            }}
        }}"#,
        temp.path().display(),
        temp.path().display(),
        temp.path().display(),
        temp.path().display()
    );

    temp.child("test_config.json")
        .write_str(&config_content)
        .unwrap();

    let result = get_all_directories_from_path(Some(temp.path().join("test_config.json"))).unwrap();

    // Should find: glob_project1, glob_project2, regular_project1 (literal), regular_project2 (literal), nested1, nested2
    assert_eq!(result.len(), 6);

    // Check that regular paths are returned as literal paths (full paths)
    let regular_project1_path = format!("{}/regular_project1", temp.path().display());
    let regular_project2_path = format!("{}/regular_project2", temp.path().display());
    assert!(result.contains(&regular_project1_path));
    assert!(result.contains(&regular_project2_path));

    // Check that glob patterns still expand to actual directories
    let glob_matches: Vec<&String> = result
        .iter()
        .filter(|p| p.contains("glob_project"))
        .collect();
    assert_eq!(glob_matches.len(), 2);

    let nested_matches: Vec<&String> = result.iter().filter(|p| p.contains("nested")).collect();
    assert_eq!(nested_matches.len(), 2);
}

#[test]
fn test_literal_paths_behavior() {
    let temp = TempDir::new().unwrap();

    // Create one existing directory
    temp.child("existing_dir").create_dir_all().unwrap();

    let config_content = format!(
        r#"{{
            "search": {{
                "dirs": [
                    "{}/existing_dir",
                    "{}/nonexistent_dir",
                    "~/dev"
                ],
                "vsc": [],
                "cmd": []
            }},
            "projects": {{
                "*": {{}},
                "personal": {{}},
                "work": {{}}
            }}
        }}"#,
        temp.path().display(),
        temp.path().display()
    );

    temp.child("test_config.json")
        .write_str(&config_content)
        .unwrap();

    let result = get_all_directories_from_path(Some(temp.path().join("test_config.json"))).unwrap();

    // Should contain all 3 paths as literals, regardless of existence
    assert_eq!(result.len(), 3);

    let existing_path = format!("{}/existing_dir", temp.path().display());
    let nonexistent_path = format!("{}/nonexistent_dir", temp.path().display());

    assert!(result.contains(&existing_path));
    assert!(result.contains(&nonexistent_path));

    // Should expand tilde
    let home_expanded = result.iter().find(|p| p.contains("/dev")).unwrap();
    assert!(
        home_expanded.starts_with('/')
            || home_expanded.starts_with(std::env::var("HOME").unwrap_or_default().as_str())
    );
}
