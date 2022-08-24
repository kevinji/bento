use clap::Parser;
use eyre::ensure;
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

impl Args {
    pub async fn try_parse_and_validate() -> eyre::Result<Self> {
        let args = Args::try_parse()?;
        ensure!(
            tokio::fs::metadata(&args.mount_dir)
                .await
                .map(|m| m.is_dir())
                .unwrap_or(false),
            "Mount dir must exist"
        );

        Ok(args)
    }
}
