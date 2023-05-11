use clap::Parser;
use cli::Commands;
use git2::Repository;
use std::{fs::create_dir_all, path::PathBuf, process::exit};
use utils::{clone, push, sync};
mod cli;
mod git;
mod utils;

fn resolve_dir(path: Option<PathBuf>) -> PathBuf {
    match path {
        Some(path) => path,
        None => match PathBuf::from(".").canonicalize() {
            Ok(path) => path,
            Err(_e) => {
                #[cfg(debug_assertions)]
                eprintln!("{_e}");

                eprintln!("failed to canonicalize url");
                exit(1);
            }
        },
    }
}

fn main() {
    let cli = cli::Cli::parse();
    match cli.command {
        Commands::Init { path } => {
            let path = resolve_dir(path);
            if !path.exists() {
                match create_dir_all(&path) {
                    Ok(_) => {}
                    Err(_e) => {
                        #[cfg(debug_assertions)]
                        eprintln!("{_e}");

                        eprintln!("failed to canonicalize url");
                        exit(1);
                    }
                };
            }
            match Repository::init(&path) {
                Ok(_) => {}
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    eprintln!("{_e}");

                    eprintln!("Failed to create diectory {}", path.display());
                    exit(1);
                }
            }
        }

        Commands::Sync { path } => {
            let path = resolve_dir(path);
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
            let path = resolve_dir(path);
            push(&path, message);
        }
    }
}
