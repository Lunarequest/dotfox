use anyhow::{anyhow, Result};
use git2::Repository;

//exit code 2 means that it was not able to resolve the branch name
pub fn get_current_branch(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    if head.is_branch() {
        let name = match head.name() {
            Some(name) => name,
            None => {
                return Err(anyhow!("failed to resolve branch name"));
            }
        };
        Ok(name.to_string())
    } else {
        Err(anyhow!("Not on a valid git branch"))
    }
}
