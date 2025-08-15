use anyhow::Result;
use std::env;

pub fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        let home = env::var("HOME").unwrap_or_default();
        path.replacen("~", &home, 1)
    } else {
        path.to_string()
    }
}

pub fn parse_project_selection(selected_text: &str) -> Result<(String, String)> {
    // Parse the selected item to get the project path
    // Format is "project_name (path)"
    // We need to find the last " (" that is followed by a matching ")" at the end

    if !selected_text.ends_with(')') {
        return Err(anyhow::anyhow!(
            "Invalid project selection format: {}",
            selected_text
        ));
    }

    // Find the matching opening parenthesis for the closing one at the end
    let mut paren_count = 0;
    let mut start_paren_pos = None;

    for (i, char) in selected_text.char_indices().rev() {
        match char {
            ')' => paren_count += 1,
            '(' => {
                paren_count -= 1;
                if paren_count == 0 {
                    // Check if this opening paren is preceded by a space
                    if i > 0 && selected_text.chars().nth(i - 1) == Some(' ') {
                        start_paren_pos = Some(i - 1); // Include the space
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(start_pos) = start_paren_pos {
        let project_name = &selected_text[..start_pos];
        let project_path = &selected_text[start_pos + 2..selected_text.len() - 1]; // +2 to skip " (", -1 to skip ")"
        return Ok((project_name.to_string(), project_path.to_string()));
    }

    Err(anyhow::anyhow!(
        "Invalid project selection format: {}",
        selected_text
    ))
}

pub fn format_project_for_selection(name: &str, path: &str) -> String {
    format!("{} ({})", name, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_expand_tilde_with_home() {
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe { env::set_var("HOME", "/home/testuser") };
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
}
