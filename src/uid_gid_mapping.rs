use eyre::eyre;
use nix::unistd::{getgid, getuid, Gid, Group, Pid, Uid, User};
use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    str::FromStr,
};
use tracing::debug;

#[derive(Debug, Eq, PartialEq)]
enum UidOrUser {
    Uid(Uid),
    User(String),
}

#[derive(Debug)]
struct UidMapping {
    uid_or_user: UidOrUser,
    sub_uid: Uid,
    sub_count: u32,
}

impl FromStr for UidMapping {
    type Err = eyre::Report;

    fn from_str(s: &str) -> eyre::Result<Self> {
        let [uid_or_user_str, sub_uid_str, sub_count_str] = s
            .split(':')
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|vec: Vec<_>| {
                eyre!(
                    "UID mapping line {s} should have 3 parts, but has {len}",
                    len = vec.len()
                )
            })?;

        let uid_or_user = match uid_or_user_str.parse() {
            Ok(uid) => UidOrUser::Uid(Uid::from_raw(uid)),
            Err(_) => UidOrUser::User(uid_or_user_str.to_owned()),
        };
        let sub_uid = Uid::from_raw(sub_uid_str.parse()?);
        let sub_count = sub_count_str.parse()?;
        Ok(Self {
            uid_or_user,
            sub_uid,
            sub_count,
        })
    }
}

const SUBUID_PATH: &str = "/etc/subuid";

fn read_subuid() -> eyre::Result<Option<UidMapping>> {
    let current_uid = getuid();
    let current_user = User::from_uid(current_uid)?
        .ok_or_else(|| eyre!("User must exist for current UID {current_uid}"))?
        .name;
    debug!("Current UID {current_uid} has name {current_user}");

    let file = File::open(SUBUID_PATH)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let uid_mapping: UidMapping = line?.parse()?;
        let is_current_user = match &uid_mapping.uid_or_user {
            UidOrUser::Uid(uid) => uid == &current_uid,
            UidOrUser::User(user) => user == &current_user,
        };

        if is_current_user {
            debug!("Found UID mapping: {uid_mapping:?}");
            return Ok(Some(uid_mapping));
        }
    }

    Ok(None)
}

fn uid_map_path(pid: Pid) -> String {
    format!("/proc/{pid}/uid_map")
}

fn write_to_uid_map(
    pid: Pid,
    UidMapping {
        uid_or_user: _,
        sub_uid,
        sub_count,
    }: &UidMapping,
) -> eyre::Result<()> {
    let path = uid_map_path(pid);
    debug!("Writing to UID map: {path}");
    let mut file = File::create(path)?;
    file.write_all(format!("0 {sub_uid} {sub_count}\n").as_bytes())?;
    Ok(())
}

#[derive(Debug)]
enum GidOrGroup {
    Gid(Gid),
    Group(String),
}

#[derive(Debug)]
struct GidMapping {
    gid_or_group: GidOrGroup,
    sub_gid: Gid,
    sub_count: u32,
}

impl FromStr for GidMapping {
    type Err = eyre::Report;

    fn from_str(s: &str) -> eyre::Result<Self> {
        let [gid_or_group_str, sub_gid_str, sub_count_str] = s
            .split(':')
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|vec: Vec<_>| {
                eyre!(
                    "GID mapping line {s} should have 3 parts, but has {len}",
                    len = vec.len()
                )
            })?;

        let gid_or_group = match gid_or_group_str.parse() {
            Ok(gid) => GidOrGroup::Gid(Gid::from_raw(gid)),
            Err(_) => GidOrGroup::Group(gid_or_group_str.to_owned()),
        };
        let sub_gid = Gid::from_raw(sub_gid_str.parse()?);
        let sub_count = sub_count_str.parse()?;
        Ok(Self {
            gid_or_group,
            sub_gid,
            sub_count,
        })
    }
}

const SUBGID_PATH: &str = "/etc/subgid";

fn read_subgid() -> eyre::Result<Option<GidMapping>> {
    let current_gid = getgid();
    let current_group = Group::from_gid(current_gid)?
        .ok_or_else(|| eyre!("Group must exist for current GID {current_gid}"))?
        .name;
    debug!("Current GID {current_gid} has name {current_group}");

    let file = File::open(SUBGID_PATH)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let gid_mapping: GidMapping = line?.parse()?;
        let is_current_group = match &gid_mapping.gid_or_group {
            GidOrGroup::Gid(gid) => gid == &current_gid,
            GidOrGroup::Group(group) => group == &current_group,
        };

        if is_current_group {
            debug!("Found GID mapping: {gid_mapping:?}");
            return Ok(Some(gid_mapping));
        }
    }

    Ok(None)
}

fn gid_map_path(pid: Pid) -> String {
    format!("/proc/{pid}/gid_map")
}

fn write_to_gid_map(
    pid: Pid,
    GidMapping {
        gid_or_group: _,
        sub_gid,
        sub_count,
    }: &GidMapping,
) -> eyre::Result<()> {
    let path = gid_map_path(pid);
    debug!("Writing to GID map: {path}");
    let mut file = File::create(path)?;
    file.write_all(format!("0 {sub_gid} {sub_count}\n").as_bytes())?;
    Ok(())
}

pub fn read_and_write_uid_and_gid_mappings(pid: Pid) -> eyre::Result<()> {
    let subuid = read_subuid()?;
    let subgid = read_subgid()?;

    if let Some(uid_mapping) = subuid {
        write_to_uid_map(pid, &uid_mapping)?;
    }
    if let Some(gid_mapping) = subgid {
        write_to_gid_map(pid, &gid_mapping)?;
    }

    Ok(())
}
