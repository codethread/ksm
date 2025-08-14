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
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(alias = "ls")]
    List,
    #[command(alias = "k")]
    Key {
        key: String,
        #[arg(short, long)]
        work: bool,
        #[arg(short, long)]
        path: bool,
    },
    /// Interactive project selection (ESC/Ctrl-C to cancel)
    #[command(alias = "s")]
    Select {
        #[arg(short, long)]
        work: bool,
    },
}
