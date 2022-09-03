use crate::container_config::ContainerConfig;
use nix::{
    mount::{mount, umount2, MntFlags, MsFlags},
    sched::{clone, CloneFlags},
    sys::signal::Signal,
    unistd::{chdir, pivot_root, Pid},
};
use std::{
    os::unix::{io::RawFd, net::UnixDatagram, prelude::FromRawFd},
    path::PathBuf,
};
use tracing::info;

const STACK_SIZE: usize = 1024 * 1024;

pub(super) fn clone_process(config: &ContainerConfig, fd: RawFd) -> eyre::Result<Pid> {
    let mut stack = [0; STACK_SIZE];

    let mut flags = CloneFlags::empty();
    flags.insert(CloneFlags::CLONE_FILES); // TODO: Is this needed?
    flags.insert(CloneFlags::CLONE_NEWCGROUP);
    flags.insert(CloneFlags::CLONE_NEWIPC);
    flags.insert(CloneFlags::CLONE_NEWNET);
    flags.insert(CloneFlags::CLONE_NEWNS);
    flags.insert(CloneFlags::CLONE_NEWUSER);
    flags.insert(CloneFlags::CLONE_NEWPID);
    flags.insert(CloneFlags::CLONE_NEWUTS);

    Ok(clone(
        Box::new(|| create(config.clone(), fd)),
        &mut stack,
        flags,
        Some(Signal::SIGCHLD as i32),
    )?)
}

fn create(config: ContainerConfig, fd: RawFd) -> isize {
    info!(
        "Running command {} with args {:?}",
        config.path.to_str().expect("Command must be valid UTF-8"),
        config.argv,
    );

    // TODO: Possibly use OwnedFd
    let socket = unsafe { UnixDatagram::from_raw_fd(fd) };

    // TODO: Pass error to socket?
    if let Err(err) = switch_root(&config.mount_dir) {
        return 1;
    };

    0
}

fn mount_dir(
    path: &Option<PathBuf>,
    mount_point: &PathBuf,
    flags: Vec<MsFlags>,
) -> eyre::Result<()> {
    mount::<_, _, PathBuf, PathBuf>(
        path.as_ref(),
        mount_point,
        None,
        MsFlags::from_iter(flags),
        None,
    )?;
    Ok(())
}

fn switch_root(new_root: &PathBuf) -> eyre::Result<()> {
    // Avoid creating a temporary dir for the old root:
    // https://man7.org/linux/man-pages/man2/pivot_root.2.html
    chdir(new_root)?;
    pivot_root(".", ".")?;
    umount2(".", MntFlags::MNT_DETACH)?;
    Ok(())
}
