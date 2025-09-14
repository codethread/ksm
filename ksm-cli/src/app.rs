use crate::config::Config;
use crate::kitty::Kitty;
use kitty_lib::{CommandExecutor, KittyExecutor};

pub struct App<E: CommandExecutor = KittyExecutor> {
    pub config: Config,
    pub kitty: Kitty<E>,
}

impl App<KittyExecutor> {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            kitty: Kitty::new(),
        }
    }
}

impl<E: CommandExecutor> App<E> {
    /// Create an App with a custom Kitty instance (for testing)
    pub fn with_kitty(config: Config, kitty: Kitty<E>) -> Self {
        Self { config, kitty }
    }
}
