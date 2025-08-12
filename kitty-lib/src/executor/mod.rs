pub mod kitty_executor;
pub mod mock_executor;

use crate::commands::focus_tab::KittenFocusTabCommand;
use crate::commands::launch::KittenLaunchCommand;
use crate::commands::ls::KittenLsCommand;
use crate::types::{KittyCommandResult, KittyLaunchResponse, KittyLsResponse};
use anyhow::Result;

pub trait CommandExecutor {
    fn ls(&self, command: KittenLsCommand) -> Result<KittyLsResponse>;
    fn focus_tab(&self, command: KittenFocusTabCommand) -> Result<KittyCommandResult<()>>;
    fn launch(
        &self,
        command: KittenLaunchCommand,
    ) -> Result<KittyCommandResult<KittyLaunchResponse>>;
}

pub use kitty_executor::KittyExecutor;
pub use mock_executor::MockExecutor;
