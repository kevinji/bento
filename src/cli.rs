use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Args {
    /// Command to execute inside the container
    #[clap(long)]
    pub(super) command: String,

    /// User ID to create inside the container
    #[clap(long)]
    pub(super) uid: u32,

    /// Directory to mount as root of the container
    #[clap(long = "mount")]
    pub(super) mount_dir: PathBuf,
}
