#!/usr/bin/env python3
"""Run every content validator: the course, the library and its shelves, the serials, morphology, listening.

    python3 apple/scripts/check_content.py

Exits non-zero if any fails; prints each one's last lines. The decoder side of the same files is
DrusskyTests (testCourseDecodesAndIsWellFormed, NarrationTests, ShelvesTests).
"""
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
RESOURCES = HERE.parent / "Drussky" / "Resources"

CHECKS = [
    ("course", [sys.executable, str(HERE / "validate_course.py")]),
    ("library", [sys.executable, str(HERE / "validate_library.py")]),
    ("math shelf", [sys.executable, str(HERE / "validate_math.py")]),
    ("math shelf (library schema)", [sys.executable, str(HERE / "validate_library.py"), str(RESOURCES / "math.json")]),
    ("math validator tests", [sys.executable, "-m", "unittest", "discover", "-s", str(HERE), "-p", "test_validate_math.py"]),
    ("series", [sys.executable, str(HERE / "validate_series.py")]),
    ("kitchen", [sys.executable, str(HERE / "validate_series.py"), str(RESOURCES / "kitchen.json")]),
    ("morphology", [sys.executable, str(HERE / "validate_morphology.py")]),
    ("listening", [sys.executable, str(HERE / "validate_listening.py")]),
]


def main() -> int:
    failed = []
    for name, cmd in CHECKS:
        r = subprocess.run(cmd, capture_output=True, text=True, cwd=HERE.parent.parent)
        tail = "\n".join((r.stdout + r.stderr).strip().splitlines()[-2:])
        mark = "ok  " if r.returncode == 0 else "FAIL"
        print(f"{mark} {name}: {tail.replace(chr(10), ' | ')}")
        if r.returncode != 0:
            failed.append(name)
    if failed:
        print(f"\n{len(failed)} failed: {', '.join(failed)}", file=sys.stderr)
        return 1
    print("\nall content checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
