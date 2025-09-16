mod common;

use common::TestEnvironment;

#[tokio::test]
async fn test_environment_setup_basic() -> Result<(), Box<dyn std::error::Error>> {
    // Test basic environment setup without git
    let test_env = TestEnvironment::new(&["project-a", "project-b", "project-c"], false).await?;

    // Verify projects were created
    assert_eq!(test_env.get_all_project_paths().len(), 3);
    assert_eq!(test_env.get_all_git_repos().len(), 0); // No git repos

    // Check project structure
    for project_path in test_env.get_all_project_paths() {
        assert!(
            project_path.exists(),
            "Project directory should exist: {:?}",
            project_path
        );
        assert!(
            project_path.join("README.md").exists(),
            "README.md should exist"
        );
        assert!(
            project_path.join("Cargo.toml").exists(),
            "Cargo.toml should exist"
        );
        assert!(
            project_path.join("src").join("main.rs").exists(),
            "src/main.rs should exist"
        );
    }

    // Test environment validation
    let validation = test_env.validate().await?;
    assert!(
        validation.is_valid(),
        "Environment should be valid: {}",
        validation.summary()
    );

    println!("Test environment setup completed successfully");
    println!("Environment ID: {}", test_env.get_env_id());
    println!("Base directory: {:?}", test_env.get_base_dir());

    Ok(())
}

#[tokio::test]
async fn test_environment_setup_with_git() -> Result<(), Box<dyn std::error::Error>> {
    // Test environment setup with git repositories
    let test_env = TestEnvironment::new(&["repo-a", "repo-b"], true).await?;

    // Verify projects and git repos were created
    assert_eq!(test_env.get_all_project_paths().len(), 2);
    assert_eq!(test_env.get_all_git_repos().len(), 2);

    // Check git repositories
    for git_repo in test_env.get_all_git_repos() {
        let git_dir = git_repo.join(".git");
        assert!(
            git_dir.exists(),
            "Git directory should exist: {:?}",
            git_dir
        );
        assert!(git_dir.is_dir(), "Git directory should be a directory");

        // Check git log (should have commits)
        let log_output = tokio::process::Command::new("git")
            .args(["log", "--oneline"])
            .current_dir(git_repo)
            .output()
            .await?;

        assert!(log_output.status.success(), "Git log should succeed");
        let log_content = String::from_utf8(log_output.stdout)?;
        assert!(
            !log_content.trim().is_empty(),
            "Git log should not be empty"
        );
        assert!(
            log_content.contains("Initial commit"),
            "Should have initial commit"
        );

        // Check for .gitignore
        assert!(
            git_repo.join(".gitignore").exists(),
            ".gitignore should exist"
        );
    }

    // Test environment validation
    let validation = test_env.validate().await?;
    assert!(
        validation.is_valid(),
        "Environment should be valid: {}",
        validation.summary()
    );

    println!("Git-enabled test environment setup completed successfully");
    println!("Environment ID: {}", test_env.get_env_id());

    Ok(())
}

#[tokio::test]
async fn test_ksm_config_generation() -> Result<(), Box<dyn std::error::Error>> {
    // Test KSM configuration file generation
    let mut test_env = TestEnvironment::new(&["config-test-a", "config-test-b"], false).await?;

    // Generate KSM config
    let config_path = test_env.create_ksm_config(None).await?;
    assert!(config_path.exists(), "Config file should exist");

    // Read and verify config content
    let config_content = std::fs::read_to_string(&config_path)?;

    // Check for required sections
    assert!(
        config_content.contains("[global]"),
        "Should have global section"
    );
    assert!(
        config_content.contains("[search]"),
        "Should have search section"
    );
    assert!(
        config_content.contains("[projects]"),
        "Should have projects section"
    );
    assert!(
        config_content.contains("[keys]"),
        "Should have keys section"
    );
    assert!(
        config_content.contains("[session]"),
        "Should have session section"
    );

    // Check for project entries
    assert!(
        config_content.contains("test_project_1"),
        "Should have project 1 entry"
    );
    assert!(
        config_content.contains("test_project_2"),
        "Should have project 2 entry"
    );
    assert!(
        config_content.contains("config-test-a"),
        "Should reference actual project path"
    );
    assert!(
        config_content.contains("config-test-b"),
        "Should reference actual project path"
    );

    // Check for key mappings
    assert!(
        config_content.contains("P1 = \"test_project_1\""),
        "Should have P1 key mapping"
    );
    assert!(
        config_content.contains("P2 = \"test_project_2\""),
        "Should have P2 key mapping"
    );

    // Test with additional config
    let additional_config = "[custom]\ntest_value = true\n";
    let config_path_2 = test_env.create_ksm_config(Some(additional_config)).await?;
    let config_content_2 = std::fs::read_to_string(&config_path_2)?;
    assert!(
        config_content_2.contains("test_value = true"),
        "Should include additional config"
    );

    println!("KSM configuration generation completed successfully");
    println!("Config file: {:?}", config_path);

    Ok(())
}

#[tokio::test]
async fn test_helper_scripts_generation() -> Result<(), Box<dyn std::error::Error>> {
    // Test helper scripts generation
    let test_env = TestEnvironment::new(&["script-test"], true).await?;

    // Generate helper scripts
    let scripts_dir = test_env.create_helper_scripts().await?;
    assert!(scripts_dir.exists(), "Scripts directory should exist");

    // Check verification script
    let verify_script = scripts_dir.join("verify_environment.sh");
    assert!(verify_script.exists(), "Verification script should exist");

    let verify_content = std::fs::read_to_string(&verify_script)?;
    assert!(
        verify_content.contains("KSM Test Environment Verification"),
        "Should have verification title"
    );
    assert!(
        verify_content.contains(test_env.get_env_id()),
        "Should contain environment ID"
    );

    // Check cleanup script
    let cleanup_script = scripts_dir.join("cleanup_environment.sh");
    assert!(cleanup_script.exists(), "Cleanup script should exist");

    let cleanup_content = std::fs::read_to_string(&cleanup_script)?;
    assert!(
        cleanup_content.contains("KSM Test Environment Cleanup"),
        "Should have cleanup title"
    );
    assert!(cleanup_content.contains("WARNING"), "Should have warning");

    // Check script permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let verify_perms = std::fs::metadata(&verify_script)?.permissions();
        assert!(
            verify_perms.mode() & 0o111 != 0,
            "Verification script should be executable"
        );

        let cleanup_perms = std::fs::metadata(&cleanup_script)?.permissions();
        assert!(
            cleanup_perms.mode() & 0o111 != 0,
            "Cleanup script should be executable"
        );
    }

    println!("Helper scripts generation completed successfully");
    println!("Scripts directory: {:?}", scripts_dir);

    Ok(())
}

#[tokio::test]
async fn test_environment_validation() -> Result<(), Box<dyn std::error::Error>> {
    // Test comprehensive environment validation
    let mut test_env = TestEnvironment::new(&["validation-test"], true).await?;

    // Create config file
    test_env.create_ksm_config(None).await?;

    // Perform validation
    let validation = test_env.validate().await?;

    println!("Validation summary:\n{}", validation.summary());

    // Check validation results
    assert!(validation.base_dir_exists, "Base directory should exist");
    assert_eq!(validation.total_projects, 1, "Should have one project");
    assert_eq!(validation.total_git_repos, 1, "Should have one git repo");
    assert!(validation.config_file_valid, "Config file should be valid");
    assert!(
        validation.project_dirs_valid[0],
        "Project directory should be valid"
    );
    assert!(
        validation.git_repos_valid[0],
        "Git repository should be valid"
    );
    assert!(
        validation.validation_errors.is_empty(),
        "Should have no validation errors"
    );
    assert!(validation.is_valid(), "Overall validation should pass");

    Ok(())
}

#[tokio::test]
async fn test_project_path_lookup() -> Result<(), Box<dyn std::error::Error>> {
    // Test project path lookup functionality
    let test_env = TestEnvironment::new(&["lookup-a", "lookup-b", "lookup-c"], false).await?;

    // Test successful lookups
    assert!(
        test_env.get_project_path("lookup-a").is_some(),
        "Should find lookup-a"
    );
    assert!(
        test_env.get_project_path("lookup-b").is_some(),
        "Should find lookup-b"
    );
    assert!(
        test_env.get_project_path("lookup-c").is_some(),
        "Should find lookup-c"
    );

    // Test failed lookup
    assert!(
        test_env.get_project_path("nonexistent").is_none(),
        "Should not find nonexistent project"
    );

    // Verify actual paths
    let path_a = test_env.get_project_path("lookup-a").unwrap();
    assert!(
        path_a.ends_with("lookup-a"),
        "Path should end with project name"
    );
    assert!(path_a.exists(), "Project path should exist");

    Ok(())
}

#[tokio::test]
async fn test_environment_isolation() -> Result<(), Box<dyn std::error::Error>> {
    // Test that multiple test environments are isolated from each other
    let env1 = TestEnvironment::new(&["isolation-test-1"], false).await?;
    let env2 = TestEnvironment::new(&["isolation-test-2"], false).await?;

    // Environments should have different IDs
    assert_ne!(
        env1.get_env_id(),
        env2.get_env_id(),
        "Environment IDs should be different"
    );

    // Environments should have different base directories
    assert_ne!(
        env1.get_base_dir(),
        env2.get_base_dir(),
        "Base directories should be different"
    );

    // Each environment should only see its own projects
    assert_eq!(env1.get_all_project_paths().len(), 1);
    assert_eq!(env2.get_all_project_paths().len(), 1);

    let env1_project = env1.get_project_path("isolation-test-1");
    let env2_project = env2.get_project_path("isolation-test-2");

    assert!(env1_project.is_some(), "Env1 should have its project");
    assert!(env2_project.is_some(), "Env2 should have its project");

    // Cross-environment lookups should fail
    assert!(
        env1.get_project_path("isolation-test-2").is_none(),
        "Env1 should not see env2 project"
    );
    assert!(
        env2.get_project_path("isolation-test-1").is_none(),
        "Env2 should not see env1 project"
    );

    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    // Test error handling in various scenarios

    // Test with empty project names list
    let empty_env = TestEnvironment::new(&[], false).await?;
    assert_eq!(empty_env.get_all_project_paths().len(), 0);

    let validation = empty_env.validate().await?;
    assert!(
        validation.is_valid(),
        "Empty environment should still be valid"
    );

    // Test validation with missing files (simulate corruption)
    let test_env = TestEnvironment::new(&["error-test"], false).await?;

    // Remove a required file to trigger validation error
    let project_path = test_env.get_project_path("error-test").unwrap();
    let readme_path = project_path.join("README.md");
    std::fs::remove_file(&readme_path)?;

    let validation = test_env.validate().await?;
    assert!(
        !validation.is_valid(),
        "Environment should be invalid after file removal"
    );
    assert!(
        !validation.validation_errors.is_empty(),
        "Should have validation errors"
    );

    println!(
        "Validation errors (expected): {:?}",
        validation.validation_errors
    );

    Ok(())
}
