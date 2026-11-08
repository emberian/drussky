#!/usr/bin/env python3
"""Every Russian string the shelves speak (series.json episodes, math.json readings, kitchen.json),
one per line: titles, whole passages, "how it is read" frames and notation readings.

  python3 apple/scripts/shelf_strings.py > /tmp/shelf-strings.txt
  .local/silero-tts/venv/bin/python apple/scripts/make_morphology_audio.py /tmp/shelf-strings.txt --set=shelves
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "Drussky/Resources"


def strings():
    out = []
    for name in ("series.json", "kitchen.json"):
        p = ROOT / name
        if p.exists():
            for serial in json.loads(p.read_text())["series"]:
                out.append(serial["title"])
                for ep in serial["episodes"]:
                    out += [ep["title"], ep["text"]]
    p = ROOT / "math.json"
    if p.exists():
        for t in json.loads(p.read_text())["texts"]:
            out += [t.get("title", ""), t.get("text", "")]
            out += [f["ru"] for f in t.get("how_read", []) if f.get("ru")]
            out += [f["ru"] for f in t.get("notation", []) if f.get("ru")]
    seen, uniq = set(), []
    for s in out:
        s = s.strip()
        if s and s not in seen:
            seen.add(s)
            uniq.append(s)
    return uniq


if __name__ == "__main__":
    print("\n".join(strings()))
