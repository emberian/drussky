#!/usr/bin/env python3
"""Bounded, read-only SDK inventory. Does not invoke xcrun/simctl or unlock keys."""
import json
import os
from pathlib import Path
import plistlib
import re
import subprocess

home = Path.home()
xcode = Path(os.environ.get("DEVELOPER_DIR", "/Applications/Xcode.app/Contents/Developer"))
android = Path(os.environ.get("ANDROID_HOME", home / "Library/Android/sdk"))

def children(directory):
    return sorted(p.name for p in directory.iterdir()) if directory.is_dir() else []

def version(command):
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=10)
        return (result.stdout + result.stderr).strip().splitlines()[0]
    except (OSError, subprocess.TimeoutExpired, IndexError):
        return "unavailable"

identities = "not queried"
if Path("/usr/bin/security").exists():
    try:
        result = subprocess.run(["/usr/bin/security", "find-identity", "-v", "-p", "codesigning"], capture_output=True, text=True, timeout=10)
        match = re.search(r"(\d+) valid identities found", result.stdout)
        identities = int(match.group(1)) if match else "unavailable"
    except subprocess.TimeoutExpired:
        identities = "query timed out"

runtimes = []
volumes = Path("/Library/Developer/CoreSimulator/Volumes")
for volume in children(volumes):
    runtime_dir = volumes / volume / "Library/Developer/CoreSimulator/Profiles/Runtimes"
    runtimes.extend(children(runtime_dir))
avd = home / ".android/avd"
report = {
    "dioxus_project_cli": version([str(Path(__file__).resolve().parents[2] / ".tools/dx"), "--version"]),
    "xcode": version([str(xcode / "usr/bin/xcodebuild"), "-version"]),
    "apple_sdks": {platform: children(xcode / "Platforms" / f"{platform}.platform/Developer/SDKs") for platform in ["MacOSX", "iPhoneOS", "iPhoneSimulator"]},
    "mounted_simulator_runtime_bundles": sorted(runtimes),
    "codesigning_identity_count": identities,
    "android_platforms": children(android / "platforms"),
    "android_build_tools": children(android / "build-tools"),
    "android_ndks": children(android / "ndk"),
    "android_virtual_devices": [name for name in children(avd) if name.endswith(".avd")],
    "android_studio_java": version(["/Applications/Android Studio.app/Contents/jbr/Contents/Home/bin/java", "-version"]),
    "rust_installed_targets": subprocess.run(["rustup", "target", "list", "--installed"], capture_output=True, text=True, check=True).stdout.splitlines(),
    "scope": "SDK files and signing identities only; no device execution, private-key access, signing, simulator boot, or account/provisioning changes.",
}
print(json.dumps(report, indent=2))
