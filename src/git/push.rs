use super::shared::get_current_branch;
use git2::{Cred, Direction, PushOptions, RemoteCallbacks, Repository};

pub fn git_push(repo: &Repository) -> Result<(), git2::Error> {
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
