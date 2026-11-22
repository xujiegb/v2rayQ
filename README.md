# v2rayQ

Modular Rust 2024 client/server application, following the shared-service / independent-presentation principle also used by v2rayN. **Native egui clients never embed WebView or browser HTML.**

## Delivery modules

| Artifact | Source | Targets |
|---|---|---|
| Native egui client | `apps/client` | Windows, Linux, macOS; Android/iOS/FreeBSD target validation in CI |
| Independent Web client | `apps/web-client` + `frontends/web/client.html` | Windows, Linux, macOS, FreeBSD |
| Independent Rust server | `apps/server` + `frontends/web/server.html` | Windows, Linux, macOS, FreeBSD |
| Cockpit client plugin | `frontends/cockpit` | Linux with Cockpit |
| LuCI client plugin | `frontends/luci` | OpenWrt, or Linux with LuCI |
| Shared contracts | `crates/contracts` | Platform-independent, std-only |
| Shared HTTP preview | `crates/http-preview` | Platform-independent, std-only |
| Shared node/URI models | `src/model`, `src/parser` | Pure Rust client service |
| Client profile store | `src/client` | Rust, JSON, no Core dependency |
| Legacy CLI/Turso | `src/cli`, `src/db` | Explicit `legacy-tui` / `database` features only |

## Native UI versus browser UI

`apps/client/src/ui` contains Rust egui widgets and page controllers. The Android, iOS, Windows, Linux, macOS, and FreeBSD clients are intended to use **this single Rust UI**, not the HTML/CSS/JS from the browser panels. Browser clients are separate artifacts and share visual assets and versioned application contracts.

`apps/client/src/lib.rs` exports a safe `ClientApp` and a `run_with_native_options` function for platform launchers. The CLI executable builds only with the `desktop` feature. Android and iOS native library targets are checked in CI; **APK/IPA packaging and host activity/Swift entry points are not implemented yet**. Official Android NativeActivity entry requires a Rust 2024 `#[unsafe(no_mangle)]` symbol, which conflicts with the repository's strict no-unsafe-source rule. This needs an explicitly audited platform bootstrap decision before an APK can be claimed. iOS likewise requires a native Xcode host. No WebView fallback will be used.

## Build

Use a current stable Rust toolchain.

```sh
cargo test -p v2rayQ --lib
cargo test -p v2rayq-contracts -p v2rayq-http-preview
cargo run -p v2rayq-client
cargo run -p v2rayq-web-client  # http://127.0.0.1:16838/
cargo run -p v2rayq-server      # http://127.0.0.1:16837/
```

Web client and server are completely different binaries and ports. Both are bound to localhost until an authenticated management API is available. Cockpit packages only the client. LuCI has a tiny native JS client and no WebAssembly/egui code.

The previous `turso_core 0.8.4` compile failure is isolated behind optional features and no longer blocks the default native client or server dependency graph. The old database code remains for migration, but its optional build is not claimed to pass until upstream fixes are available.

## Functionality and limitations

Implemented: native share URI import and manual node addition/deletion (VLESS, VMess, Trojan, Shadowsocks, AnyTLS, Hysteria2), HTTPS subscription refresh, plain/base64 URI-list parsing, offline state persistence, routing and network-mode preferences.

**Not connected:** The external Rust Core has not published its API/config format. TUN, transparent/system proxy, routing execution, traffic charts, server-side management, administrator accounts, Web client, Cockpit and LuCI write operations are not functional. Never present offline switches as active connections. Native client settings are not executed on the host.

This PR builds a reviewable cross-platform foundation, not signed/runnable packages for every target. FreeBSD native egui support, Android/iOS host packaging and real Core features require target-specific checks, system integration and follow-up work.
