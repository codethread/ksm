use crate::config::Config;
use crate::kitty::Kitty;
use kitty_lib::KittyExecutor;

pub struct App {
    pub config: Config,
    pub kitty: Kitty<KittyExecutor>,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            kitty: Kitty::new(),
        }
    }
}
