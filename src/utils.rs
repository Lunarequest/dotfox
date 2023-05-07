use dirs::{config_dir, home_dir};
use git2::{build::RepoBuilder, Cred, FetchOptions, RemoteCallbacks};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    fs::{canonicalize, read_dir, read_to_string},
    os::unix::fs::symlink,
    path::PathBuf,
};

pub async fn clone(url: String, path: &PathBuf) {
    let mut builder = RepoBuilder::new();
    let mut callbacks = RemoteCallbacks::new();
    let mut fetch_options = FetchOptions::new();

    // ssh
    if url.starts_with("git@") {
        callbacks.credentials(|_, _, _| {
            let creds =
                Cred::ssh_key_from_agent("git").expect("Could not create credentials object");
            return Ok(creds);
        });
        fetch_options.remote_callbacks(callbacks);
    } else {
        fetch_options.remote_callbacks(callbacks);
    }

    builder.fetch_options(fetch_options);
    builder
        .clone(&url, path.as_path())
        .expect("failed to clone directory");
}

pub async fn sync_config(path: PathBuf) {
    let config_dir = match config_dir() {
        Some(config) => config,
        None => panic!("unable to resolve xdgconfig direcotry"),
    };
    let files = read_dir(path).expect("unable to read given path");
    for file in files {
        match file {
            Err(e) => {
                eprintln!("{e}");
            }
            Ok(file) => {
                let file_path = file.path();
                let filename = file.file_name();
                let target = &config_dir.join(&filename);
                match symlink(&file_path, target) {
                    Ok(_) => {}
                    Err(e) => {
                        if e.to_string() != *"File exists (os error 17)" {
                            eprintln!("{e}")
                        } else {
                            if target.is_symlink() {
                                if canonicalize(target).unwrap()
                                    != canonicalize(&file_path).unwrap()
                                {
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

pub async fn sync(path: &PathBuf) {
    let home_dir = match home_dir() {
        Some(home) => home,
        None => panic!("unable to resolve home direcotry"),
    };
    let ignore_file_path = path.join(PathBuf::from(".foxignore"));
    let mut ignores = String::from(".git\n.github\n");
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
            Err(e) => {
                eprintln!("{e}");
            }
            Ok(file) => {
                let file_path = file.path();
                let file_as_string = file_path.as_os_str().to_string_lossy();
                if !ignore_path
                    .par_iter()
                    .any(|&i| file_as_string.contains(i) && !i.is_empty())
                    && file_path.is_dir()
                {
                    let inner_paths =
                        read_dir(&file_path).expect("unable to read inner direcotries");
                    for inner_file in inner_paths {
                        match inner_file {
                            Err(e) => eprintln!("{e}"),
                            Ok(inner_file) => {
                                let filename = inner_file.file_name();
                                let inner_file = inner_file.path();
                                if inner_file.as_os_str().to_string_lossy().contains(".config") {
                                    sync_config(inner_file).await;
                                } else {
                                    let target = &home_dir.join(filename);
                                    match symlink(inner_file, &target) {
                                        Ok(_) => {}
                                        Err(e) => {
                                            if e.to_string() != *"File exists (os error 17)" {
                                                eprintln!("{e}")
                                            } else {
                                                if target.is_symlink() {
                                                    if canonicalize(target).unwrap()
                                                        != canonicalize(&file_path).unwrap()
                                                    {
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
}
