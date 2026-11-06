#!/usr/bin/env python3
"""Run one macOS command with sampled footprint and verified-process cleanup.

Scope: root and observed same-UID descendants, not an entire Apple coalition.
WebKit XPC services can live outside this tree; global memory pressure is an
additional stop condition, not attribution. Tree-only mode is insufficient for
WebKit/native UI QA until its external helpers are accounted for. This is a
sampled guard, not a kernel
memory cap: sampling/collection takes time and limits can overshoot. Invoke the
executable directly, never a detaching launcher such as `open -a`.
"""
import argparse
import ctypes
import errno
import json
import os
from pathlib import Path
import signal
import struct
import subprocess
import sys
import time


class MacProcesses:
    def __init__(self):
        if sys.platform != "darwin":
            raise RuntimeError("This supervisor requires macOS libproc.")
        self.lib = ctypes.CDLL("/usr/lib/libSystem.B.dylib", use_errno=True)
        for name, arguments in {
            "proc_listpids": [ctypes.c_uint32, ctypes.c_uint32, ctypes.c_void_p, ctypes.c_int],
            "proc_pidinfo": [ctypes.c_int, ctypes.c_int, ctypes.c_uint64, ctypes.c_void_p, ctypes.c_int],
            "proc_pid_rusage": [ctypes.c_int, ctypes.c_int, ctypes.c_void_p],
            "sysctlbyname": [ctypes.c_char_p, ctypes.c_void_p, ctypes.POINTER(ctypes.c_size_t),
                             ctypes.c_void_p, ctypes.c_size_t],
        }.items():
            function = getattr(self.lib, name)
            function.argtypes = arguments
            function.restype = ctypes.c_int

    def identity(self, pid):
        # SDK proc_bsdinfo: sizeof136, UID20, pgid100, start timeval120/128.
        bsd = ctypes.create_string_buffer(136)
        ctypes.set_errno(0)
        size = self.lib.proc_pidinfo(pid, 3, 0, bsd, len(bsd))
        if size != len(bsd):
            error = ctypes.get_errno()
            if size == 0 and error in (0, errno.ESRCH, errno.ENOENT):
                return None
            raise OSError(error, f"Cannot inspect process identity {pid} (bytes={size})")
        return {"pid": pid, "ppid": struct.unpack_from("I", bsd, 16)[0],
                "uid": struct.unpack_from("I", bsd, 20)[0],
                "pgid": struct.unpack_from("I", bsd, 100)[0],
                "start": list(struct.unpack_from("QQ", bsd, 120)),
                "status": struct.unpack_from("I", bsd, 4)[0]}

    def snapshot(self):
        size = self.lib.proc_listpids(1, 0, None, 0)
        if size <= 0:
            raise RuntimeError("Cannot enumerate processes; refusing unmonitored run.")
        capacity = size + 4096
        while capacity <= 8 * 1024 * 1024:
            pids = (ctypes.c_int * ((capacity + 3) // 4))()
            count = self.lib.proc_listpids(1, 0, pids, ctypes.sizeof(pids))
            if count <= 0 or count % 4:
                raise RuntimeError("Process enumeration failed or returned invalid bytes.")
            if count < ctypes.sizeof(pids):
                break
            capacity *= 2
        else:
            raise RuntimeError("Process enumeration exceeded its bounded buffer.")
        result = {}
        for pid in pids[:count // 4]:
            if pid <= 0:
                continue
            try:
                row = self.identity(pid)
            except PermissionError:
                # Protected unrelated system processes are not ours. Known owned
                # identities are inspected again explicitly by ProcessTree.
                continue
            if row and row["uid"] == os.getuid():
                result[pid] = row
        return result

    def exists(self, pid):
        try:
            os.kill(pid, 0)
            return True
        except ProcessLookupError:
            return False
        except PermissionError:
            return True

    def unreaped_exit(self, pid):
        try:
            result = os.waitid(os.P_PID, pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            return bool(result and result.si_pid == pid)
        except ChildProcessError:
            return False

    def usage(self, pid):
        # SDK rusage_info_v2 is160bytes; resident64, footprint72, start80.
        buf = ctypes.create_string_buffer(160)
        if self.lib.proc_pid_rusage(pid, 2, buf) != 0:
            return None
        return {"rss": struct.unpack_from("Q", buf, 64)[0],
                "footprint": struct.unpack_from("Q", buf, 72)[0],
                "usage_start": struct.unpack_from("Q", buf, 80)[0]}

    def pressure(self):
        value = ctypes.c_int()
        size = ctypes.c_size_t(ctypes.sizeof(value))
        if self.lib.sysctlbyname(b"kern.memorystatus_vm_pressure_level",
                                ctypes.byref(value), ctypes.byref(size), None, 0) \
                or size.value != ctypes.sizeof(value):
            raise RuntimeError("Cannot read macOS memory pressure; refusing unmonitored run.")
        return value.value


def same_identity(left, right):
    return bool(left and right and left["pid"] == right["pid"]
                and left["uid"] == right["uid"] and left["start"] == right["start"])


def alive(row):
    return bool(row and row["status"] != 5)  # SDK SZOMB: exited, awaiting reaping.


class ProcessTree:
    def __init__(self, monitor, root_pid):
        self.monitor = monitor
        self.root_pid = root_pid
        self.owned = {}
        self.observed = {}
        self.unknown_since = {}
        row = monitor.identity(root_pid)
        if not row or row["uid"] != os.getuid():
            raise RuntimeError("Cannot pin the launched process's kernel identity.")
        self.remember(row)

    def remember(self, row):
        self.owned[row["pid"]] = dict(row)
        self.observed[(row["pid"], tuple(row["start"]))] = dict(row)

    def verified_identity(self, expected):
        row = self.monitor.identity(expected["pid"])
        if row is None and expected["pid"] == self.root_pid and self.monitor.unreaped_exit(self.root_pid):
            # macOS libproc omits zombies. WNOWAIT confirms our unreaped child
            # still reserves this exact PID, so its original group is safe to
            # discover before Popen releases the child identity.
            return {**expected, "status": 5}
        return row

    def missing_identity(self, pid):
        if not self.monitor.exists(pid):
            self.unknown_since.pop(pid, None)
            return
        since = self.unknown_since.setdefault(pid, time.monotonic())
        if time.monotonic() - since >= 0.5:
            raise RuntimeError(f"Kernel identity remains unavailable for observed process {pid}.")

    def measure(self, expected):
        pid = expected["pid"]
        before = self.verified_identity(expected)
        if before is None:
            self.missing_identity(pid)
        if not same_identity(before, expected) or not alive(before):
            return None
        known = same_identity(self.owned.get(pid), expected)
        if not known and (before["ppid"] != expected["ppid"] or before["pgid"] != expected["pgid"]):
            return None
        usage = self.monitor.usage(pid)
        after = self.verified_identity(expected)
        if after is None:
            self.missing_identity(pid)
        if not same_identity(before, after) or not alive(after):
            return None
        if not known and (after["ppid"] != expected["ppid"] or after["pgid"] != expected["pgid"]):
            return None
        if not usage or not usage["usage_start"]:
            # Ownership is already established by stable ancestry/identity even
            # when memory measurement fails; retain it for safe teardown.
            self.remember(after)
            raise RuntimeError(f"Lost footprint measurement for live owned process {pid}.")
        self.unknown_since.pop(pid, None)
        prior = self.owned.get(pid)
        if same_identity(prior, before) and prior.get("usage_start", usage["usage_start"]) != usage["usage_start"]:
            raise RuntimeError(f"Kernel identity changed during usage measurement for {pid}.")
        return {**after, **usage}

    def collect(self):
        table = self.monitor.snapshot()
        trusted = {}
        current = {}
        # Validate identities BEFORE using any numeric PID as an ancestry seed.
        for pid, expected in list(self.owned.items()):
            identity = self.verified_identity(expected)
            if same_identity(identity, expected):
                # An unreaped zombie still reserves its PID/group identity and
                # can establish ownership of children reparented on its exit.
                trusted[pid] = identity
            row = self.measure(expected)
            if row:
                trusted[pid] = row
                current[pid] = row
                self.remember(row)
        while True:
            added = False
            for pid, expected in table.items():
                if pid in trusted or not alive(expected):
                    continue
                parent = trusted.get(expected["ppid"])
                group_anchor = next((r for r in trusted.values()
                                     if r["pgid"] == self.root_pid), None) \
                    if expected["pgid"] == self.root_pid else None
                anchor = parent or group_anchor
                if not anchor or not same_identity(anchor, self.verified_identity(anchor)):
                    continue
                row = self.measure(expected)
                # Parent/group fields must still belong to the same observed
                # identity. Bracketing measurement prevents PID replacement
                # between the graph snapshot and first ownership admission.
                if not row or row["ppid"] != expected["ppid"] or row["pgid"] != expected["pgid"]:
                    continue
                if not same_identity(anchor, self.verified_identity(anchor)):
                    continue
                trusted[pid] = row
                current[pid] = row
                self.remember(row)
                added = True
            if not added:
                break
        return current

    def survivors(self, errors):
        remaining = []
        for pid, expected in list(self.owned.items()):
            try:
                row = self.verified_identity(expected)
                if row is None and self.monitor.exists(pid):
                    remaining.append(pid)
                elif same_identity(row, expected) and alive(row):
                    remaining.append(pid)
            except Exception as error:
                errors.append(f"identity {pid}: {error}")
                remaining.append(pid)  # Unknown is not successful cleanup.
        return remaining

    def terminate(self, sig, errors, sent=None, kill=None):
        kill = kill or os.kill
        for pid, expected in list(self.owned.items()):
            identity = (pid, tuple(expected["start"]))
            if sent is not None and identity in sent:
                continue
            try:
                row = self.verified_identity(expected)
                if same_identity(row, expected) and alive(row):
                    kill(pid, sig)
                    if sent is not None:
                        sent.add(identity)
            except ProcessLookupError:
                pass
            except Exception as error:
                errors.append(f"signal {int(sig)} to {pid}: {error}")


def poll_root(tree, root):
    if root.returncode is not None:
        return root.returncode
    expected = tree.owned.get(root.pid)
    row = tree.verified_identity(expected)
    if same_identity(row, expected) and alive(row):
        # Do not reap a process that exits just after this read. The next pass
        # observes its zombie identity and descendants before releasing its PID.
        return None
    tree.collect()
    return root.poll()


def cleanup(tree, root, grace=0.75, kill_grace=1.0):
    errors = []
    if tree is None:
        # Popen still owns its unreaped child; never discover arbitrary numeric
        # descendants when the initial kernel identity could not be established.
        try:
            root.kill()
            root.wait(timeout=kill_grace)
        except Exception as error:
            errors.append(f"unidentified root cleanup: {error}")
        return [], errors
    sent = set()
    for sig, seconds in ((signal.SIGTERM, grace), (signal.SIGKILL, kill_grace)):
        deadline = time.monotonic() + seconds
        while True:
            try:
                tree.collect()  # Includes children forked during TERM cleanup.
            except Exception as error:
                errors.append(f"cleanup observation: {error}")
            tree.terminate(sig, errors, sent if sig == signal.SIGTERM else None)
            try:
                poll_root(tree, root)
            except Exception as error:
                errors.append(f"root poll: {error}")
            remaining = tree.survivors(errors)
            if not remaining or time.monotonic() >= deadline:
                break
            time.sleep(0.05)
    try:
        root.wait(timeout=0.25)
    except Exception as error:
        errors.append(f"root reap: {error}")
    return tree.survivors(errors), sorted(set(errors))


def result_exit_code(result):
    if not result["cleanup_ok"]:
        return 125
    return result.get("command_exit", 124) if result.get("reason") == "command_exit" else 124


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--max-mib", type=int, required=True)
    parser.add_argument("--seconds", type=float, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command or args.max_mib <= 0 or not 0 < args.seconds <= 1800:
        parser.error("Provide a command, positive memory limit, and lifetime <=1800seconds.")
    if Path(command[0]).name == "open":
        parser.error("Invoke the executable directly, not the detaching `open` launcher.")
    monitor = MacProcesses()
    if monitor.pressure() != 1:
        parser.error("System memory pressure is elevated. No command was launched.")
    monitor.snapshot()  # Fail closed before launching if enumeration is unavailable.
    args.report.mkdir(parents=True, exist_ok=False)
    result = {"command_executable": command[0], "max_mib": args.max_mib,
              "seconds": args.seconds, "scope": "root and observed same-UID descendants; excludes external XPC",
              "peak_mib": 0, "samples": 0}
    root = tree = None
    stop_signal = None

    def stop(signum, _frame):
        nonlocal stop_signal
        stop_signal = signum

    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        signal.signal(sig, stop)
    started = time.monotonic()
    try:
        with (args.report / "stdout.log").open("wb") as out, \
             (args.report / "stderr.log").open("wb") as err, \
             (args.report / "memory.jsonl").open("w") as samples:
            root = subprocess.Popen(command, stdout=out, stderr=err, start_new_session=True)
            tree = ProcessTree(monitor, root.pid)
            result["pid"] = root.pid
            result["root_start"] = tree.owned[root.pid]["start"]
            (args.report / "running.json").write_text(json.dumps(result, indent=2) + "\n")
            print(json.dumps({"started_pid": root.pid, "report": str(args.report)}), flush=True)
            while True:
                current = tree.collect()
                code = poll_root(tree, root)
                if code is None and root.pid not in current:
                    raise RuntimeError("The live root process is no longer measurable.")
                elapsed = time.monotonic() - started
                footprint = sum(v["footprint"] for v in current.values())
                pressure = monitor.pressure()
                result["peak_mib"] = max(result["peak_mib"], footprint / 1048576)
                result["samples"] += 1
                if result["samples"] % 4 == 1:
                    samples.write(json.dumps({"seconds": round(elapsed, 2), "pressure": pressure,
                                              "processes": current}) + "\n")
                    samples.flush()
                if stop_signal:
                    result["reason"] = "signal"
                    break
                if footprint > args.max_mib * 1048576:
                    result["reason"] = "memory_limit"
                    break
                if pressure != 1:
                    result["reason"] = "system_memory_pressure"
                    break
                if code is not None:
                    result.update(reason="command_exit", command_exit=code)
                    break
                if elapsed >= args.seconds:
                    result["reason"] = "time_limit"
                    break
                time.sleep(0.25)
    except BaseException as error:
        result.update(reason="supervisor_error", error=str(error))
    finally:
        remaining, errors = cleanup(tree, root) if root is not None else ([], [])
        result["elapsed_seconds"] = round(time.monotonic() - started, 2)
        result["observed_identities"] = list(tree.observed.values()) if tree else []
        result["observed_pids"] = sorted({r["pid"] for r in result["observed_identities"]})
        result["remaining_pids"] = remaining
        result["cleanup_errors"] = errors
        result["cleanup_ok"] = not remaining and not errors
        (args.report / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result), flush=True)
    return result_exit_code(result)


if __name__ == "__main__":
    sys.exit(main())
