use crate::container_config::ContainerConfig;
use nix::{
    mount::{mount, MsFlags},
    sched::{clone, CloneFlags},
    sys::signal::Signal,
    unistd::Pid,
};
use std::path::PathBuf;
use tracing::info;

const STACK_SIZE: usize = 1024 * 1024;

pub(super) fn clone_process(config: &ContainerConfig) -> eyre::Result<Pid> {
    let mut stack = [0; STACK_SIZE];

    let mut flags = CloneFlags::empty();
    flags.insert(CloneFlags::CLONE_NEWCGROUP);
    flags.insert(CloneFlags::CLONE_NEWIPC);
    flags.insert(CloneFlags::CLONE_NEWNET);
    flags.insert(CloneFlags::CLONE_NEWNS);
    flags.insert(CloneFlags::CLONE_NEWUSER);
    flags.insert(CloneFlags::CLONE_NEWPID);
    flags.insert(CloneFlags::CLONE_NEWUTS);

    Ok(clone(
        Box::new(|| create(config.clone())),
        &mut stack,
        flags,
        Some(Signal::SIGCHLD as i32),
    )?)
}

fn create(config: ContainerConfig) -> isize {
    info!(
        "Running command {} with args {:?}",
        config.path.to_str().expect("Command must be valid UTF-8"),
        config.argv,
    );
    0
}

fn mount_dir(path: &Option<PathBuf>, mount_point: &PathBuf, flags: Vec<MsFlags>) {}
