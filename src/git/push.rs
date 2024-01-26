use anyhow::{Context, Result};
use git2::{Config, PushOptions, RemoteCallbacks, Repository};
use git2_credentials::CredentialHandler;

pub fn git_push(repo: &Repository) -> Result<()> {
    let head = repo.head()?.resolve()?;
    let config = Config::open_default().context("failed to open gitconfig")?;
    let mut cred_handler = CredentialHandler::new(config);
    let mut callbacks = RemoteCallbacks::new();

    callbacks.credentials(move |url, username, allowed_types| {
        cred_handler.try_next_credential(url, username, allowed_types)
    });
    let mut remote = repo
        .find_remote("origin")
        .map_err(|_| git2::Error::from_str("failed to resolve remote origin"))?;

    let mut push_options = PushOptions::new();

    remote.push(
        &[&format!("refs/heads/{}", head.shorthand().unwrap())],
        Some(&mut push_options),
    )?;
    Ok(())
}
