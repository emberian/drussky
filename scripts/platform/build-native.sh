#!/bin/sh
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$project"
dx_bin=${RUFREN_DX:-"$project/.tools/dx"}
case $("$dx_bin" --version) in
  'dioxus 0.7.10 '*) ;;
  *) echo 'Set RUFREN_DX to Dioxus CLI 0.7.10; PATH dx may be Deno.' >&2; exit 1 ;;
esac
# Coordinate the repository's one-build-per-tree slot before running.
# No signing identity or provisioning profile is embedded in source control.
case "${1:-}" in
  macos) MACOSX_DEPLOYMENT_TARGET=12.0 "$dx_bin" build --package rufren-app --platform macos --no-default-features --features desktop --locked ;;
  ios-simulator) IPHONEOS_DEPLOYMENT_TARGET=15.0 "$dx_bin" build --package rufren-app --platform ios --target aarch64-apple-ios-sim --no-default-features --features mobile --locked ;;
  ios-device) IPHONEOS_DEPLOYMENT_TARGET=15.0 "$dx_bin" build --package rufren-app --platform ios --target aarch64-apple-ios --no-default-features --features mobile --locked ;;
  android) "$dx_bin" build --package rufren-app --platform android --target aarch64-linux-android --no-default-features --features mobile --locked ;;
  *) echo 'Usage: scripts/platform/build-native.sh macos|ios-simulator|ios-device|android' >&2; exit 1 ;;
esac
case "$1" in
  macos) python3 scripts/platform/finalize-apple-bundle.py macos target/dx/rufren-app/debug/macos ;;
  ios-simulator) python3 scripts/platform/finalize-apple-bundle.py ios target/dx/rufren-app/debug/ios --ios-environment simulator ;;
  ios-device) python3 scripts/platform/finalize-apple-bundle.py ios target/dx/rufren-app/debug/ios --ios-environment device ;;
esac
