use git2::{Cred, Direction, PushOptions, RemoteCallbacks, Repository};
use std::process::exit;
//exit code 2 means that it was not able to resolve the branch name
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
