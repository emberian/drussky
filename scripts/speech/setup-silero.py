#!/usr/bin/env python3
"""Explicit bounded official-model acquisition; no model loading or inference."""
import hashlib
import json
import os
import sys
import urllib.request
from importlib.metadata import version
sys.dont_write_bytecode = True

from silero_model import (
    LICENSE, MANIFEST, MODEL, MODEL_BYTES, MODEL_ID, MODEL_SHA256, MODEL_URL,
    SOURCE_REPO, SOURCE_REVISION, STATE, TORCH_VERSION,
)


def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            result.update(chunk)
    return result.hexdigest()


def download(url, target, maximum, expected_size=None):
    if target.exists():
        if target.is_symlink() or not target.is_file() or target.stat().st_size > maximum:
            raise RuntimeError("Existing task-owned model/source file is invalid")
        return
    temporary = target.with_suffix(target.suffix + ".partial")
    target.parent.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(url, timeout=30) as response, temporary.open("xb") as output:
        declared = response.headers.get("Content-Length")
        if declared and int(declared) > maximum:
            raise RuntimeError("Official artifact exceeds the acquisition limit")
        size = 0
        while block := response.read(1024 * 1024):
            size += len(block)
            if size > maximum:
                raise RuntimeError("Official artifact exceeds the acquisition limit")
            output.write(block)
        if expected_size is not None and size != expected_size:
            raise RuntimeError("Official artifact size differs from the pinned inventory")
        output.flush(); os.fsync(output.fileno())
    os.replace(temporary, target)


def main():
    os.nice(10)
    if version("torch") != TORCH_VERSION:
        raise RuntimeError("Use the independently pinned Silero Python environment")
    STATE.mkdir(parents=True, exist_ok=True, mode=0o700); os.chmod(STATE, 0o700)
    print(f"Acquiring official {MODEL_ID}: {MODEL_BYTES:,} bytes", flush=True)
    download(MODEL_URL, MODEL, MODEL_BYTES, MODEL_BYTES)
    actual = digest(MODEL)
    if MODEL.stat().st_size != MODEL_BYTES or MODEL_SHA256 and actual != MODEL_SHA256:
        raise RuntimeError("Pinned Silero model size/SHA-256 check failed")
    sources = {}
    for name in ("LICENSE", "README.md", "models.yml", "examples_tts.ipynb"):
        target = STATE / "sources" / name
        url = f"https://raw.githubusercontent.com/{SOURCE_REPO}/{SOURCE_REVISION}/{name}"
        download(url, target, 1_000_000)
        sources[name] = {"url": url, "bytes": target.stat().st_size, "sha256": digest(target)}
    manifest = {
        "format": "rufren-silero-model-v1", "model": MODEL_ID, "model_url": MODEL_URL,
        "model_bytes": MODEL_BYTES, "model_sha256": actual, "license": LICENSE,
        "source_repo": SOURCE_REPO, "source_revision": SOURCE_REVISION,
        "runtime": f"torch=={TORCH_VERSION}", "sources": sources,
        "model_code_trust": "Official torch.package artifact; packaged Python code is loaded only after model identity verification",
    }
    temporary = MANIFEST.with_suffix(".partial")
    temporary.write_text(json.dumps(manifest, indent=2) + "\n"); os.replace(temporary, MANIFEST)
    print(json.dumps({"model_bytes": MODEL_BYTES, "model_sha256": actual, "license": LICENSE,
                      "source_revision": SOURCE_REVISION, "inference_run": False}), flush=True)


if __name__ == "__main__":
    main()
