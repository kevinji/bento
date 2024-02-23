cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod child;
        mod cli;
        mod container;
        mod container_config;
        mod sockets;
        mod uid_gid_mapping;

        pub use cli::Args;
        pub use container::start;
    } else {
        compile_error!("Only linux is supported");
    }
}
