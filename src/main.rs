use clap::Parser;
use cli::Commands;
use std::path::PathBuf;
use utils::{clone, push, sync};
mod cli;
mod utils;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let cli = cli::Cli::parse();
    match cli.command {
        Commands::Sync { path } => {
            let path = match path {
                Some(path) => path,
                None => PathBuf::from("."),
            };
            sync(&path).await;
        }
        Commands::Clone { url, path } => {
            let path = match path {
                Some(path) => path,
                None => {
                    let base = url.split('/').last().unwrap().replace(".git", "");
                    PathBuf::from(base)
                }
            };
            clone(url, &path).await;
            sync(&path).await;
        }
        Commands::Push { message, path } => {
            let path = match path {
                Some(path) => path,
                None => PathBuf::from(".").canonicalize().unwrap(),
            };
            push(&path, message).await;
        }
    }
}
