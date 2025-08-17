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
}
