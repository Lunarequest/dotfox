use super::{
    git::{add, commit, push},
    resolve_dir,
};
use dirs::{config_dir, home_dir};
use git2::{build::RepoBuilder, Cred, FetchOptions, RemoteCallbacks, Repository, StatusOptions};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    env::set_current_dir,
    fs::{read_dir, read_to_string},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::exit,
};

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

pub fn sync_config(path: PathBuf) {
    let config_dir = match config_dir() {
        Some(config) => config,
        None => {
            eprintln!("Unable to resolve xdg-config");
            exit(1);
        }
    };
    let files = read_dir(path).expect("unable to read given path");
    for file in files {
        match file {
            Err(_e) => {
                eprintln!("{_e}");
            }
            Ok(file) => {
                let file_path = file.path();
                let filename = file.file_name();
                let target = &config_dir.join(&filename);
                match symlink(&file_path, target) {
                    Ok(_) => {}
                    Err(_e) => {
                        if _e.to_string() != *"File exists (os error 17)" {
                            eprintln!("{_e}")
                        } else if target.is_symlink() {
                            let target = resolve_dir(Some(target.to_owned()));
                            let source = resolve_dir(Some(file_path.to_owned()));
                            if source != target {
                                println!(
                                    "{} is not symlinked to {}",
                                    target.display(),
                                    file_path.display()
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn sync(path: &PathBuf) {
    let home_dir = match home_dir() {
        Some(home) => home,
        None => {
            eprintln!("unable to resolve home direcotry");
            exit(1);
        }
    };
    let ignore_file_path = path.join(PathBuf::from(".foxignore"));
    let mut ignores = String::from("\n.git\n.github");
    if ignore_file_path.exists() && ignore_file_path.is_file() {
        ignores.push_str(
            read_to_string(ignore_file_path)
                .expect("unable to read .foxignore file")
                .as_str(),
        );
    }
    let ignore_path = ignores.split('\n').collect::<Vec<&str>>();
    let files = read_dir(path).expect("unable to read given path");
    for file in files {
        match file {
            Err(_e) => {
                eprintln!("{_e}");
            }
            Ok(file) => {
                let file_path = file.path();
                let file_as_string = file_path.to_string_lossy();
                if !ignore_path
                    .par_iter()
                    .any(|&i| file_as_string.contains(i) && !i.is_empty())
                    && file_path.is_dir()
                {
                    let inner_paths =
                        read_dir(&file_path).expect("unable to read inner direcotries");
                    for inner_file in inner_paths {
                        match inner_file {
                            Err(_e) => eprintln!("{_e}"),
                            Ok(inner_file) => {
                                let filename = inner_file.file_name();
                                let inner_file = inner_file.path();
                                if inner_file.as_os_str().to_string_lossy().contains(".config") {
                                    sync_config(inner_file);
                                } else {
                                    let target = &home_dir.join(filename);
                                    match symlink(&inner_file, target) {
                                        Ok(_) => {}
                                        Err(_e) => {
                                            if _e.to_string() != *"File exists (os error 17)" {
                                                eprintln!("{_e}")
                                            } else if target.is_symlink() {
                                                let target = resolve_dir(Some(target.to_owned()));
                                                let source =
                                                    resolve_dir(Some(file_path.to_owned()));
                                                if source != target {
                                                    println!(
                                                        "{} is not symlinked to {}",
                                                        target.display(),
                                                        file_path.display()
                                                    )
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
