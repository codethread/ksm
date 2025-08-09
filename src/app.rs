use crate::kitty::Kitty;

pub struct App {
    pub kitty: Kitty,
}

impl App {
    pub fn new() -> Self {
        Self {
            kitty: Kitty::new(),
        }
    }
}
