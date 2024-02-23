use crate::{
    child, cli::Args, container_config::ContainerConfig, sockets,
    uid_gid_mapping::read_and_write_uid_and_gid_mappings,
};
use cgroups_rs::{cgroup::Cgroup, cgroup_builder::CgroupBuilder};
use eyre::{bail, WrapErr};
use nix::{
    libc::uid_t,
    sys::wait::{waitpid, WaitStatus},
    unistd::Pid,
};
use std::{net::Shutdown, os::unix::net::UnixDatagram, path::PathBuf};
use tracing::{debug, error};

#[derive(Debug)]
pub struct Container {
    child_pid: Pid,
    socket: UnixDatagram,
    cgroup: Cgroup,
}

const CGROUP_NAME: &str = "bento";

impl Container {
    pub fn new(
        command: String,
        uid: uid_t,
        mount_dir: PathBuf,
        hostname: Option<String>,
        commands_to_copy: Vec<String>,
    ) -> eyre::Result<Self> {
        let config = ContainerConfig::new(command, uid, mount_dir, hostname, commands_to_copy)?;

        let cgroup = build_cgroup(CGROUP_NAME)?;
        debug!("Created cgroup {CGROUP_NAME}");

        let cgroup_pid = u64::try_from(Pid::this().as_raw()).unwrap().into();
        cgroup.add_task_by_tgid(cgroup_pid)?;
        debug!("Added PID to cgroup");

        let (container_socket, child_socket) = UnixDatagram::pair()?;

        let child_pid = child::clone_process(&config, child_socket)?;
        debug!("Created container with child PID {child_pid}");

        let user_namespace_created = sockets::recv_bool(&container_socket)?;
        debug!("User namespace created {user_namespace_created}, received");
        if user_namespace_created {
            read_and_write_uid_and_gid_mappings(child_pid)?;
        }

        debug!("Notifying child that UID and GID mappings are ready");
        sockets::send_bool(&container_socket, true)?;

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
        debug!("Destroying container");

        self.socket.shutdown(Shutdown::Both)?;
        debug!("Socket shut down");

        let cgroup_pid = u64::try_from(Pid::this().as_raw()).unwrap().into();
        self.cgroup.remove_task_by_tgid(cgroup_pid)?;
        debug!("Removed PID from cgroup");

        self.cgroup.delete()?;
        debug!("Cgroup deleted");

        Ok(())
    }
}

const GIB: i64 = 1024 * 1024 * 1024;
const KERNEL_MEMORY_LIMIT: i64 = GIB;
const MEMORY_HARD_LIMIT: i64 = GIB;
const MAX_PROCESSES: i64 = 10;
const CPU_SHARES: u64 = 250;

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
        commands_to_copy,
    }: Args,
) -> eyre::Result<()> {
    debug!("Container PID: {}", Pid::this());

    let mut container = Container::new(command, uid, mount_dir, hostname, commands_to_copy)
        .wrap_err("Error creating container")?;
    container.wait_for_child()?;

    debug!("Cleaning up container...");
    container.destroy()?;
    Ok(())
}
