#[derive(Debug, Clone)]
pub struct KittenFocusTabCommand {
    pub tab_id: u32,
}

impl KittenFocusTabCommand {
    pub fn new(tab_id: u32) -> Self {
        Self { tab_id }
    }
}
