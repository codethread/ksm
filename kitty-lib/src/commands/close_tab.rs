#[derive(Debug, Clone)]
pub struct KittenCloseTabCommand {
    pub tab_id: u32,
}

impl KittenCloseTabCommand {
    pub fn new(tab_id: u32) -> Self {
        Self { tab_id }
    }
}
