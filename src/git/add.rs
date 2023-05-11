use git2::{IndexAddOption, Repository};
use std::process::exit;

pub fn git_add(repo: &Repository) {
    let mut index = match repo.index() {
        Ok(index) => index,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("failed to get image");
            exit(9);
        }
    };

    match index.add_all(["."].into_iter(), IndexAddOption::DEFAULT, None) {
        Ok(_) => (),
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("failed to add files to repo");
            exit(1);
        }
    }
    match index.write() {
        Ok(_) => {}
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("{_e}");

            eprintln!("failed to add files to repo");
            exit(1);
        }
    }
}
