# v2rayQ

Modular Rust 2024 client/server foundation. **egui is a native UI using real Rust widgets, never an embedded browser or WebView.** The client and server are separate processes and deployment artifacts.

## Build targets and reuse

| Artifact | Implementation | Target systems |
|---|---|---|
| Native client | `apps/client`, egui/eframe + shared services | Windows, macOS, Linux; FreeBSD build validation; Android/iOS library-only checks |
| Web client | `apps/web-client`, std HTTP host + light HTML/CSS/JS | Windows, macOS, Linux, FreeBSD |
| Server | `apps/server`, independent std HTTP host | Windows, macOS, Linux, FreeBSD |
| Cockpit client plugin | `frontends/cockpit`, shared Web assets | Linux Cockpit |
| LuCI client plugin | `frontends/luci`, small native LuCI view | OpenWrt or Linux with LuCI |
| Shared API model | `crates/contracts` | std-only, platform independent |
| HTTP asset delivery | `crates/http-preview` | std-only read-only preview; injected HTML per app |
| Embedded database | `crates/storage`, **redb 5.0.0** | Rust client persistence; usable independently by future server services |
| Node model and parsers | `src/model`, `src/parser` | Shared Rust client domain |
| Offline client service | `src/client` | Imports, subscriptions, manual nodes and settings |
| Legacy terminal UI | `src/cli` | Optional `legacy-tui` feature, no SQL runtime |

The two HTTP applications inject only their own HTML page into a shared *transport* crate. Native egui does not include HTML/CSS/JS or WebView components, and the server binary does not depend on egui, wgpu or a browser engine.

## Database replacement

**Turso and Tokio's SQL runtime have been removed.** Persistent client state is stored as one versioned redb document using ACID transactions. Future record-based storage can use namespaced keys in the shared `v2rayq-storage` crate; it has get/put/delete/prefix-list operations.

On first run the client uses `client.redb` in the platform application-data/config directory. If `client.redb` is absent but the previous `client.json` is present, the JSON file is imported once into redb and **retained as a backup**. If the JSON is invalid, loading fails rather than deleting or overwriting it. On Unix, database file permissions are restricted to the current user. redb does not encrypt secrets at rest; use OS storage protection for devices that require it.

**Existing legacy `v2rayQ.db` Turso/SQLite data is not automatically imported.** Do not delete it. Any migration from those table layouts must be a separate, tested export/import step before replacing user data.

## Build and test

Use current stable Rust with appropriate target SDKs:

```sh
cargo test -p v2rayq-storage -p v2rayQ --lib
cargo test -p v2rayq-contracts -p v2rayq-http-preview
cargo check -p v2rayq-client -p v2rayq-web-client -p v2rayq-server
cargo run -p v2rayq-client
cargo run -p v2rayq-web-client  # http://127.0.0.1:16838/
cargo run -p v2rayq-server      # http://127.0.0.1:16837/
```

Cockpit: `sh frontends/cockpit/install.sh` from the repository root, with permission to write `/usr/share/cockpit/v2rayq`. LuCI: the `frontends/luci/Makefile` generates an OpenWrt LuCI package using only its native JS and CSS assets.

CI includes Windows/Linux/macOS native check, FreeBSD 15.1 VM check, Android arm64 and iOS arm64 Rust-library checks. Native APK/IPA host wrappers, signing and real device validation are **not complete** (see `packaging/mobile/README.md`). Android's exported entry point cannot be provided in first-party Rust while strictly forbidding unsafe attributes; a separately audited launcher is required. No WebView workaround will be used.

## What works and what is a preview

Native egui: manual node management and deletion, VLESS/VMess/Trojan/Shadowsocks/AnyTLS/Hysteria2 share import, HTTPS subscription refresh, local data persistence and settings UI.

**Not yet connected:** the third-party Rust Core API and config format are not public. TUN, transparent/system proxy, traffic measurements, actual routing and server CRUD are unavailable. Preference switches do not imply live tunnel status. Web, Cockpit and LuCI panels currently provide their unified layout but **not** a writable proxy-control API. Both local Web hosts bind only to localhost, with no public administration or authentication enabled.

A real privileged network helper will need OS-mediated authorization, narrowly scoped commands and platform review; it must not collect sudo credentials inside a custom text field.
