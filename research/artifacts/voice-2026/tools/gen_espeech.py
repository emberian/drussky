#!/usr/bin/env python3
"""Generate audition clips with ESpeech-TTS-1 RL-V2 (Russian F5-TTS; explicit '+' stress) on MPS.

  .local/tts-2026/espeech/venv/bin/python tools/gen_espeech.py items --ref narrator/narrator-voxcpm2-1.wav
  .local/tts-2026/espeech/venv/bin/python tools/gen_espeech.py texts --ref ... --texts shadow.json --out shadow/espeech

Loaded exactly as the model card's app does: DiT with the card's MODEL_CFG dict (NOT F5-TTS's F5TTS_Base
yaml, whose text_mask_padding/pe_attn_head differ and yield gibberish), custom vocab, local Vocos. The
model is a voice-cloning model: the reference clip and its stress-marked transcript are required. Our
'+' corpus notation is ESpeech's native stress syntax, so inputs are passed verbatim (no RUAccent).
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
from stress import spoken  # noqa: E402

os.environ.update(HF_HOME=str(STATE / "hf-cache"), HF_HUB_OFFLINE="1", TRANSFORMERS_OFFLINE="1", OMP_NUM_THREADS="4")
MODEL_DIR = STATE / "models/ESpeech-TTS-1_RL-V2"
VOCODER = STATE / "models/vocos-mel-24khz"
MODEL_ID = "ESpeech/ESpeech-TTS-1_RL-V2@f582b6e5897fe8a5835059405a8439d13bdf7684"
MODEL_CFG = dict(dim=1024, depth=22, heads=16, ff_mult=2, text_dim=512, conv_layers=4)  # from the model card
SETTINGS = {"nfe_step": 48, "cfg_strength": 2.0, "speed": 1.0, "sway_sampling_coef": -1.0, "cross_fade_duration": 0.15}
SEED = 42


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("stage", choices=["items", "texts"])
    ap.add_argument("--ref", type=Path, required=True)
    ap.add_argument("--texts", type=Path)
    ap.add_argument("--out", type=Path)
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--device", default="mps")
    ap.add_argument("--speed", type=float, help="override SETTINGS['speed'] (F5-TTS sizes the output from ref length / speed; "
                                                "lower values give very short inputs room)")
    args = ap.parse_args()
    if args.speed:
        SETTINGS["speed"] = args.speed
    args.ref = args.ref.resolve()
    items = json.loads((HERE.parent / "items.json").read_text())
    # The reference transcript keeps its '+' marks and punctuation, as ESpeech's own app feeds it (via RUAccent).
    ref_text = items["narrator_design"]["text"]
    import torch
    import soundfile as sf
    from f5_tts.infer.utils_infer import infer_process, load_model, load_vocoder, preprocess_ref_audio_text
    from f5_tts.model import DiT
    from f5_tts.model.utils import seed_everything
    quiet = lambda *a, **k: None  # noqa: E731
    t = time.monotonic()
    model = load_model(DiT, MODEL_CFG, str(MODEL_DIR / "espeech_tts_rlv2.pt"), vocab_file=str(MODEL_DIR / "vocab.txt"), device=args.device)
    vocoder = load_vocoder("vocos", True, str(VOCODER), device=args.device)
    ref_audio, ref_text_final = preprocess_ref_audio_text(str(args.ref), ref_text, show_info=quiet)
    load_s = time.monotonic() - t
    print(f"espeech loaded in {load_s:.1f}s on {args.device}", flush=True)
    common = {"model": MODEL_ID, "runtime": f"f5-tts 1.1.22 / torch {torch.__version__} {args.device}", "load_seconds": round(load_s, 2),
              "reference": str(args.ref.relative_to(ROOT)), "ref_text": ref_text_final, "notation": "plus", "model_cfg": MODEL_CFG}
    if args.stage == "items":
        todo = items["core"] + items["stress_words"]
        out = args.out or HERE.parent / "audition" / "espeech"
    else:
        todo = json.loads(args.texts.read_text())
        out = args.out
    if args.only:
        todo = [x for x in todo if x["id"] in args.only]
    out.mkdir(parents=True, exist_ok=True)
    for it in todo:
        if (out / f"{it['id']}.wav").exists():
            continue
        text = it["text"]
        try:
            seed_everything(SEED)
            t = time.monotonic()
            wav, sr, _ = infer_process(ref_audio, ref_text_final, text, model, vocoder, show_info=quiet, progress=None,
                                       device=args.device, **SETTINGS)
            gen = time.monotonic() - t
        except Exception as e:  # noqa: BLE001
            print(f"  {it['id']}: FAILED {e}", flush=True)
            (out / f"{it['id']}.json").write_text(json.dumps({**common, "id": it["id"], "error": str(e), "input": text}, ensure_ascii=False, indent=1))
            continue
        sf.write(out / f"{it['id']}.wav", wav, sr, subtype="PCM_16")
        meta = {**common, "id": it["id"], "mode": "reference cloning", "input": text, "source_text": text, "expected": spoken(text),
                "generation_seconds": round(gen, 3), "audio_seconds": round(len(wav) / sr, 3), "sample_rate": sr,
                "settings": SETTINGS, "seed": SEED, "peak_abs": round(float(abs(wav).max()), 4)}
        (out / f"{it['id']}.json").write_text(json.dumps(meta, ensure_ascii=False, indent=1) + "\n")
        print(f"  {it['id']}: {meta['audio_seconds']}s audio in {meta['generation_seconds']}s", flush=True)


if __name__ == "__main__":
    main()
