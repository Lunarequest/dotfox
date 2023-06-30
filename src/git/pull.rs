use git2::{build::CheckoutBuilder, AnnotatedCommit, Config, Cred, Error, Reference, Repository};
use gpgme::Context;
use std::process::exit;

fn fast_forward(repo: &Repository, lb: &mut Reference, rc: AnnotatedCommit) -> Result<(), Error> {
    let name = match lb.name() {
        Some(s) => s.to_string(),
        None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
    };

    let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
    println!("{}", msg);
    lb.set_target(rc.id(), &msg)?;
    repo.set_head(&name)?;
    repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
    Ok(())
}

pub fn do_fetch<'a>(
    repo: &'a git2::Repository,
    refs: &[&str],
    remote: &'a mut git2::Remote,
) -> Result<git2::AnnotatedCommit<'a>, git2::Error> {
    let mut cb = git2::RemoteCallbacks::new();

    let url = remote
        .url()
        .ok_or(git2::Error::from_str("Unable to get remote url"))?;
    if url.starts_with("git@") {
        cb.credentials(|_, _, _| {
            let creds =
                Cred::ssh_key_from_agent("git").expect("Could not create credentials object");
            Ok(creds)
        });
    }

    // Print out our transfer progress.
    cb.transfer_progress(|stats| {
        if stats.received_objects() == stats.total_objects() {
            print!(
                "Resolving deltas {}/{}\r",
                stats.indexed_deltas(),
                stats.total_deltas()
            );
        } else if stats.total_objects() > 0 {
            print!(
                "Received {}/{} objects ({}) in {} bytes\r",
                stats.received_objects(),
                stats.total_objects(),
                stats.indexed_objects(),
                stats.received_bytes()
            );
        }
        true
    });

    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(cb);
    // Always fetch all tags.
    // Perform a download and also update tips
    fo.download_tags(git2::AutotagOption::All);
    println!("Fetching {} for repo", remote.name().unwrap());
    remote.fetch(refs, Some(&mut fo), None)?;

    // If there are local objects (we got a thin pack), then tell the user
    // how many objects we saved from having to cross the network.
    let stats = remote.stats();
    if stats.local_objects() > 0 {
        println!(
            "\rReceived {}/{} objects in {} bytes (used {} local \
             objects)",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes(),
            stats.local_objects()
        );
    } else {
        println!(
            "\rReceived {}/{} objects in {} bytes",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes()
        );
    }

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    repo.reference_to_annotated_commit(&fetch_head)
}

fn normal_merge(
    repo: &Repository,
    local: &AnnotatedCommit,
    remote: &AnnotatedCommit,
) -> Result<(), Error> {
    let config = Config::open_default()?;
    let signing_key = config.get_string("user.signingkey");
    let local_tree = repo.find_commit(local.id())?.tree()?;
    let remote_tree = repo.find_commit(remote.id())?.tree()?;
    let ancestor = repo
        .find_commit(repo.merge_base(local.id(), remote.id())?)?
        .tree()?;
    let mut idx = repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;
    if idx.has_conflicts() {
        eprintln!("Merge conflicts detected...");
        repo.checkout_index(Some(&mut idx), None)?;
        return Ok(());
    }

    let result_tree = repo.find_tree(idx.write_tree_to(repo)?)?;
    let msg = format!("Merge: {} into {}", remote.id(), local.id());

    let sig = repo.signature()?;
    let local_commit = repo.find_commit(local.id())?;
    let remote_commit = repo.find_commit(remote.id())?;

    match signing_key {
        Ok(key) => {
            let commit_as_string = String::from_utf8_lossy(&repo.commit_create_buffer(
                &sig,
                &sig,
                &msg,
                &result_tree,
                &[&local_commit, &remote_commit],
            )?)
            .to_string();

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
        Err(_) => {
            let _merge_commit = repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                &msg,
                &result_tree,
                &[&local_commit, &remote_commit],
            )?;
        }
    }
    repo.checkout_head(None)?;

    Ok(())
}

pub fn do_merge<'a>(
    repo: &'a Repository,
    remote_branch: &str,
    fetch_commit: AnnotatedCommit<'a>,
) -> Result<(), git2::Error> {
    // 1. do a merge analysis
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    // 2. Do the appropriate merge
    if analysis.0.is_fast_forward() {
        println!("Doing a fast forward");
        // do a fast forward
        let refname = format!("refs/heads/{}", remote_branch);
        match repo.find_reference(&refname) {
            Ok(mut r) => {
                fast_forward(repo, &mut r, fetch_commit)?;
            }
            Err(_) => {
                // The branch doesn't exist so just set the reference to the
                // commit directly. Usually this is because you are pulling
                // into an empty repository.
                repo.reference(
                    &refname,
                    fetch_commit.id(),
                    true,
                    &format!("Setting {} to {}", remote_branch, fetch_commit.id()),
                )?;
                repo.set_head(&refname)?;
                repo.checkout_head(Some(
                    git2::build::CheckoutBuilder::default()
                        .allow_conflicts(true)
                        .conflict_style_merge(true)
                        .force(),
                ))?;
            }
        };
    } else if analysis.0.is_normal() {
        // do a normal merge
        let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
        normal_merge(repo, &head_commit, &fetch_commit)?;
    } else {
        println!("Nothing to do...");
    }
    Ok(())
}
