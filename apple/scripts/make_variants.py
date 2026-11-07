#!/usr/bin/env python3
"""Build the extra-speaker clips (Audio/variants.json) from scripts/variant_audio.json.

variant_audio.json: key -> [ {orig, title, author, credit, license, source_page, speaker} ], the
additional human takes chosen per word (ASR-checked, quiet, unclipped; see the chooser in
.local/drussky-audio/variants/). Each is trimmed and levelled exactly like the primary clips
(make_audio.to_m4a), written as <translit>-<speaker>-<hash>.m4a, and recorded in sources.json.
index.json (one primary clip per key) is never touched.

  python3 apple/scripts/make_variants.py
"""
import hashlib
import json
import re
import sys
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
REPO = APPLE.parent
sys.path.insert(0, str(APPLE / "scripts"))
from make_audio import AUDIO, TRANSLIT, to_m4a  # noqa: E402

CHOSEN = APPLE / "scripts/variant_audio.json"
FIELDS = ("kind", "title", "author", "credit", "license", "source_page", "speaker")


def translit(s, n):
    s = "".join(TRANSLIT.get(c, c if c.isascii() and c.isalnum() else "-") for c in s.lower())
    return re.sub(r"-+", "-", s)[:n].strip("-") or "x"


def name(k, take):
    return f"{translit(k, 32)}-{translit(take['speaker'], 20)}-{hashlib.sha1(take['title'].encode()).hexdigest()[:6]}.m4a"


def main():
    chosen = json.loads(CHOSEN.read_text())
    index = json.loads((AUDIO / "index.json").read_text())
    sources = json.loads((AUDIO / "sources.json").read_text())
    variants = {}
    for k, takes in sorted(chosen.items()):
        assert k in index, k
        files = []
        for t in takes:
            f = name(k, t)
            assert f != index[k] and f not in files, (k, f)
            if not (AUDIO / f).exists():
                to_m4a(REPO / t["orig"], AUDIO / f)
            sources[f] = {"key": k, "kind": "human_recording", **{x: t[x] for x in FIELDS[1:] if t.get(x)}}
            files.append(f)
        variants[k] = files
    (AUDIO / "variants.json").write_text(json.dumps(variants, ensure_ascii=False, indent=1) + "\n")
    (AUDIO / "sources.json").write_text(json.dumps(dict(sorted(sources.items())), ensure_ascii=False, indent=1) + "\n")
    print(f"{len(variants)} keys, {sum(map(len, variants.values()))} extra clips")


if __name__ == "__main__":
    main()
