mod cli;
mod container;
mod container_config;

pub use cli::Args;
use container::Container;
use tracing::debug;

pub fn start(args: Args) -> eyre::Result<()> {
    let mut container = Container::new(args)?;
    if let Err(err) = container.create() {
        debug!("Error creating container: {err}");

        // TODO: Combine the destroy error with the create error
        container.destroy()?;
        return Err(err);
    }

    debug!("Cleaning up container...");
    container.destroy()?;
    Ok(())
}
