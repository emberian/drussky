#!/usr/bin/env python3
"""Every Russian string the Word-families / Cognates pages can speak, one per line, from morphology.json.

  python3 apple/scripts/morphology_strings.py > /tmp/morph-strings.txt

Feed the list to make_morphology_audio.py (new clips) or audio_stale.py (clips whose stress changed).
Lemma headwords, their example sentences, cognate headwords, and the prefix/suffix/rule example words.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "Drussky/Resources"


def strings(m):
    out = []
    for lemma in m["lemmas"].values():
        out.append(lemma["ru"])
        ex = lemma.get("example")
        if isinstance(ex, (list, tuple)) and ex:
            out.append(ex[0])
        elif isinstance(ex, dict) and ex.get("ru"):
            out.append(ex["ru"])
        elif isinstance(ex, str):
            out.append(ex)
    out += [c["ru"] for c in m["cognates"]]
    for group in ("prefixes", "suffixes", "rules"):
        for item in m[group]:
            out += item.get("examples", [])
    seen, uniq = set(), []
    for s in out:
        if s and s not in seen:
            seen.add(s)
            uniq.append(s)
    return uniq


if __name__ == "__main__":
    print("\n".join(strings(json.loads((ROOT / "morphology.json").read_text()))))
