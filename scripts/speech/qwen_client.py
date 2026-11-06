#!/usr/bin/env python3
"""Bounded authenticated client for the project-owned loopback speech worker."""
import argparse
import base64
import hashlib
import io
import json
import re
import time
import urllib.error
import urllib.request
import wave
from pathlib import Path

import sys
sys.dont_write_bytecode = True

from qwen_model import MAX_PCM_BYTES, MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, MODEL_ID, MODEL_REVISION, STATE, TOKEN_FILE
from qwen_worker import validate
from qwen_frontend import synthesis_input

PUBLIC_TEXT = "Здравствуйте! Как у вас дела? Я учу русский язык. На двери висит замо́к, а на холме стоит старый за́мок. Всё готово? Для каждого натурального числа существует следующее число."


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *_args, **_kwargs):
        raise ValueError("Local worker redirects are not permitted")


def call(path, request=None, port=8790):
    if path not in ("/status", "/generate", "/prepare") or not 1024 <= port <= 65535:
        raise ValueError("Invalid fixed worker route or port")
    if TOKEN_FILE.is_symlink() or TOKEN_FILE.stat().st_mode & 0o077:
        raise ValueError("Worker credentials must have private permissions")
    token = TOKEN_FILE.read_text().strip()
    data = None if request is None else json.dumps(validate(request), ensure_ascii=False).encode("utf-8")
    if data is not None and len(data) > MAX_REQUEST_BYTES:
        raise ValueError("Request exceeds the worker input limit")
    headers = {"Authorization": "Bearer " + token}
    if data is not None:
        headers["Content-Type"] = "application/json"
    query = urllib.request.Request(f"http://127.0.0.1:{port}{path}", data=data, headers=headers)
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    started = time.monotonic()
    deadline = 145 if path == "/prepare" else 55 if path == "/generate" else 5
    try:
        with opener.open(query, timeout=deadline) as response:
            if response.headers.get("Content-Length") and int(response.headers["Content-Length"]) > MAX_RESPONSE_BYTES:
                raise ValueError("Worker response exceeded the size limit")
            output = bytearray()
            while block := response.read(64 * 1024):
                output.extend(block)
                if len(output) > MAX_RESPONSE_BYTES or time.monotonic() - started > deadline:
                    raise ValueError("Worker response exceeded its size or time limit")
    except urllib.error.HTTPError as error:
        raise RuntimeError(f"Local speech worker returned HTTP {error.code}") from None
    value = json.loads(output)
    if request is not None:
        verify_response(request, value)
    return value


def verify_response(request, value):
    request = validate(request)
    if (not isinstance(value, dict) or value.get("request_id") != request["request_id"]
            or value.get("revision_id") != request["revision_id"] or value.get("source_text") != request["text"]
            or value.get("sample_rate") != 24000 or value.get("channels") != 1):
        raise ValueError("Worker response does not match its request")
    provenance = value.get("provenance", {})
    if provenance.get("synthesis_input") != synthesis_input(request["text"]):
        raise ValueError("Actual model input provenance does not match the declared frontend")
    if (provenance.get("provider") != "local_qwen3_tts" or provenance.get("kind") != "ai_generated_speech"
            or provenance.get("model") != f"{MODEL_ID}@{MODEL_REVISION}"):
        raise ValueError("Unexpected local speech provenance")
    encoded = value.get("wav_base64")
    digest = value.get("sha256")
    if not isinstance(encoded, str) or len(encoded) > ((MAX_PCM_BYTES + 4096 + 2) // 3) * 4:
        raise ValueError("Invalid encoded audio size")
    if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
        raise ValueError("Invalid original audio digest")
    data = base64.b64decode(encoded, validate=True)
    if len(data) > MAX_PCM_BYTES + 4096 or hashlib.sha256(data).hexdigest() != digest:
        raise ValueError("Original audio integrity check failed")
    with wave.open(io.BytesIO(data)) as audio:
        if (audio.getnchannels() != 1 or audio.getsampwidth() != 2 or audio.getframerate() != 24000
                or audio.getnframes() != value.get("frames") or not 0 < audio.getnframes() * 2 <= MAX_PCM_BYTES):
            raise ValueError("Unexpected original WAV format")
        pcm = audio.readframes(audio.getnframes())
        if len(pcm) != audio.getnframes() * 2 or not any(pcm):
            raise ValueError("Empty or truncated original WAV")
    return data


def main():
    parser = argparse.ArgumentParser(); parser.add_argument("action", choices=["status", "prepare", "public-fixture", "public-fixture-warm"])
    parser.add_argument("--port", type=int, default=8790)
    args = parser.parse_args()
    if args.action == "status":
        print(json.dumps(call("/status", port=args.port), indent=2)); return
    if args.action == "prepare":
        request = {"text": PUBLIC_TEXT.replace("\u0301", ""), "request_id": "qwen-operator-prewarm", "revision_id": None}
        response = call("/prepare", request, args.port)
        print(json.dumps({"sha256": response["sha256"], "status": call("/status", port=args.port)}, indent=2)); return
    warm = args.action == "public-fixture-warm"
    fixture = "public-russian-v2-warm" if warm else "public-russian-v2"
    request = {"text": PUBLIC_TEXT.replace("\u0301", ""), "request_id": "qwen-" + fixture, "revision_id": None}
    directory = STATE / "samples"; directory.mkdir(parents=True, exist_ok=True)
    target = directory / (fixture + ".wav")
    if target.exists() or target.with_suffix(".json").exists():
        raise FileExistsError("Public fixture already exists; use prepare to warm without replacing it")
    response = call("/generate" if warm else "/prepare", request, args.port)
    data = verify_response(request, response)
    with target.open("xb") as output:
        output.write(data)
    metadata = {key: value for key, value in response.items() if key != "wav_base64"}
    stats = call("/status", port=args.port).get("last_stats")
    metadata["generation_stats"] = stats
    with target.with_suffix(".json").open("x") as output:
        output.write(json.dumps(metadata, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"sample": str(target), "duration_seconds": response["frames"] / 24000,
                      "sha256": response["sha256"], "stats": stats}, indent=2))


if __name__ == "__main__":
    main()
