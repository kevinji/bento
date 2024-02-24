# Bento
Bento is a container implementation written in Rust.

It supports the following features:
- New root filesystem via `pivot_root` and `umount2`
- New cgroup, IPC, network, mount, PID, UTS, and user (if supported) namespaces
- Settable UID/GID within container, with allowed UIDs from `/etc/subuid` and GIDs from `/etc/subgid`
- cgroup v2 restrictions on memory, PIDs, and CPU shares
- rlimit restriction on file descriptors
- Capabilities dropped to control privileges
- Seccomp BPF filter to restrict syscalls

It also uses the [lddtree](https://crates.io/crates/lddtree) crate to automatically find the necessary .so files needed to run a given command by parsing its ELF header.

**TODO**
- Fix networking inside the container

## References

- [Linux containers in 500 lines of code](https://blog.lizzie.io/linux-containers-in-500-loc.html) by Lizzie Dixon
- [Writing a container in Rust](https://litchipi.site/post/911064764032430296) by  Litchi Pi
- [Namespaces in operation](https://lwn.net/Articles/531114/) by Michael Kerrisk
- Digging into Linux namespaces [part 1](https://blog.quarkslab.com/digging-into-linux-namespaces-part-1.html) and [part 2](https://blog.quarkslab.com/digging-into-linux-namespaces-part-2.html) by Mihail Kirov
