#!/usr/bin/env python3
"""Explicit bounded acquisition of pinned public BF16 ASR weights; no inference."""
import hashlib
import json
import os
import ssl
import urllib.request
import sys
sys.dont_write_bytecode = True
from asr_model import MANIFEST, MODEL_ID, MODEL_PATH, MODEL_REVISION, RUNTIME_VERSION, STATE, UPSTREAM_ID, UPSTREAM_REVISION


def main():
    os.nice(10)
    STATE.mkdir(parents=True, exist_ok=True, mode=0o700)
    os.chmod(STATE, 0o700)
    os.environ.update(HF_HOME=str(STATE / "hf-cache"), HF_HUB_DISABLE_TELEMETRY="1",
                      HF_HUB_DISABLE_PROGRESS_BARS="1", HF_XET_HIGH_PERFORMANCE="0")
    from importlib.metadata import version
    from huggingface_hub import snapshot_download
    if version("mlx-audio") != RUNTIME_VERSION:
        raise RuntimeError("Install the pinned ASR runtime")
    url = f"https://huggingface.co/api/models/{MODEL_ID}/tree/{MODEL_REVISION}?recursive=true"
    with urllib.request.urlopen(url, context=ssl.create_default_context(cafile="/etc/ssl/cert.pem"), timeout=30) as response:
        metadata = json.load(response)
    files = {i["path"]: i for i in metadata if i.get("type") == "file"}
    if not files or sum(i["size"] for i in files.values()) > 5_000_000_000:
        raise RuntimeError("Unexpected ASR inventory")
    for name in files:
        if name.startswith("/") or ".." in name.split("/"):
            raise RuntimeError("Invalid model path")
    print(f"Downloading pinned {MODEL_ID}@{MODEL_REVISION}: {sum(i['size'] for i in files.values()):,} bytes", flush=True)
    snapshot_download(MODEL_ID, revision=MODEL_REVISION, local_dir=MODEL_PATH,
                      cache_dir=STATE / "hf-cache", max_workers=2)
    verified = []
    for name, info in sorted(files.items()):
        path = MODEL_PATH / name
        if not path.is_file() or path.is_symlink() or path.stat().st_size != info["size"]:
            raise RuntimeError("Invalid model asset")
        digest = hashlib.sha256()
        with path.open("rb") as source:
            while block := source.read(1024 * 1024):
                digest.update(block)
        actual = digest.hexdigest()
        if info.get("lfs", {}).get("oid") not in (None, actual):
            raise RuntimeError("Pinned LFS hash mismatch")
        verified.append({"path": name, "bytes": info["size"], "sha256": actual})
    manifest = {"format": "rufren-asr-model-v1", "model": MODEL_ID, "revision": MODEL_REVISION,
                "upstream_model": UPSTREAM_ID, "upstream_observed_revision": UPSTREAM_REVISION,
                "conversion_upstream_revision_verified": False,
                "license": "Apache-2.0", "runtime": f"mlx-audio=={RUNTIME_VERSION}", "files": verified}
    temporary = MANIFEST.with_suffix(".partial")
    temporary.write_text(json.dumps(manifest, indent=2) + "\n")
    os.replace(temporary, MANIFEST)
    print(f"Verified {len(verified)} pinned ASR files", flush=True)


if __name__ == "__main__":
    main()
