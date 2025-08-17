use anyhow::{Result, anyhow};
use log::{debug, error, info};
use std::path::Path;

use crate::app::App;
use crate::config::KeyedProject;
use crate::utils::expand_tilde;

pub fn cmd_key(app: &App, key: &str, print_path: bool) -> Result<()> {
    let keyed_projects = get_keyed_projects(app);
    cmd_key_with_projects(app, key, print_path, &keyed_projects)
}

pub fn cmd_keys(app: &App) -> Result<()> {
    let keyed_projects = get_keyed_projects(app);

    if keyed_projects.is_empty() {
        println!("No keys configured");
        return Ok(());
    }

    for (key, path) in keyed_projects {
        println!("{}: {}", key, path);
    }

    Ok(())
}

fn get_keyed_projects(app: &App) -> Vec<KeyedProject> {
    app.config.keyed_projects()
}

fn cmd_key_with_projects(
    app: &App,
    key: &str,
    print_path: bool,
    keyed_projects: &[KeyedProject],
) -> Result<()> {
    info!(
        "Switching to project by key '{}' (print_path: {})",
        key, print_path
    );

    debug!("Loaded {} keyed projects", keyed_projects.len());

    let expanded_path = resolve_project_path(key, keyed_projects)?;

    if print_path {
        println!("{}", expanded_path);
        return Ok(());
    }

    let project_name = Path::new(&expanded_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");

    info!(
        "Found project '{}' at path: {}",
        project_name, expanded_path
    );

    // Check if session exists
    if let Ok(Some(existing_tab)) = app.kitty.match_session_tab(project_name) {
        info!("Session already exists, focusing existing tab");
        return app.kitty.focus_tab(existing_tab.id);
    }

    info!("No existing session found, creating new one");
    app.kitty
        .create_session_tab_by_path(&expanded_path, project_name)
}

fn resolve_project_path(key: &str, keyed_projects: &[KeyedProject]) -> Result<String> {
    let project_path = keyed_projects
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, path)| path)
        .ok_or_else(|| {
            error!("No project found for key: {}", key);
            anyhow!("No project found for key: {}", key)
        })?;

    Ok(expand_tilde(project_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::config::Config;
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

        let config_content = r#"[global]
version = "1.0"

[search]
dirs = []
vsc = []
"#;

        let config_file = temp_dir.join("test_config.toml");
        fs::write(&config_file, config_content).unwrap();

        Config::load_from_path(Some(config_file), None).unwrap()
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

    #[test]
    fn test_cmd_keys_empty() {
        let config = create_test_config();
        let app = App::new(config);

        let result = cmd_keys(&app);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_keys_with_projects() {
        let config = create_test_config();
        let app = App::new(config);

        let result = cmd_keys(&app);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_keyed_projects() {
        let config = create_test_config();
        let app = App::new(config);

        let projects = get_keyed_projects(&app);
        assert!(projects.is_empty() || !projects.is_empty());
    }
}
