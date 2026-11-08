#!/usr/bin/env python3
"""Transcribe every WAV in the given directories with the pinned local Qwen3-ASR (expected text withheld).

  .local/qwen3-asr/venv/bin/python -I tools/asr.py DIR [DIR ...]

Writes DIR/asr.json: {clip: {heard, expected, word_errors, words}}. The clip is trimmed and levelled
first (apple/scripts/audio_level.level), exactly as the app pipeline does, then resampled to 16 kHz.
"""
import json
import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(ROOT / "apple/scripts"))
sys.path.insert(0, str(ROOT / "scripts/speech"))
os.environ.update(HF_HUB_OFFLINE="1", TRANSFORMERS_OFFLINE="1", OMP_NUM_THREADS="2")
from stress import spoken  # noqa: E402


def distance(a, b):
    row = list(range(len(b) + 1))
    for i, x in enumerate(a, 1):
        prev, row[0] = row[0], i
        for j, y in enumerate(b, 1):
            prev, row[j] = row[j], min(row[j] + 1, row[j - 1] + 1, prev + (x != y))
    return row[-1]


def main(dirs):
    from audio_level import decode, level
    from asr_model import MODEL_PATH, verify_install
    import numpy as np
    from scipy.signal import resample_poly
    verify_install()
    import mlx.core as mx
    mx.set_memory_limit(8 * 1024**3); mx.set_cache_limit(1024**3)
    from mlx_audio.stt import load
    model = load(str(MODEL_PATH))
    for d in map(Path, dirs):
        outp = d / "asr.json"
        heard = json.loads(outp.read_text()) if outp.exists() else {}
        for wav in sorted(d.glob("*.wav")):
            if wav.stem in heard:
                continue
            meta_p = wav.with_suffix(".json")
            meta = json.loads(meta_p.read_text()) if meta_p.exists() else {}
            expected = meta.get("expected") or (spoken(meta["source_text"]) if "source_text" in meta else None)
            try:
                pcm, info = level(decode(wav))
            except Exception as e:  # noqa: BLE001
                heard[wav.stem] = {"error": str(e)}
                continue
            x = np.frombuffer(pcm.tobytes(), dtype="<i2").astype(np.float32) / 32768.0
            x = np.concatenate([np.zeros(4000, np.float32), resample_poly(x, 2, 3).astype(np.float32), np.zeros(4000, np.float32)])
            text = model.generate(x, language="Russian", temperature=0.0, max_tokens=128, batch_size=1, verbose=False,
                                  system_prompt=None, hotwords=None).text
            got = spoken(text)
            entry = {"heard": text, "heard_norm": got, "expected": expected, "speech_seconds": info["speech_seconds"]}
            if expected:
                e, g = expected.split(), got.split()
                entry["word_errors"] = distance(e, g)
                entry["words"] = len(e)
            heard[wav.stem] = entry
            print(d.name, wav.stem, "|", expected, "=>", got, flush=True)
            outp.write_text(json.dumps(heard, ensure_ascii=False, indent=1) + "\n")
        outp.write_text(json.dumps(heard, ensure_ascii=False, indent=1) + "\n")


if __name__ == "__main__":
    main(sys.argv[1:])
