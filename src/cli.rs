use clap::{Parser, Subcommand};
use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Parser)]
#[clap(author="Luna D. Dragon", version=VERSION, about="My cli tool to manage dotfiles", long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[clap(about = "sync repo to home directory")]
    Sync {
        #[clap(help = "path to repo, optional defaults to current dir")]
        path: Option<PathBuf>,
    },
    #[clap(about = "Clone a git repoistory and sync files to home dir")]
    Clone {
        #[clap(help = "url of repoistory")]
        url: String,
        #[clap(help = "path to repo, defaults to repositry name")]
        path: Option<PathBuf>,
    },
    #[clap(about = "commit and push changes")]
    Push {
        #[clap(short = 'm',long="message", help = "message for commit")]
        message: String,
        #[clap(help = "path to repo, optional defaults to current dir")]
        path: Option<PathBuf>,
    },
}
