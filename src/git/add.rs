#[cfg(debug_assertions)]
use crate::utils::print_debug;
use crate::utils::print_error;
use git2::{IndexAddOption, Repository};
use std::process::exit;

pub fn git_add(repo: &Repository) {
    let mut index = match repo.index() {
        Ok(index) => index,
        Err(_e) => {
            #[cfg(debug_assertions)]
            print_debug(_e.to_string());

            print_error("failed to get image".to_string());
            exit(9);
        }
    };

    match index.add_all(["."].into_iter(), IndexAddOption::DEFAULT, None) {
        Ok(_) => (),
        Err(_e) => {
            #[cfg(debug_assertions)]
            print_debug(_e.to_string());

            print_error("failed to add files to repo".to_string());
            exit(1);
        }
    }
    match index.write() {
        Ok(_) => {}
        Err(_e) => {
            #[cfg(debug_assertions)]
            print_debug(_e.to_string());

            print_error("failed to add files to repo".to_string());
            exit(1);
        }
    }
}
