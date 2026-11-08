#!/usr/bin/env python3
"""Generate audition clips with an MLX-Audio model (VoxCPM2 or OmniVoice).

  VENV=.local/tts-2026/mlx/venv/bin/python
  $VENV tools/gen_mlx.py --model voxcpm2 design                      # 3 narrator candidates (voice design)
  $VENV tools/gen_mlx.py --model voxcpm2 items --ref narrator.wav --notation acute
  $VENV tools/gen_mlx.py --model omnivoice items --ref narrator.wav --notation acute
  $VENV tools/gen_mlx.py --model voxcpm2 texts --ref narrator.wav --texts shadow.json --out shadow/voxcpm2

Every clip gets a same-basename JSON with the exact input, settings, timings and peak memory.
"""
import argparse
import json
import os
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
STATE = ROOT / ".local/tts-2026"
sys.path.insert(0, str(HERE))
from stress import convert, spoken  # noqa: E402

for key in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "VECLIB_MAXIMUM_THREADS", "MKL_NUM_THREADS"):
    os.environ.setdefault(key, "4")
os.environ.update(HF_HOME=str(STATE / "hf-cache"), HF_HUB_OFFLINE="1", TRANSFORMERS_OFFLINE="1", TOKENIZERS_PARALLELISM="false")

MODELS = {
    "voxcpm2": {"path": STATE / "models/VoxCPM2-bf16", "id": "mlx-community/VoxCPM2-bf16",
                "settings": {"inference_timesteps": 24, "cfg_value": 2.0}},
    "omnivoice": {"path": ROOT / ".local/omnivoice-audition/model", "id": "mlx-community/OmniVoice-bf16",
                  "settings": {"language": "ru", "num_steps": 32, "guidance_scale": 2.0}},
}
SEED = 42


def load(name):
    import mlx.core as mx
    from mlx_audio.tts.utils import load_model
    mx.set_memory_limit(20 * 1024**3)
    t = time.monotonic()
    model = load_model(str(MODELS[name]["path"]))
    return model, time.monotonic() - t


def synth(model, name, text, ref=None, ref_text=None, instruct=None, seed=SEED):
    import mlx.core as mx
    import numpy as np
    kw = dict(MODELS[name]["settings"])
    if instruct:
        kw["instruct"] = instruct
    if ref:
        kw["ref_audio"] = str(ref)
        if name == "omnivoice":
            kw["ref_text"] = ref_text
    mx.random.seed(seed)
    t = time.monotonic()
    results = list(model.generate(text=text, **kw))
    gen = time.monotonic() - t
    audio = np.concatenate([np.asarray(r.audio, dtype=np.float32).reshape(-1) for r in results])
    sr = results[0].sample_rate
    peak = max(getattr(r, "peak_memory_usage", 0) or 0 for r in results)
    return audio, sr, {"generation_seconds": round(gen, 3), "audio_seconds": round(len(audio) / sr, 3),
                       "sample_rate": sr, "peak_memory_gb": round(float(peak), 3), "settings": kw, "seed": seed,
                       "peak_abs": round(float(np.abs(audio).max()), 4)}


def write(out, stem, audio, sr, meta):
    import soundfile as sf
    out.mkdir(parents=True, exist_ok=True)
    sf.write(out / f"{stem}.wav", audio, sr, subtype="PCM_16")
    (out / f"{stem}.json").write_text(json.dumps(meta, ensure_ascii=False, indent=1) + "\n")
    print(f"  {stem}: {meta['audio_seconds']}s audio in {meta['generation_seconds']}s", flush=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", choices=MODELS, required=True)
    ap.add_argument("stage", choices=["design", "items", "texts"])
    ap.add_argument("--ref", type=Path, help="narrator reference WAV (cloning)")
    ap.add_argument("--notation", default="acute", choices=["acute", "plain", "plus"])
    ap.add_argument("--texts", type=Path, help="JSON list of {id, text} for stage texts")
    ap.add_argument("--out", type=Path)
    ap.add_argument("--only", nargs="*")
    args = ap.parse_args()
    items = json.loads((HERE.parent / "items.json").read_text())
    design = items["narrator_design"]
    model, load_s = load(args.model)
    print(f"{args.model} loaded in {load_s:.1f}s", flush=True)
    common = {"model": MODELS[args.model]["id"], "runtime": "mlx-audio 0.6.4 / mlx 0.32.4", "load_seconds": round(load_s, 2)}
    if args.stage == "design":
        out = args.out or HERE.parent / "narrator"
        for n in (1, 2, 3):
            text = convert(design["text"], args.notation)
            audio, sr, meta = synth(model, args.model, text, instruct=design["instruct"], seed=SEED + n)
            write(out, f"narrator-{args.model}-{n}", audio, sr,
                  {**common, "mode": "voice design", "instruct": design["instruct"], "input": text,
                   "source_text": design["text"], "expected": spoken(design["text"]), **meta})
        return
    ref_text = spoken(design["text"]) if args.ref else None
    if args.ref:
        args.ref = args.ref.resolve()
        ref_meta = json.loads(args.ref.with_suffix(".json").read_text())
        ref_text = ref_meta.get("expected", ref_text)
    if args.stage == "items":
        todo = items["core"] + items["stress_words"]
        out = args.out or HERE.parent / "audition" / args.model
    else:
        todo = json.loads(args.texts.read_text())
        out = args.out
    if args.only:
        todo = [t for t in todo if t["id"] in args.only]
    for it in todo:
        if (out / f"{it['id']}.wav").exists():
            continue
        text = convert(it["text"], args.notation)
        try:
            audio, sr, meta = synth(model, args.model, text, ref=args.ref, ref_text=ref_text)
        except Exception as e:  # noqa: BLE001 -- record the failure, keep going
            print(f"  {it['id']}: FAILED {e}", flush=True)
            out.mkdir(parents=True, exist_ok=True)
            (out / f"{it['id']}.json").write_text(json.dumps({**common, "id": it["id"], "error": str(e), "input": text}, ensure_ascii=False, indent=1))
            continue
        write(out, it["id"], audio, sr, {**common, "id": it["id"], "mode": "reference cloning" if args.ref else "zero-shot",
                                         "reference": str(args.ref.relative_to(ROOT)) if args.ref else None, "ref_text": ref_text,
                                         "notation": args.notation, "input": text, "source_text": it["text"],
                                         "expected": spoken(it["text"]), **meta})


if __name__ == "__main__":
    main()
