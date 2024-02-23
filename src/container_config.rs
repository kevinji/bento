use nix::{libc::uid_t, unistd::Uid};
use std::{ffi::CString, path::PathBuf};

#[derive(Debug, Clone)]
pub struct ContainerConfig {
    pub command: String,
    pub argv: Vec<CString>,
    pub uid: Uid,
    pub mount_dir: PathBuf,
    pub hostname: Option<String>,
    pub commands_to_copy: Vec<String>,
}

impl ContainerConfig {
    pub fn new(
        command: String,
        uid: uid_t,
        mount_dir: PathBuf,
        hostname: Option<String>,
        commands_to_copy: Vec<String>,
    ) -> eyre::Result<ContainerConfig> {
        let argv = command
            .split_ascii_whitespace()
            .map(CString::new)
            .collect::<Result<Vec<_>, _>>()?;
        let uid = Uid::from_raw(uid);

        Ok(Self {
            command,
            argv,
            uid,
            mount_dir,
            hostname,
            commands_to_copy,
        })
    }
}
