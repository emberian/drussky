#!/usr/bin/env python3
"""Start/stop only the project-owned worker, using its exact recorded PID."""
import argparse
import json
import os
import signal
import shlex
import subprocess
import sys
import time
from pathlib import Path

import sys
sys.dont_write_bytecode = True

from qwen_model import LOG_FILE, PID_FILE, PROJECT, STATE

WORKER = PROJECT / "scripts" / "speech" / "qwen_worker.py"
PYTHON = STATE / "venv" / "bin" / "python"
PORT_FILE = STATE / "worker.port"


def command(pid):
    result = subprocess.run(["/bin/ps", "-ww", "-p", str(pid), "-o", "command="], capture_output=True, text=True, timeout=5)
    return result.stdout.strip() if result.returncode == 0 else ""


def owned(pid):
    value = command(pid)
    try:
        arguments = shlex.split(value)
        # macOS framework Python rewrites argv[0] to its real executable.
        interpreter_root = PYTHON.resolve().parent.parent
        return (len(arguments) == 4 and Path(arguments[0]).resolve().is_relative_to(interpreter_root)
                and arguments[1] == str(WORKER) and arguments[2] == "--port" and arguments[3].isdigit())
    except (ValueError, OSError):
        return False


def start(port):
    if PID_FILE.exists():
        pid = int(PID_FILE.read_text())
        if owned(pid):
            print(f"Owned local worker already running with PID {pid}"); return
        if command(pid):
            raise RuntimeError("Recorded PID belongs to another process; retained PID file for inspection")
        PID_FILE.unlink()
    STATE.mkdir(parents=True, exist_ok=True, mode=0o700); LOG_FILE.parent.mkdir(exist_ok=True)
    descriptor = os.open(PID_FILE, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    environment = dict(os.environ, PYTHONUNBUFFERED="1", PYTHONDONTWRITEBYTECODE="1", OMP_NUM_THREADS="2", OPENBLAS_NUM_THREADS="2", VECLIB_MAXIMUM_THREADS="2")
    try:
        log = os.open(LOG_FILE, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
        try:
            child = subprocess.Popen([str(PYTHON), str(WORKER), "--port", str(port)], cwd=PROJECT,
                                     env=environment, stdin=subprocess.DEVNULL, stdout=log, stderr=log, start_new_session=True)
        finally:
            os.close(log)
        os.write(descriptor, str(child.pid).encode("ascii")); os.fsync(descriptor)
        PORT_FILE.write_text(str(port))
    finally:
        os.close(descriptor)
    from qwen_client import call
    for _ in range(50):
        if child.poll() is not None:
            raise RuntimeError("Local worker did not start; inspect its task-owned log")
        try:
            status = call("/status", port=port)
        except Exception:
            time.sleep(0.1); continue
        print(json.dumps({"pid": child.pid, "port": port, "loaded": status["loaded"], "model": status["model"]})); return
    raise RuntimeError("Worker readiness timed out; its PID/log were retained for inspection")


def stop():
    if not PID_FILE.exists():
        print("No owned worker PID file"); return
    pid = int(PID_FILE.read_text())
    if not command(pid):
        PID_FILE.unlink(); print("Owned worker already stopped"); return
    if not owned(pid):
        raise RuntimeError("Recorded PID does not match this project's worker; no signal sent")
    # Re-read immediately before signaling: never kill by process name or stale pattern.
    if not owned(pid):
        raise RuntimeError("Worker identity changed before stop")
    os.kill(pid, signal.SIGTERM)
    for _ in range(50):
        if not command(pid):
            PID_FILE.unlink(); print("Owned local worker stopped"); return
        time.sleep(0.1)
    raise RuntimeError("Worker has not stopped; PID file retained, no unrelated processes touched")


def main():
    parser = argparse.ArgumentParser(); parser.add_argument("action", choices=["start", "stop", "status"])
    parser.add_argument("--port", type=int, default=8790)
    args = parser.parse_args()
    if not 1024 <= args.port <= 65535:
        parser.error("Use a nonprivileged loopback port")
    if args.action == "start":
        start(args.port)
    elif args.action == "stop":
        stop()
    else:
        from qwen_client import call
        print(json.dumps(call("/status", port=int(PORT_FILE.read_text()) if PORT_FILE.exists() else args.port), indent=2))


if __name__ == "__main__":
    main()
