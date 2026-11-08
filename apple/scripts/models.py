#!/usr/bin/env python3
"""The phone recognizer is not in git: this downloads the upstream model and builds the Core ML one.

  python3 apple/scripts/models.py status
  python3 apple/scripts/models.py fetch [--cache-root DIR] [--precision fp16|w8] [--force]

`fetch` puts the compiled model in apple/Drussky/Resources/Models/ (gitignored), which the app
bundles as a folder; without it the app falls back to word-level comparison. Steps:

  1. a uv venv at <cache-root>/pronunciation/venv with pinned torch / onnx / coremltools
  2. ZIPA small CR-CTC (anyspeech/zipa-small-crctc-ns-700k, CC BY-NC-SA 4.0; Zhu et al. 2025,
     "ZIPA: a family of efficient models for multilingual phone recognition") at a pinned
     revision, SHA-256 checked: model.onnx (fp32, 260 MB) and tokens.txt
  3. onnx_torch.py traces the ONNX graph at a fixed window of WINDOW_FRAMES fbank frames
     (10 s) with a runtime length input, coremltools converts it to an ML program (fp16
     compute; --precision w8 also quantizes weights to 8-bit, per channel)
  4. xcrun coremlcompiler compiles it to ZipaPhones.mlmodelc, and ZipaPhones.json records the
     token list, framing and provenance.

Model input: `features` (1, 1000, 80) Kaldi fbank as lhotse computes it (25 ms Povey window,
10 ms hop, 80 mel bins 20-7600 Hz, no dither, snip_edges false; Swift computes it in
Pronunciation/Fbank.swift) and `length` (1,) int32, the number of real frames (the rest is
masked padding). Output `log_probs` (1, 496, 127): CTC log posteriors, one frame per 20 ms,
the first (length - 7) / 2 valid. Default --cache-root is <repo>/.local (in a worktree, point it at the main
checkout's .local to reuse the download).
"""
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
REPO = APPLE.parent
DEST = APPLE / "Drussky/Resources/Models"
NAME = "ZipaPhones"
WINDOW_FRAMES = 1000
REPO_ID = "anyspeech/zipa-small-crctc-ns-700k"
REVISION = "797523ffb49e31e821a972313813cc4a26d902ef"
FILES = {
    "model.onnx": "6ae923a80103809041d7b3d53f1bff5215690c598b110a8ad4d1f402a8811a7a",
    "tokens.txt": "f8e042a0c9130532b22d03ec7cae2f75a23fbec70c450c31a8efb51787b2b8fe",
}
PINS = ["torch==2.7.0", "onnx==1.23.2", "coremltools==9.0", "numpy<2.4"]


def sha256(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def venv_python(cache):
    return cache / "pronunciation/venv/bin/python"


def ensure_venv(cache):
    py = venv_python(cache)
    if py.exists():
        return py
    uv = shutil.which("uv")
    if not uv:
        sys.exit("needs uv (brew install uv) to build the conversion venv")
    subprocess.run([uv, "venv", "--python", "3.12", str(py.parents[1])], check=True)
    subprocess.run([uv, "pip", "install", "--python", str(py), *PINS], check=True)
    return py


def download(cache):
    d = cache / "pronunciation/hf" / REPO_ID.split("/")[1]
    d.mkdir(parents=True, exist_ok=True)
    for name, want in FILES.items():
        p = d / name
        if p.exists() and sha256(p) == want:
            continue
        url = f"https://huggingface.co/{REPO_ID}/resolve/{REVISION}/{name}"
        print(f"downloading {url}")
        tmp = p.with_suffix(p.suffix + ".part")
        with urllib.request.urlopen(url) as r, open(tmp, "wb") as f:
            shutil.copyfileobj(r, f, 1 << 20)
        got = sha256(tmp)
        if got != want:
            tmp.unlink()
            sys.exit(f"{name}: SHA-256 {got} != pinned {want}")
        tmp.replace(p)
    return d


def tokens(src):
    out = {}
    for line in (src / "tokens.txt").read_text().splitlines():
        t, i = line.rsplit(" ", 1)
        out[int(i)] = t
    return [out[i] for i in range(len(out))]


def convert(src, out, precision):
    """Runs inside the venv: ONNX -> traced TorchScript -> Core ML ML program."""
    import numpy as np
    import onnx
    import torch
    import coremltools as ct
    sys.path.insert(0, str(APPLE / "scripts"))
    from onnx_torch import Runner, prepare

    T = WINDOW_FRAMES
    runner = prepare(Runner(onnx.load(str(src / "model.onnx"))).eval(), T)

    class Wrap(torch.nn.Module):
        def __init__(self, r):
            super().__init__()
            self.r = r

        def forward(self, x, x_lens):
            return self.r(x, x_lens.to(torch.int64))[0]

    with torch.no_grad():
        traced = torch.jit.trace(Wrap(runner).eval(), (torch.randn(1, T, 80), torch.tensor([T - 137], dtype=torch.int32)),
                                 check_trace=False)
    ml = ct.convert(traced, inputs=[ct.TensorType(name="features", shape=(1, T, 80), dtype=np.float32),
                                    ct.TensorType(name="length", shape=(1,), dtype=np.int32)],
                    outputs=[ct.TensorType(name="log_probs")], minimum_deployment_target=ct.target.iOS18,
                    compute_precision=ct.precision.FLOAT16, convert_to="mlprogram")
    if precision == "w8":
        from coremltools.optimize.coreml import OpLinearQuantizerConfig, OptimizationConfig, linear_quantize_weights
        ml = linear_quantize_weights(ml, OptimizationConfig(global_config=OpLinearQuantizerConfig(
            mode="linear_symmetric", granularity="per_channel", weight_threshold=2048)))
    ml.short_description = "ZIPA small CR-CTC phone recognizer (anyspeech/zipa-small-crctc-ns-700k), 10 s window"
    ml.save(str(out))


def fetch(args):
    cache = Path(args.cache_root).resolve()
    target = DEST / f"{NAME}.mlmodelc"
    if target.exists() and not args.force:
        print(f"{target.relative_to(APPLE)} already present (--force rebuilds)")
        return
    src = download(cache)
    py = ensure_venv(cache)
    build = cache / "pronunciation/build"
    build.mkdir(parents=True, exist_ok=True)
    pkg = build / f"{NAME}-{args.precision}.mlpackage"
    t0 = time.time()
    subprocess.run([str(py), "-I", str(Path(__file__).resolve()), "_convert", str(src), str(pkg), args.precision], check=True)
    print(f"converted in {time.time() - t0:.0f} s")
    with tempfile.TemporaryDirectory() as tmp:
        subprocess.run(["xcrun", "coremlcompiler", "compile", str(pkg), tmp], check=True, stdout=subprocess.DEVNULL)
        DEST.mkdir(parents=True, exist_ok=True)
        if target.exists():
            shutil.rmtree(target)
        shutil.move(str(Path(tmp) / f"{NAME}-{args.precision}.mlmodelc"), target)
    meta = {
        "model": REPO_ID, "revision": REVISION, "license": "CC BY-NC-SA 4.0",
        "citation": "Zhu, Jian et al. (2025). ZIPA: A family of efficient models for multilingual phone recognition. ACL.",
        "precision": args.precision, "window_frames": WINDOW_FRAMES, "frame_shift_s": 0.01, "output_frame_s": 0.02,
        "features": "lhotse kaldi fbank: 80 mel 20-7600 Hz, 25 ms povey window, 10 ms shift, preemph 0.97, no dither, snip_edges false",
        "blank": 0, "tokens": tokens(src), "built": time.strftime("%Y-%m-%d"),
    }
    (DEST / f"{NAME}.json").write_text(json.dumps(meta, ensure_ascii=False, indent=1) + "\n")
    size = sum(f.stat().st_size for f in target.rglob("*") if f.is_file())
    print(f"installed {target.relative_to(APPLE)} ({size / 1e6:.0f} MB)")


def status(_):
    target = DEST / f"{NAME}.mlmodelc"
    if not target.exists():
        print(f"missing: {target.relative_to(APPLE)} (python3 apple/scripts/models.py fetch)")
        sys.exit(1)
    meta = json.loads((DEST / f"{NAME}.json").read_text())
    size = sum(f.stat().st_size for f in target.rglob("*") if f.is_file())
    print(f"{NAME}: {meta['model']}@{meta['revision'][:8]} {meta['precision']} {size / 1e6:.0f} MB, built {meta['built']}")


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "_convert":
        convert(Path(sys.argv[2]), Path(sys.argv[3]), sys.argv[4])
        return
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    f = sub.add_parser("fetch")
    f.add_argument("--cache-root", default=str(REPO / ".local"))
    f.add_argument("--precision", choices=["fp16", "w8"], default="w8")
    f.add_argument("--force", action="store_true")
    f.set_defaults(fn=fetch)
    sub.add_parser("status").set_defaults(fn=status)
    a = ap.parse_args()
    a.fn(a)


if __name__ == "__main__":
    main()
