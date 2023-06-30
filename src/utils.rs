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

pub fn push(path: &Path, message: String) {
    let repo = match Repository::open(path) {
        Ok(repo) => repo,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!(
                "unable to open repo {} is it really a git repo?",
                path.display()
            );
            exit(9);
        }
    };

    match set_current_dir(path) {
        Ok(()) => {}
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("failed to get set current directory");
            exit(1);
        }
    };

    add::git_add(&repo);

    let mut status_opts = StatusOptions::default();

    let statuses = match repo.statuses(Some(&mut status_opts)) {
        Ok(statuses) => statuses,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("failed to get status of files in repo");
            exit(9);
        }
    };

    if statuses.is_empty() {
        eprintln!("No files to commit");
        exit(1);
    }

    commit::sign_commit_or_regular(&repo, &message);
    match push::git_push(&repo) {
        Ok(_) => {}
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("failed to push changes, commit has been made.");
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
            eprintln!("Unable to resolve xdg-config");
            exit(1);
        }
    };
    let files = read_dir(path).expect("unable to read given path");
    let mut sync_files: Vec<(PathBuf, PathBuf)> = vec![];
    for file in files {
        match file {
            Err(_e) => {
                eprintln!("{_e}");
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
            println!("{} -> {}", target.display(), file.display());
        }
        Err(e) => {
            if e.to_string() != *"File exists (os error 17)" {
                eprintln!("{e}")
            } else if target.is_symlink() {
                let target_canon = target.canonicalize().unwrap();
                let source = file.canonicalize().unwrap();
                if source != target_canon {
                    println!(
                        "{} is not symlinked to {}",
                        target
                            .display()
                            .if_supports_color(Stdout, |text| text.cyan()),
                        file.display()
                            .if_supports_color(Stdout, |text| text.green())
                    )
                }
            }
        }
    }
}

pub fn sync(path: &Path) {
    let home_dir = match home_dir() {
        Some(home) => home,
        None => {
            eprintln!("unable to resolve home direcotry");
            exit(1);
        }
    };
    let config_path = path.join("dotfox.json");

    if !config_path.exists() || config_path.is_dir() {
        eprintln!("Missing config");
        exit(78);
    }

    let config_reader = match OpenOptions::new().read(true).open(config_path) {
        Ok(reader) => reader,
        Err(e) => {
            eprintln!("failed to read config.\n{e}");
            exit(71);
        }
    };

    let config: Config = match from_reader(config_reader) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to Deseralize config\n{e}");
            exit(78);
        }
    };

    let mut files = config.folders();
    let mut sync_files: Vec<(PathBuf, PathBuf)> = vec![];
    let mut table: Vec<Map> = vec![];

    files.sort();
    files.dedup();

    println!(
        "{}",
        "Resolving symlinks".if_supports_color(Stdout, |text| text.green())
    );

    for dir in files {
        let dir = path.join(dir);
        if !dir.is_dir() {
            eprintln!(
                "{}",
                format!("Path {} is not a direcotory", dir.display())
                    .if_supports_color(Stdout, |text| text.red())
            );
            exit(1);
        }
        let in_files = read_dir(dir).unwrap();

        for inner_file in in_files {
            match inner_file {
                Err(_e) => eprintln!("{_e}"),
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

        let error_style: Style = Style::new().red().bold();

        if sync_files.len() != pre_len {
            eprintln!(
                "{}",
                "There is a conflict, resolution could not be complete"
                    .if_supports_color(Stdout, |text| text.style(error_style))
            );
            exit(9);
        }

        let table = Table::new(&table).to_string();

        println!("{}", table.if_supports_color(Stdout, |text| text.bold()));
    } else {
        eprintln!(
            "{}",
            "there are no files to sync".if_supports_color(Stdout, |text| text.red())
        );
        exit(1);
    }

    println!(
        "{}",
        "Symlinks resolved".if_supports_color(Stdout, |text| text.green())
    );

    for file in &sync_files {
        symlink_internal(&file.0, &file.1);
    }
}

pub fn pull(path: &PathBuf) {
    let repo = match Repository::open(path) {
        Ok(repo) => repo,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!(
                "{}",
                "failed to open repo".if_supports_color(Stdout, |text| text.red())
            );
            exit(9);
        }
    };

    let mut remote = repo.find_remote("origin").unwrap();
    let branch = get_current_branch(&repo).unwrap();
    let fetch_commit = match do_fetch(&repo, &[&branch], &mut remote) {
        Ok(a) => a,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!(
                "{}",
                "failed to fetch latest commit".if_supports_color(Stdout, |text| text.red())
            );
            exit(9);
        }
    };
    match do_merge(&repo, &branch, fetch_commit) {
        Ok(_) => {}
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!(
                "{}",
                "failed to merge".if_supports_color(Stdout, |text| text.red())
            );
            exit(9);
        }
    }
}
