use clap::Parser;
use nix::libc::uid_t;
use std::{fs, path::PathBuf};

#[derive(Debug, Parser)]
pub struct Args {
    /// Command to execute inside the container
    #[clap(long)]
    pub command: String,

    /// User ID to create inside the container
    #[clap(long)]
    pub uid: uid_t,

    // TODO: Consider creating a tempdir as default
    /// Directory to mount as root of the container
    #[clap(long = "mount")]
    pub mount_dir: PathBuf,

    /// Hostname of the container
    #[clap(long)]
    pub hostname: Option<String>,
}

impl Args {
    /// # Errors
    ///
    /// Returns an error if parsing `Args` fails, or creating `mount_dir` fails.
    pub fn try_parse_and_validate() -> eyre::Result<Self> {
        let args = Args::try_parse()?;
        fs::create_dir_all(&args.mount_dir)?;

        Ok(args)
    }
}
