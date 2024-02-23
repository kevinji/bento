use crate::{container_config::ContainerConfig, sockets};
use eyre::bail;
use lddtree::DependencyAnalyzer;
use nix::{
    mount::{mount, umount2, MntFlags, MsFlags},
    sched::{clone, unshare, CloneFlags},
    sys::signal::Signal,
    unistd::{
        chdir, execve, pivot_root, setgroups, sethostname, setresgid, setresuid, Gid, Pid, Uid,
    },
};
use std::{
    collections::HashSet,
    convert::Infallible,
    ffi::CString,
    fs,
    os::{
        fd::{FromRawFd, IntoRawFd, RawFd},
        unix::net::UnixDatagram,
    },
    path::{Path, PathBuf},
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
        command,
        argv,
        uid,
        mount_dir,
        hostname,
        commands_to_copy,
    }: ContainerConfig,
    socket_fd: RawFd,
) -> eyre::Result<Infallible> {
    let socket = unsafe { UnixDatagram::from_raw_fd(socket_fd) };

    if let Some(hostname) = hostname {
        sethostname(&hostname)?;
        debug!("Hostname is now {hostname}");
    }

    mount_roots_and_paths(&mount_dir, &command, commands_to_copy)?;
    switch_root(&mount_dir)?;

    let user_namespace_created = create_user_namespace();
    debug!("User namespace created {user_namespace_created}, sending to container");
    sockets::send_bool(&socket, user_namespace_created)?;

    let uid_and_gid_created = sockets::recv_bool(&socket)?;
    if !uid_and_gid_created {
        bail!("BUG: uid_and_gid_created should never be false");
    }
    set_uid(uid)?;

    info!("Running command {command} with args {argv:?}");
    Ok(execve::<_, CString>(&CString::new(command)?, &argv, &[])?)
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

fn mount_at_path(path: Option<&Path>, mount_point: &Path, flags: Vec<MsFlags>) -> eyre::Result<()> {
    debug!("Mounting {path:?} at {mount_point:?} with flags {flags:?}");
    mount::<_, _, Path, Path>(path, mount_point, None, MsFlags::from_iter(flags), None)?;
    Ok(())
}

fn get_library_paths(command: &str) -> eyre::Result<HashSet<PathBuf>> {
    let dependency_analyzer = DependencyAnalyzer::new("/".into());
    let dependency_tree = dependency_analyzer.analyze(command)?;
    Ok(dependency_tree
        .libraries
        .into_values()
        .map(|library| (library.path))
        .collect())
}

fn mkdir_and_copy(new_root: &Path, paths: &HashSet<PathBuf>) -> eyre::Result<()> {
    for path in paths {
        let stripped_path = path.strip_prefix("/").unwrap_or(path);
        if let Some(new_parent_path) = new_root.join(stripped_path).parent() {
            debug!("Creating dir {}", new_parent_path.display());
            fs::create_dir_all(new_parent_path)?;
        }

        let new_path = new_root.join(stripped_path);
        debug!("Copying file {} to {}", path.display(), new_path.display());
        fs::copy(path, &new_path)?;

        debug!("Setting {} as readonly", new_path.display());
        let mut perms = fs::metadata(&new_path)?.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&new_path, perms)?;
    }
    Ok(())
}

fn mount_roots_and_paths(
    new_root: &Path,
    command: &str,
    mut commands_to_copy: Vec<String>,
) -> eyre::Result<()> {
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

    commands_to_copy.push(command.to_owned());
    let mut command_and_lib_paths = commands_to_copy
        .iter()
        .map(PathBuf::from)
        .collect::<HashSet<_>>();
    for command in commands_to_copy {
        command_and_lib_paths.extend(get_library_paths(&command)?);
    }
    debug!("Commands and libraries: {command_and_lib_paths:?}");

    mkdir_and_copy(new_root, &command_and_lib_paths)?;

    Ok(())
}

fn switch_root(new_root: &Path) -> eyre::Result<()> {
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
