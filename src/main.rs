use anyhow::{anyhow, Result};
use clap::Parser;
use cli::Commands;
use git2::Repository;
use std::{fs::create_dir_all, path::PathBuf, process::exit};
use utils::{clone, commit, print_error, print_info, pull, push, sync, verify};
mod cli;
mod config;
mod git;
mod map;
mod utils;

fn resolve_dir(path: Option<PathBuf>) -> Result<PathBuf> {
    match path {
        Some(path) => Ok(path),
        None => match PathBuf::from(".").canonicalize() {
            Ok(path) => Ok(path),
            Err(_e) => Err(anyhow!("failed to canonicalize path")),
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

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    startup();

    match cli.command {
        Commands::Init { path } => {
            let path = resolve_dir(path)?;
            if !path.exists() {
                match create_dir_all(&path) {
                    Ok(_) => {}
                    Err(_e) => {
                        print_error("failed to canonicalize oath".to_string());
                        exit(1);
                    }
                };
            }
            Repository::init(&path)?;
            Ok(())
        }

        Commands::Sync { path } => {
            let path = resolve_dir(path)?;
            sync(&path)?;
            Ok(())
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
            clone(url, &path)?;
            sync(&path)?;
            Ok(())
        }

        Commands::Commit { message, path } => {
            let path = resolve_dir(path)?;
            commit(&path, message)?;
            Ok(())
        }

        Commands::Push { message, path } => {
            let path = resolve_dir(path)?;
            push(&path, message)?;
            Ok(())
        }
        Commands::Pull { path } => {
            let path = resolve_dir(path)?;
            pull(&path)?;
            sync(&path)?;
            Ok(())
        }
        Commands::Verify { path } => {
            let path = resolve_dir(path)?;
            verify(&path)?;
            Ok(())
        }
    }
}
