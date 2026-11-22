#![forbid(unsafe_code)]

use v2rayq_contracts::WEB_CLIENT_PORT;
use v2rayq_http_preview::{Mode, serve};

fn main() -> std::io::Result<()> {
    // Independent binary from v2rayq-server. No Core or system privileges.
    serve(Mode::Client, WEB_CLIENT_PORT)
}
