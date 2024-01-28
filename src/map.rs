use std::path::Path;
use tabled::Tabled;

#[derive(Debug, Tabled)]
pub struct Map {
    source: String,
    target: String,
}

#[derive(Debug, Tabled)]
pub struct VerifyMap {
    source: String,
    target: String,
    tainted: bool,
}

impl Map {
    pub fn new(source: &Path, target: &Path) -> Self {
        Self {
            source: format!("{}", source.display()),
            target: format!("{}", target.display()),
        }
    }
}

impl VerifyMap {
    pub fn new(source: &Path, target: &Path) -> Self {
        Self {
            source: format!("{}", source.display()),
            target: format!("{}", target.display()),
            tainted: false,
        }
    }

    pub fn taint(&mut self) {
        self.tainted = true;
    }
}
