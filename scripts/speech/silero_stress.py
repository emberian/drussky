#!/usr/bin/env python3
"""Run exactly one public explicit-stress pair after an approved compute slot."""
import base64
import hashlib
import io
import json
import os
from pathlib import Path
import selectors
import shlex
import subprocess
import sys
import time
import wave
sys.dont_write_bytecode = True

from silero_frontend import synthesis_input
from silero_model import (
    MAX_PCM_BYTES, MAX_RESPONSE_BYTES, MODEL_ID, MODEL_SHA256, PROJECT, PUBLIC_CASES,
    RATE, STATE, VOICE,
)

WORKER = PROJECT / "scripts" / "speech" / "silero_worker.py"
PYTHON = STATE / "venv" / "bin" / "python"
PID_FILE = STATE / "pair.pid"
LOG_FILE = STATE / "logs" / "pair-worker.log"


def command(pid):
    result = subprocess.run(["/bin/ps", "-ww", "-p", str(pid), "-o", "command="], capture_output=True, text=True, timeout=3)
    return result.stdout.strip() if result.returncode == 0 else ""


def owned(child):
    if child.poll() is not None:
        return False
    try:
        args = shlex.split(command(child.pid))
        return (len(args) == 2 and args[1] == str(WORKER)
                and Path(args[0]).resolve().is_relative_to(PYTHON.resolve().parent.parent)
                and PID_FILE.read_text() == str(child.pid))
    except (OSError, ValueError):
        return False


def stop_owned(child):
    if child.poll() is not None:
        return
    if not owned(child):
        raise RuntimeError("Recorded child identity changed; no signal sent")
    child.terminate()
    try:
        child.wait(timeout=2)
    except subprocess.TimeoutExpired:
        if not owned(child):
            raise RuntimeError("Child identity changed before forced stop")
        child.kill(); child.wait(timeout=2)


def exchange(requests):
    payload = json.dumps({"requests": requests}, ensure_ascii=False).encode()
    descriptor = os.open(PID_FILE, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    child = None
    selector = selectors.DefaultSelector()
    output = {"stdout": bytearray(), "stderr": bytearray()}
    try:
        environment = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", PYTHONUNBUFFERED="1",
                           OMP_NUM_THREADS="2", OPENBLAS_NUM_THREADS="2", VECLIB_MAXIMUM_THREADS="2")
        child = subprocess.Popen([str(PYTHON), str(WORKER)], cwd=PROJECT, env=environment,
                                 stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                 start_new_session=True)
        os.write(descriptor, str(child.pid).encode()); os.fsync(descriptor)
        child.stdin.write(payload); child.stdin.close()
        for label, pipe in (("stdout", child.stdout), ("stderr", child.stderr)):
            os.set_blocking(pipe.fileno(), False); selector.register(pipe, selectors.EVENT_READ, label)
        started = time.monotonic()
        while selector.get_map():
            if time.monotonic() - started > 30:
                stop_owned(child); raise TimeoutError("Controlled Silero pair exceeded 30 seconds")
            for key, _ in selector.select(timeout=0.1):
                block = os.read(key.fileobj.fileno(), 65536)
                if not block:
                    selector.unregister(key.fileobj); continue
                destination = output[key.data]
                maximum = 2 * MAX_RESPONSE_BYTES if key.data == "stdout" else 262144
                if len(destination) + len(block) > maximum:
                    stop_owned(child); raise ValueError("Controlled worker exceeded its bounded output")
                destination.extend(block)
        returncode = child.wait(timeout=2)
        LOG_FILE.parent.mkdir(parents=True, exist_ok=True)
        with LOG_FILE.open("ab") as log:
            log.write(output["stderr"])
        if returncode != 0:
            raise RuntimeError(f"Controlled Silero worker failed with exit status {returncode}; inspect its private log")
        return json.loads(output["stdout"])
    finally:
        selector.close(); os.close(descriptor)
        if child is not None:
            stop_owned(child)
            try:
                if PID_FILE.read_text() == str(child.pid):
                    PID_FILE.unlink()
            except OSError:
                pass
        else:
            try:
                if PID_FILE.read_text() == "":
                    PID_FILE.unlink()
            except OSError:
                pass


def verify(request, response):
    if (response.get("source_text") != request["text"] or response.get("request_id") != request["request_id"]
            or response.get("revision_id") is not None or response.get("sample_rate") != RATE
            or response.get("channels") != 1):
        raise ValueError("Public comparison response lost its source binding")
    provenance = response.get("provenance", {})
    if (provenance.get("provider") != "local_silero_tts" or provenance.get("kind") != "ai_generated_speech"
            or provenance.get("model") != f"{MODEL_ID}@{MODEL_SHA256}" or provenance.get("voice") != VOICE
            or provenance.get("synthesis_input") != synthesis_input(request["text"])):
        raise ValueError("Unexpected source/model/synthesis provenance")
    data = base64.b64decode(response["wav_base64"], validate=True)
    if len(data) > MAX_PCM_BYTES + 4096 or hashlib.sha256(data).hexdigest() != response.get("sha256"):
        raise ValueError("Original WAV failed its integrity limit/check")
    with wave.open(io.BytesIO(data)) as audio:
        if (audio.getnchannels() != 1 or audio.getframerate() != RATE or audio.getsampwidth() != 2
                or audio.getnframes() != response.get("frames") or not 0 < audio.getnframes() * 2 <= MAX_PCM_BYTES):
            raise ValueError("Original WAV has an invalid format or duration")
        pcm = audio.readframes(audio.getnframes())
        if len(pcm) != audio.getnframes() * 2 or not any(pcm):
            raise ValueError("Original WAV is truncated or silent")
    return data


def main():
    if sys.argv[1:] != ["pair"]:
        raise SystemExit("Usage: silero_stress.py pair — run only after an approved compute slot")
    STATE.mkdir(parents=True, exist_ok=True, mode=0o700)
    directory = STATE / "samples"; directory.mkdir(exist_ok=True)
    destinations = [directory / (case["request_id"] + ".wav") for case in PUBLIC_CASES]
    if any(path.exists() or path.with_suffix(".json").exists() for path in destinations):
        raise FileExistsError("Comparison artifacts already exist; no repeat synthesis was made")
    requests = [{"text": case["text"], "request_id": case["request_id"], "revision_id": None} for case in PUBLIC_CASES]
    result = exchange(requests)
    responses = result.get("responses")
    if not isinstance(responses, list) or len(responses) != len(requests):
        raise ValueError("Controlled pair returned an invalid item count")
    verified = [verify(request, response) for request, response in zip(requests, responses)]
    artifacts = []
    for case, request, response, original, path, measurement in zip(PUBLIC_CASES, requests, responses, verified, destinations, result["measurements"]):
        with path.open("xb") as audio:
            audio.write(original)
        metadata = {key: value for key, value in response.items() if key != "wav_base64"}
        metadata["intended_meaning"] = case["meaning"]
        metadata["generation_stats"] = measurement
        metadata["review_status"] = "controlled_synthetic_comparator; pronunciation not certified"
        with path.with_suffix(".json").open("x") as sidecar:
            json.dump(metadata, sidecar, ensure_ascii=False, indent=2)
        artifacts.append({"path": str(path), "source_text": request["text"], "synthesis_text": synthesis_input(request["text"])["synthesis_text"],
                          "sha256": response["sha256"], "duration_seconds": response["frames"] / RATE})
    stats = {key: value for key, value in result.items() if key != "responses"}
    print(json.dumps({"artifacts": artifacts, "stats": stats}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
