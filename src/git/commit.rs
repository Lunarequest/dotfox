use anyhow::{anyhow, Context as anyhowContext, Result};
use git2::{Commit, Config, FetchOptions, ObjectType, RemoteCallbacks, Repository};
use git2_credentials::CredentialHandler;
use gpgme::Context;

fn find_last_commit(repo: &Repository) -> Result<Commit, git2::Error> {
    let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
    obj.into_commit()
        .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}

pub fn sign_commit_or_regular(repo: &Repository, message: &str) -> Result<()> {
    let config = Config::open_default().context("unable to open git config")?;
    let signing_key = config.get_string("user.signingkey");

    let mut index = repo.index().expect("Unable to open index");
    let oid = index.write_tree()?;
    let signature = repo.signature()?;
    let parent_commit = find_last_commit(repo)?;
    let tree = repo.find_tree(oid)?;

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
                Err(e) => return Err(anyhow!(e.to_string())),
            }
        }
        Ok(key) => {
            let commit_as_string = String::from_utf8_lossy(&repo.commit_create_buffer(
                &signature,
                &signature,
                message,
                &tree,
                &[&parent_commit],
            )?)
            .to_string();

            let mut ctx = Context::from_protocol(gpgme::Protocol::OpenPgp)?;

            ctx.set_armor(true);
            let gpg_key = ctx.get_secret_key(&key)?;

            ctx.add_signer(&gpg_key)?;

            let mut output = Vec::new();

            match ctx.sign_detached(commit_as_string.clone(), &mut output) {
                Err(e) => return Err(anyhow!(e.to_string())),
                Ok(_) => {
                    let sig = String::from_utf8(output)?;
                    let oid = repo.commit_signed(&commit_as_string, &sig, None)?;
                    let mut head = repo.head()?;
                    head.set_target(oid, "REFLOG_MSG")?;
                }
            }
        }
    };
    Ok(())
}

pub fn unsynced_commits(repo: &Repository) -> bool {
    let local_head = match repo.head() {
        Ok(head) => match head.peel_to_commit() {
            Ok(commit) => commit,
            Err(e) => {
                eprintln!("{e}");
                return false;
            }
        },
        Err(e) => {
            eprintln!("{e}");
            return false;
        }
    };
    let config = match git2::Config::open_default() {
        Ok(config) => config,
        Err(_) => {
            eprintln!("Failed to open gitconfig");
            return false;
        }
    };

    let mut fetch_opts = FetchOptions::new();
    let mut callbacks = RemoteCallbacks::new();
    let mut cred_handler = CredentialHandler::new(config);
    callbacks.credentials(move |url, username, allowed_types| {
        cred_handler.try_next_credential(url, username, allowed_types)
    });
    fetch_opts.remote_callbacks(callbacks);

    let mut remote = repo.find_remote("origin").unwrap();
    remote
        .fetch(
            &["refs/heads/*:refs/remotes/origin/*"],
            Some(&mut fetch_opts),
            None,
        )
        .unwrap();

    let remote_head = repo
        .find_reference("refs/remotes/origin/HEAD")
        .unwrap()
        .peel_to_commit()
        .unwrap();
    local_head.id() != remote_head.id()
}
