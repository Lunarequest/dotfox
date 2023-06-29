use git2::{Commit, Config, ObjectType, Repository};
use gpgme::Context;
use std::process::exit;

fn find_last_commit(repo: &Repository) -> Result<Commit, git2::Error> {
    let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
    obj.into_commit()
        .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}

pub fn sign_commit_or_regular(repo: &Repository, message: &str) {
    let config = match Config::open_default() {
        Ok(conf) => conf,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("Unable to open .gitconfig");
            exit(3);
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
    let signature = match repo.signature() {
        Ok(sig) => sig,
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
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
