#!/usr/bin/env python3
"""Use an existing Xcode account to provision/sign the actual Dioxus iOS app.

No SDK installation, account creation, agreement acceptance or substitute UI.
Automatic provisioning may register the explicitly selected development device.
Run within the repository's one-build-per-tree slot.
"""
import argparse
import json
from pathlib import Path
import plistlib
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("source_app", type=Path)
parser.add_argument("--team-id", required=True)
parser.add_argument("--device", required=True, help="Physical device UDID for Xcode's destination")
parser.add_argument("--output", type=Path, required=True, help="Ignored directory for derived data and private signing logs")
args = parser.parse_args()
project = Path(__file__).resolve().parents[2]
source = args.source_app.resolve()
output = args.output.resolve()
assert output.is_relative_to(project / ".local"), "Signing artifacts/logs must stay under ignored .local"
with (source / "Info.plist").open("rb") as file:
    info = plistlib.load(file)
assert info.get("CFBundleIdentifier") == "town.lunar.rufren", "Unexpected source identity"
assert info.get("CFBundleSupportedPlatforms") == ["iPhoneOS"], "Pass a finalized physical-device build"
output.mkdir(parents=True, exist_ok=True)
command = [
    "/Applications/Xcode.app/Contents/Developer/usr/bin/xcodebuild",
    "-quiet", "-project", str(project / "crates/rufren-app/platform/apple/Rufren.xcodeproj"),
    "-scheme", "Rufren", "-configuration", "Debug", "-sdk", "iphoneos",
    "-destination", f"id={args.device}", "-derivedDataPath", str(output / "DerivedData"),
    "-allowProvisioningUpdates", "-allowProvisioningDeviceRegistration", "-jobs", "2",
    f"DEVELOPMENT_TEAM={args.team_id}", f"RUFREN_IOS_APP={source}", "build",
]
log = output / "xcode-signing.log"
print("Provisioning and signing the compiled Rust app with the existing Xcode account.", flush=True)
with log.open("w") as file:
    result = subprocess.run(command, stdout=file, stderr=subprocess.STDOUT)
if result.returncode:
    raise SystemExit(f"Xcode signing failed (exit {result.returncode}); inspect {log}. No device app was removed.")
apps = list((output / "DerivedData/Build/Products/Debug-iphoneos").glob("*.app"))
matches = []
for app in apps:
    with (app / "Info.plist").open("rb") as file:
        metadata = plistlib.load(file)
    if metadata.get("CFBundleIdentifier") == "town.lunar.rufren":
        matches.append(app)
assert len(matches) == 1, "Could not uniquely identify the signed Rust app"
signed = matches[0]
assert (signed / "embedded.mobileprovision").is_file(), "Xcode did not embed a development profile"
subprocess.run(["codesign", "--verify", "--deep", "--strict", str(signed)], check=True, capture_output=True)
(output / "signed-app.json").write_text(json.dumps({"app": str(signed), "bundle_id": "town.lunar.rufren"}, indent=2) + "\n")
print(f"Signed Rust app ready: {signed}")
