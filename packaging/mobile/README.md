# Native mobile egui

Mobile and desktop share the same Rust widgets, profile commands and storage. A native host can pass an explicit sandbox database path to `v2rayq_client::run_with_native_options_and_data_path`.

## iOS

On macOS with Xcode, build the arm64 simulator app bundle with these commands.

```sh
rustup target add aarch64-apple-ios-sim
sh packaging/mobile/ios/build.sh aarch64-apple-ios-sim
xcrun simctl install booted dist/aarch64-apple-ios-sim/v2rayQ.app
xcrun simctl launch booted org.v2rayq.client
```

`aarch64-apple-ios` builds an unsigned device bundle. Installing it on a physical device requires Apple's signing and provisioning. The build uses winit's iOS lifecycle and the shared egui/WGPU renderer. The simulator bundle is built by the iOS job. Device signing, distribution and mobile VPN integration are pending.

## Android

With the Android NDK Clang configured for the target, build-check the shared Rust UI library with these commands.

```sh
rustup target add aarch64-linux-android
cargo check --locked -p v2rayq-client --lib --no-default-features --target aarch64-linux-android
```

The `android-activity` backend needs an exported unmangled `android_main` entry point. Rust 2024 treats the export attribute as an unsafe declaration. Installable APK packaging requires a launcher compatible with the project's first-party Rust restrictions. Android VPN authorization and integration are pending.
