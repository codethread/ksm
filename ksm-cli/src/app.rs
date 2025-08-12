use crate::kitty::Kitty;
use kitty_lib::KittyExecutor;

pub struct App {
    pub kitty: Kitty<KittyExecutor>,
}

impl App {
    pub fn new() -> Self {
        Self {
            kitty: Kitty::new(),
        }
    }
}
