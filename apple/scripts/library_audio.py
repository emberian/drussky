#!/usr/bin/env python3
"""Add human-recorded library sentences (Tatoeba) to an audio directory.

  python3 apple/scripts/library_audio.py <takes.json> [--audio-dir DIR]

takes.json: list of {ru, key, file, wav, speaker, license, audio_id, id, attribution_url} where `wav` is
the take already trimmed and levelled by audio_level.level (see .local/drussky-library/tools/), `file` the
m4a name (make_audio.slug(key) + ".m4a"). Default audio dir: Drussky/Resources/Audio; the private pack
uses PrivatePack/Audio.

Only ADDS: a key already in index.json keeps its mapping, an existing m4a is never overwritten.
Every clip gets a sources.json entry with "set": "library-sentences", which make_audio.py preserves.
"""
import json
import subprocess
import sys
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
SET = "library-sentences"


def encode(wav, m4a):
    tmp = m4a.with_suffix(".tmp.m4a")
    subprocess.run(["afconvert", "-f", "m4af", "-d", "aac", "-b", "64000", str(wav), str(tmp)], check=True)
    tmp.replace(m4a)


def main():
    takes = json.loads(Path(sys.argv[1]).read_text())
    audio = Path(sys.argv[sys.argv.index("--audio-dir") + 1]) if "--audio-dir" in sys.argv else APPLE / "Drussky/Resources/Audio"
    audio.mkdir(parents=True, exist_ok=True)
    ip, sp = audio / "index.json", audio / "sources.json"
    index = json.loads(ip.read_text()) if ip.exists() else {}
    sources = json.loads(sp.read_text()) if sp.exists() else {}
    added = skipped = 0
    for t in takes:
        f = t["file"]
        if f in sources and sources[f].get("set") != SET:
            raise SystemExit(f"{f} already belongs to another clip")
        if not (audio / f).exists():
            encode(t["wav"], audio / f)
        if t["key"] not in index:
            index[t["key"]] = f
            added += 1
        else:
            skipped += 1
        sources[f] = {
            "key": t["key"], "kind": "human_recording", "set": SET, "provider": "Tatoeba",
            "title": f"Tatoeba sentence #{t['id']}, audio #{t['audio_id']}", "text": t["ru"],
            "author": t["speaker"], "speaker": t["speaker"], "license": t["license"],
            "credit": f"Recording by {t['speaker']} on Tatoeba ({t['license']})"
                      + (f", {t['attribution_url']}" if t.get("attribution_url") else ""),
            "source_page": f"https://tatoeba.org/en/sentences/show/{t['id']}",
            "audio_url": f"https://tatoeba.org/en/audio/download/{t['audio_id']}",
            "processing": "trimmed and levelled with scripts/audio_level.py, AAC 64 kbps; text confirmed by local Qwen3-ASR",
        }
    ip.write_text(json.dumps(dict(sorted(index.items())), ensure_ascii=False, indent=1) + "\n")
    sp.write_text(json.dumps(dict(sorted(sources.items())), ensure_ascii=False, indent=1) + "\n")
    print(f"{len(takes)} clips in {audio}: {added} new index keys, {skipped} keys already mapped (clip kept, index untouched)")


if __name__ == "__main__":
    main()
