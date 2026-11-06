#!/usr/bin/env python3
"""Disposable local-speech acceptance; never touches the working app relay."""
import argparse
import base64
import hashlib
import io
import json
import os
import secrets
import shlex
import shutil
import signal
import subprocess
import sys
import time
import urllib.error
import urllib.request
import wave
from pathlib import Path
sys.dont_write_bytecode = True
from qwen_model import PROJECT, STATE, TOKEN_FILE
from qwen_client import NoRedirect, call as worker_call, verify_response

ROOT = STATE / "gateway-qa"
TOKEN = ROOT / "relay.token"
PID = ROOT / "relay.pid"
LOG = ROOT / "relay.log"
BINARY = ROOT / "rufren-relay"
PORT = 8872
PUBLIC_TEXT = "Сего́дня я изуча́ю ру́сский язы́к и чита́ю интере́сную кни́гу о звёздах."


def process_command(pid):
    return subprocess.run(["/bin/ps", "-ww", "-p", str(pid), "-o", "command="], capture_output=True, text=True, timeout=5).stdout.strip()


def owned(pid):
    return shlex.split(process_command(pid)) == [str(BINARY)]


def exchange(path, body=None, authenticated=True):
    if path not in ("/health", "/api/speech/status", "/api/speech/generate"):
        raise ValueError("Unknown acceptance route")
    headers = {}
    if authenticated:
        headers["Authorization"] = "Bearer " + TOKEN.read_text().strip()
    encoded = None
    if body is not None:
        encoded = json.dumps(body, ensure_ascii=False).encode()
        headers["Content-Type"] = "application/json"
    query = urllib.request.Request(f"http://127.0.0.1:{PORT}{path}", data=encoded, headers=headers)
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    try:
        with opener.open(query, timeout=58) as response:
            raw = response.read(4 * 1024 * 1024 + 1)
            if len(raw) > 4 * 1024 * 1024:
                raise ValueError("Acceptance response exceeded limit")
            return response.status, json.loads(raw) if path != "/health" else raw.decode()
    except urllib.error.HTTPError as error:
        return error.code, error.read(1024).decode("utf-8", errors="replace")


def start(web_port, provider):
    if ROOT.exists() and any(ROOT.iterdir()):
        raise RuntimeError("Acceptance directory already contains work; retained without overwrite")
    ROOT.mkdir(parents=True, exist_ok=True, mode=0o700); os.chmod(ROOT, 0o700)
    source = PROJECT / "target" / "debug" / "rufren-relay"
    with source.open("rb") as source_file, BINARY.open("xb") as target:
        shutil.copyfileobj(source_file, target)
    BINARY.chmod(0o700)
    descriptor = os.open(TOKEN, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w") as output:
        output.write(secrets.token_urlsafe(32) + "\n")
    environment = {key: value for key, value in os.environ.items() if not key.startswith("RUFREN_")}
    for key in ("ANTHROPIC_API_KEY", "GEMINI_API_KEY", "OPENAI_API_KEY", "GOOGLE_API_KEY", "XAI_API_KEY"):
        environment.pop(key, None)
    environment.update(
        RUFREN_SYNC_TOKEN=TOKEN.read_text().strip(), RUFREN_RELAY_BIND=f"127.0.0.1:{PORT}",
        RUFREN_RELAY_DB=str(ROOT / "relay.sqlite"), RUFREN_RELAY_ASSETS=str(ROOT / "assets"),
        RUFREN_ALLOWED_ORIGINS=f"http://127.0.0.1:{web_port},http://localhost:{web_port}",
        RUFREN_TUTOR_PROVIDER="disabled", RUFREN_ASR_PROVIDER="disabled", RUFREN_SPEECH_PROVIDER=provider,
        RUFREN_QWEN_WORKER_TOKEN_FILE=str(TOKEN_FILE), RUFREN_QWEN_WORKER_PORT="8790", RUFREN_SPEECH_MAX_REQUESTS="40",
    )
    log = os.open(LOG, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        child = subprocess.Popen([str(BINARY)], cwd=PROJECT, env=environment, stdin=subprocess.DEVNULL,
                                 stdout=log, stderr=log, start_new_session=True)
    finally:
        os.close(log)
    descriptor = os.open(PID, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w") as output:
        output.write(str(child.pid))
    for _ in range(50):
        if child.poll() is not None:
            raise RuntimeError("Disposable gateway exited; inspect its retained private log")
        try:
            code, status = exchange("/api/speech/status")
        except Exception:
            time.sleep(0.1); continue
        if code != 200 or not status.get("enabled") or status.get("provider") != ("local_russian" if provider == "local_russian" else "local_qwen3_tts"):
            raise RuntimeError("Disposable gateway is not ready for the pinned local provider")
        print(json.dumps({"url": f"http://127.0.0.1:{PORT}", "pid": child.pid, "token_file": str(TOKEN),
                          "binary_sha256": hashlib.sha256(BINARY.read_bytes()).hexdigest(), "status": status}, indent=2)); return
    raise RuntimeError("Disposable gateway readiness timed out; PID and log retained")


def reject_stress():
    if not PID.exists() or not owned(int(PID.read_text())):
        raise RuntimeError("No matching disposable gateway process")
    before = worker_call("/status").get("last_stats")
    rejections = []
    for index, text in enumerate(("Это за́мок.", "Это замо́к.")):
        request = {"text": text, "request_id": f"qwen-gateway-reject-{index}",
                   "revision_id": "22222222-2222-4222-8222-222222222222"}
        code, message = exchange("/api/speech/generate", request)
        if code != 400:
            raise RuntimeError("Marked contrast was not rejected before generation")
        rejections.append({"source_text": text, "status": code, "message": message})
    after = worker_call("/status").get("last_stats")
    if before != after:
        raise RuntimeError("Rejected requests unexpectedly changed worker generation evidence")
    report = {"stress_rejections": rejections, "rejections_did_not_generate": True,
              "generation_reserved_for_root_ui": True}
    with (ROOT / "stress-rejections.json").open("x") as output:
        json.dump(report, output, ensure_ascii=False, indent=2)
    print(json.dumps(report, ensure_ascii=False, indent=2))


def reject_ambiguous():
    if not PID.exists() or not owned(int(PID.read_text())):
        raise RuntimeError("No matching disposable gateway process")
    code, before = exchange("/api/speech/status")
    if code != 200 or before.get("provider") != "local_russian":
        raise RuntimeError("Ambiguity acceptance requires the declared local Russian router")
    request = {"text": "Это замок.", "request_id": "hybrid-gateway-ambiguous",
               "revision_id": "22222222-2222-4222-8222-222222222222"}
    code, message = exchange("/api/speech/generate", request)
    after_code, after = exchange("/api/speech/status")
    if code != 400 or after_code != 200 or before["remaining_reservations"] != after["remaining_reservations"]:
        raise RuntimeError("Ambiguous short input was not rejected before admission")
    report = {"source_text": request["text"], "http": code, "message": message,
              "request_allowance_unchanged": True, "generation_reserved_for_root": True}
    with (ROOT / "ambiguity-rejection.json").open("x") as output:
        json.dump(report, output, ensure_ascii=False, indent=2)
    print(json.dumps(report, ensure_ascii=False, indent=2))


def stop():
    pid = int(PID.read_text())
    if not owned(pid):
        raise RuntimeError("Recorded acceptance PID does not match; no signal sent")
    if not owned(pid):
        raise RuntimeError("Acceptance process changed before stop")
    os.kill(pid, signal.SIGTERM)
    for _ in range(50):
        if not process_command(pid):
            PID.unlink(); print("Disposable gateway stopped; data and artifacts retained"); return
        time.sleep(0.1)
    raise RuntimeError("Disposable gateway has not stopped; PID retained")


def main():
    global ROOT, TOKEN, PID, LOG, BINARY
    parser = argparse.ArgumentParser(); parser.add_argument("action", choices=["start", "reject-stress", "reject-ambiguous", "stop", "status"])
    parser.add_argument("--web-port", type=int, default=8091)
    parser.add_argument("--provider", choices=["qwen", "local_russian"], default="qwen")
    args = parser.parse_args()
    if args.provider == "local_russian":
        ROOT = STATE / "hybrid-gateway-qa"
        TOKEN, PID, LOG, BINARY = (ROOT / name for name in ("relay.token", "relay.pid", "relay.log", "rufren-relay"))
    if not 1024 <= args.web_port <= 65535:
        parser.error("Use the exact nonprivileged local QA web port")
    if args.action == "start":
        start(args.web_port, args.provider)
    elif args.action == "reject-stress":
        if args.provider != "qwen":
            parser.error("Marked text is supported by local_russian; use reject-ambiguous")
        reject_stress()
    elif args.action == "reject-ambiguous":
        if args.provider != "local_russian":
            parser.error("Use the explicit local_russian provider for this check")
        reject_ambiguous()
    elif args.action == "stop":
        stop()
    else:
        code, status = exchange("/api/speech/status"); print(json.dumps({"http": code, "status": status}, indent=2))


if __name__ == "__main__":
    main()
