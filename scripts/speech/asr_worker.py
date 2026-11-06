#!/usr/bin/env python3
"""One bounded, offline Russian transcription per JSON stdin request.

The supervisor owns only its child process group. No HTTP server, model download,
expected-text prompt, pronunciation score, or invented word timestamps.
"""
import base64
import binascii
import contextlib
import datetime
import fcntl
import hashlib
import io
import json
import math
import os
import re
import signal
import struct
import subprocess
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
from asr_model import MANIFEST, MODEL_ID, MODEL_PATH, MODEL_REVISION, RUNTIME_VERSION, STATE, verify_install

for key in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "VECLIB_MAXIMUM_THREADS", "NUMEXPR_NUM_THREADS"):
    os.environ[key] = "2"
os.environ.update(HF_HOME=str(STATE / "hf-cache"), HF_HUB_OFFLINE="1", TRANSFORMERS_OFFLINE="1", HF_HUB_DISABLE_TELEMETRY="1",
                  TOKENIZERS_PARALLELISM="false", RAYON_NUM_THREADS="2")
MAX_AUDIO_BYTES = 9_000_000
MAX_REQUEST_BYTES = 12_100_000
MAX_RESPONSE_BYTES = 65_536
DEADLINE_SECONDS = 120
RSS_LIMIT_BYTES = 16 * 1024**3
ID = re.compile(r"[A-Za-z0-9_.:-]{1,128}\Z")
SHA = re.compile(r"[0-9a-f]{64}\Z")


def decode_wav(data):
    """Bounded RIFF PCM16 / IEEE float32 only, retaining original-frame indices."""
    if len(data) < 44 or len(data) > MAX_AUDIO_BYTES or data[:4] != b"RIFF" or data[8:12] != b"WAVE":
        raise ValueError("Unsupported WAV container")
    if struct.unpack_from("<I", data, 4)[0] + 8 != len(data):
        raise ValueError("WAV container length mismatch")
    fmt = payload = None
    offset = 12
    while offset < len(data):
        if offset + 8 > len(data):
            raise ValueError("Truncated WAV chunk")
        tag, size = struct.unpack_from("<4sI", data, offset)
        start, end = offset + 8, offset + 8 + size
        if end > len(data):
            raise ValueError("Truncated WAV payload")
        if tag == b"fmt ":
            if fmt is not None or size < 16:
                raise ValueError("Duplicate or invalid WAV format")
            fmt = struct.unpack_from("<HHIIHH", data, start)
        elif tag == b"data":
            if payload is not None:
                raise ValueError("Duplicate WAV data")
            payload = data[start:end]
        offset = end + (size & 1)
    if offset != len(data) or fmt is None or payload is None:
        raise ValueError("Incomplete WAV")
    encoding, channels, rate, byte_rate, block_align, bits = fmt
    if (encoding, bits) not in ((1, 16), (3, 32)) or channels not in (1, 2) or not 8000 <= rate <= 48000:
        raise ValueError("Requires mono/stereo PCM16 or float32 WAV at 8–48 kHz")
    if block_align != channels * bits // 8 or byte_rate != rate * block_align or len(payload) % block_align:
        raise ValueError("Inconsistent WAV frame format")
    frames = len(payload) // block_align
    if not 1 <= frames <= rate * 45:
        raise ValueError("Requires nonempty audio of at most 45 seconds")
    if encoding == 3:
        for (value,) in struct.iter_unpack("<f", payload):
            if not math.isfinite(value) or not -1 <= value <= 1:
                raise ValueError("Invalid floating-point sample")
    return {"sample_rate": rate, "channels": channels, "frames": frames,
            "encoding": "pcm_s16le" if encoding == 1 else "ieee_f32le", "bits": bits}, payload


def validate(request):
    allowed = {"request_id", "attempt_id", "revision_id", "asset_id", "audio_sha256", "language", "wav_base64", "span"}
    if not isinstance(request, dict) or set(request) - allowed:
        raise ValueError("Invalid request fields")
    for name in ("request_id", "asset_id"):
        if not isinstance(request.get(name), str) or not ID.fullmatch(request[name]):
            raise ValueError("Missing request or asset binding")
    for name in ("attempt_id", "revision_id"):
        if request.get(name) is not None and (not isinstance(request[name], str) or not ID.fullmatch(request[name])):
            raise ValueError("Invalid attempt/revision binding")
    if request.get("language") != "ru":
        raise ValueError("Only explicitly selected Russian is supported")
    digest = request.get("audio_sha256")
    if not isinstance(digest, str) or not SHA.fullmatch(digest):
        raise ValueError("Invalid audio hash")
    encoded = request.get("wav_base64")
    if not isinstance(encoded, str) or len(encoded) > 4 * ((MAX_AUDIO_BYTES + 2) // 3):
        raise ValueError("Invalid bounded audio")
    try:
        data = base64.b64decode(encoded, validate=True)
    except (binascii.Error, ValueError) as error:
        raise ValueError("Invalid audio encoding") from error
    if hashlib.sha256(data).hexdigest() != digest:
        raise ValueError("Audio hash mismatch")
    shape, payload = decode_wav(data)
    span = request.get("span", {"start_frame": 0, "end_frame": shape["frames"]})
    if (not isinstance(span, dict) or set(span) != {"start_frame", "end_frame"}
            or any(type(span[k]) is not int for k in span)
            or not 0 <= span["start_frame"] < span["end_frame"] <= shape["frames"]):
        raise ValueError("Invalid original-frame crop")
    return shape, payload, span


def infer(request):
    shape, payload, span = validate(request)
    started = time.monotonic()
    started_at = datetime.datetime.now(datetime.timezone.utc).isoformat()
    from importlib.metadata import version
    if version("mlx-audio") != RUNTIME_VERSION:
        raise RuntimeError("Unexpected ASR runtime")
    import numpy as np
    from scipy.signal import resample_poly
    values = np.frombuffer(payload, dtype="<i2" if shape["bits"] == 16 else "<f4").astype(np.float32)
    if shape["bits"] == 16:
        values /= 32768.0
    source_pcm_hash = hashlib.sha256(values.astype("<f4").tobytes()).hexdigest()
    values = values.reshape(-1, shape["channels"])[span["start_frame"]:span["end_frame"]].mean(axis=1)
    rms = float(np.sqrt(np.mean(values.astype(np.float64) ** 2)))
    if rms < 1e-8:
        raise ValueError("No signal in selected crop")
    common = math.gcd(shape["sample_rate"], 16000)
    if shape["sample_rate"] != 16000:
        values = resample_poly(values, 16000 // common, shape["sample_rate"] // common).astype(np.float32)
    analysis_hash = hashlib.sha256(values.astype("<f4").tobytes()).hexdigest()
    verify_install()
    import mlx.core as mx
    mx.set_memory_limit(12 * 1024**3)
    mx.set_cache_limit(1024**3)
    mx.random.seed(42)
    from mlx_audio.stt import load
    with contextlib.redirect_stdout(sys.stderr):
        model = load(str(MODEL_PATH))
        result = model.generate(values, language="Russian", temperature=0.0, max_tokens=1024,
                                batch_size=1, verbose=False, system_prompt=None, hotwords=None)
    if (not isinstance(result.text, str) or len(result.text.encode("utf-8")) > 16384
            or result.generation_tokens >= 1024):
        raise RuntimeError("Incomplete or oversized transcription")
    import resource
    elapsed = time.monotonic() - started
    return {"schema_version": "rufren-speech-transcript-v1",
            "request_id": request["request_id"], "attempt_id": request.get("attempt_id"),
            "revision_id": request.get("revision_id"), "asset_id": request["asset_id"],
            "audio_sha256": request["audio_sha256"], "original_audio": shape,
            "source_pcm_sha256": source_pcm_hash,
            "source_pcm_hash_format": "interleaved-little-endian-f32-no-header-v1",
            "span": span, "span_coordinate_system": "original_frames_half_open",
            "derivative": {"sample_rate": 16000, "channels": 1, "frames": len(values),
                           "pcm_sha256": analysis_hash, "pcm_hash_format": "little-endian-f32-no-header-v1",
                           "method": "arithmetic-channel-mean; scipy.signal.resample_poly default Kaiser5",
                           "scipy_version": version("scipy"), "amplitude_normalization": False},
            "evidence_kind": "transcript_hypothesis", "transcript": result.text,
            "language": "ru", "language_selection": "explicit_not_detected",
            "target_conditioned": False, "word_timestamps": None, "word_alignment_status": "not_run",
            "confidence": None, "acoustic_assessment": None,
            "provenance": {"provider": "local_qwen3_asr", "model": MODEL_ID, "revision": MODEL_REVISION,
                           "runtime": f"mlx-audio=={RUNTIME_VERSION}", "mlx_version": version("mlx"),
                           "manifest_sha256": hashlib.sha256(MANIFEST.read_bytes()).hexdigest(),
                           "worker_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                           "decoding": {"temperature": 0.0, "max_tokens": 1024, "seed": 42, "language": "Russian", "hotwords": []}},
            "timing": {"started_at": started_at, "completed_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                       "elapsed_seconds": round(elapsed, 3), "process_peak_rss_bytes": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
                       "metal_peak_bytes": mx.get_peak_memory()},
            "usage": {"prompt_tokens": result.prompt_tokens, "generation_tokens": result.generation_tokens}}


def stop_child(child):
    if child.poll() is None:
        try:
            os.killpg(child.pid, signal.SIGTERM)
        except ProcessLookupError:
            return
        try:
            child.wait(timeout=1)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            child.wait(timeout=5)


def supervise(raw):
    child = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "--child"],
                             stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=sys.stderr, start_new_session=True)
    start = time.monotonic()
    try:
        pending = raw
        while True:
            try:
                output, _ = child.communicate(input=pending, timeout=1)
                break
            except subprocess.TimeoutExpired:
                pending = None
                if time.monotonic() - start > DEADLINE_SECONDS:
                    raise TimeoutError("ASR operation exceeded 120 seconds")
                rss = subprocess.run(["/bin/ps", "-o", "rss=", "-p", str(child.pid)], capture_output=True, text=True, timeout=2)
                if rss.returncode == 0 and rss.stdout.strip() and int(rss.stdout.strip()) * 1024 > RSS_LIMIT_BYTES:
                    raise MemoryError("ASR process exceeded 16 GiB RSS")
        if child.returncode or len(output) > MAX_RESPONSE_BYTES:
            raise RuntimeError("ASR child failed")
        json.loads(output)
        return output
    finally:
        stop_child(child)


def main():
    def terminate(_signal, _frame):
        raise KeyboardInterrupt("ASR canceled")
    signal.signal(signal.SIGTERM, terminate)
    os.nice(10)
    raw = sys.stdin.buffer.read(MAX_REQUEST_BYTES + 1)
    if len(raw) > MAX_REQUEST_BYTES:
        raise ValueError("Request exceeds byte limit")
    request = json.loads(raw)
    validate(request)
    if sys.argv[1:] == ["--child"]:
        output = json.dumps(infer(request), ensure_ascii=False, allow_nan=False).encode()
    elif sys.argv[1:]:
        raise ValueError("No command arguments are supported")
    else:
        STATE.mkdir(parents=True, exist_ok=True, mode=0o700)
        with (STATE / "inference.lock").open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            output = supervise(raw)
    if len(output) > MAX_RESPONSE_BYTES:
        raise RuntimeError("Response exceeds byte limit")
    sys.stdout.buffer.write(output + b"\n")


if __name__ == "__main__":
    try:
        main()
    except (Exception, KeyboardInterrupt) as error:
        print(json.dumps({"error": type(error).__name__, "message": str(error)[:256]}), file=sys.stderr)
        sys.exit(1)
