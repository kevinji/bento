use clap::Parser;
use eyre::ensure;
use std::{fs, path::PathBuf};

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

    /// Hostname of the container
    #[clap(long)]
    pub(super) hostname: Option<String>,
}

impl Args {
    /// # Errors
    ///
    /// Returns an error if parsing `Args` fails, or if `mount_dir` is not a directory.
    pub fn try_parse_and_validate() -> eyre::Result<Self> {
        let args = Args::try_parse()?;
        ensure!(
            fs::metadata(&args.mount_dir)
                .map(|m| m.is_dir())
                .unwrap_or(false),
            "Mount dir must exist"
        );

        Ok(args)
    }
}
