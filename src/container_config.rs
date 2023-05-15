use nix::{libc::uid_t, unistd::Uid};
use std::{ffi::CString, path::PathBuf};

#[derive(Debug, Clone)]
pub(super) struct ContainerConfig {
    pub path: CString,
    pub argv: Vec<CString>,
    pub uid: Uid,
    pub mount_dir: PathBuf,
    pub hostname: Option<String>,
}

impl ContainerConfig {
    pub(super) fn new(
        command: &str,
        uid: uid_t,
        mount_dir: PathBuf,
        hostname: Option<String>,
    ) -> eyre::Result<ContainerConfig> {
        let argv = command
            .split_ascii_whitespace()
            .map(CString::new)
            .collect::<Result<Vec<_>, _>>()?;
        let path = argv[0].clone();
        let uid = Uid::from_raw(uid);

        Ok(Self {
            path,
            argv,
            uid,
            mount_dir,
            hostname,
        })
    }
}
