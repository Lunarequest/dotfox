use std::path::Path;
use tabled::Tabled;

#[derive(Debug, Tabled)]
pub struct Map {
    source: String,
    target: String,
}

impl Map {
    pub fn new(source: &Path, target: &Path) -> Self {
        Self {
            source: format!("{}", source.display()),
            target: format!("{}", target.display()),
        }
    }
}
