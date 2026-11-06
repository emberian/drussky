#!/usr/bin/env python3
"""Arm a bounded watcher, then launch the named QA app through LaunchServices.

Only a newly launched, uniquely identified fixture and Apple XPC services in
its new resource coalition are owned. A shared/pre-existing coalition is refused.
This watcher never launches an app and never kills by a broad process name.
"""
import argparse
import ctypes
import importlib.util
import json
import os
from pathlib import Path
import plistlib
import signal
import struct
import sys
import time

spec = importlib.util.spec_from_file_location("bounded", Path(__file__).with_name("run-bounded.py"))
bounded = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bounded)


def same(left, right):
    return bool(left and right and left["pid"] == right["pid"] and left["start"] == right["start"])


class Inspector(bounded.MacProcesses):
    def coalition(self, pid):
        # Apple XNU proc_info_private.h: PROC_PIDCOALITIONINFO20, 5xu64.
        # coalition.h: resource coalition index0. Inspect only; no private app API.
        value = ctypes.create_string_buffer(40)
        if self.lib.proc_pidinfo(pid, 20, 0, value, 40) != 40:
            raise RuntimeError(f"Cannot attribute resource coalition for {pid}")
        return struct.unpack_from("Q", value)[0]

    def executable(self, pid):
        value = ctypes.create_string_buffer(4096)
        if not self.lib.proc_pidpath(pid, value, len(value)):
            return None
        return os.path.realpath(os.fsdecode(value.value))

    def attribute(self, row):
        before = self.identity(row["pid"])
        if not same(row, before):
            return None
        path = self.executable(row["pid"])
        coalition = self.coalition(row["pid"])
        if not same(before, self.identity(row["pid"])):
            return None
        if not path:
            raise RuntimeError(f"Cannot identify live process {row['pid']}")
        return {**row, "path": path, "coalition": coalition}


def is_webkit(path):
    return (path.startswith("/System/") and "/WebKit.framework/" in path
            and "/XPCServices/com.apple.WebKit." in path
            and Path(path).name.startswith("com.apple.WebKit."))


def is_qa_helper(path):
    # macOS also starts AudioToolbox/CascadeSets helpers for this app. Their
    # fresh exclusive coalition, stable identity and Apple system XPC path
    # establish attribution; do not chase an expanding list of service names.
    return (path.startswith("/System/") and "/Library/" in path
            and "/XPCServices/" in path and ".xpc/Contents/MacOS/" in path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--max-mib", type=int, default=768)
    parser.add_argument("--seconds", type=float, default=180)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    executable = args.executable.resolve(strict=True)
    bundle = next((p for p in executable.parents if p.suffix == ".app"), None)
    if not bundle or not 0 < args.seconds <= 600 or not 0 < args.max_mib <= 2048:
        parser.error("Expected a QA bundle, lifetime <=600sec and memory <=2048MiB.")
    info = plistlib.loads((bundle / "Contents/Info.plist").read_bytes())
    if not info.get("CFBundleIdentifier", "").startswith("town.lunar.rufren.qa.fixture-"):
        parser.error("Only an isolated Rufren QA fixture may be watched.")
    monitor = Inspector()
    if monitor.pressure() != 1:
        parser.error("Memory pressure elevated; watcher not armed.")
    initial = monitor.snapshot()
    baseline_coalitions = set()
    for row in initial.values():
        if (monitor.executable(row["pid"]) == str(executable)
                and same(row, monitor.identity(row["pid"]))):
            parser.error("The QA fixture is already running. Refusing ambiguous ownership.")
        try:
            sample = monitor.attribute(row)
        except (OSError, RuntimeError):
            # Unknown baseline attribution cannot safely prove a new coalition.
            # Include its readable coalition even if protected memory is unavailable.
            try:
                baseline_coalitions.add(monitor.coalition(row["pid"]))
            except (OSError, RuntimeError):
                if monitor.identity(row["pid"]):
                    parser.error("Cannot establish baseline coalition ownership.")
            continue
        if sample:
            baseline_coalitions.add(sample["coalition"])
            if sample["path"] == str(executable):
                parser.error("The QA fixture is already running. Refusing ambiguous ownership.")
    args.report.mkdir(parents=True, exist_ok=False)
    result = {"bundle": str(bundle), "max_mib": args.max_mib, "seconds": args.seconds,
              "peak_mib": 0, "peak_host_mib": 0, "peak_webkit_mib": 0,
              "peak_other_helpers_mib": 0}
    owned = {}
    root = None
    coalition = None
    stopped = False
    start = time.monotonic()

    def stop(_sig, _frame):
        nonlocal stopped
        stopped = True

    for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(sig, stop)

    def collect():
        nonlocal root, coalition
        table = monitor.snapshot()
        if root is None:
            for row in table.values():
                if same(row, initial.get(row["pid"])):
                    continue
                if monitor.executable(row["pid"]) != str(executable):
                    continue
                # Exact-path ownership is established before any fallible
                # coalition/footprint query so failed monitoring still stops it.
                if not same(row, monitor.identity(row["pid"])):
                    continue
                root = row
                owned[row["pid"]] = row
                sample = monitor.attribute(row)
                if sample is None:
                    continue
                coalition = sample["coalition"]
                if not coalition or coalition in baseline_coalitions:
                    raise RuntimeError("QA app inherited a shared coalition; stopped without adopting helpers.")
                result.update(pid=row["pid"], coalition=coalition)
                break
        current = {}
        for row in table.values():
            if coalition is None or coalition in baseline_coalitions:
                continue
            try:
                if monitor.coalition(row["pid"]) != coalition:
                    continue
                sample = monitor.attribute(row)
            except (OSError, RuntimeError):
                if not same(row, monitor.identity(row["pid"])):
                    continue
                raise
            if sample is None:
                continue
            if sample["coalition"] != coalition:
                continue
            if not same(row, root) and not is_qa_helper(sample["path"]):
                raise RuntimeError(f"Unexpected process in QA coalition: {row['pid']} {sample['path']}; refusing ownership")
            prior = owned.get(row["pid"])
            if prior and not same(prior, row):
                raise RuntimeError("An owned PID was reused; refusing new identity.")
            owned[row["pid"]] = row
            current[row["pid"]] = sample
        # Discover every attributable member before measuring any of them.
        # A live measurement failure must not strand that process or its peers.
        for pid, sample in list(current.items()):
            usage = monitor.usage(pid)
            if not same(sample, monitor.identity(pid)):
                current.pop(pid)
                continue
            if not usage:
                raise RuntimeError(f"Cannot measure owned live process {pid}")
            current[pid] = {**sample, **{k: v for k, v in usage.items() if k != "start"}}
        return current

    def terminate(sig):
        errors = []
        for pid, identity in list(owned.items()):
            try:
                if same(identity, monitor.identity(pid)):
                    if pid != (root["pid"] if root else None) and monitor.coalition(pid) != coalition:
                        continue
                    os.kill(pid, sig)
            except ProcessLookupError:
                pass
            except Exception as error:
                errors.append(str(error))
        return errors

    try:
        (args.report / "armed.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps({"armed": str(executable), "report": str(args.report)}), flush=True)
        with (args.report / "memory.jsonl").open("w") as output:
            while True:
                current = collect()
                elapsed = time.monotonic() - start
                host = current.get(root["pid"], {}).get("footprint", 0) if root else 0
                webkit = sum(p["footprint"] for p in current.values() if is_webkit(p["path"]))
                total = sum(p["footprint"] for p in current.values())
                result["peak_mib"] = max(result["peak_mib"], total / 1048576)
                result["peak_host_mib"] = max(result["peak_host_mib"], host / 1048576)
                result["peak_webkit_mib"] = max(result["peak_webkit_mib"], webkit / 1048576)
                result["peak_other_helpers_mib"] = max(result["peak_other_helpers_mib"], (total-host-webkit) / 1048576)
                output.write(json.dumps({"seconds": round(elapsed, 2), "processes": current}) + "\n")
                output.flush()
                if stopped:
                    result["reason"] = "signal"
                    break
                if total > args.max_mib * 1048576:
                    result["reason"] = "memory_limit"
                    break
                if monitor.pressure() != 1:
                    result["reason"] = "system_memory_pressure"
                    break
                if root and not same(root, monitor.identity(root["pid"])):
                    result["reason"] = "app_exit"
                    break
                if elapsed >= args.seconds:
                    result["reason"] = "time_limit"
                    break
                time.sleep(0.5)
    except BaseException as error:
        result.update(reason="watcher_error", error=str(error))
    finally:
        errors = []
        for _ in range(4):
            try:
                collect()
            except Exception as error:
                errors.append(str(error))
            errors += terminate(signal.SIGTERM)
            time.sleep(0.25)
        errors += terminate(signal.SIGKILL)
        time.sleep(0.25)
        remaining = []
        for pid, identity in owned.items():
            try:
                row = monitor.identity(pid)
                if same(identity, row) and row.get("status") != 5: # SZOMB
                    remaining.append(pid)
            except Exception as error:
                errors.append(str(error))
                remaining.append(pid)
        result.update(elapsed_seconds=round(time.monotonic()-start, 2),
                      owned_pids=sorted(owned), remaining_pids=remaining,
                      cleanup_errors=sorted(set(errors)))
        (args.report / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result), flush=True)
    return 0 if result.get("reason") == "app_exit" and not remaining and not errors else 124


if __name__ == "__main__":
    sys.exit(main())
