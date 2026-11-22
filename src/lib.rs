#![forbid(unsafe_code)]

#[cfg(feature = "legacy-tui")]
pub mod cli;
pub mod client;
#[cfg(feature = "database")]
pub mod db;
pub mod model;
pub mod parser;
