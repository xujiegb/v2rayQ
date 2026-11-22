# v2rayQ

Rust 2024 proxy-management foundation. Client and server are separate binaries with separate responsibilities.

## New in this PR

- Keep existing VLESS, VMess, Trojan, Shadowsocks, AnyTLS and Hysteria2 share parsers.
- Native egui client with Overview, Nodes, Routing and Settings.
- Add, select and delete manual nodes; import share links.
- Add/delete HTTPS subscriptions; async background fetch; import plain or base64 share-link lists.
- Retain previous subscribed nodes when updates fail, persist local configuration as JSON.
- Separate std-only Rust read-only localhost server preview (127.0.0.1:16837).
- Light client Web preview, dark server admin preview, Cockpit manifest and compact LuCI native view.

## Run locally

- Client: cargo run -p v2rayq-client
- Server: cargo run -p v2rayq-server
- Server panel: http://127.0.0.1:16837/
- Client Web preview: http://127.0.0.1:16837/client
- Tests: cargo test -p v2rayQ --lib
- Cockpit preview: run sh frontends/cockpit/install.sh from repository root to install assets.
- LuCI preview: package frontends/luci/{htdocs,root} in the corresponding router filesystem paths.

## Important limitations

**External Rust Core API and configuration are not yet public.** There is no functioning proxy,
TUN, transparent proxy, Core lifecycle, traffic statistics or remote administration in this PR.
The egui switches save *desired* TUN and proxy settings only. Server administration modules,
Web client, Cockpit and LuCI are UI previews, not functional Core management yet.

**Do not expose the development server publicly.** It has no remote authentication or write
API, so it deliberately binds only to localhost. Future remote access requires authenticated
Core adapters, permissions, TLS and auditing. Future macOS/Linux/FreeBSD TUN operations must
use an OS authorization prompt with a constrained helper, not a custom sudo-password field.

## Layout

- src/model, src/parser, src/db: shared data model, existing URI parsers and Turso storage.
- src/client: platform-independent client storage, subscription parsing.
- apps/client: standalone egui native executable.
- apps/server: standalone std-only Rust localhost preview; never links egui/Turso.
- frontends/shared, frontends/web: lightweight HTML, CSS and JavaScript presentation.
- frontends/cockpit: Cockpit plugin manifest and packaging script.
- frontends/luci: tiny native LuCI shell without WASM.

Windows, macOS and Linux are desktop targets. FreeBSD, Android, iOS and OpenWrt are planned,
but their native packaging/cross-compilation has not yet been validated in this UI PR.

The UI uses its own neutral visual design inspired by contemporary proxy dashboards; no
Surge or S-UI assets are distributed.
