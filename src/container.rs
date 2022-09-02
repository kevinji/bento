use crate::{child::clone_process, cli::Args, container_config::ContainerConfig};
use nix::{sys::wait::waitpid, unistd::Pid};
use tracing::debug;

#[derive(Debug)]
pub(super) struct Container {
    config: ContainerConfig,
    child_pid: Option<Pid>,
}

impl Container {
    pub(super) fn new(args: Args) -> eyre::Result<Self> {
        let config = ContainerConfig::new(&args.command, args.uid, args.mount_dir)?;
        Ok(Self {
            config,
            child_pid: None,
        })
    }

    pub(super) fn create(&mut self) -> eyre::Result<()> {
        let child_pid = clone_process(&self.config)?;
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

pub fn start(args: Args) -> eyre::Result<()> {
    let mut container = Container::new(args)?;
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
