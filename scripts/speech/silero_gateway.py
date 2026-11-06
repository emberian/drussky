#!/usr/bin/env python3
"""Bounded one-shot Silero gateway protocol; no downloads or caller-selected paths.

Normal stdin schema: rufren-silero-worker-v1. --check performs only metadata,
file-identity and SHA checks; it never imports torch or loads the model.
"""
import base64
import contextlib
import hashlib
import io
import json
import os
import re
import resource
import sys
import unicodedata
from importlib.metadata import version
sys.dont_write_bytecode = True

for _name in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "VECLIB_MAXIMUM_THREADS", "NUMEXPR_NUM_THREADS"):
    os.environ[_name] = "2"
os.environ.update(HF_HUB_OFFLINE="1", TRANSFORMERS_OFFLINE="1")

from silero_frontend import synthesis_input, validate_request
from silero_model import (
    LICENSE, MAX_PCM_BYTES, MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, MAX_TEXT_BYTES,
    MAX_TEXT_CHARS, MODEL, MODEL_ID, MODEL_SHA256, RATE, SOURCE_REVISION,
    TORCH_VERSION, VOICE,
)
from silero_worker import encode_wav, verify_model

SCHEMA = "rufren-silero-worker-v1"
STATUS_SCHEMA = "rufren-silero-worker-status-v1"
MODEL_IDENTITY = f"{MODEL_ID}@{MODEL_SHA256};repo={SOURCE_REVISION}"
REQUEST_KEYS = frozenset(("schema", "text", "request_id", "revision_id", "source_text", "source_text_sha256", "mode"))
EXPLICIT_FLAGS = {"put_accent": False, "put_stress_homo": False, "put_yo": False, "put_yo_homo": False, "stress_single_vowel": False}
AUTOMATIC_FLAGS = {key: True for key in EXPLICIT_FLAGS}


class InvalidInput(ValueError):
    pass


class RedactedDiagnostics(io.TextIOBase):
    """Keep model printouts off stdout and avoid logging source text or tracebacks."""
    def __init__(self):
        self.suppressed_bytes = 0

    def write(self, value):
        self.suppressed_bytes = min(262144, self.suppressed_bytes + len(value.encode("utf-8", errors="replace")))
        return len(value)

    def flush(self):
        return None


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise InvalidInput("Duplicate field")
        result[key] = value
    return result


def parse(data):
    if not isinstance(data, bytes) or not 0 < len(data) <= MAX_REQUEST_BYTES:
        raise InvalidInput("Request limit")
    try:
        return validate(json.loads(data, object_pairs_hook=_unique_object))
    except (ValueError, UnicodeError, TypeError, RecursionError) as error:
        raise InvalidInput("Invalid source-bound request") from None


def validate(payload):
    if not isinstance(payload, dict) or set(payload) != REQUEST_KEYS or payload.get("schema") != SCHEMA:
        raise InvalidInput("Unknown schema or fields")
    source = validate_request({"text": payload["source_text"], "request_id": payload["request_id"], "revision_id": payload["revision_id"]})
    digest = payload["source_text_sha256"]
    if (not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest)
            or hashlib.sha256(source["text"].encode("utf-8")).hexdigest() != digest):
        raise InvalidInput("Source digest mismatch")
    actual = payload["text"]
    if (not isinstance(actual, str) or not actual.strip() or len(actual) > MAX_TEXT_CHARS
            or len(actual.encode("utf-8")) > MAX_TEXT_BYTES
            or any(unicodedata.category(c) == "Cc" and c not in "\n\t" for c in actual)
            or any(c in actual for c in "<>") or "[[" in actual):
        raise InvalidInput("Invalid actual synthesis input")
    provenance = synthesis_input(source["text"])
    mode = payload["mode"]
    if mode == "explicit_stress":
        if provenance["transform"] != "pedagogical_acute_to_silero_plus" or actual != provenance["synthesis_text"]:
            raise InvalidInput("Explicit input must exactly match the authored-stress transform")
    elif mode == "automatic":
        if actual != source["text"] or provenance["transform"] != "verbatim" or "+" in actual:
            raise InvalidInput("Automatic mode requires exact ordinary source text")
    else:
        raise InvalidInput("Unknown mode")
    return {**source, "text": actual, "source_text": source["text"], "source_text_sha256": digest, "mode": mode,
            "synthesis_input": provenance}


def check_installation():
    """Read-only readiness: inspect installed distribution metadata, not torch code."""
    if version("torch") != TORCH_VERSION:
        raise ValueError("Unexpected installed runtime version")
    verify_model()
    return {"schema": STATUS_SCHEMA, "enabled": True, "provider": "local_silero_tts", "model": MODEL_IDENTITY,
            "voice": VOICE, "runtime": f"torch=={TORCH_VERSION}", "license": LICENSE,
            "max_text_bytes": MAX_TEXT_BYTES, "max_text_chars": MAX_TEXT_CHARS,
            "max_duration_seconds": 45, "sample_rate": RATE, "channels": 1,
            "model_loaded": False, "inference_run": False}


class LoadedModel:
    def __init__(self, torch, model):
        self.torch, self.model = torch, model

    def samples(self, text, flags):
        self.torch.manual_seed(42)
        with self.torch.inference_mode():
            audio = self.model.apply_tts(text=text, speaker=VOICE, sample_rate=RATE, **flags)
        if audio.ndim != 1 or audio.numel() * 2 > MAX_PCM_BYTES:
            raise ValueError("Unexpected generated audio shape/size")
        return audio.detach().cpu().tolist()


def load_model():
    # Only this function imports numerical code or deserializes the pinned official package.
    import torch
    torch.set_num_threads(2); torch.set_num_interop_threads(1); torch.set_grad_enabled(False)
    model = torch.package.PackageImporter(str(MODEL)).load_pickle("tts_models", "model")
    model.to(torch.device("cpu")); torch.set_num_threads(2)
    return LoadedModel(torch, model)


def synthesize(request, loader=None):
    flags = dict(EXPLICIT_FLAGS if request["mode"] == "explicit_stress" else AUTOMATIC_FLAGS)
    loaded = (loader or load_model)()
    samples = loaded.samples(request["text"], flags)
    wav, frames, signals = encode_wav(samples)
    mode = request["mode"]
    flag_description = ",".join(f"{key}={str(value).lower()}" for key, value in flags.items())
    signal_description = (f"peak_amplitude={signals['peak_amplitude']:.9g};"
                          f"clipped_samples={signals['clipped_samples']};"
                          f"clipped_fraction={signals['clipped_fraction']:.9g};"
                          f"full_scale_fraction={signals['full_scale_fraction']:.9g}")
    return {
        "request_id": request["request_id"], "revision_id": request["revision_id"], "source_text": request["source_text"],
        "wav_base64": base64.b64encode(wav).decode("ascii"), "sha256": hashlib.sha256(wav).hexdigest(),
        "sample_rate": RATE, "channels": 1, "frames": frames,
        "provenance": {"kind": "ai_generated_speech", "provider": "local_silero_tts", "model": MODEL_IDENTITY,
                       "voice": VOICE, "prompt_version": f"rufren-silero-{'explicit' if mode == 'explicit_stress' else 'automatic'}-v1",
                       "validation": f"CPU2;seed=42;{flag_description};{signal_description}; actual API input recorded; acoustic correctness/stress not certified by ASR",
                       "synthesis_input": request["synthesis_input"]},
        "usage": {"input_tokens": None, "output_tokens": None, "estimated_micro_usd": 0, "reserved_micro_usd": 0},
    }, signals


def bounded_json(value):
    result = json.dumps(value, ensure_ascii=False, allow_nan=False).encode("utf-8")
    if len(result) > MAX_RESPONSE_BYTES:
        raise ValueError("Response limit")
    return result


def main():
    diagnostics = RedactedDiagnostics()
    try:
        if sys.argv[1:] == ["--check"]:
            # This path is deliberately outside numerical imports/diagnostic redirection.
            result = check_installation()
        elif not sys.argv[1:]:
            request = parse(sys.stdin.buffer.read(MAX_REQUEST_BYTES + 1))
            check_installation()
            os.nice(10)
            resource.setrlimit(resource.RLIMIT_CPU, (30, 30))
            with contextlib.redirect_stdout(diagnostics), contextlib.redirect_stderr(diagnostics):
                result, signals = synthesize(request)
            # Signal observations are bounded, text-free diagnostic data, not pronunciation scores.
            sys.stderr.write(json.dumps({"event": "generated", "mode": request["mode"], "frames": result["frames"],
                                         "diagnostics_suppressed_bytes": diagnostics.suppressed_bytes, **signals}) + "\n")
        else:
            raise InvalidInput("Unsupported worker arguments")
        sys.stdout.buffer.write(bounded_json(result))
    except Exception as error:
        # Neither raw parser/model errors nor a traceback may leak authored text.
        category = "invalid_request" if isinstance(error, (InvalidInput, UnicodeError)) else "unavailable"
        sys.stderr.write(json.dumps({"error": category, "error_type": type(error).__name__,
                                     "diagnostics_suppressed_bytes": diagnostics.suppressed_bytes}) + "\n")
        raise SystemExit(2 if category == "invalid_request" else 1)


if __name__ == "__main__":
    main()
