#!/usr/bin/env python3
"""Normalize the user-facing name in the actual Dioxus-generated bundle.

CLI 0.7.10 derives its display name from the Rust executable (RufrenApp), ignoring
application.name here. Preserve generated executable/version/platform metadata;
modify only names, before any signature is applied.
"""
import argparse
from pathlib import Path
import plistlib
import shutil

parser = argparse.ArgumentParser()
parser.add_argument("platform", choices=["macos", "ios"])
parser.add_argument("output", type=Path, help="Dioxus platform directory or exact .app path")
parser.add_argument("--ios-environment", choices=["device", "simulator"], default="device")
args = parser.parse_args()
offset = Path("Contents/Info.plist" if args.platform == "macos" else "Info.plist")
candidates = [args.output] if args.output.suffix == ".app" else list(args.output.glob("*.app"))
matches = []
for bundle in candidates:
    metadata = bundle / offset
    if not metadata.is_file():
        continue
    with metadata.open("rb") as file:
        info = plistlib.load(file)
    if info.get("CFBundleIdentifier") == "town.lunar.rufren" and info.get("CFBundleExecutable") == "rufren-app":
        matches.append((bundle, metadata, info))
if len(matches) != 1:
    raise SystemExit(f"Expected one Rufren app in {args.output}; found {len(matches)}. Pass the exact newly built .app path.")
bundle, metadata, info = matches[0]
info["CFBundleDisplayName"] = "Drussky"
info["CFBundleName"] = "Drussky"
if args.platform == "macos":
    icon = Path(__file__).resolve().parents[2] / "crates/rufren-app/platform/apple/icon.icns"
    if not icon.is_file():
        raise SystemExit("Native icon missing; run scripts/platform/generate-macos-icon.py on macOS.")
    resources = bundle / "Contents/Resources"
    resources.mkdir(exist_ok=True)
    shutil.copyfile(icon, resources / "icon.icns")
    info["CFBundleIconFile"] = "icon.icns"
else:
    icons = Path(__file__).resolve().parents[2] / "crates/rufren-app/platform/apple/ios-icons"
    if not (icons / "AppIcon76x76@2x.png").is_file():
        raise SystemExit("Native iOS icons missing; run generate-ios-icons.py.")
    for icon in icons.glob("*.png"):
        shutil.copyfile(icon, bundle / icon.name)
    info["CFBundleIcons"] = {"CFBundlePrimaryIcon": {"CFBundleIconFiles": ["AppIcon60x60"], "UIPrerenderedIcon": False}}
    info["CFBundleIcons~ipad"] = {"CFBundlePrimaryIcon": {"CFBundleIconFiles": ["AppIcon60x60", "AppIcon76x76", "AppIcon83.5x83.5"], "UIPrerenderedIcon": False}}
    # CLI 0.7.10 hardcodes [iPhoneOS, iPadOS], including for simulator builds.
    info["CFBundleSupportedPlatforms"] = ["iPhoneSimulator" if args.ios_environment == "simulator" else "iPhoneOS"]
    info["CFBundlePackageType"] = "APPL"
    info["CFBundleInfoDictionaryVersion"] = "6.0"
with metadata.open("wb") as file:
    plistlib.dump(info, file)
print(bundle.resolve())
