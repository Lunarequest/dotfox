use std::path::PathBuf;
use tabled::Tabled;

#[derive(Debug, Tabled)]
pub struct Map {
    source: String,
    target: String,
}

impl Map {
    pub fn new(source: &PathBuf, target: &PathBuf) -> Self {
        Self {
            source: format!("{}", source.display()),
            target: format!("{}", target.display()),
        }
    }
}
