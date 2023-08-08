use anyhow::{Context, Result};
use git2::{IndexAddOption, Repository};

pub fn git_add(repo: &Repository) -> Result<()> {
    let mut index = repo.index().context("Failed to get index of repo")?;

    index
        .add_all(["."].into_iter(), IndexAddOption::DEFAULT, None)
        .context("Failed to add files to repo")?;
    index.write()?;
    Ok(())
}
