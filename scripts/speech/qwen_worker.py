#!/usr/bin/env python3
"""Authenticated loopback-only resident Russian Qwen3 worker; no runtime downloads."""
import argparse
import concurrent.futures
import queue
import base64
import hashlib
import hmac
import io
import json
import math
import os
import re
import resource
import signal
import socket
import threading
import time
import wave
import unicodedata
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import sys
sys.dont_write_bytecode = True

from qwen_model import (
    MANIFEST, MAX_PCM_BYTES, MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, MAX_TEXT_BYTES,
    MAX_TEXT_CHARS, MODEL_ID, MODEL_PATH, MODEL_REVISION, PID_FILE, PROMPT_VERSION,
    RUNTIME_VERSION, STATE, TOKEN_FILE, VOICE_DESCRIPTION, VOICE_ID, WORKER_SCHEMA,
)

# Fixed conservative host controls apply before importing a numerical runtime.
for name in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "VECLIB_MAXIMUM_THREADS", "NUMEXPR_NUM_THREADS"):
    os.environ[name] = "2"
os.environ.update(HF_HOME=str(STATE / "hf-cache"), HF_HUB_OFFLINE="1", TRANSFORMERS_OFFLINE="1", HF_HUB_DISABLE_TELEMETRY="1")

from qwen_frontend import UnsupportedStress, synthesis_input

ID = re.compile(r"[A-Za-z0-9_.:-]{1,128}\Z")


def validate(request):
    if not isinstance(request, dict) or set(request) - {"text", "request_id", "revision_id"}:
        raise ValueError("Invalid request fields")
    text, request_id, revision_id = (request.get(k) for k in ("text", "request_id", "revision_id"))
    if (not isinstance(text, str) or not text.strip() or len(text) > MAX_TEXT_CHARS
            or len(text.encode("utf-8")) > MAX_TEXT_BYTES or not any(c.isalpha() for c in text)
            or any(unicodedata.category(c) == "Cc" and c not in "\n\t" for c in text)
            or any(c in text for c in "<>") or "[[" in text
            or not isinstance(request_id, str) or not ID.fullmatch(request_id)
            or revision_id is not None and (not isinstance(revision_id, str) or not ID.fullmatch(revision_id))):
        raise ValueError("Invalid short text or binding")
    return {"text": text, "request_id": request_id, "revision_id": revision_id}


def verify_install():
    if not MANIFEST.is_file() or MANIFEST.stat().st_size > 65536:
        raise RuntimeError("Run the explicit pinned model setup first")
    manifest = json.loads(MANIFEST.read_text())
    if manifest.get("format") != "rufren-qwen-model-v1" or manifest.get("model") != MODEL_ID or manifest.get("revision") != MODEL_REVISION:
        raise RuntimeError("Pinned model manifest does not match this worker")
    for entry in manifest["files"]:
        name = entry["path"]
        if not isinstance(name, str) or name.startswith("/") or ".." in name.split("/"):
            raise RuntimeError("Invalid pinned model file path")
        path = MODEL_PATH / name
        if not path.is_file() or path.is_symlink() or path.stat().st_size != entry["bytes"]:
            raise RuntimeError("Pinned model file is missing or changed")
    return manifest


def canonical_wav(samples):
    # A single sample-rate/container contract shared with the Rust gateway.
    pcm = bytearray()
    peak = 0.0
    clipped = 0
    for sample in samples:
        sample = float(sample)
        if not math.isfinite(sample):
            raise ValueError("Nonfinite model audio")
        peak = max(peak, abs(sample))
        clipped += abs(sample) > 1.0
        value = round(max(-1.0, min(1.0, sample)) * 32767)
        pcm.extend(value.to_bytes(2, "little", signed=True))
        if len(pcm) > MAX_PCM_BYTES:
            raise ValueError("Model audio exceeds its duration limit")
    if not pcm or peak < 0.00001:
        raise ValueError("Model returned empty or silent audio")
    stream = io.BytesIO()
    with wave.open(stream, "wb") as output:
        output.setnchannels(1); output.setsampwidth(2); output.setframerate(24000); output.writeframes(pcm)
    return stream.getvalue(), len(pcm) // 2, peak, clipped


class Engine:
    def __init__(self):
        self.model = None
        self.loaded_at = None
        self.last_stats = None
        self.prepared = False
        self.phase = "cold"
        self.last_error = None
        self.jobs = queue.Queue(maxsize=1)
        # MLX model/streams stay on one persistent owner thread across requests.
        self.owner = threading.Thread(target=self._run, daemon=True, name="rufren-qwen-inference")
        self.owner.start()

    def _run(self):
        while True:
            request, future, deadline = self.jobs.get()
            try:
                self.phase = "preparing" if not self.prepared else "generating"
                self.last_error = None
                value = self._generate(request, deadline)
                self.phase = "ready"
                future.set_result(value)
            except BaseException as error:
                self.prepared = False
                self.phase = "failed"
                self.last_error = type(error).__name__
                future.set_exception(error)
            finally:
                self.jobs.task_done()

    def generate(self, request, preparing=False):
        if not preparing and not self.prepared:
            raise RuntimeError("Operator prewarm is required")
        future = concurrent.futures.Future()
        self.jobs.put_nowait((request, future, 120 if preparing else 50))
        # The HTTP gate admits one request; its process watchdog bounds native hangs.
        return future.result()

    def load(self):
        if self.model is None:
            verify_install()
            from importlib.metadata import version
            if version("mlx-audio") != RUNTIME_VERSION:
                raise RuntimeError("Unexpected numerical runtime version")
            import mlx.core as mx
            # A hard Metal memory ceiling protects the shared development Mac.
            if hasattr(mx, "set_memory_limit"):
                mx.set_memory_limit(16 * 1024**3)
            from mlx_audio.tts.utils import load_model
            self.model = load_model(str(MODEL_PATH))
            self.loaded_at = time.time()

    def _generate(self, request, deadline):
        started = time.monotonic()
        actual_input = synthesis_input(request["text"])
        self.load()
        import mlx.core as mx
        mx.random.seed(42)
        chunks = []
        frames = 0
        # Streaming bounds accumulation and permits a wall-clock deadline between chunks.
        for result in self.model.generate_voice_design(
            text=actual_input["synthesis_text"], language="Russian", instruct=VOICE_DESCRIPTION,
            max_tokens=560, stream=True, streaming_interval=0.32,
        ):
            if time.monotonic() - started > deadline:
                raise TimeoutError("Generation deadline exceeded")
            if result.sample_rate != 24000:
                raise ValueError("Unexpected model sample rate")
            audio = result.audio
            mx.eval(audio)
            values = audio.tolist()
            if not isinstance(values, list) or values and isinstance(values[0], list):
                raise ValueError("Unexpected model audio shape")
            frames += len(values)
            if frames * 2 > MAX_PCM_BYTES:
                raise ValueError("Generated reference exceeds duration limit")
            chunks.extend(values)
        wav, frames, peak, clipped = canonical_wav(chunks)
        elapsed = time.monotonic() - started
        rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
        # ru_maxrss is bytes on macOS; model/runtime memory is also reported when available.
        self.prepared = True
        self.last_stats = {"elapsed_seconds": round(elapsed, 3), "duration_seconds": frames / 24000,
                           "peak_amplitude": peak, "full_scale_fraction": sum(abs(s) >= 1.0 for s in chunks) / frames, "clipped_samples": clipped, "clipped_fraction": clipped / frames, "process_peak_rss_bytes": rss,
                           "metal_peak_bytes": mx.get_peak_memory() if hasattr(mx, "get_peak_memory") else None}
        return {
            "request_id": request["request_id"], "revision_id": request["revision_id"], "source_text": request["text"],
            "wav_base64": base64.b64encode(wav).decode("ascii"), "sha256": hashlib.sha256(wav).hexdigest(),
            "sample_rate": 24000, "channels": 1, "frames": frames,
            "provenance": {"kind": "ai_generated_speech", "provider": "local_qwen3_tts",
                           "model": f"{MODEL_ID}@{MODEL_REVISION}", "voice": VOICE_ID,
                           "prompt_version": PROMPT_VERSION, "synthesis_input": actual_input,
                           "validation": f"Local BF16 VoiceDesign; mlx-audio {RUNTIME_VERSION}; seed 42; no human reference or learner pronunciation assessment"},
            "usage": {"input_tokens": None, "output_tokens": None, "estimated_micro_usd": 0, "reserved_micro_usd": 0},
        }


class Server(ThreadingHTTPServer):
    daemon_threads = True
    request_queue_size = 4

    def __init__(self, address, engine, token):
        self.engine, self.token = engine, token
        self.gate = threading.BoundedSemaphore(1)
        self.clients = threading.BoundedSemaphore(4)
        super().__init__(address, Handler)

    def process_request(self, request, client_address):
        if not self.clients.acquire(blocking=False):
            request.close(); return
        try:
            super().process_request(request, client_address)
        except BaseException:
            self.clients.release(); raise

    def process_request_thread(self, request, client_address):
        try:
            super().process_request_thread(request, client_address)
        finally:
            self.clients.release()


class Handler(BaseHTTPRequestHandler):
    server_version = "RufrenLocalSpeech/1"
    protocol_version = "HTTP/1.0"

    def setup(self):
        super().setup(); self.connection.settimeout(10)

    def log_message(self, *_):
        # No request text, URL, token, or client exception details enter the log.
        return

    def output(self, status, value):
        data = json.dumps(value, ensure_ascii=False, allow_nan=False).encode("utf-8")
        if len(data) > MAX_RESPONSE_BYTES:
            status, data = 502, b'{"error":"Generated response exceeded its size limit"}'
        self.send_response(status); self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data))); self.send_header("Cache-Control", "no-store")
        self.end_headers()
        try:
            self.wfile.write(data)
        except (BrokenPipeError, ConnectionResetError, socket.timeout):
            pass

    def authorized(self):
        supplied = self.headers.get("Authorization", "")
        return hmac.compare_digest(supplied.encode("utf-8"), ("Bearer " + self.server.token).encode("ascii"))

    def do_GET(self):
        if not self.authorized():
            self.output(401, {"error": "Worker authentication required"}); return
        if self.path != "/status":
            self.output(404, {"error": "Unknown worker route"}); return
        engine = self.server.engine
        self.output(200, {"contract_version": WORKER_SCHEMA, "enabled": bool(engine.prepared and engine.model is not None), "provider": "local_qwen3_tts", "model": f"{MODEL_ID}@{MODEL_REVISION}",
                          "voice": VOICE_ID, "loaded": engine.model is not None, "max_text_bytes": MAX_TEXT_BYTES,
                          "max_text_chars": MAX_TEXT_CHARS, "phase": getattr(engine, "phase", "ready"),
                          "last_error": getattr(engine, "last_error", None), "last_stats": engine.last_stats})

    def do_POST(self):
        if not self.authorized():
            self.output(401, {"error": "Worker authentication required"}); return
        if self.path not in ("/generate", "/prepare"):
            self.output(404, {"error": "Unknown worker route"}); return
        if self.headers.get("Transfer-Encoding") or self.headers.get("Content-Type", "").split(";")[0] != "application/json":
            self.output(400, {"error": "Use bounded JSON with Content-Length"}); return
        try:
            length = int(self.headers.get("Content-Length", "-1"))
            if not 0 < length <= MAX_REQUEST_BYTES:
                self.output(413, {"error": "Worker request exceeds its size limit"}); return
            data = self.rfile.read(length)
            if len(data) != length:
                raise ValueError("Incomplete request")
            request = validate(json.loads(data))
            synthesis_input(request["text"])
        except UnsupportedStress:
            self.output(422, {"error":"This local voice cannot honor marked short phrases or stress contrasts. Keep the original text and choose an explicit-stress voice.","code":"unsupported_stress"}); return
        except (ValueError, UnicodeError, socket.timeout):
            self.output(400, {"error": "Invalid short text or binding"}); return
        if self.path == "/generate" and not self.server.engine.prepared:
            self.output(503, {"error": "Operator must prewarm the installed model before enabling this provider"}); return
        if not self.server.gate.acquire(blocking=False):
            self.output(429, {"error": "Local speech worker is busy"}); return
        # A worker-wide watchdog exits the owned worker on a stuck native kernel.
        # The private gateway keeps its request/budget state; the launcher can restart explicitly.
        preparing = self.path == "/prepare"
        watchdog = threading.Timer(150 if preparing else 53, lambda: os._exit(124))
        watchdog.daemon = True; watchdog.start()
        try:
            self.output(200, self.server.engine.generate(request, preparing=preparing))
            print(json.dumps({"event": "generated", **self.server.engine.last_stats}), flush=True)
        except TimeoutError:
            self.output(504, {"error": "Local speech generation timed out"})
        except Exception as error:
            print(json.dumps({"event":"generation_failed","error_type":type(error).__name__,"loaded":self.server.engine.model is not None}), flush=True)
            self.output(502, {"error": "Local speech generation failed; inspect its installation and public fixture"})
        finally:
            watchdog.cancel(); self.server.gate.release()


def main():
    parser = argparse.ArgumentParser(); parser.add_argument("--port", type=int, default=8790)
    args = parser.parse_args()
    if not 1024 <= args.port <= 65535:
        parser.error("Use a nonprivileged loopback port")
    os.nice(10)
    verify_install()
    if TOKEN_FILE.is_symlink() or TOKEN_FILE.stat().st_mode & 0o077:
        raise RuntimeError("Worker token file must be private")
    token = TOKEN_FILE.read_text().strip()
    if len(token) < 32:
        raise RuntimeError("Invalid worker token file")
    engine = Engine()
    server = Server(("127.0.0.1", args.port), engine, token)
    def stop(_signal, _frame):
        threading.Thread(target=server.shutdown, daemon=True).start()
    signal.signal(signal.SIGTERM, stop); signal.signal(signal.SIGINT, stop)
    print(json.dumps({"event": "ready", "port": args.port, "model": MODEL_ID, "revision": MODEL_REVISION, "loaded": False}), flush=True)
    try:
        server.serve_forever(poll_interval=0.2)
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
