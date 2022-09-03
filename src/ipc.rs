use nix::sys::socket::{socketpair, AddressFamily, SockFlag, SockType};
use std::os::unix::io::RawFd;

pub(super) fn create_socketpair() -> eyre::Result<(RawFd, RawFd)> {
    Ok(socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,                   // Use the default protocol for the SockType
        SockFlag::SOCK_CLOEXEC, // TODO: NONBLOCK?
    )?)
}
