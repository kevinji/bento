use crate::{child, cli::Args, container_config::ContainerConfig, ipc};
use cgroups_rs::{cgroup::Cgroup, cgroup_builder::CgroupBuilder};
use nix::{sys::wait::waitpid, unistd::Pid};
use std::{os::unix::io::RawFd, path::PathBuf};
use tracing::debug;

#[derive(Debug)]
pub(super) struct Container {
    config: ContainerConfig,
    child_pid: Option<Pid>,
    socketpair: (RawFd, RawFd),
    cgroup: Cgroup,
}

impl Container {
    pub(super) fn new(
        command: &str,
        uid: u32,
        mount_dir: PathBuf,
        hostname: Option<String>,
    ) -> eyre::Result<Self> {
        let config = ContainerConfig::new(command, uid, mount_dir, hostname)?;
        let socketpair = ipc::create_socketpair()?;
        let cgroup = build_cgroup("bento"); // TODO: Update cgroup name

        Ok(Self {
            config,
            child_pid: None,
            socketpair,
            cgroup,
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
        self.cgroup.delete()?;
        Ok(())
    }
}

const GIB: i64 = 1024 * 1024 * 1024;
const KERNEL_MEMORY_LIMIT: i64 = GIB;
const MEMORY_HARD_LIMIT: i64 = GIB;
const MAX_PROCESSES: i64 = 10;
const CPU_SHARES: u64 = 250;

// TODO: Verify that the Cgroup actually gets created
fn build_cgroup(name: &str) -> Cgroup {
    use cgroups_rs::{hierarchies::V2, MaxValue};
    CgroupBuilder::new(name)
        .memory()
        .kernel_memory_limit(KERNEL_MEMORY_LIMIT)
        .memory_hard_limit(MEMORY_HARD_LIMIT)
        .done()
        .pid()
        .maximum_number_of_processes(MaxValue::Value(MAX_PROCESSES))
        .done()
        .cpu()
        .shares(CPU_SHARES)
        .done()
        .build(Box::new(V2::new()))
}

pub fn start(
    Args {
        command,
        uid,
        mount_dir,
        hostname,
    }: Args,
) -> eyre::Result<()> {
    let mut container = Container::new(&command, uid, mount_dir, hostname)?;

    container.create().map_err(|err| {
        debug!("Error creating container: {err}");

        match container.destroy() {
            Ok(()) => err,
            Err(destroy_err) => err.wrap_err(format!("Error destroying container: {destroy_err}")),
        }
    })?;

    container.wait_for_child()?;

    debug!("Cleaning up container...");
    container.destroy()?;
    Ok(())
}
