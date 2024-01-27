use crate::utils::{print_error, print_info};
use anyhow::{anyhow, Context, Result};
use git2::{build::CheckoutBuilder, AnnotatedCommit, Config, Reference, Repository};
use git2_credentials::CredentialHandler;

fn fast_forward(repo: &Repository, lb: &mut Reference, rc: AnnotatedCommit) -> Result<()> {
    let name = match lb.name() {
        Some(s) => s.to_string(),
        None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
    };

    let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
    print_info(msg.clone());
    lb.set_target(rc.id(), &msg)?;
    repo.set_head(&name)?;
    repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
    Ok(())
}

pub fn do_fetch<'a>(
    repo: &'a git2::Repository,
    refs: &[&str],
    remote: &'a mut git2::Remote,
) -> Result<git2::AnnotatedCommit<'a>> {
    let mut cb = git2::RemoteCallbacks::new();
    let config = Config::open_default().context("failed to open gitconfig")?;
    let mut ch = CredentialHandler::new(config);

    cb.credentials(move |url, username, allowed_types| {
        ch.try_next_credential(url, username, allowed_types)
    });
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
    print_info(format!("Fetching {} for repo", remote.name().unwrap()));
    remote.fetch(refs, Some(&mut fo), None)?;

    // If there are local objects (we got a thin pack), then tell the user
    // how many objects we saved from having to cross the network.
    let stats = remote.stats();
    if stats.local_objects() > 0 {
        print_info(format!(
            "\rReceived {}/{} objects in {} bytes (used {} local \
             objects)",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes(),
            stats.local_objects()
        ));
    } else {
        print_info(format!(
            "\rReceived {}/{} objects in {} bytes",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes()
        ));
    }

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let commit = repo.reference_to_annotated_commit(&fetch_head)?;
    Ok(commit)
}

fn normal_merge(
    repo: &Repository,
    local: &AnnotatedCommit,
    remote: &AnnotatedCommit,
) -> Result<()> {
    let config = Config::open_default()?;
    let signing_key = config.get_string("user.signingkey");
    let local_tree = repo.find_commit(local.id())?.tree()?;
    let remote_tree = repo.find_commit(remote.id())?.tree()?;
    let ancestor = repo
        .find_commit(repo.merge_base(local.id(), remote.id())?)?
        .tree()?;
    let mut idx = repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;
    if idx.has_conflicts() {
        print_error("Merge conflicts detected...".to_string());
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

            let mut ctx = gpgme::Context::from_protocol(gpgme::Protocol::OpenPgp)?;
            ctx.set_armor(true);
            let gpg_key = ctx.get_secret_key(&key)?;

            ctx.add_signer(&gpg_key)?;
            let mut output = Vec::new();
            match ctx.sign_detached(commit_as_string.clone(), &mut output) {
                Err(_e) => {
                    return Err(anyhow!("failed to sign commit"));
                }
                Ok(_) => {
                    let sig = String::from_utf8(output)?;
                    let oid = repo.commit_signed(&commit_as_string, &sig, None)?;
                    let mut head = repo.head()?;
                    head.set_target(oid, "REFLOG_MSG")?;
                }
            };
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
) -> Result<()> {
    // 1. do a merge analysis
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    // 2. Do the appropriate merge
    if analysis.0.is_fast_forward() {
        print_info("Doing a fast forward".to_string());
        // do a fast forward
        match repo.find_reference(remote_branch) {
            Ok(mut r) => {
                fast_forward(repo, &mut r, fetch_commit)?;
            }
            Err(_) => {
                // The branch doesn't exist so just set the reference to the
                // commit directly. Usually this is because you are pulling
                // into an empty repository.
                repo.reference(
                    remote_branch,
                    fetch_commit.id(),
                    true,
                    &format!("Setting {} to {}", remote_branch, fetch_commit.id()),
                )?;
                repo.set_head(remote_branch)?;
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
        print_info("Nothing to do...".to_string());
    }
    Ok(())
}
