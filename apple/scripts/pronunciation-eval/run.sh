#!/bin/sh
# Builds and runs the pronunciation evaluation on macOS (needs scripts/models.py fetch and the
# restored audio). Usage: apple/scripts/pronunciation-eval/run.sh pairs|native|one ...
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
APPLE=$(cd "$HERE/../.." && pwd)
OUT=${TMPDIR:-/tmp}/drussky-peval
A=$APPLE/Drussky
SRC=$(ls "$A"/Pronunciation/*.swift | grep -v "+App.swift")
# shellcheck disable=SC2086
swiftc -O -swift-version 6 -o "$OUT" $SRC "$A/Services/Pitch.swift" "$A/Model/RussianText.swift" \
  "$HERE/Shim.swift" "$HERE/main.swift"
DRUSSKY_APPLE=$APPLE "$OUT" "$@"
