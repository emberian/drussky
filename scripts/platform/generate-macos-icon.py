#!/usr/bin/env python3
"""Generate the native ICNS from the same code-native SVG as the web icons."""
from pathlib import Path
import shutil
import subprocess
import tempfile

project = Path(__file__).resolve().parents[2]
svg = project / "crates/rufren-app/public/icons/icon.svg"
output = project / "crates/rufren-app/platform/apple/icon.icns"
converter = shutil.which("rsvg-convert")
iconutil = Path("/usr/bin/iconutil")
if not converter or not iconutil.exists():
    raise SystemExit("Regenerating ICNS requires librsvg and macOS iconutil; the generated icon is checked in.")
with tempfile.TemporaryDirectory(prefix="rufren-icon-") as temporary:
    iconset = Path(temporary) / "Rufren.iconset"
    iconset.mkdir()
    for size in [16, 32, 128, 256, 512]:
        for scale in [1, 2]:
            filename = f"icon_{size}x{size}{'@2x' if scale == 2 else ''}.png"
            subprocess.run([converter, "--width", str(size * scale), "--height", str(size * scale), str(svg), "--output", str(iconset / filename)], check=True)
    subprocess.run([str(iconutil), "--convert", "icns", "--output", str(output), str(iconset)], check=True)
print(output)
