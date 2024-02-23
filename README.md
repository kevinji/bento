# Bento
Bento is a container implementation written in Rust.

It supports the following features:
- New root filesystem via `pivot_root` and `umount2`
- New cgroup, IPC, network, mount, PID, UTS, and user (if supported) namespaces
- Settable UID/GID within container
- cgroup v2 restrictions on memory, PIDs, and CPU shares
- rlimit restriction on file descriptors
- Capabilities dropped to control privileges
- Seccomp BPF filter to restrict syscalls

It also uses the [lddtree](https://crates.io/crates/lddtree) crate to automatically find the necessary .so files needed to run a given command by parsing its ELF header.

**TODO**
- Fix networking inside the container
