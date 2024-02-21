use nix::{
    sched::{unshare, CloneFlags},
    unistd::{setgroups, setresgid, setresuid, Gid, Uid},
};
use tracing::{debug, warn};

pub fn setup_user_namespace(uid: Uid) -> eyre::Result<()> {
    debug!("Setting up user namespace with uid {uid}");
    match unshare(CloneFlags::CLONE_NEWUSER) {
        Ok(()) => (),
        Err(err) => {
            warn!("User namespaces not supported: {err}");
            return Ok(());
        }
    }

    let gid = Gid::from_raw(uid.as_raw());
    setgroups(&[gid])?;
    setresgid(gid, gid, gid)?;
    setresuid(uid, uid, uid)?;

    Ok(())
}
