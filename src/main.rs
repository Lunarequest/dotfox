use clap::Parser;
use cli::Commands;
use std::{path::PathBuf, process::exit};
use utils::{clone, push, sync};
mod cli;
mod utils;

fn main() {
    let cli = cli::Cli::parse();
    match cli.command {
        Commands::Sync { path } => {
            let path = match path {
                Some(path) => path,
                None => PathBuf::from("."),
            };
            sync(&path);
        }
        Commands::Clone { url, path } => {
            let path = match path {
                Some(path) => path,
                None => {
                    let base = match url.split('/').last() {
                        Some(s) => s.replace(".git", ""),
                        None => {
                            eprintln!("not valid url");
                            exit(1);
                        }
                    };
                    PathBuf::from(base)
                }
            };
            clone(url, &path);
            sync(&path);
        }
        Commands::Push { message, path } => {
            let path = match path {
                Some(path) => path,
                None => match PathBuf::from(".").canonicalize() {
                    Ok(path) => path,
                    Err(e) => {
                        #[cfg(debug_assertions)]
                        eprintln!("{e}");
                        exit(1);
                    }
                },
            };
            push(&path, message);
        }
    }
}
