use super::{
    config::Config,
    git::{
        add, commit,
        pull::{do_fetch, do_merge},
        push,
        shared::get_current_branch,
    },
    map::Map,
};
use dirs::{config_dir, home_dir};
use git2::{build::RepoBuilder, Cred, FetchOptions, RemoteCallbacks, Repository, StatusOptions};
use owo_colors::{OwoColorize, Stream::Stdout, Style};
use serde_json::from_reader;
use std::{
    env::set_current_dir,
    fs::{read_dir, OpenOptions},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::exit,
    vec,
};
use tabled::Table;

pub fn print_error(msg: String) {
    let style = Style::new().bold().red();
    eprintln!(
        "{}",
        msg.if_supports_color(Stdout, |text| text.style(style))
    );
}

pub fn print_info(msg: String) {
    let style = Style::new().bold().green();
    println!(
        "{}",
        msg.if_supports_color(Stdout, |text| text.style(style))
    );
}

#[cfg(debug_assertions)]
pub fn print_debug(msg: String) {
    let style = Style::new().bold().cyan();
    eprintln!(
        "{}",
        msg.if_supports_color(Stdout, |text| text.style(style))
    );
}

pub fn push(path: &Path, message: String) {
    let repo = match Repository::open(path) {
        Ok(repo) => repo,
        Err(_e) => {
            #[cfg(debug_assertions)]
            print_debug(format!("{_e}"));

            print_error(format!(
                "unable to open repo {} is it really a git repo?",
                path.display()
            ));
            exit(9);
        }
    };

    match set_current_dir(path) {
        Ok(()) => {}
        Err(_e) => {
            #[cfg(debug_assertions)]
            print_debug(format!("{_e}"));

            print_error("failed to get set current directory".to_string());
            exit(1);
        }
    };

    add::git_add(&repo);

    let mut status_opts = StatusOptions::default();

    let statuses = match repo.statuses(Some(&mut status_opts)) {
        Ok(statuses) => statuses,
        Err(_e) => {
            #[cfg(debug_assertions)]
            print_debug(format!("{_e}"));

            print_error("failed to get status of files in repo".to_string());
            exit(9);
        }
    };

    if statuses.is_empty() {
        print_error("No files to commit".to_string());
        exit(1);
    }

    commit::sign_commit_or_regular(&repo, &message);
    match push::git_push(&repo) {
        Ok(_) => {}
        Err(_e) => {
            #[cfg(debug_assertions)]
            print_debug(format!("{_e}"));

            print_error("failed to push changes, commit has been made.".to_string());
            exit(9);
        }
    }
}

pub fn clone(url: String, path: &Path) {
    let mut builder = RepoBuilder::new();
    let mut callbacks = RemoteCallbacks::new();
    let mut fetch_options = FetchOptions::new();

    // ssh
    if url.starts_with("git@") {
        callbacks.credentials(|_, _, _| {
            let creds =
                Cred::ssh_key_from_agent("git").expect("Could not create credentials object");
            Ok(creds)
        });
        fetch_options.remote_callbacks(callbacks);
    } else {
        fetch_options.remote_callbacks(callbacks);
    }

    builder.fetch_options(fetch_options);
    builder
        .clone(&url, path)
        .expect("failed to clone directory");
}

pub fn sync_config(path: PathBuf) -> Vec<(PathBuf, PathBuf)> {
    let config_dir = match config_dir() {
        Some(config) => config,
        None => {
            print_error("Unable to resolve xdg-config".to_string());
            exit(1);
        }
    };
    let files = read_dir(path).expect("unable to read given path");
    let mut sync_files: Vec<(PathBuf, PathBuf)> = vec![];
    for file in files {
        match file {
            Err(_e) => {
                print_debug(_e.to_string());
            }
            Ok(file) => {
                let file_path = file.path();
                let filename = file.file_name();
                let target = config_dir.join(&filename);

                sync_files.append(&mut vec![(file_path, target)]);
            }
        }
    }
    sync_files
}

pub fn symlink_internal(file: &Path, target: &Path) {
    match symlink(file, target) {
        Ok(_) => {
            print_info(format!("{} -> {}", target.display(), file.display()));
        }
        Err(e) => {
            if e.to_string() != *"File exists (os error 17)" {
                print_debug(e.to_string())
            } else if target.is_symlink() {
                let target_canon = target.canonicalize().unwrap();
                let source = file.canonicalize().unwrap();
                if source != target_canon {
                    print_info(format!(
                        "{} is not symlinked to {}",
                        target
                            .display()
                            .if_supports_color(Stdout, |text| text.cyan()),
                        file.display()
                            .if_supports_color(Stdout, |text| text.green())
                    ))
                }
            }
        }
    }
}

pub fn sync(path: &Path) {
    let home_dir = match home_dir() {
        Some(home) => home,
        None => {
            print_error("unable to resolve home direcotry".to_string());
            exit(1);
        }
    };
    let config_path = path.join("dotfox.json");

    if !config_path.exists() || config_path.is_dir() {
        print_error("Missing config".to_string());
        exit(78);
    }

    let config_reader = match OpenOptions::new().read(true).open(config_path) {
        Ok(reader) => reader,
        Err(e) => {
            print_error(format!("failed to read config.\n{e}"));
            exit(71);
        }
    };

    let config: Config = match from_reader(config_reader) {
        Ok(config) => config,
        Err(e) => {
            print_error(format!("Failed to Deseralize config\n{e}"));
            exit(78);
        }
    };

    let mut files = config.folders();
    let mut sync_files: Vec<(PathBuf, PathBuf)> = vec![];
    let mut table: Vec<Map> = vec![];

    files.sort();
    files.dedup();

    print_info("Resolving symlinks".to_string());

    for dir in files {
        let dir = path.join(dir);
        if !dir.is_dir() {
            print_error(format!("Path {} is not a direcotory", dir.display()));
            exit(1);
        }
        let in_files = read_dir(dir).unwrap();

        for inner_file in in_files {
            match inner_file {
                Err(_e) =>
                {
                    #[cfg(debug_assertions)]
                    print_error(_e.to_string())
                }
                Ok(file) => {
                    let filename = file.file_name();
                    let file = file.path();
                    if filename == *".config" {
                        let mut f = sync_config(file);
                        sync_files.append(&mut f);
                    } else {
                        let target = home_dir.join(filename);
                        sync_files.append(&mut vec![(file, target)])
                    }
                }
            }
        }
    }

    if !sync_files.is_empty() {
        for file in &sync_files {
            table.append(&mut vec![Map::new(&file.0, &file.1)])
        }

        let pre_len = sync_files.len();
        sync_files.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        sync_files.dedup_by(|a, b| a.1.eq(&b.1));

        if sync_files.len() != pre_len {
            print_error("There is a conflict, resolution could not be complete".to_string());
            exit(9);
        }

        let table = Table::new(&table).to_string();

        println!("{}", table.if_supports_color(Stdout, |text| text.bold()));
    } else {
        print_error("there are no files to sync".to_string());
        exit(1);
    }

    print_error("Symlinks resolved".to_string());

    for file in &sync_files {
        symlink_internal(&file.0, &file.1);
    }
}

pub fn pull(path: &PathBuf) {
    let repo = match Repository::open(path) {
        Ok(repo) => repo,
        Err(_e) => {
            #[cfg(debug_assertions)]
            print_debug(format!("{_e}"));

            print_error("failed to open repo".to_string());
            exit(9);
        }
    };

    let mut remote = repo.find_remote("origin").unwrap();
    let branch = get_current_branch(&repo).unwrap();
    let fetch_commit = match do_fetch(&repo, &[&branch], &mut remote) {
        Ok(a) => a,
        Err(_e) => {
            #[cfg(debug_assertions)]
            print_debug(format!("{_e}"));

            print_error("failed to fetch latest commit".to_string());
            exit(9);
        }
    };
    match do_merge(&repo, &branch, fetch_commit) {
        Ok(_) => {}
        Err(_e) => {
            #[cfg(debug_assertions)]
            print_debug(format!("{_e}"));

            print_error("failed to merge".to_string());
            exit(9);
        }
    }
}
