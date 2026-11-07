#!/bin/sh
# Build beargarden/sk8 as a static library for the platform Xcode is building, into
# apple/build/lib/$PLATFORM_NAME/libsk8.a (the app links it with -lsk8), and the web's wasm.
set -e
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
here=$(cd "$(dirname "$0")" && pwd)
case "${PLATFORM_NAME:-macosx}" in
  iphonesimulator) target=aarch64-apple-ios-sim ;;
  iphoneos) target=aarch64-apple-ios ;;
  *) target=aarch64-apple-darwin ;;
esac
out="$here/build/lib/${PLATFORM_NAME:-macosx}"
mkdir -p "$out"
tdir=${CARGO_TARGET_DIR:-$here/../sk8/target}
# Xcode's environment must not leak into cargo's own builds (a host build script would
# otherwise link against the iOS SDK).
env -u SDKROOT -u IPHONEOS_DEPLOYMENT_TARGET -u MACOSX_DEPLOYMENT_TARGET -u PLATFORM_NAME \
  sh -c "cd '$here/../sk8' && cargo build --release --lib --target $target"
cp "$tdir/$target/release/libsk8.a" "$out/libsk8.a"
echo "libsk8.a for $target"
