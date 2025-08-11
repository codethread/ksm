use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "ksm")]
#[command(about = "Kitty Session Manager - Rust implementation")]
pub struct Cli {
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
