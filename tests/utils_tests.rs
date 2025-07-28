use ksm::utils::{expand_tilde, format_project_for_selection, parse_project_selection};
use std::env;

#[test]
fn test_expand_tilde_with_home() {
    env::set_var("HOME", "/home/testuser");
    assert_eq!(expand_tilde("~/Documents"), "/home/testuser/Documents");
    assert_eq!(expand_tilde("~/dev/project"), "/home/testuser/dev/project");
}

#[test]
fn test_expand_tilde_without_tilde() {
    assert_eq!(expand_tilde("/absolute/path"), "/absolute/path");
    assert_eq!(expand_tilde("relative/path"), "relative/path");
}

#[test]
fn test_parse_project_selection_valid() {
    let input = "my-project (/home/user/dev/my-project)";
    let result = parse_project_selection(input).unwrap();
    assert_eq!(result.0, "my-project");
    assert_eq!(result.1, "/home/user/dev/my-project");
}

#[test]
fn test_parse_project_selection_with_spaces() {
    let input = "my awesome project (/home/user/dev/my awesome project)";
    let result = parse_project_selection(input).unwrap();
    assert_eq!(result.0, "my awesome project");
    assert_eq!(result.1, "/home/user/dev/my awesome project");
}

#[test]
fn test_parse_project_selection_invalid() {
    let invalid_inputs = vec![
        "project-without-path",
        "project (/incomplete/path",
        "project incomplete/path)",
        "",
    ];

    for input in invalid_inputs {
        assert!(parse_project_selection(input).is_err());
    }
}

#[test]
fn test_format_project_for_selection() {
    assert_eq!(
        format_project_for_selection("my-project", "/home/user/dev/my-project"),
        "my-project (/home/user/dev/my-project)"
    );

    assert_eq!(
        format_project_for_selection("project with spaces", "/path/with spaces"),
        "project with spaces (/path/with spaces)"
    );
}

#[test]
fn test_format_and_parse_roundtrip() {
    let name = "test-project";
    let path = "/home/user/dev/test-project";

    let formatted = format_project_for_selection(name, path);
    let (parsed_name, parsed_path) = parse_project_selection(&formatted).unwrap();

    assert_eq!(parsed_name, name);
    assert_eq!(parsed_path, path);
}

#[test]
fn test_parse_project_selection_edge_cases() {
    // Test with parentheses in project name
    let input = "project (with parens) (/home/user/project (with parens))";
    let result = parse_project_selection(input).unwrap();
    assert_eq!(result.0, "project (with parens)");
    assert_eq!(result.1, "/home/user/project (with parens)");

    // Test with multiple spaces
    let input = "project   with   spaces (/path/with/spaces)";
    let result = parse_project_selection(input).unwrap();
    assert_eq!(result.0, "project   with   spaces");
    assert_eq!(result.1, "/path/with/spaces");
}
