#![forbid(unsafe_code)]

use serde_json::json;
use v2rayQ::client::{Access, ClientService, Command, language};
use v2rayq_contracts::WEB_CLIENT_PORT;
use v2rayq_http_preview::{Asset, Panel, serve};

const ASSETS: &[Asset] = &[
    Asset {
        path: "/assets/appearance.js",
        mime: "text/javascript; charset=utf-8",
        body: include_str!(
            "../../../frontends/luci/htdocs/luci-static/resources/v2rayq/appearance.js"
        ),
    },
    Asset {
        path: "/assets/i18n.js",
        mime: "text/javascript; charset=utf-8",
        body: include_str!("../../../frontends/luci/htdocs/luci-static/resources/v2rayq/i18n.js"),
    },
    Asset {
        path: "/assets/client-ui.js",
        mime: "text/javascript; charset=utf-8",
        body: include_str!(
            "../../../frontends/luci/htdocs/luci-static/resources/v2rayq/client-ui.js"
        ),
    },
    Asset {
        path: "/assets/client.css",
        mime: "text/css; charset=utf-8",
        body: include_str!(
            "../../../frontends/luci/htdocs/luci-static/resources/v2rayq/client.css"
        ),
    },
    Asset {
        path: "/assets/client-entry.mjs",
        mime: "text/javascript; charset=utf-8",
        body: include_str!("../../../frontends/web/client-entry.mjs"),
    },
    Asset {
        path: "/assets/transport.mjs",
        mime: "text/javascript; charset=utf-8",
        body: include_str!("../../../frontends/shared/transport.mjs"),
    },
];

fn main() -> std::io::Result<()> {
    let client = ClientService::default();
    serve(
        Panel {
            title: "Client",
            html: include_str!("../../../frontends/web/client.html"),
            assets: ASSETS,
            locales: language::LOCALES,
            locale_manifest: language::MANIFEST,
        },
        WEB_CLIENT_PORT,
        move |body| {
            match serde_json::from_slice::<Command>(body) {
                Ok(command) => match client.call(command, Access::Desktop) {
                    Ok(result) => json!({ "result": result }),
                    Err(error) => json!({ "error": error }),
                },
                Err(_) => json!({ "error": "Invalid client command" }),
            }
            .to_string()
        },
    )
}
