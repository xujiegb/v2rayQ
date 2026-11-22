#!/bin/sh
set -eu
cd "$(dirname "$0")/../../.."
target=${1:-aarch64-apple-ios-sim}
case "$target" in
    aarch64-apple-ios) sdk=iphoneos ;;
    aarch64-apple-ios-sim|x86_64-apple-ios) sdk=iphonesimulator ;;
    *) printf 'Unsupported iOS target: %s\n' "$target" >&2; exit 2 ;;
esac
export SDKROOT="$(xcrun --sdk "$sdk" --show-sdk-path)"
export IPHONEOS_DEPLOYMENT_TARGET=16.0
build=${CARGO_TARGET_DIR:-target}
bundle="${2:-dist/$target/v2rayQ.app}"
cargo build --locked --release -p v2rayq-client --target "$target"
mkdir -p "$bundle"
install -m 0755 "$build/$target/release/v2rayq-client" "$bundle/v2rayq-client"
install -m 0644 packaging/mobile/ios/Info.plist "$bundle/Info.plist"
/usr/libexec/PlistBuddy -c "Add :CFBundleSupportedPlatforms array" "$bundle/Info.plist"
platform=iPhoneSimulator
if [ "$sdk" = iphoneos ]; then platform=iPhoneOS; fi
/usr/libexec/PlistBuddy -c "Add :CFBundleSupportedPlatforms:0 string $platform" "$bundle/Info.plist"
if [ "$sdk" = iphonesimulator ]; then codesign --force --sign - "$bundle"; fi
printf '%s\n' "$bundle"
