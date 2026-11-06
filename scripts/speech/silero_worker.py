#!/usr/bin/env python3
"""One bounded CPU-only official Silero job. No downloads, network or voice cloning."""
import base64
import contextlib
import hashlib
import io
import json
import math
import os
import resource
import sys
import time
import wave
sys.dont_write_bytecode = True

for name in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "VECLIB_MAXIMUM_THREADS", "NUMEXPR_NUM_THREADS"):
    os.environ[name] = "2"
os.environ.update(HF_HUB_OFFLINE="1", TRANSFORMERS_OFFLINE="1")

from silero_frontend import synthesis_input, validate_request
from silero_model import (
    LICENSE, MANIFEST, MAX_PCM_BYTES, MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, MODEL,
    MODEL_BYTES, MODEL_ID, MODEL_SHA256, RATE, SOURCE_REVISION, TORCH_VERSION, VOICE,
)


class BoundedDiagnostics(io.TextIOBase):
    def __init__(self):
        self.parts = []; self.bytes = 0

    def write(self, value):
        encoded = value.encode("utf-8", errors="replace")
        if self.bytes + len(encoded) <= 65536:
            self.parts.append(value); self.bytes += len(encoded)
        return len(value)


def verify_model():
    if MODEL.is_symlink() or not MODEL.is_file() or MODEL.stat().st_size != MODEL_BYTES:
        raise ValueError("Pinned model is missing or changed")
    if MANIFEST.is_symlink() or MANIFEST.stat().st_size > 65536:
        raise ValueError("Invalid pinned model manifest")
    manifest = json.loads(MANIFEST.read_text())
    if (manifest.get("format") != "rufren-silero-model-v1" or manifest.get("model") != MODEL_ID
            or manifest.get("model_sha256") != MODEL_SHA256 or manifest.get("source_revision") != SOURCE_REVISION
            or manifest.get("runtime") != f"torch=={TORCH_VERSION}"):
        raise ValueError("Pinned model identity is not this worker's configured artifact")
    digest = hashlib.sha256()
    with MODEL.open("rb") as source:
        while block := source.read(1024 * 1024):
            digest.update(block)
    if digest.hexdigest() != MODEL_SHA256:
        raise ValueError("Pinned model SHA-256 mismatch")


def encode_wav(samples):
    pcm = bytearray(); peak = 0.0; clipped = 0; full_scale = 0
    for sample in samples:
        sample = float(sample)
        if not math.isfinite(sample):
            raise ValueError("Nonfinite generated audio")
        peak = max(peak, abs(sample)); clipped += abs(sample) > 1.0; full_scale += abs(sample) >= 1.0
        pcm.extend(round(max(-1.0, min(1.0, sample)) * 32767).to_bytes(2, "little", signed=True))
        if len(pcm) > MAX_PCM_BYTES:
            raise ValueError("Generated audio exceeded its duration limit")
    frames = len(pcm) // 2
    if not frames or peak < 0.00001:
        raise ValueError("Generated audio is empty or silent")
    output = io.BytesIO()
    with wave.open(output, "wb") as container:
        container.setnchannels(1); container.setsampwidth(2); container.setframerate(RATE); container.writeframes(pcm)
    return output.getvalue(), frames, {"peak_amplitude": peak, "clipped_samples": clipped,
                                      "clipped_fraction": clipped / frames, "full_scale_fraction": full_scale / frames}


def run(requests):
    if not isinstance(requests, list) or not 1 <= len(requests) <= 2:
        raise ValueError("Run one request or the controlled two-item pair")
    requests = [validate_request(request) for request in requests]
    if len({request["request_id"] for request in requests}) != len(requests):
        raise ValueError("Request identities must be distinct")
    inputs = [synthesis_input(request["text"]) for request in requests]
    verify_model()
    from importlib.metadata import version
    if version("torch") != TORCH_VERSION:
        raise ValueError("Unexpected local numerical runtime version")
    import torch
    torch.set_num_threads(2); torch.set_num_interop_threads(1); torch.manual_seed(42)
    torch.set_grad_enabled(False)
    started = time.monotonic()
    diagnostic = BoundedDiagnostics()
    # Packaged Python code comes only from the hash-verified official artifact.
    with contextlib.redirect_stdout(diagnostic), contextlib.redirect_stderr(diagnostic):
        model = torch.package.PackageImporter(str(MODEL)).load_pickle("tts_models", "model")
        model.to(torch.device("cpu"))
    torch.set_num_threads(2)
    load_seconds = time.monotonic() - started
    responses = []; measurements = []
    with torch.inference_mode():
        for request, actual_input in zip(requests, inputs):
            before = time.monotonic()
            with contextlib.redirect_stdout(diagnostic), contextlib.redirect_stderr(diagnostic):
                audio = model.apply_tts(
                    text=actual_input["synthesis_text"], speaker=VOICE, sample_rate=RATE,
                    put_accent=False, put_stress_homo=False, put_yo=False, put_yo_homo=False,
                    stress_single_vowel=False,
                )
            if audio.ndim != 1 or audio.numel() * 2 > MAX_PCM_BYTES:
                raise ValueError("Unexpected generated audio shape/size")
            wav, frames, signals = encode_wav(audio.detach().cpu().tolist())
            elapsed = time.monotonic() - before
            response = {
                "request_id": request["request_id"], "revision_id": request["revision_id"], "source_text": request["text"],
                "wav_base64": base64.b64encode(wav).decode("ascii"), "sha256": hashlib.sha256(wav).hexdigest(),
                "sample_rate": RATE, "channels": 1, "frames": frames,
                "provenance": {
                    "kind": "ai_generated_speech", "provider": "local_silero_tts",
                    "model": f"{MODEL_ID}@{MODEL_SHA256}", "voice": VOICE,
                    "prompt_version": "rufren-silero-russian-stress-v1",
                    "validation": "Explicit + stress input; lexical and acoustic correctness are not certified by ASR",
                    "synthesis_input": actual_input,
                },
                "usage": {"input_tokens": None, "output_tokens": None, "estimated_micro_usd": 0, "reserved_micro_usd": 0},
            }
            responses.append(response)
            measurements.append({"request_id": request["request_id"], "duration_seconds": frames / RATE,
                                 "generation_seconds": round(elapsed, 6), **signals})
    result = {"responses": responses, "measurements": measurements,
              "runtime": f"torch=={TORCH_VERSION}", "device": "cpu", "threads": torch.get_num_threads(),
              "model_load_seconds": round(load_seconds, 6), "total_seconds": round(time.monotonic() - started, 6),
              "process_peak_rss_bytes": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
              "model_license": LICENSE, "model_source_revision": SOURCE_REVISION}
    encoded = json.dumps(result, ensure_ascii=False, allow_nan=False).encode("utf-8")
    if len(encoded) > 2 * MAX_RESPONSE_BYTES:
        raise ValueError("Generated response exceeded its pair output limit")
    return encoded


def main():
    os.nice(10)
    # This is a short controlled CPU process, never a resident or GPU service.
    resource.setrlimit(resource.RLIMIT_CPU, (30, 30))
    data = sys.stdin.buffer.read(MAX_REQUEST_BYTES + 1)
    if len(data) > MAX_REQUEST_BYTES:
        raise ValueError("Worker request exceeds its input limit")
    payload = json.loads(data)
    if not isinstance(payload, dict) or set(payload) != {"requests"}:
        raise ValueError("Invalid worker envelope")
    try:
        result = run(payload["requests"])
    except Exception as error:
        # Public/error-type diagnostics only: no source text or arbitrary model errors.
        sys.stderr.write(json.dumps({"error_type": type(error).__name__, "message": "Silero controlled synthesis failed"}) + "\n")
        raise SystemExit(1)
    sys.stdout.buffer.write(result)


if __name__ == "__main__":
    main()
