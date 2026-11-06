#!/usr/bin/env python3
"""Generate opaque iPhone/iPad icons from the shared SVG, without an asset catalog."""
from pathlib import Path
import shutil
import subprocess

project = Path(__file__).resolve().parents[2]
converter = shutil.which("rsvg-convert")
if not converter:
    raise SystemExit("Regenerating iOS PNG icons requires librsvg; generated files are checked in.")
svg = project / "crates/rufren-app/public/icons/icon.svg"
icons = project / "crates/rufren-app/platform/apple/ios-icons"
icons.mkdir(exist_ok=True)
for name, size in [("AppIcon60x60@2x", 120), ("AppIcon60x60@3x", 180), ("AppIcon76x76", 76), ("AppIcon76x76@2x", 152), ("AppIcon83.5x83.5@2x", 167)]:
    subprocess.run([converter, "--background-color", "#203e3b", "--width", str(size), "--height", str(size), str(svg), "--output", str(icons / f"{name}.png")], check=True)
print("Generated iPhone and iPad icon derivatives.")
