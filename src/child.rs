use crate::{container_config::ContainerConfig, sockets};
use eyre::bail;
use nix::{
    mount::{mount, umount2, MntFlags, MsFlags},
    sched::{clone, unshare, CloneFlags},
    sys::signal::Signal,
    unistd::{
        chdir, execve, pivot_root, setgroups, sethostname, setresgid, setresuid, Gid, Pid, Uid,
    },
};
use std::{
    convert::Infallible,
    ffi::CString,
    os::{
        fd::{FromRawFd, IntoRawFd, RawFd},
        unix::net::UnixDatagram,
    },
    path::PathBuf,
};
use tracing::{debug, error, info, warn};

const STACK_SIZE: usize = 1024 * 1024;

pub fn clone_process(config: &ContainerConfig, socket: UnixDatagram) -> eyre::Result<Pid> {
    let socket_fd = socket.into_raw_fd();
    let cb = Box::new(|| spawn(config.clone(), socket_fd));
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

fn spawn(config: ContainerConfig, socket_fd: RawFd) -> isize {
    match spawn_with_result(config, socket_fd) {
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
        uid,
        mount_dir,
        hostname,
    }: ContainerConfig,
    socket_fd: RawFd,
) -> eyre::Result<Infallible> {
    let socket = unsafe { UnixDatagram::from_raw_fd(socket_fd) };

    if let Some(hostname) = hostname {
        sethostname(&hostname)?;
        debug!("Hostname is now {hostname}");
    }

    switch_root(&mount_dir)?;

    let user_namespace_created = create_user_namespace();
    debug!("User namespace created {user_namespace_created}, sending to container");
    sockets::send_bool(&socket, user_namespace_created)?;

    let uid_and_gid_created = sockets::recv_bool(&socket)?;
    if !uid_and_gid_created {
        bail!("BUG: uid_and_gid_created should never be false");
    }
    set_uid(uid)?;

    let command = path.to_str().expect("Command must be valid UTF-8");
    info!("Running command {command} with args {argv:?}");

    Ok(execve::<_, CString>(&path, &argv, &[])?)
}

fn create_user_namespace() -> bool {
    match unshare(CloneFlags::CLONE_NEWUSER) {
        Ok(()) => {
            debug!("User namespace created");
            true
        }
        Err(err) => {
            warn!("User namespaces not supported: {err}");
            false
        }
    }
}

fn set_uid(uid: Uid) -> eyre::Result<()> {
    debug!("Setting uid as {uid}");

    let gid = Gid::from_raw(uid.as_raw());
    setgroups(&[gid])?;
    setresgid(gid, gid, gid)?;
    setresuid(uid, uid, uid)?;

    Ok(())
}

fn mount_at_path(
    path: Option<&PathBuf>,
    mount_point: &PathBuf,
    flags: Vec<MsFlags>,
) -> eyre::Result<()> {
    debug!("Mounting {mount_point:?} at {path:?} with flags {flags:?}");
    mount::<_, _, PathBuf, PathBuf>(path, mount_point, None, MsFlags::from_iter(flags), None)?;
    Ok(())
}

fn switch_root(new_root: &PathBuf) -> eyre::Result<()> {
    mount_at_path(
        None,
        &PathBuf::from("/"),
        vec![MsFlags::MS_REC, MsFlags::MS_PRIVATE],
    )?;
    mount_at_path(
        Some(new_root),
        new_root,
        vec![MsFlags::MS_BIND, MsFlags::MS_PRIVATE],
    )?;

    // Avoid creating a temporary dir for the old root:
    // https://man7.org/linux/man-pages/man2/pivot_root.2.html
    debug!("Running chdir({new_root})", new_root = new_root.display());
    chdir(new_root)?;

    debug!("Running pivot_root(\".\", \".\")");
    pivot_root(".", ".")?;

    debug!("Unmounting old root");
    umount2(".", MntFlags::MNT_DETACH)?;
    Ok(())
}
