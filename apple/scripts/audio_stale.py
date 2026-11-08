#!/usr/bin/env python3
"""Find synthetic clips whose stress no longer matches the text they are filed under.

  python3 apple/scripts/audio_stale.py [STRINGS.txt …] [--remove]

The audio key strips stress, so when an editor corrects пла́нета → плане́та the key stays the same
and `make_audio.py --missing` sees nothing to do — but a Silero clip was synthesised from the old
stressed text and now says the wrong thing. sources.json records the exact `input` each synthetic
clip was made from; this compares it with what the current course (and any extra string lists,
e.g. the morphology strings) would synthesise today. Human recordings are never stale: the speaker
knew the stress. With --remove, the stale clip, its index entry and its sources entry are deleted so
`make_audio.py --missing` / `make_morphology_audio.py` regenerate them.
"""
import json
import sys
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(APPLE / "scripts"))
from make_audio import AUDIO, COURSE, SYNTH_FINAL, SYNTH_OVERRIDES, silero_text  # noqa: E402
from validate_course import audio_key, spoken_strings  # noqa: E402


def wanted_inputs(string_files):
    course = json.loads(COURSE.read_text())
    wanted = {}
    for s in spoken_strings(course):
        wanted.setdefault(audio_key(s), s)
    for f in string_files:
        for line in Path(f).read_text().splitlines():
            line = line.strip()
            if line and not line.startswith("#"):
                wanted.setdefault(audio_key(line), line)
    return {k: (SYNTH_FINAL.get(k) or silero_text(SYNTH_OVERRIDES.get(k, s))) for k, s in wanted.items()}


def main():
    files = [a for a in sys.argv[1:] if not a.startswith("--")]
    index = json.loads((AUDIO / "index.json").read_text())
    sources = json.loads((AUDIO / "sources.json").read_text())
    want = wanted_inputs(files)
    stale = []
    for k, synth in want.items():
        f = index.get(k)
        src = sources.get(f) if f else None
        if not src or src.get("kind") != "ai_generated_speech" or "input" not in src:
            continue
        if src["input"].lower() != synth.lower():  # capitalisation does not change what Silero says
            stale.append((k, f, src["input"], synth))
    for k, f, old, new in stale:
        print(f"{f}: {old!r} -> {new!r}")
    print(f"{len(stale)} stale synthetic clip(s) of {len(want)} wanted keys")
    if "--remove" in sys.argv and stale:
        for k, f, _, _ in stale:
            (AUDIO / f).unlink(missing_ok=True)
            index.pop(k, None)
            sources.pop(f, None)
        (AUDIO / "index.json").write_text(json.dumps(dict(sorted(index.items())), ensure_ascii=False, indent=1) + "\n")
        (AUDIO / "sources.json").write_text(json.dumps(dict(sorted(sources.items())), ensure_ascii=False, indent=1) + "\n")
        print(f"removed {len(stale)}; now run make_audio.py --missing (and make_morphology_audio.py for morphology strings)")


if __name__ == "__main__":
    main()
