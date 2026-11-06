#!/bin/sh
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
icons="$project/crates/rufren-app/public/icons"
command -v rsvg-convert >/dev/null || { echo 'Install librsvg to regenerate PNG derivatives of icon.svg.' >&2; exit 1; }
for size in 192 512; do
  rsvg-convert --width "$size" --height "$size" "$icons/icon.svg" --output "$icons/icon-$size.png"
done
rsvg-convert --width 180 --height 180 "$icons/icon.svg" --output "$icons/apple-touch-icon.png"
