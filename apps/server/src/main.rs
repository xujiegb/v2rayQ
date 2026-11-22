#![forbid(unsafe_code)]

use v2rayq_contracts::SERVER_PORT;
use v2rayq_http_preview::{Mode, serve};

fn main() -> std::io::Result<()> {
    // Standalone server application. Core is not available yet.
    serve(Mode::Server, SERVER_PORT)
}
