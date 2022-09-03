use crate::{child, cli::Args, container_config::ContainerConfig, ipc};
use nix::{sys::wait::waitpid, unistd::Pid};
use std::{os::unix::io::RawFd, path::PathBuf};
use tracing::debug;

#[derive(Debug)]
pub(super) struct Container {
    config: ContainerConfig,
    child_pid: Option<Pid>,
    socketpair: (RawFd, RawFd),
}

impl Container {
    pub(super) fn new(command: &str, uid: u32, mount_dir: PathBuf) -> eyre::Result<Self> {
        let config = ContainerConfig::new(command, uid, mount_dir)?;
        let socketpair = ipc::create_socketpair()?;

        Ok(Self {
            config,
            child_pid: None,
            socketpair,
        })
    }

    pub(super) fn create(&mut self) -> eyre::Result<()> {
        let child_pid = child::clone_process(&self.config, self.socketpair.1)?;
        self.child_pid = Some(child_pid);

        debug!("Created container with child PID {child_pid}");
        Ok(())
    }

    pub(super) fn wait_for_child(&mut self) -> eyre::Result<()> {
        if let Some(child_pid) = self.child_pid {
            debug!("Waiting for child PID {child_pid} to finish");
            let wait_status = waitpid(child_pid, None)?;
        }

        Ok(())
    }

    pub(super) fn destroy(&mut self) -> eyre::Result<()> {
        debug!("Destroyed container");
        Ok(())
    }
}

pub fn start(
    Args {
        command,
        uid,
        mount_dir,
    }: Args,
) -> eyre::Result<()> {
    let mut container = Container::new(&command, uid, mount_dir)?;
    if let Err(err) = container.create() {
        debug!("Error creating container: {err}");

        // TODO: Combine the destroy error with the create error
        container.destroy()?;
        return Err(err);
    }

    container.wait_for_child()?;

    debug!("Cleaning up container...");
    container.destroy()?;
    Ok(())
}
