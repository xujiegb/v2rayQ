# v2rayQ

Rust 2024 proxy client and server panels. The client uses a single Rust profile service across native egui, Web, CLI, LuCI and Cockpit. The server has its own executable, storage and interface.

| Entry point | Platforms | Features |
| --- | --- | --- |
| `v2rayq-client` | Windows, macOS, Linux, FreeBSD; iOS simulator build | Native egui profile editor, node search and network service controls |
| `v2rayq-web-client` | Windows, macOS, Linux, FreeBSD | Local browser UI sharing the desktop and CLI profile database |
| `v2rayq-cli` | Windows, macOS, Linux, FreeBSD | Node and subscription commands, routing preferences, network service controls |
| LuCI client | OpenWrt | Shared Web UI assets with a small LuCI/RPC wrapper |
| Cockpit client | Linux | Shared Web client UI, using the logged-in host user's profile |
| `v2rayq-server` | Windows, macOS, Linux, FreeBSD | Standalone administrator UI and persistent configuration drafts |
| Cockpit server | Linux | Independent server plugin, using the same server backend |
| Android | Android arm64 | Native egui library target. [Mobile packaging details](packaging/mobile/README.md) |

## Run locally

```sh
cargo run --locked -p v2rayq-client
cargo run --locked -p v2rayq-cli -- --help
cargo run --locked -p v2rayq-web-client
cargo run --locked -p v2rayq-server
```

The Web client and server listen on `127.0.0.1:16838` and `127.0.0.1:16837` respectively. Open the access URL printed by the corresponding process. Each launch issues a new access token. Cockpit uses its authenticated host session in place of a launch token.

For Cockpit, install `v2rayq-cli` or `v2rayq-server` into `/usr/bin`. Run `sh frontends/cockpit/install.sh /usr/share/cockpit client` or pass `server` for the server package. Omit the second argument to install both packages.

For OpenWrt, build `frontends/luci` as an OpenWrt package and install the target-architecture `v2rayq-cli` into `/usr/bin`. The LuCI RPC plugin uses `/etc/v2rayq/client.redb`. LuCI grants profile and network service permissions through separate `luci-app-v2rayq` and `luci-app-v2rayq-services` ACLs. Restart `rpcd` after installing the executable plugin.

## Profiles and network services

The client accepts HTTPS subscriptions containing plain-text or base64-wrapped share links. Supported schemes are VLESS, VMess, Trojan, Shadowsocks, AnyTLS and Hysteria2. A failed subscription refresh keeps the previously imported nodes. The CLI accepts node and subscription secrets on stdin. The clients share transport and TLS/ALPN options for manual nodes. Use `v2rayq-cli add-node` with a JSON object containing `protocol`, `name`, `host`, `port` and `credential`. Optional fields include `method`, `tls`, `server_name`, `alpn`, `transport`, `transport_path`, `transport_host` and `service_name`. The protocol keys are `vless`, `vmess`, `trojan`, `shadowsocks`, `any_tls` and `hysteria2`.

Local nodes support endpoint edits without retrieving their credentials. To change a node owned by a subscription, make a local copy first. Route rules support domain, domain suffix, IP / CIDR and destination port matches with selected proxy, direct or block actions. The list stores rules in priority order. The editor stores profile settings for the Core integration.

```sh
v2rayq-cli routes
printf '%s\n' '{"kind":"domain_suffix","value":"example.org","target":"direct"}' | v2rayq-cli add-rule
```

In Nodes, **Import links** previews and imports newline-separated or base64 link lists. The browser panel also accepts a local UTF-8 text file. Batch imports skip duplicates already saved locally and report unsupported lines. Subscription edits keep the existing URL when the replacement field is blank, and keep the imported nodes until a successful refresh. The CLI offers `preview-import`, `import`, `update-subscription` and `replace-subscription-url`. Pass links and replacement URLs on stdin to keep credentials out of command arguments.

The default client database is `client.redb` in the user configuration directory. The first read or edit imports a previous `client.json` and retains that file. Keep any Turso/SQLite `v2rayQ.db` for a separate migration. Both redb stores may contain unencrypted credentials. `V2RAYQ_CLIENT_DB` and `V2RAYQ_SERVER_DB` override their respective paths.

Client Settings exports and merges a versioned `v2rayq-client` JSON file shared by egui, Web, LuCI, Cockpit and CLI. It contains nodes, subscription ownership, the selected node and client preferences. Merging previews counts, skips equivalent records and preserves current preferences by default. **Use imported preferences** also applies the imported language, DNS and routing mode. Existing route rules remain and new rules are appended. Merging stores the profile without starting network services or fetching subscriptions. The export includes passwords, keys and subscription URLs; keep the file private. Native **Save to file** creates a new file with private Unix permissions and requires a different path if the file already exists.

```sh
v2rayq-cli export-config > client.json
v2rayq-cli merge-config < client.json
v2rayq-cli merge-config --preferences < client.json
v2rayq-cli language zh-CN
v2rayq-cli --lang fr --help
```

Both applications support Simplified Chinese, Traditional Chinese, English and French. Select the client language in Settings and the server language in the sidebar. Client preferences are shared across its frontends; the server stores its language independently. The first launch follows the browser or system locale. All catalogs live under [`locales/client` and `locales/server`](locales/README.md); adding a catalog automatically includes it in Rust builds and plugin packaging. Cockpit installation and LuCI package building use Python 3 on the build host to prepare resources; the installed UI has no Python runtime requirement.

TUN and transparent proxy controls start and stop administrator-provisioned host services. The panel reflects the service manager's reported state. The installed services own their own proxy configuration, so selecting a node or editing routes in the v2rayQ profile does not reconfigure those services. See [network service setup](packaging/network/README.md).

## Dashboard, groups and QR import

The client's Overview puts the installed proxy service switch, current profile node, traffic slots and host IP location together. The TUN and transparent modes share the existing authorization path. The switch follows observed service status and prevents starting a second mode while the other is running. Proxy throughput remains unavailable until the custom Core adapter supplies counters; the UI displays an em dash instead of a synthetic speed.

**Check IP location** sends an explicit HTTPS request from the backend host to [IP.SB](https://ip.sb/api/). No lookup runs at startup. LuCI and Cockpit therefore inspect their host, and this address may differ from a proxy exit. Lookup errors keep the panel usable.

Client and server browser forms confirm before discarding edits during navigation. Editors use a consistent save/cancel flow.

Nodes can be filtered by local origin, all subscriptions, a specific subscription or a custom group. Create, rename and remove groups, then move nodes using the row selector. New manual and imported nodes enter the selected custom group in the same transaction. Groups preserve subscription ownership; an unchanged subscription node keeps its group after a refresh. Removing a group retains its nodes. Portable export and merge include groups and memberships.

The native **Scan QR code** editor reads a PNG/JPEG file path or a dropped image in a background worker. It previews recognized shares through the common importer and opens recognized HTTPS subscriptions for confirmation. Images are bounded to 8 MiB, 4096 × 4096 and a decoder allocation limit. This pass supplies image scanning; camera capture belongs to the mobile host integration.

```sh
v2rayq-cli groups
v2rayq-cli add-group Travel
v2rayq-cli rename-group GROUP_ID Work
v2rayq-cli move-node NODE_ID GROUP_ID
v2rayq-cli delete-group GROUP_ID
v2rayq-cli inspect-network
```

## Server panel

Overview and System display persisted configuration counts and local host information. Inbounds, Users, Outbounds, Nodes, Services, TLS, REALITY, Routes, DNS, Administrators and Settings store independent drafts in `server.redb`. The Nodes editor shows fields by protocol and supports references to profile names in the separate TLS and REALITY sections. TLS drafts hold certificate paths and ALPN. REALITY drafts hold handshake parameters and keys. An optional JSON view preserves additional draft settings. The overview links the security → endpoint → user/service workflow. Record lists support search; unsaved forms prompt before navigation, retain values during language changes, and disable editing during save. Inbound and outbound drafts offer protocol and TLS/REALITY profile selectors. The external Core adapter is pending. Draft records do not configure a running Core or create active administrator accounts.

## Source layout

| Area | Location |
| --- | --- |
| Client operations, parsers, model and native theme | `src/client`, `src/parser`, `src/model`, `apps/client/src/ui` |
| Shared lightweight Web, Cockpit and LuCI client UI | `frontends/luci/htdocs/luci-static/resources/v2rayq` |
| Native dashboard, groups and QR UI | `apps/client/src/ui/dashboard.rs`, `groups.rs`, `scan.rs` |
| QR image decoding without graphics dependencies | `crates/qr` |
| Server configuration and styling | `apps/server/src/config.rs`, `frontends/web/server.*` |
| Loopback transport and persistence | `crates/http-preview`, `crates/storage` |
| Separate client/server language catalogs and resource embedding | `locales/client`, `locales/server`, `locales/embed.rs`, `crates/i18n` |

Visual and interaction references include [Surge](https://nssurge.com/), [VX](https://github.com/5VNetwork/VX), [v2rayN](https://github.com/2dust/v2rayN), [Passwall](https://github.com/Openwrt-Passwall/openwrt-passwall), [S-UI](https://github.com/alireza0/s-ui) and [ShellClash](https://github.com/Kuingsmile/ShellClash), and [3x-ui](https://github.com/MHSanaei/3x-ui). Client surfaces use compact blue navigation, a connection card and calm status typography; server surfaces use a dark rail, teal accents, metric cards and searchable configuration lists. Styling remains plain CSS and small egui view modules.

## Validation

```sh
cargo check --locked --workspace --all-targets --all-features
cargo test --locked -p v2rayQ -p v2rayq-storage -p v2rayq-contracts -p v2rayq-http-preview -p v2rayq-i18n
cargo build --locked -p v2rayq-cli -p v2rayq-web-client -p v2rayq-server
python3 tools/test_panel_api.py
python3 tools/check_locales.py
python3 tools/check_ui_parity.py
python3 tools/check_workspace_boundaries.py
cargo test --locked -p v2rayq-qr
```
