use crate::container_config::ContainerConfig;
use nix::{
    mount::{mount, umount2, MntFlags, MsFlags},
    sched::{clone, CloneFlags},
    sys::signal::Signal,
    unistd::{chdir, execve, pivot_root, sethostname, Pid},
};
use std::{
    convert::Infallible,
    ffi::CString,
    os::unix::{io::RawFd, net::UnixDatagram, prelude::FromRawFd},
    path::PathBuf,
};
use tracing::{debug, error, info};

const STACK_SIZE: usize = 1024 * 1024;

pub(super) fn clone_process(config: &ContainerConfig, fd: RawFd) -> eyre::Result<Pid> {
    let mut stack = [0; STACK_SIZE];

    let flags = CloneFlags::from_iter([
        CloneFlags::CLONE_FILES, // TODO: Is this needed?
        CloneFlags::CLONE_NEWCGROUP,
        CloneFlags::CLONE_NEWIPC,
        CloneFlags::CLONE_NEWNET,
        CloneFlags::CLONE_NEWNS,
        CloneFlags::CLONE_NEWPID,
        CloneFlags::CLONE_NEWUTS,
    ]);

    Ok(clone(
        Box::new(|| spawn(config.clone(), fd)),
        &mut stack,
        flags,
        Some(Signal::SIGCHLD as i32),
    )?)
}

fn spawn(config: ContainerConfig, fd: RawFd) -> isize {
    match spawn_with_result(config, fd) {
        Ok(never) => match never {
           // When Rust supports !, remove this branch 
        },
        Err(err) => {
            error!("{}", err);
            1
        }
    }
}

fn spawn_with_result(
    ContainerConfig {
        path,
        argv,
        uid,
        mount_dir,
        hostname,
    }: ContainerConfig,
    fd: RawFd,
) -> eyre::Result<Infallible> {
    if let Some(hostname) = hostname {
        sethostname(&hostname)?;
        debug!("Hostname is now {hostname}");
    }

    let command = path.to_str().expect("Command must be valid UTF-8");
    info!("Running command {command} with args {argv:?}");

    // TODO: Possibly use OwnedFd
    let socket = unsafe { UnixDatagram::from_raw_fd(fd) };

    switch_root(&mount_dir)?;

    Ok(execve::<_, CString>(&path, &argv, &[])?)
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
