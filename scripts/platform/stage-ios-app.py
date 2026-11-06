#!/usr/bin/env python3
"""Xcode's signing wrapper stages the actual prebuilt Dioxus app, without a UI substitute."""
import os
from pathlib import Path
import plistlib
import shutil

source = Path(os.environ["RUFREN_IOS_APP"]).resolve()
destination = Path(os.environ["TARGET_BUILD_DIR"]) / os.environ["WRAPPER_NAME"]
with (source / "Info.plist").open("rb") as file:
    info = plistlib.load(file)
assert info["CFBundleIdentifier"] == "town.lunar.rufren", "Unexpected app identity"
assert (source / info["CFBundleExecutable"]).is_file(), "Dioxus's compiled executable is missing"
destination.mkdir(parents=True, exist_ok=True)
for child in source.iterdir():
    if child.name in {"Info.plist", "_CodeSignature", "embedded.mobileprovision"}:
        continue
    target = destination / child.name
    if child.is_dir():
        shutil.copytree(child, target, dirs_exist_ok=True, symlinks=True)
    else:
        shutil.copy2(child, target)
executable = Path(os.environ["TARGET_BUILD_DIR"]) / os.environ["EXECUTABLE_PATH"]
assert executable.is_file(), "Xcode executable name must match Dioxus's actual binary"
print("Staged compiled Rufren executable and resources for Xcode development signing.")
