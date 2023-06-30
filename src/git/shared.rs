use crate::utils::print_error;
use git2::Repository;
use std::process::exit;

//exit code 2 means that it was not able to resolve the branch name
pub fn get_current_branch(repo: &Repository) -> Result<String, git2::Error> {
    let head = repo.head()?;
    if head.is_branch() {
        let name = match head.name() {
            Some(name) => name,
            None => {
                print_error("failed to resolve branch name".to_string());
                exit(2);
            }
        };
        Ok(name.to_string())
    } else {
        print_error("Not on a valid git branch".to_string());
        exit(9);
    }
}
