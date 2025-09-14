#[derive(Debug, Clone)]
pub struct KittenNavigateTabCommand {
    pub direction: TabNavigationDirection,
    pub session_name: Option<String>,
    pub allow_wrap: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TabNavigationDirection {
    Next,
    Previous,
}

impl Default for KittenNavigateTabCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl KittenNavigateTabCommand {
    pub fn new() -> Self {
        Self {
            direction: TabNavigationDirection::Next,
            session_name: None,
            allow_wrap: true,
        }
    }

    pub fn next() -> Self {
        Self {
            direction: TabNavigationDirection::Next,
            session_name: None,
            allow_wrap: true,
        }
    }

    pub fn previous() -> Self {
        Self {
            direction: TabNavigationDirection::Previous,
            session_name: None,
            allow_wrap: true,
        }
    }

    pub fn with_session(mut self, session_name: impl Into<String>) -> Self {
        self.session_name = Some(session_name.into());
        self
    }

    pub fn with_wrap(mut self, allow_wrap: bool) -> Self {
        self.allow_wrap = allow_wrap;
        self
    }

    pub fn no_wrap(mut self) -> Self {
        self.allow_wrap = false;
        self
    }
}
