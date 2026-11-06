#!/usr/bin/env python3
"""Copy a built app into a separate persistent WebKit/sandbox namespace.

Does not launch, sign, change the original bundle or touch personal records.
The QA-only path exception allows RUFREN_DATA_DIR inside the fixture despite
App Sandbox; the actual deliverable's entitlements retain no such exception.
"""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import shutil

parser = argparse.ArgumentParser()
parser.add_argument("bundle", type=Path)
parser.add_argument("fixture", type=Path)
args = parser.parse_args()
project = Path(__file__).resolve().parents[2]
original = args.bundle.resolve()
fixture = args.fixture.resolve()
assert fixture.is_relative_to(project / ".local"), "QA fixture must be under this repository's ignored .local directory"
assert not fixture.exists(), "Choose a new QA fixture directory; existing records are never overwritten"
with (original / "Contents/Info.plist").open("rb") as file:
    info = plistlib.load(file)
assert info.get("CFBundleIdentifier") == "town.lunar.rufren", "Expected the actual Rufren bundle as source"
fixture.mkdir(parents=True)
bundle = fixture / "Rufren QA.app"
shutil.copytree(original, bundle, symlinks=True)
# A fixture-specific bundle identifier separates WebKit and sandbox data from
# personal Rufren. Reopening this same fixture retains the same identifier.
identifier = "town.lunar.rufren.qa.fixture-" + hashlib.sha256(str(fixture).encode()).hexdigest()[:12]
info.update(CFBundleIdentifier=identifier, CFBundleDisplayName="Rufren QA", CFBundleName="Rufren QA")
with (bundle / "Contents/Info.plist").open("wb") as file:
    plistlib.dump(info, file)
data = fixture / "data"
data.mkdir()
with (project / "crates/rufren-app/platform/apple/macos.entitlements").open("rb") as file:
    entitlements = plistlib.load(file)
entitlements["com.apple.security.temporary-exception.files.absolute-path.read-write"] = [str(data) + "/"]
entitlement_file = fixture / "qa.entitlements"
with entitlement_file.open("wb") as file:
    plistlib.dump(entitlements, file)
report = {"bundle": str(bundle), "identifier": identifier, "entitlements": str(entitlement_file), "RUFREN_DATA_DIR": str(data), "executable": str(bundle / "Contents/MacOS" / info["CFBundleExecutable"]), "scope": "Separate app identity and QA-only data path exception; no original bundle/profile/data changes."}
(fixture / "fixture.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
