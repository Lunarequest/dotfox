use dirs::{config_dir, home_dir};
use git2::{
    build::RepoBuilder, Commit, Config, Cred, Direction, FetchOptions, IndexAddOption, ObjectType,
    PushOptions, RemoteCallbacks, Repository, Signature, StatusOptions,
};
use gpgme::Context;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    env::set_current_dir,
    fs::{canonicalize, read_dir, read_to_string},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::exit,
};

fn find_last_commit(repo: &Repository) -> Result<Commit, git2::Error> {
    let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
    obj.into_commit()
        .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}
//exit code 2 manes that it was not able to resolve the branch name
fn get_current_branch(repo: &Repository) -> Result<String, git2::Error> {
    let head = repo.head()?;
    if head.is_branch() {
        let name = match head.name() {
            Some(name) => name,
            None => {
                eprintln!("failed to resolve branch name");
                exit(2)
            }
        };
        Ok(name.to_string())
    } else {
        eprintln!("Not on a valid git branch");
        exit(9);
    }
}
fn git_push(repo: &Repository) -> Result<(), git2::Error> {
    let mut callbacks = RemoteCallbacks::new();
    let mut push_opts = PushOptions::new();
    let mut remote = repo
        .find_remote("origin")
        .map_err(|_| git2::Error::from_str("failed to resolve remote origin"))?;
    let branch = get_current_branch(repo)?;
    let url = remote
        .url()
        .ok_or(git2::Error::from_str("Unable to get remote url"))?;
    if url.starts_with("git@") {
        callbacks.credentials(|_, _, _| {
            let creds =
                Cred::ssh_key_from_agent("git").expect("Could not create credentials object");
            Ok(creds)
        });

        //remote.connect(Direction::Push)?;
        push_opts.remote_callbacks(callbacks);
        remote.push(&[&branch], Some(&mut push_opts))
    } else {
        remote.connect(Direction::Push)?;
        remote.push(&[&branch], None)
    }
}

fn git_add(repo: &Repository) {
    let mut index = match repo.index() {
        Ok(index) => index,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("failed to get image");
            exit(9);
        }
    };

    match index.add_all(["."].into_iter(), IndexAddOption::DEFAULT, None) {
        Ok(_) => (),
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("failed to add files to repo");
            exit(1);
        }
    }
    match index.write() {
        Ok(_) => {}
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("failed to add files to repo");
            exit(1);
        }
    }
}

fn sign_commit_or_regular(repo: &Repository, message: &str) {
    let config = match Config::open_default() {
        Ok(conf) => conf,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("Unable to open .gitconfig");
            exit(3);
        }
    };

    let name = match config.get_string("user.name") {
        Ok(name) => name,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("Missing name from git field, I need you to tell me who you are using git");
            exit(4);
        }
    };
    let email = match config.get_string("user.email") {
        Ok(email) => email,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("Missing email from git field, I need you to tell me who you are using git");
            exit(4);
        }
    };
    let signing_key = config.get_string("user.signingkey");

    let mut index = repo.index().expect("Unable to open index");
    let oid = match index.write_tree() {
        Ok(oid) => oid,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("failed to write tree");
            exit(5);
        }
    };
    let signature = match Signature::now(&name, &email) {
        Ok(sig) => sig,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("failed to create signature");
            exit(6);
        }
    };
    let parent_commit = match find_last_commit(repo) {
        Ok(parent) => parent,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("failed to find parent commit");
            exit(7);
        }
    };
    let tree = match repo.find_tree(oid) {
        Ok(oid) => oid,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("failed to find commit in tree");
            exit(8);
        }
    };

    match signing_key {
        Err(_) => {
            match repo.commit(
                Some("HEAD"), //  point HEAD to our new commit
                &signature,   // author
                &signature,   // committer
                message,      // commit message
                &tree,        // tree
                &[&parent_commit],
            ) {
                Ok(_) => {}
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    eprintln!("{_e}");

                    println!("failed to commit");
                    exit(9);
                }
            }
        }
        Ok(key) => {
            let commit_as_string = match repo.commit_create_buffer(
                &signature,
                &signature,
                message,
                &tree,
                &[&parent_commit],
            ) {
                Ok(commit) => String::from_utf8_lossy(&commit).to_string(),
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    eprintln!("{_e}");

                    println!("failed to create buffer commit");
                    exit(9);
                }
            };

            let mut ctx = match Context::from_protocol(gpgme::Protocol::OpenPgp) {
                Ok(ctx) => ctx,
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    eprintln!("{_e}");

                    println!("Openpgp contexted failed to initzalize");
                    exit(10);
                }
            };

            ctx.set_armor(true);
            let gpg_key = match ctx.get_secret_key(&key) {
                Ok(key) => key,
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    eprintln!("{_e}");

                    eprintln!("Secret key for {key} could not be accessed does it exist?");
                    exit(10);
                }
            };

            match ctx.add_signer(&gpg_key) {
                Ok(_) => (),
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    eprintln!("{_e}");

                    eprintln!("could not add key as signer");
                    exit(10);
                }
            };

            let mut output = Vec::new();

            match ctx.sign_detached(commit_as_string.clone(), &mut output) {
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    eprintln!("{_e}");

                    eprintln!("failed to sign commit");
                    exit(1);
                }
                Ok(_) => {
                    let sig = match String::from_utf8(output) {
                        Ok(sig) => sig,
                        Err(_e) => {
                            #[cfg(debug_assertions)]
                            eprintln!("{_e}");

                            eprintln!("Failed to conert signature to string from bytes");
                            exit(1);
                        }
                    };
                    let oid = match repo.commit_signed(&commit_as_string, &sig, None) {
                        Ok(oid) => oid,
                        Err(_e) => {
                            #[cfg(debug_assertions)]
                            eprintln!("{_e}");

                            eprintln!("failed to create signed commit");
                            exit(9);
                        }
                    };
                    let head = repo.head();
                    match head {
                        Ok(mut head) => match head.set_target(oid, "REFLOG_MSG") {
                            Ok(_) => {}
                            Err(_e) => {
                                #[cfg(debug_assertions)]
                                eprintln!("{_e}");

                                eprintln!("failed to point HEAD to latest commit");
                                exit(9);
                            }
                        },
                        Err(_e) => {
                            #[cfg(debug_assertions)]
                            eprintln!("{_e}");

                            eprintln!("failed to get HEAD");
                            exit(9);
                        }
                    }
                }
            }
        }
    }
}

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

    git_add(&repo);

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

    sign_commit_or_regular(&repo, &message);
    match git_push(&repo) {
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
                            let target = match canonicalize(target) {
                                Ok(target) => target,
                                Err(_) => {
                                    #[cfg(debug_assertions)]
                                    eprintln!("{_e}");

                                    eprintln!("failed to canoncalize path");
                                    exit(11);
                                }
                            };
                            let source = match canonicalize(&file_path) {
                                Ok(target) => target,
                                Err(_e) => {
                                    #[cfg(debug_assertions)]
                                    eprintln!("{_e}");

                                    eprintln!("failed to canoncalize path");
                                    exit(11);
                                }
                            };
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
                                                let target = match canonicalize(target) {
                                                    Ok(target) => target,
                                                    Err(_e) => {
                                                        #[cfg(debug_assertions)]
                                                        eprintln!("{_e}");

                                                        eprintln!("failed to canoncalize path");
                                                        exit(11);
                                                    }
                                                };
                                                let source = match canonicalize(&file_path) {
                                                    Ok(target) => target,
                                                    Err(_e) => {
                                                        #[cfg(debug_assertions)]
                                                        eprintln!("{_e}");

                                                        eprintln!("failed to canoncalize path");
                                                        exit(11);
                                                    }
                                                };
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
