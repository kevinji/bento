use eyre::WrapErr;
use std::os::unix::net::UnixDatagram;

pub fn send_bool(socket: &UnixDatagram, value: bool) -> eyre::Result<()> {
    socket
        .send(&[value.into()])
        .wrap_err_with(|| format!("Failed to send {value} through socket"))?;
    Ok(())
}

pub fn recv_bool(socket: &UnixDatagram) -> eyre::Result<bool> {
    let mut buf = [0];
    socket
        .recv(&mut buf)
        .wrap_err("Failed to recv bool through socket")?;
    Ok(buf[0] != 0)
}
