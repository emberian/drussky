#!/usr/bin/env python3
"""Check generated bundle metadata; this does not assert device execution."""
import argparse
from pathlib import Path
import plistlib
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("platform", choices=["macos", "ios"])
parser.add_argument("bundle", type=Path)
parser.add_argument("--identifier", default="town.lunar.rufren", help="Expected identity; use a distinct value only for an isolated QA copy")
parser.add_argument("--signed", action="store_true", help="Also verify signature + signed macOS microphone entitlement")
args = parser.parse_args()
metadata = args.bundle / ("Contents/Info.plist" if args.platform == "macos" else "Info.plist")
with metadata.open("rb") as file:
    info = plistlib.load(file)
assert info.get("CFBundleIdentifier") == args.identifier, "Unexpected bundle identity"
if args.identifier == "town.lunar.rufren":
    assert info.get("CFBundleDisplayName") == "Drussky", "Run finalize-apple-bundle.py to set the app's display name"
if args.platform == "macos":
    assert (args.bundle / "Contents/Resources" / info.get("CFBundleIconFile", "icon.icns")).is_file(), "Native app icon is missing; run finalize-apple-bundle.py"
assert info.get("NSMicrophoneUsageDescription"), "Missing microphone purpose string"
if args.platform == "ios":
    assert {1, 2}.issubset(set(info.get("UIDeviceFamily", []))), "Bundle must support iPhone + iPad"
    assert len(info.get("UISupportedInterfaceOrientations~ipad", [])) == 4, "iPad rotation metadata missing"
if args.signed:
    subprocess.run(["codesign", "--verify", "--deep", "--strict", str(args.bundle)], check=True, capture_output=True)
    if args.platform == "macos":
        signed = subprocess.run(["codesign", "--display", "--entitlements", "-", "--xml", str(args.bundle)], check=True, capture_output=True)
        entitlements = plistlib.loads(signed.stdout)
        assert entitlements.get("com.apple.security.device.audio-input"), "Signed bundle lacks audio-input entitlement"
        assert entitlements.get("com.apple.security.network.client"), "Signed bundle lacks relay networking entitlement"
        assert entitlements.get("com.apple.security.network.server"), "Signed bundle lacks Dioxus's local edit-channel listener entitlement"
print(f"{args.platform} bundle metadata verified" + ("; signature verified" if args.signed else "; signature and execution not checked"))
