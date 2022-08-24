use crate::{cli::Args, container_config::ContainerConfig};
use tracing::debug;

#[derive(Debug)]
pub(super) struct Container {
    config: ContainerConfig,
}

impl Container {
    pub(super) fn new(args: Args) -> eyre::Result<Self> {
        let config = ContainerConfig::new(&args.command, args.uid, args.mount_dir)?;
        Ok(Self { config })
    }

    pub(super) fn create(&mut self) -> eyre::Result<()> {
        debug!("Created container");
        Ok(())
    }

    pub(super) fn destroy(&mut self) -> eyre::Result<()> {
        debug!("Destroyed container");
        Ok(())
    }
}
