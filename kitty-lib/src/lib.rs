pub mod commands;
pub mod executor;
pub mod types;
pub mod utils;

// Re-export commonly used types
pub use commands::close_tab::KittenCloseTabCommand;
pub use commands::focus_tab::KittenFocusTabCommand;
pub use commands::launch::KittenLaunchCommand;
pub use commands::ls::KittenLsCommand;
pub use commands::navigate_tab::{KittenNavigateTabCommand, TabNavigationDirection};
pub use executor::{CommandExecutor, KittyExecutor, MockExecutor};
pub use types::{
    KittyCommandResult, KittyLaunchResponse, KittyLsResponse, KittyOsWindow, KittyTab, KittyWindow,
};
