use clap::Parser;
use tracing::debug;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn setup_tracing() -> eyre::Result<()> {
    tracing_subscriber::registry()
        .with(console_subscriber::spawn())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_error::ErrorLayer::default())
        .try_init()?;

    Ok(())
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    setup_tracing()?;

    let args = crabcan::Args::try_parse()?;
    debug!("{:?}", args);

    Ok(())
}
