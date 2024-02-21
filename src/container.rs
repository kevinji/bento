use crate::{child, cli::Args, container_config::ContainerConfig, namespace};
use cgroups_rs::{cgroup::Cgroup, cgroup_builder::CgroupBuilder};
use eyre::{bail, WrapErr};
use nix::{
    libc::uid_t,
    sys::wait::{waitpid, WaitStatus},
    unistd::Pid,
};
use std::{os::unix::net::UnixDatagram, path::PathBuf, process};
use tracing::{debug, error};

#[derive(Debug)]
pub struct Container {
    child_pid: Pid,
    socket: UnixDatagram,
    cgroup: Cgroup,
}

impl Container {
    pub fn new(
        command: &str,
        uid: uid_t,
        mount_dir: PathBuf,
        hostname: Option<String>,
    ) -> eyre::Result<Self> {
        let config = ContainerConfig::new(command, uid, mount_dir, hostname)?;

        let cgroup = build_cgroup("bento")?;

        namespace::setup_user_namespace(config.uid)?;

        let (container_socket, child_socket) = UnixDatagram::pair()?;
        container_socket.set_nonblocking(true)?;
        child_socket.set_nonblocking(true)?;

        let child_pid = child::clone_process(&config, child_socket)?;
        debug!("Created container with child PID {child_pid}");

        Ok(Self {
            child_pid,
            socket: container_socket,
            cgroup,
        })
    }

    pub fn wait_for_child(&mut self) -> eyre::Result<()> {
        debug!(
            "Waiting for child PID {child_pid} to finish",
            child_pid = self.child_pid
        );
        let wait_status = waitpid(self.child_pid, None)?;
        match wait_status {
            WaitStatus::Exited(_, exit_code) => {
                if exit_code != 0 {
                    error!("Child process exited with code {exit_code}");
                }
                Ok(())
            }
            _ => {
                bail!("Unexpected wait status from child: {wait_status:?}");
            }
        }
    }

    pub fn destroy(self) -> eyre::Result<()> {
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
fn build_cgroup(name: &str) -> eyre::Result<Cgroup> {
    use cgroups_rs::{hierarchies::V2, MaxValue};
    Ok(CgroupBuilder::new(name)
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
        .build(Box::new(V2::new()))?)
}

pub fn start(
    Args {
        command,
        uid,
        mount_dir,
        hostname,
    }: Args,
) -> eyre::Result<()> {
    let mut container =
        Container::new(&command, uid, mount_dir, hostname).wrap_err("Error creating container")?;
    container.wait_for_child()?;

    debug!("Cleaning up container...");
    container.destroy()?;
    Ok(())
}
