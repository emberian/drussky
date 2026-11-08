#!/usr/bin/env python3
"""Control condition: the current production voice (Silero v5_6_ru xenia) through apple/scripts/audio_store.Silero.

  .local/silero-tts/venv/bin/python tools/gen_silero.py items
  .local/silero-tts/venv/bin/python tools/gen_silero.py texts --texts shadow/shadow.json --out shadow/silero
"""
import argparse
import json
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(ROOT / "apple/scripts"))
from stress import spoken  # noqa: E402
import audio_store as store  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("stage", choices=["items", "texts"])
    ap.add_argument("--texts", type=Path)
    ap.add_argument("--out", type=Path)
    args = ap.parse_args()
    items = json.loads((HERE.parent / "items.json").read_text())
    todo = items["core"] + items["stress_words"] if args.stage == "items" else json.loads(args.texts.read_text())
    out = args.out or HERE.parent / "audition" / "silero"
    out.mkdir(parents=True, exist_ok=True)
    silero = store.Silero(threads=4)
    for it in todo:
        if (out / f"{it['id']}.wav").exists():
            continue
        t = time.monotonic()
        data, frames = silero.wav(it["text"], "xenia")
        gen = time.monotonic() - t
        (out / f"{it['id']}.wav").write_bytes(data)
        meta = {"model": "Silero v5_6_ru", "voice": "xenia", "id": it["id"], "input": it["text"], "source_text": it["text"],
                "expected": spoken(it["text"]), "notation": "plus", "synth": store.SILERO_SETTINGS,
                "generation_seconds": round(gen, 3), "audio_seconds": round(frames / 24000, 3), "sample_rate": 24000}
        (out / f"{it['id']}.json").write_text(json.dumps(meta, ensure_ascii=False, indent=1) + "\n")
        print(f"  {it['id']}: {meta['audio_seconds']}s audio in {meta['generation_seconds']}s", flush=True)


if __name__ == "__main__":
    main()
