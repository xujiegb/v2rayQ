#![forbid(unsafe_code)]

mod config;

use std::io::{self, Read};
use v2rayq_contracts::SERVER_PORT;
use v2rayq_http_preview::{Asset, COMMAND_LIMIT, Panel, serve};

const ASSETS: &[Asset] = &[
    Asset {
        path: "/assets/server.css",
        mime: "text/css; charset=utf-8",
        body: include_str!("../../../frontends/web/server.css"),
    },
    Asset {
        path: "/assets/server.mjs",
        mime: "text/javascript; charset=utf-8",
        body: include_str!("../../../frontends/web/server.mjs"),
    },
    Asset {
        path: "/assets/transport.mjs",
        mime: "text/javascript; charset=utf-8",
        body: include_str!("../../../frontends/shared/transport.mjs"),
    },
];

fn main() -> io::Result<()> {
    if std::env::args().skip(1).eq(["--rpc"]) {
        let mut input = Vec::new();
        io::stdin()
            .take((COMMAND_LIMIT + 1) as u64)
            .read_to_end(&mut input)?;
        if input.len() > COMMAND_LIMIT {
            println!(
                "{}",
                serde_json::json!({ "error": "Command exceeds 128 KiB" })
            );
        } else {
            println!("{}", config::dispatch(&input));
        }
        return Ok(());
    }
    serve(
        Panel {
            title: "Server",
            html: include_str!("../../../frontends/web/server.html"),
            assets: ASSETS,
        },
        SERVER_PORT,
        config::dispatch,
    )
}
