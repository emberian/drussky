#!/bin/sh
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$project"
dx_bin=${RUFREN_DX:-"$project/.tools/dx"}
case $("$dx_bin" --version) in
  'dioxus 0.7.10 '*) ;;
  *) echo 'Rufren requires Dioxus CLI 0.7.10. Set RUFREN_DX to the matching executable; PATH dx may be Deno.' >&2; exit 1 ;;
esac
# Coordinate the repository's one-build-per-tree slot before running this script.
mode=${1:-debug}
case "$mode" in
  debug) "$dx_bin" build --package rufren-app --platform web --no-default-features --features web --locked ;;
  release) "$dx_bin" build --package rufren-app --platform web --no-default-features --features web --locked --release ;;
  *) echo 'Usage: scripts/platform/build-web.sh [debug|release]' >&2; exit 1 ;;
esac
node scripts/platform/prepare-web.mjs "target/dx/rufren-app/$mode/web/public"
