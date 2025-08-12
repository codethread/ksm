use crate::kitty::Kitty;
use kitty_lib::KittyExecutor;

pub struct App {
    pub kitty: Kitty<KittyExecutor>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            kitty: Kitty::new(),
        }
    }
}
