use nix::unistd::Uid;
use std::{ffi::CString, path::PathBuf};

#[derive(Debug, Clone)]
pub(super) struct ContainerConfig {
    pub path: CString,
    pub argv: Vec<CString>,
    pub uid: Uid,
    pub mount_dir: PathBuf,
}

impl ContainerConfig {
    pub(super) fn new(
        command: &str,
        uid: u32,
        mount_dir: PathBuf,
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
        })
    }
}
