pub mod kitty_executor;
pub mod mock_executor;

use crate::commands::focus_tab::KittenFocusTabCommand;
use crate::commands::launch::KittenLaunchCommand;
use crate::commands::ls::KittenLsCommand;
use anyhow::Result;
use std::process::{ExitStatus, Output};

pub trait CommandExecutor {
    fn ls(&self, command: KittenLsCommand) -> Result<Output>;
    fn focus_tab(&self, command: KittenFocusTabCommand) -> Result<ExitStatus>;
    fn launch(&self, command: KittenLaunchCommand) -> Result<ExitStatus>;
}

pub use kitty_executor::KittyExecutor;
pub use mock_executor::MockExecutor;
