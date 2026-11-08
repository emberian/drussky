#!/usr/bin/env python3
"""Silero clips for the listening strand (Drussky/Resources/listening.json), in the voices the
items ask for.

  .local/silero-tts/venv/bin/python apple/scripts/make_listening_audio.py [--plan]

Voice assignment, mirrored by `ListeningItem.voice(forLine:)` in Model/Listening.swift:
  voice_hint any         xenia (the course voice);  an exchange: A xenia, B aidar
  voice_hint unfamiliar  aidar;                      an exchange: A aidar, B baya
Crucial spans, scene labels and object choices (played only after the answer) are xenia.

A text whose key has no clip yet gets one in its assigned voice as the primary entry of
Audio/index.json (file `listening-<voice>-<slug>.m4a`, sources "set": "listening"). A text that
already has a primary clip keeps it only when that clip is in the assigned voice: for xenia, a
course clip Silero made with xenia (sources.json "voice": "xenia") serves; a human recording, a
pre-Silero clip or another voice's clip does not, so a page of xenia items never borrows a third
voice for its spans and choices. Anything else gets the assigned voice's clip as a variant in
Audio/variants-listening.json — the same mechanism variants.json uses for other speakers of one
word — and the app picks the speaker's clip by the `listening-<voice>-` file prefix
(`Voice.recordingURL(for:voice:)`, which for xenia falls back to the primary this script has
vouched for). Everything goes through make_morphology_audio.generate (the shared Silero + encode
pipeline); index/sources/variants files are re-read right before writing and no existing entry is
changed.
"""
import json
import sys
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(APPLE / "scripts"))
from make_audio import AUDIO  # noqa: E402
from make_morphology_audio import generate  # noqa: E402
from validate_course import audio_key  # noqa: E402

LISTENING = APPLE / "Drussky/Resources/listening.json"
VARIANTS = AUDIO / "variants-listening.json"
SOURCES = AUDIO / "sources.json"


def voice(item, line=None):
    unfamiliar = item["voice_hint"] == "unfamiliar"
    if item["kind"] in ("who_needs", "disagreement"):
        speaker = (line or {}).get("speaker", "A")
        return ("aidar" if unfamiliar else "xenia") if speaker == "A" else ("baya" if unfamiliar else "aidar")
    return "aidar" if unfamiliar else "xenia"


def assignments(data):
    """(text, voice) for every playable string, first assignment per (key, voice) kept."""
    out, seen = [], set()

    def want(text, v):
        k = (audio_key(text), v)
        if k not in seen:
            seen.add(k)
            out.append((text, v))

    for s in data["sets"] + data["weekly_samples"]:
        for it in s["items"]:
            if "lines" in it:
                for line in it["lines"]:
                    want(line["ru"], voice(it, line))
            elif "forms" in it:
                for f in it["forms"]:
                    want(f["ru"], voice(it))
            else:
                want(it["ru"], voice(it))
            for e in it.get("scene", []):
                want(e["ru"], "xenia")
            if it["kind"] == "pick_object":
                for o in it["options"]:
                    want(o, "xenia")
            want(it["crucial_span"], "xenia")
            for q in it.get("questions", []):
                want(q["crucial_span"], "xenia")
    return out


def served(key, v, index, variants, sources):
    """Whether `key` already has a clip in voice `v`: a listening clip of that voice, or (xenia,
    the course voice) a course clip Silero made with xenia. Mirrored by the xenia fallback in
    `Voice.recordingURL(for:voice:)`, which this guarantee makes safe."""
    files = [index[key]] + variants.get(key, [])
    if any(f.startswith(f"listening-{v}-") for f in files):
        return True
    return v == "xenia" and sources.get(index[key], {}).get("voice") == "xenia"


def main():
    plan = "--plan" in sys.argv
    data = json.loads(LISTENING.read_text())
    index = json.loads((AUDIO / "index.json").read_text())
    sources = json.loads(SOURCES.read_text())
    variants = json.loads(VARIANTS.read_text()) if VARIANTS.exists() else {}
    primary, variant = {}, {}  # voice -> [text]
    kept = 0
    for text, v in assignments(data):
        k = audio_key(text)
        if k not in index:
            primary.setdefault(v, []).append(text)
        elif served(k, v, index, variants, sources):
            kept += 1
        else:
            variant.setdefault(v, []).append(text)
    print(f"{kept} already served; primaries to make: " + ", ".join(f"{v} {len(t)}" for v, t in sorted(primary.items()))
          + "; variants to make: " + (", ".join(f"{v} {len(t)}" for v, t in sorted(variant.items())) or "none"), flush=True)
    for v, texts in sorted(primary.items()):
        generate(texts, voice=v, tag="listening", plan=plan)
    for v, texts in sorted(variant.items()):
        entries = generate(texts, voice=v, tag="listening", plan=plan, variants=True)
        if plan:
            continue
        # Append only: the file is re-read here; other tools own the rest of these files.
        variants = json.loads(VARIANTS.read_text()) if VARIANTS.exists() else {}
        sources = json.loads(SOURCES.read_text())
        added = 0
        for k, (f, entry) in entries.items():
            if f in variants.get(k, []):
                continue
            variants.setdefault(k, []).append(f)
            sources.setdefault(f, entry)
            added += 1
        VARIANTS.write_text(json.dumps(dict(sorted(variants.items())), ensure_ascii=False, indent=1) + "\n")
        SOURCES.write_text(json.dumps(dict(sorted(sources.items())), ensure_ascii=False, indent=1) + "\n")
        print(f"appended {added} {v} variant(s) to {VARIANTS.name}")


if __name__ == "__main__":
    main()
