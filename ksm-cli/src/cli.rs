use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "ksm")]
#[command(about = "Kitty Session Manager")]
pub struct Cli {
    /// Path to config file
    #[arg(short, long)]
    pub config: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// List of profile(s) names to load, overriding the implicit profile
    #[arg(short, long)]
    pub profile: Option<Vec<String>>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// List all available projects
    #[command(alias = "ls")]
    List,
    /// Get project by Key
    #[command(alias = "k")]
    Key {
        key: String,
        #[arg(short, long)]
        path: bool,
    },
    /// List all keys
    Keys,
    /// Interactive project selection (ESC/Ctrl-C to cancel)
    #[command(alias = "s")]
    Select,
    /// Navigate to the next tab in the current session
    #[command(name = "next-tab")]
    NextTab {
        /// Disable wrapping from last tab to first tab
        #[arg(long)]
        no_wrap: bool,
    },
    /// Navigate to the previous tab in the current session
    #[command(name = "prev-tab")]
    PrevTab {
        /// Disable wrapping from first tab to last tab
        #[arg(long)]
        no_wrap: bool,
    },
    /// Create a new tab with automatic session context inheritance
    #[command(name = "new-tab")]
    NewTab {
        /// Working directory for the new tab
        #[arg(long)]
        cwd: Option<String>,
        /// Title for the new tab
        #[arg(long)]
        title: Option<String>,
    },
    /// Close all tabs in the current session (or specified session)
    #[command(name = "close-all-session-tabs")]
    CloseAllSessionTabs {
        /// Specific session name to close (if not provided, uses current session)
        #[arg(long)]
        session: Option<String>,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}
