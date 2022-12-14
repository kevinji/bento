#[cfg(not(target_os = "linux"))]
compile_error!("Only linux is supported");

mod child;
mod cli;
mod container;
mod container_config;
mod ipc;

pub use cli::Args;
pub use container::start;
