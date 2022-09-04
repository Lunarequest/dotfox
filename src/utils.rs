use dirs::home_dir;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

pub async fn sync(path: PathBuf) {
    let home_dir = match home_dir() {
        Some(home) => home,
        None => panic!("unable to resolve home direcotry"),
    };
    let ignore_file_path = path.join(PathBuf::from(".foxignore"));
    let mut ignores = String::from(".git\n.github\n");
    if ignore_file_path.exists() && ignore_file_path.is_file() {
        ignores.push_str(
            fs::read_to_string(ignore_file_path)
                .expect("unable to read .foxignore file")
                .as_str(),
        );
    }
    let ignore_path = ignores.split('\n').collect::<Vec<&str>>();
    let files = fs::read_dir(path).expect("unable to read given path");
    for file in files {
        match file {
            Err(e) => {
                eprintln!("{e}");
            }
            Ok(file) => {
                let file_path = file.path();
                let file_as_string = file_path.as_os_str().to_string_lossy();
                if !ignore_path
                    .par_iter()
                    .any(|&i| file_as_string.contains(i) && !i.is_empty())
                    && file_path.is_dir()
                {
                    let inner_paths =
                        fs::read_dir(file_path).expect("unable to read inner direcotries");
                    for inner_file in inner_paths {
                        match inner_file {
                            Err(e) => eprintln!("{e}"),
                            Ok(inner_file) => {
                                let inner_file = inner_file.path();
                                match symlink(inner_file, &home_dir) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        if e.to_string() != *"File exists (os error 17)" {
                                            eprintln!("{e}")
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
