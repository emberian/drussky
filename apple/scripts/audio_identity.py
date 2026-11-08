#!/usr/bin/env python3
"""Give every bundled clip the identity of what it actually says, intonation included.

The audio index used to be keyed by the answer-normalization key (no punctuation), so a
rising question ("Э́то чай?") and its statement ("Э́то чай.") shared one clip. A clip's key is
now validate_course.audio_key of its own spoken text: the plain key, plus "?" when that text is
a rising question. The text comes from the clip's provenance: the Silero `input`, the library
sentence `text`, or the human recording's Commons/Lingua Libre `title`. A clip without any of
these (foundation WAVs, word recordings) is a statement.

    python3 apple/scripts/audio_identity.py           # re-key index*.json / sources*.json in place
    python3 apple/scripts/audio_identity.py --check   # report only

Idempotent. Only keys change; no clip is touched. Variant tables (other speakers of words) are
left as they are; it refuses to run if a variant clip is a rising question.
"""
import json
import sys
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(APPLE / "scripts"))
from validate_course import rising_question  # noqa: E402

AUDIO = APPLE / "Drussky/Resources/Audio"


def spoken_text(entry):
    if entry.get("input"):
        return entry["input"]
    if entry.get("text"):
        return entry["text"]
    if entry.get("kind") == "human_recording" and entry.get("title"):
        # "LL-Q7737 (rus)-Speaker-вы любите музыку ?.wav" -> "вы любите музыку ?"
        return entry["title"].rsplit(".", 1)[0].split("-")[-1]
    return None


def clip_key(plain, entry):
    return plain + ("?" if rising_question(spoken_text(entry)) else "")


def main():
    check = "--check" in sys.argv
    changed = 0
    for ip in sorted(AUDIO.glob("index*.json")):
        sp = AUDIO / ip.name.replace("index", "sources", 1)
        index = json.loads(ip.read_text())
        sources = json.loads(sp.read_text()) if sp.exists() else {}
        out = {}
        for k, f in index.items():
            plain = k.rstrip("?")
            nk = clip_key(plain, sources.get(f, {}))
            if nk != k:
                print(f"{ip.name}: {k!r} -> {nk!r} ({spoken_text(sources.get(f, {}))!r})")
                changed += 1
            if nk in out and out[nk] != f:
                raise SystemExit(f"{ip.name}: two clips for {nk!r}: {out[nk]} and {f}")
            out[nk] = f
            if f in sources:
                sources[f]["key"] = nk
        vp = AUDIO / ip.name.replace("index", "variants", 1)
        if vp.exists():
            for k, files in json.loads(vp.read_text()).items():
                for f in files:
                    if rising_question(spoken_text(sources.get(f, {}))):
                        raise SystemExit(f"{vp.name}: variant {f} of {k!r} is a question")
        if not check:
            ip.write_text(json.dumps(dict(sorted(out.items())), ensure_ascii=False, indent=1) + "\n")
            if sp.exists():
                sp.write_text(json.dumps(dict(sorted(sources.items())), ensure_ascii=False, indent=1) + "\n")
    print(f"{changed} key(s) {'would change' if check else 'changed'}")


if __name__ == "__main__":
    main()
