// bosua-lib: shared library for all Bosua binary variants

// Always-available modules
pub mod cli;
pub mod cloud;
pub mod commands;
pub mod config;
pub mod crypto;
pub mod daemon;
pub mod download;
pub mod errors;
pub mod fileops;
pub mod http_client;
pub mod json;
pub mod logger;
pub mod notifications;
pub mod output;
pub mod platform;
pub mod search;
pub mod signal;
pub mod text;
pub mod tui;
pub mod utils;

// Feature-gated modules
#[cfg(feature = "http-server")]
pub mod server;
