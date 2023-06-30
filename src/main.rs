#[cfg(debug_assertions)]
use crate::utils::print_debug;
use clap::Parser;
use cli::Commands;
use git2::Repository;
use std::{
    fs::{create_dir_all, read_link},
    path::PathBuf,
    process::exit,
};
use utils::{clone, print_error, print_info, pull, push, sync};
mod cli;
mod config;
mod git;
mod map;
mod utils;

fn resolve_dir(path: Option<PathBuf>) -> PathBuf {
    match path {
        Some(path) => path,
        None => match PathBuf::from(".").canonicalize() {
            Ok(path) => {
                if path.is_symlink() {
                    match read_link(path) {
                        Ok(path) => path,
                        Err(_e) => {
                            #[cfg(debug_assertions)]
                            print_debug(format!("{_e}"));

                            print_error("failed to resolve symlink".to_string());
                            exit(1);
                        }
                    }
                } else {
                    path
                }
            }
            Err(_e) => {
                #[cfg(debug_assertions)]
                print_debug(format!("{_e}"));

                print_error("failed to canonicalize path".to_string());
                exit(1);
            }
        },
    }
}

fn startup() {
    let startup_text = "
       _       _    __
    __| | ___ | |_ / _| _____  __
   / _` |/ _ \\| __| |_ / _ \\ \\/ /
  | (_| | (_) | |_|  _| (_) >  <
   \\__,_|\\___/ \\__|_|  \\___/_/\\_\\
    ";
    print_info(startup_text.to_string());
}

fn main() {
    let cli = cli::Cli::parse();
    startup();

    match cli.command {
        Commands::Init { path } => {
            let path = resolve_dir(path);
            if !path.exists() {
                match create_dir_all(&path) {
                    Ok(_) => {}
                    Err(_e) => {
                        #[cfg(debug_assertions)]
                        print_debug(_e.to_string());

                        print_error("failed to canonicalize url".to_string());
                        exit(1);
                    }
                };
            }
            match Repository::init(&path) {
                Ok(_) => {}
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    print_debug(_e.to_string());

                    print_error(format!("Failed to create diectory {}", path.display()));
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
                            print_error("not valid url".to_string());
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
        Commands::Pull { path } => {
            let path = resolve_dir(path);
            pull(&path);
            sync(&path);
        }
    }
}
