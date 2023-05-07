use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(author="Luna D. Dragon", version="1.0.0", about="My cli tool to manage dotfiles", long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Sync {
        path: Option<PathBuf>,
    },
    Clone {
        url: String,
        path: Option<PathBuf>,
    },
    Push {
        #[clap(short = 'm')]
        message: String,
        path: Option<PathBuf>,
    },
}
