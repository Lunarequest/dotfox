use anyhow::{anyhow, Result};
use platform_info::{PlatformInfo, PlatformInfoAPI, UNameAPI};
use serde::Deserialize;
use std::{
    env::consts::{ARCH, OS},
    path::PathBuf,
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub config: Vec<Programs>,
}

#[derive(Debug, Deserialize)]
pub struct Programs {
    os: Option<String>,
    hostname: Option<Hostname>,
    folder: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Hostname {
    Single(String),
    Multiple(Vec<String>),
}

impl Config {
    pub fn folders(self) -> Result<Vec<PathBuf>> {
        let sys = match PlatformInfo::new() {
            Ok(sys) => sys,
            Err(e) => return Err(anyhow!(e.to_string())),
        };
        let current_hostname = sys.nodename().to_string_lossy();
        let mut folders: Vec<PathBuf> = vec![];
        let current_os = format!("{OS}-{ARCH}");

        for program in self.config {
            if program.os.is_none() && program.hostname.is_none() {
                folders.append(&mut vec![program.folder]);
            } else if program.os.is_none() {
                let targethost = program.hostname;
                if let Some(target) = targethost {
                    match target {
                        Hostname::Single(host) => {
                            if host == current_hostname {
                                folders.append(&mut vec![program.folder]);
                            }
                        }
                        Hostname::Multiple(hosts) => {
                            if hosts.contains(&current_hostname.to_string()) {
                                folders.append(&mut vec![program.folder]);
                            }
                        }
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
                        if current_os == targetos {
                            match target {
                                Hostname::Single(host) => {
                                    if host == current_hostname {
                                        folders.append(&mut vec![program.folder]);
                                    }
                                }
                                Hostname::Multiple(hosts) => {
                                    if hosts.contains(&current_hostname.to_string()) {
                                        folders.append(&mut vec![program.folder]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        folders.dedup();
        Ok(folders)
    }
}
