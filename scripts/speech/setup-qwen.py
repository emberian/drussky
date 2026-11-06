#!/usr/bin/env python3
"""Explicit one-time model download. Does not run inference or alter shared caches."""
import hashlib
import json
import os
import secrets
import urllib.request
from pathlib import Path

import sys
sys.dont_write_bytecode = True

from qwen_model import (
    MANIFEST, MODEL_ID, MODEL_PATH, MODEL_REVISION, RUNTIME_VERSION, STATE,
    TOKEN_FILE, UPSTREAM_ID, UPSTREAM_REVISION,
)


def main():
    os.nice(10)
    STATE.mkdir(parents=True, exist_ok=True, mode=0o700)
    os.chmod(STATE, 0o700)
    os.environ["HF_HOME"] = str(STATE / "hf-cache")
    os.environ["HF_HUB_DISABLE_TELEMETRY"] = "1"
    os.environ["HF_HUB_DISABLE_PROGRESS_BARS"] = "1"
    os.environ["HF_XET_HIGH_PERFORMANCE"] = "0"
    from huggingface_hub import snapshot_download
    from importlib.metadata import version

    if version("mlx-audio") != RUNTIME_VERSION:
        raise RuntimeError("Install the pinned mlx-audio runtime in the task-owned virtual environment")
    url = f"https://huggingface.co/api/models/{MODEL_ID}/tree/{MODEL_REVISION}?recursive=true"
    with urllib.request.urlopen(url, timeout=30) as response:
        metadata = json.load(response)
    files = {item["path"]: item for item in metadata if item.get("type") == "file"}
    if not files or sum(item["size"] for item in files.values()) > 6_000_000_000:
        raise RuntimeError("Unexpected pinned model inventory")
    print(f"Downloading pinned {MODEL_ID}@{MODEL_REVISION}; {sum(f['size'] for f in files.values()):,} bytes", flush=True)
    snapshot_download(
        MODEL_ID, revision=MODEL_REVISION, local_dir=MODEL_PATH,
        cache_dir=STATE / "hf-cache", max_workers=2,
    )
    verified = []
    for name, metadata in sorted(files.items()):
        path = MODEL_PATH / name
        if not path.is_file() or path.is_symlink() or path.stat().st_size != metadata["size"]:
            raise RuntimeError(f"Invalid model asset: {name}")
        digest = hashlib.sha256()
        with path.open("rb") as source:
            while block := source.read(1024 * 1024):
                digest.update(block)
        actual = digest.hexdigest()
        expected = metadata.get("lfs", {}).get("oid")
        if expected and actual != expected:
            raise RuntimeError(f"Pinned LFS SHA-256 mismatch: {name}")
        verified.append({"path": name, "bytes": metadata["size"], "sha256": actual})
    manifest = {
        "format": "rufren-qwen-model-v1", "model": MODEL_ID, "revision": MODEL_REVISION,
        "upstream_model": UPSTREAM_ID, "upstream_revision": UPSTREAM_REVISION,
        "license": "Apache-2.0", "runtime": f"mlx-audio=={RUNTIME_VERSION}", "files": verified,
    }
    temporary = MANIFEST.with_suffix(".partial")
    temporary.write_text(json.dumps(manifest, indent=2) + "\n")
    os.replace(temporary, MANIFEST)
    try:
        fd = os.open(TOKEN_FILE, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError:
        if TOKEN_FILE.is_symlink() or TOKEN_FILE.stat().st_mode & 0o077:
            raise RuntimeError("Worker token permissions must be private")
    else:
        with os.fdopen(fd, "w") as output:
            output.write(secrets.token_urlsafe(32) + "\n")
    print(f"Verified {len(verified)} pinned files. Worker credentials remain in the private task directory.", flush=True)


if __name__ == "__main__":
    main()
