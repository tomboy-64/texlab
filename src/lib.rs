#[cfg(feature = "citation")]
pub mod citeproc;

cfg_if::cfg_if! {
    if #[cfg(feature = "server")] {
        mod config;

        pub mod server;
    }
}

pub mod components;
pub mod diagnostics;
pub mod features;
pub mod forward_search;
pub mod protocol;
pub mod syntax;
pub mod tex;
pub mod workspace;
