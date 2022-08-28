use nix::mount::{mount, MsFlags};
use std::path::PathBuf;

fn mount_dir(path: &Option<PathBuf>, mount_point: &PathBuf, flags: Vec<MsFlags>) {}
