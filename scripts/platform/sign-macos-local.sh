#!/bin/sh
# Ad-hoc local signature: no certificate selection, private-key unlock or account
# provisioning. Run only on a built Rufren .app; distribution signing is separate.
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
bundle=${1:?Usage: scripts/platform/sign-macos-local.sh /path/to/Rufren.app}
python3 "$project/scripts/platform/check-apple-bundle.py" macos "$bundle"
codesign --force --entitlements "$project/crates/rufren-app/platform/apple/macos.entitlements" --sign - "$bundle"
python3 "$project/scripts/platform/check-apple-bundle.py" macos "$bundle" --signed
