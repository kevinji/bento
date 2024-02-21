use crate::container_config::ContainerConfig;
use nix::{
    mount::{mount, umount2, MntFlags, MsFlags},
    sched::{clone, CloneFlags},
    sys::signal::Signal,
    unistd::{chdir, execve, pivot_root, sethostname, Pid},
};
use std::{convert::Infallible, ffi::CString, os::unix::net::UnixDatagram, path::PathBuf};
use tracing::{debug, error, info};

const STACK_SIZE: usize = 1024 * 1024;

pub fn clone_process(config: &ContainerConfig, socket: UnixDatagram) -> eyre::Result<Pid> {
    let cb = Box::new(|| spawn(config.clone(), &socket));
    let mut stack = [0; STACK_SIZE];

    let flags = CloneFlags::from_iter([
        CloneFlags::CLONE_FILES, // fd of socket must be shared to child
        CloneFlags::CLONE_NEWCGROUP,
        CloneFlags::CLONE_NEWIPC,
        CloneFlags::CLONE_NEWNET,
        CloneFlags::CLONE_NEWNS,
        CloneFlags::CLONE_NEWPID,
        CloneFlags::CLONE_NEWUTS,
    ]);

    let signal = Signal::SIGCHLD as i32;

    Ok(unsafe { clone(cb, &mut stack, flags, Some(signal)) }?)
}

fn spawn(config: ContainerConfig, socket: &UnixDatagram) -> isize {
    match spawn_with_result(config, socket) {
        Ok(infallible) => match infallible {
           // When Rust supports !, remove this branch 
        },
        Err(err) => {
            error!("{err}");
            1
        }
    }
}

fn spawn_with_result(
    ContainerConfig {
        path,
        argv,
        uid: _,
        mount_dir,
        hostname,
    }: ContainerConfig,
    socket: &UnixDatagram,
) -> eyre::Result<Infallible> {
    if let Some(hostname) = hostname {
        sethostname(&hostname)?;
        debug!("Hostname is now {hostname}");
    }

    let command = path.to_str().expect("Command must be valid UTF-8");
    info!("Running command {command} with args {argv:?}");

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
