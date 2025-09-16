#[derive(Debug, Clone)]
pub struct KittenSetTabTitleCommand {
    pub title: String,
    pub match_pattern: Option<String>,
}

impl KittenSetTabTitleCommand {
    /// Create a new set tab title command that will set the title for the current tab
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            match_pattern: None,
        }
    }

    /// Set the title for a specific tab matching the given pattern
    pub fn with_match(mut self, pattern: impl Into<String>) -> Self {
        self.match_pattern = Some(pattern.into());
        self
    }

    /// Set the title for a specific tab by ID
    pub fn for_tab_id(mut self, tab_id: u32) -> Self {
        self.match_pattern = Some(format!("id:{}", tab_id));
        self
    }
}
