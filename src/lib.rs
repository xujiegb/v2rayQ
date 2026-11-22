#![forbid(unsafe_code)]

#[cfg(feature = "legacy-tui")]
pub mod cli;
pub mod client;
pub mod db;
pub mod model;
pub mod parser;
