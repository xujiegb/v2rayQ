#![forbid(unsafe_code)]

use v2rayq_contracts::WEB_CLIENT_PORT;
use v2rayq_http_preview::{Panel, serve};

fn main() -> std::io::Result<()> {
    // Independent Web client, no server admin panel bundled.
    serve(Panel {
        title: "Client",
        html: include_str!("../../../frontends/web/client.html"),
    }, WEB_CLIENT_PORT)
}
