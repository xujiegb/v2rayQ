#![forbid(unsafe_code)]

use v2rayq_contracts::SERVER_PORT;
use v2rayq_http_preview::{Panel, serve};

fn main() -> std::io::Result<()> {
    // Server-only administrative panel. No client UI is bundled.
    serve(Panel {
        title: "Server",
        html: include_str!("../../../frontends/web/server.html"),
    }, SERVER_PORT)
}
