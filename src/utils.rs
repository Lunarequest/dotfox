use dirs::{config_dir, home_dir};
use git2::{
    build::RepoBuilder, Commit, Cred, Direction, FetchOptions, ObjectType, PushOptions,
    RemoteCallbacks, Repository, Signature,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    env::set_current_dir,
    fs::{canonicalize, read_dir, read_to_string},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
};

fn find_last_commit(repo: &Repository) -> Result<Commit, git2::Error> {
    let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
    obj.into_commit()
        .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}

fn git_push(repo: &Repository) -> Result<(), git2::Error> {
    let mut callbacks = RemoteCallbacks::new();
    let mut push_opts = PushOptions::new();
    let mut remote = match repo.find_remote("origin") {
        Ok(r) => r,
        Err(_) => panic!("Unable to find remote origin"),
    };
    let url = remote.url().unwrap();
    println!("{url}");
    if url.starts_with("git@") {
        callbacks.credentials(|_, _, _| {
            let creds =
                Cred::ssh_key_from_agent("git").expect("Could not create credentials object");
            Ok(creds)
        });

        //remote.connect(Direction::Push)?;
        push_opts.remote_callbacks(callbacks);
        remote.push(
            &["refs/heads/mistress:refs/heads/mistress"],
            Some(&mut push_opts),
        )
    } else {
        remote.connect(Direction::Push)?;
        remote.push(&["refs/heads/mistress:refs/heads/mistress"], None)
    }
}

pub async fn push(path: &Path, message: String) {
    let repo = match Repository::open(path) {
        Ok(repo) => repo,
        Err(e) => {
            panic!(
                "unable to open repo {} is it really a git repo?\n{}",
                path.display(),
                e
            );
        }
    };

    set_current_dir(path).unwrap();

    let add = Command::new("git").arg("add").arg(".").status().unwrap();
    if !add.success() {
        panic!("git add failed");
    }

    let mut index = repo.index().expect("Unable to open index");
    let oid = index.write_tree().unwrap();
    println!("{:#?}", oid);
    let signature = Signature::now("Lunarequest", "nullrequest@vivaldi.net").unwrap();
    let parent_commit = find_last_commit(&repo).unwrap();
    let tree = repo.find_tree(oid).unwrap();
    repo.commit(
        Some("HEAD"), //  point HEAD to our new commit
        &signature,   // author
        &signature,   // committer
        &message,     // commit message
        &tree,        // tree
        &[&parent_commit],
    )
    .unwrap(); // parents
    git_push(&repo).unwrap();
}

pub async fn clone(url: String, path: &Path) {
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
                        } else if target.is_symlink()
                            && canonicalize(target).unwrap() != canonicalize(&file_path).unwrap()
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

pub async fn sync(path: &PathBuf) {
    let home_dir = match home_dir() {
        Some(home) => home,
        None => panic!("unable to resolve home direcotry"),
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
            Err(e) => {
                eprintln!("{e}");
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
                            Err(e) => eprintln!("{e}"),
                            Ok(inner_file) => {
                                let filename = inner_file.file_name();
                                let inner_file = inner_file.path();
                                if inner_file.as_os_str().to_string_lossy().contains(".config") {
                                    sync_config(inner_file).await;
                                } else {
                                    let target = &home_dir.join(filename);
                                    match symlink(&inner_file, target) {
                                        Ok(_) => {}
                                        Err(e) => {
                                            if e.to_string() != *"File exists (os error 17)" {
                                                eprintln!("{e}")
                                            } else if target.is_symlink()
                                                && canonicalize(target).unwrap()
                                                    != canonicalize(&inner_file).unwrap()
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
