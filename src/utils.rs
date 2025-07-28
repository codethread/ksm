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
        return Err(anyhow::anyhow!("Invalid project selection format: {}", selected_text));
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
    
    Err(anyhow::anyhow!("Invalid project selection format: {}", selected_text))
}

pub fn format_project_for_selection(name: &str, path: &str) -> String {
    format!("{} ({})", name, path)
}