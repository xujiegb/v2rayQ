# Android and iOS native egui integration

The UI implementation is `apps/client/src/ui`. It is written in Rust and uses egui/eframe directly;
there is no embedded browser or WebView.

The library exposes two safe entry points:

- `v2rayq_client::ClientApp::new_with_data_path(cc, sandbox_path)`
- `v2rayq_client::run_with_native_options_and_data_path(options, sandbox_path)`

Both allow mobile launchers to pass a writable application-private configuration directory.
The app UI and node parsers are shared with the Windows/Linux/macOS/FreeBSD client.

## Android

```sh
rustup target add aarch64-linux-android
cargo check -p v2rayq-client --lib --no-default-features --target aarch64-linux-android
```

This is a Rust UI library check, **not an installable APK**. A real Android Activity
needs `android-native-activity`, an NDK-generated `cdylib`, Gradle/Android resources,
signing and a native exported Rust entry point. Android Activity requires an unmangled
`android_main` symbol; Rust 2024 demands the `#[unsafe(no_mangle)]` attribute for
this symbol. This project deliberately does not add the attribute while the strict
first-party no-unsafe-source policy applies. Do not ship an empty APK or silently
replace egui with a WebView. Agree on a narrowly audited bootstrap boundary before
completing packaging.

## iOS

```sh
rustup target add aarch64-apple-ios
cargo check -p v2rayq-client --lib --no-default-features --target aarch64-apple-ios
```

Requires macOS/Xcode to build and codesign an actual iOS application, plus a native
UIKit or SwiftUI window/bootstrap hosting the Rust egui renderer. This is **not**
the same as displaying the independent HTML Web client. Do not claim an IPA until
the Xcode shell, device lifecycle, keyboard/input handling, sandbox paths and
signing are implemented and tested.

## Notes

- Desktop and mobile share page controllers, form validation, subscription parsing
  and Core contracts.
- Mobile browsers are not the UI engine.
- Native Apple/Android lifecycle glue and TUN permissions are platform-specific.
- TUN cannot be activated by current UI switches; Core remains unavailable.
