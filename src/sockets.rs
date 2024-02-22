use std::os::unix::net::UnixDatagram;

pub fn send_bool(socket: &UnixDatagram, value: bool) -> eyre::Result<()> {
    socket.send(&[value.into()])?;
    Ok(())
}

pub fn recv_bool(socket: &UnixDatagram) -> eyre::Result<bool> {
    let mut buf = [0];
    socket.recv(&mut buf)?;
    Ok(buf[0] != 0)
}
