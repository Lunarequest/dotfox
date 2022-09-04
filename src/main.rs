use std::path::PathBuf;

use clap::Parser;
use cli::Commands;
use utils::sync;
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
            sync(path).await;
        }
    }
}
