use serde::Deserialize;
use std::{
    env::consts::{ARCH, OS},
    path::PathBuf,
};
use sysinfo::{System, SystemExt};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub config: Vec<Programs>,
}

#[derive(Debug, Deserialize)]
pub struct Programs {
    os: Option<String>,
    hostname: Option<String>,
    folder: PathBuf,
}

impl Config {
    pub fn folders(self) -> Vec<PathBuf> {
        let mut sys = System::new_all();
        sys.refresh_all();
        let current_hostname = sys.host_name().unwrap();
        let mut folders: Vec<PathBuf> = vec![];
        let current_os = format!("{OS}-{ARCH}");

        for program in self.config {
            if program.os.is_none() && program.hostname.is_none() {
                folders.append(&mut vec![program.folder]);
            } else if program.os.is_none() {
                let targethost = program.hostname;
                if let Some(target) = targethost {
                    if current_hostname == target {
                        folders.append(&mut vec![program.folder]);
                    }
                }
            } else if program.hostname.is_none() {
                let targetos = program.os;
                if let Some(targetos) = targetos {
                    if current_os == targetos {
                        folders.append(&mut vec![program.folder]);
                    }
                }
            } else {
                let targetos = program.os;
                let targethost = program.hostname;
                if let Some(targetos) = targetos {
                    if let Some(target) = targethost {
                        if current_hostname == target && current_os == targetos {
                            folders.append(&mut vec![program.folder]);
                        }
                    }
                }
            }
        }
        folders.dedup();
        folders
    }
}
