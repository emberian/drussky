"""Pinned ASR assets. Importing this module never loads/downloads a model."""
from pathlib import Path

PROJECT = Path(__file__).resolve().parents[2]
STATE = PROJECT / ".local" / "qwen3-asr"
MODEL_ID = "mlx-community/Qwen3-ASR-1.7B-bf16"
MODEL_REVISION = "e1f6c266914abc5a46e8756e02580f834a6cf8a7"
UPSTREAM_ID = "Qwen/Qwen3-ASR-1.7B"
UPSTREAM_REVISION = "7278e1e70fe206f11671096ffdd38061171dd6e5"
RUNTIME_VERSION = "0.6.4"
MODEL_PATH = STATE / "models" / MODEL_REVISION
MANIFEST = STATE / "model-manifest.json"


def verify_install():
    """Verify bytes, not merely sizes, before loading executable model metadata."""
    import hashlib
    import json
    if not MANIFEST.is_file() or MANIFEST.stat().st_size > 65536:
        raise RuntimeError("Run the explicit ASR setup first")
    manifest = json.loads(MANIFEST.read_text())
    if (manifest.get("format") != "rufren-asr-model-v1"
            or manifest.get("model") != MODEL_ID
            or manifest.get("revision") != MODEL_REVISION):
        raise RuntimeError("Wrong ASR manifest")
    names = set()
    for entry in manifest["files"]:
        name = entry["path"]
        if not isinstance(name, str) or name.startswith("/") or ".." in name.split("/") or name in names:
            raise RuntimeError("Invalid ASR manifest path")
        names.add(name)
        path = MODEL_PATH / name
        if not path.is_file() or path.is_symlink() or path.stat().st_size != entry["bytes"]:
            raise RuntimeError("Missing or changed ASR asset")
        digest = hashlib.sha256()
        with path.open("rb") as source:
            while block := source.read(1024 * 1024):
                digest.update(block)
        if digest.hexdigest() != entry["sha256"]:
            raise RuntimeError("ASR asset hash mismatch")
    required = {"config.json", "model.safetensors", "vocab.json", "merges.txt", "tokenizer_config.json", "preprocessor_config.json"}
    if not required.issubset(names):
        raise RuntimeError("Incomplete ASR asset manifest")
    actual = {str(p.relative_to(MODEL_PATH)) for p in MODEL_PATH.rglob("*")
              if p.is_file() and ".cache" not in p.relative_to(MODEL_PATH).parts}
    if actual != names:
        raise RuntimeError("Unmanifested ASR asset")
    return manifest
