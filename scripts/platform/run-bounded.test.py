#!/usr/bin/env python3
"""Small identity/cleanup tests plus <=48MiB, <=3second real fixture payloads."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).with_name("run-bounded.py")
spec = importlib.util.spec_from_file_location("bounded", SCRIPT)
bounded = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bounded)


def row(pid, parent=1, group=10, start=1, status=2):
    return {"pid": pid, "ppid": parent, "pgid": group, "uid": os.getuid(),
            "start": [start, 0], "status": status}


class Monitor:
    def __init__(self, rows):
        self.rows = {r["pid"]: r for r in rows}
        self.usage_hook = None

    def identity(self, pid):
        return copy.deepcopy(self.rows.get(pid))

    def exists(self, pid):
        return pid in self.rows

    def unreaped_exit(self, pid):
        r=self.rows.get(pid)
        return bool(r and r["status"]==5)

    def snapshot(self):
        return copy.deepcopy(self.rows)

    def usage(self, pid):
        if self.usage_hook:
            return self.usage_hook(pid)
        r = self.rows[pid]
        return {"rss": 4096, "footprint": 8192, "usage_start": r["start"][0]}


class Root:
    def __init__(self, monitor):
        self.pid = 10
        self.returncode = None
        self.monitor = monitor

    def poll(self):
        if not bounded.alive(self.monitor.rows.get(self.pid)):
            self.returncode = 0
        return self.returncode

    def wait(self, timeout):
        if self.poll() is None:
            raise subprocess.TimeoutExpired("mock-root", timeout)
        return self.returncode


class IdentityTests(unittest.TestCase):
    def test_recycled_ancestor_never_adopts_unrelated_children(self):
        m = Monitor([row(10), row(20, 10)])
        tree = bounded.ProcessTree(m, 10)
        self.assertEqual(set(tree.collect()), {10, 20})
        m.rows[20] = row(20, 1, 900, start=2)
        m.rows[30] = row(30, 20, 900)
        self.assertEqual(set(tree.collect()), {10})
        killed = []
        tree.terminate(signal.SIGTERM, [], kill=lambda pid, sig: killed.append(pid))
        self.assertEqual(killed, [10])

    def test_recycled_root_and_its_new_group_are_never_owned(self):
        m = Monitor([row(10)])
        tree = bounded.ProcessTree(m, 10)
        m.rows[10] = row(10, start=2)
        m.rows[20] = row(20, 10)
        self.assertEqual(tree.collect(), {})
        self.assertNotIn(20, tree.owned)

    def test_snapshot_to_usage_replacement_does_not_admit_new_identity(self):
        m = Monitor([row(10), row(20, 10)])
        tree = bounded.ProcessTree(m, 10)
        def usage(pid):
            if pid == 20:
                m.rows[20] = row(20, 1, 900, start=9)
            return {"rss": 1, "footprint": 2, "usage_start": 1}
        m.usage_hook = usage
        self.assertEqual(set(tree.collect()), {10})
        self.assertNotIn(20, tree.owned)

    def test_new_legitimate_child_can_reuse_an_old_child_pid(self):
        m = Monitor([row(10), row(20, 10)])
        tree = bounded.ProcessTree(m, 10)
        tree.collect()
        m.rows[20] = row(20, 10, start=2)
        self.assertEqual(set(tree.collect()), {10, 20})
        self.assertEqual(tree.owned[20]["start"], [2, 0])
        self.assertEqual(len(tree.observed), 3)

    def test_unreaped_root_identity_anchors_reparented_group_child(self):
        m = Monitor([row(10, status=5), row(20, 1)])
        tree = bounded.ProcessTree(m, 10)
        self.assertEqual(set(tree.collect()), {20})
        self.assertIn(20, tree.owned)

    def test_live_usage_loss_fails_closed_but_keeps_child_for_cleanup(self):
        m = Monitor([row(10), row(20, 10)])
        tree = bounded.ProcessTree(m, 10)
        m.usage_hook = lambda pid: None if pid == 20 else {"rss": 1, "footprint": 2, "usage_start": 1}
        with self.assertRaisesRegex(RuntimeError, "Lost footprint"):
            tree.collect()
        self.assertIn(20, tree.owned)

    def test_enumeration_failure_is_not_an_empty_tree(self):
        class Lib:
            def proc_listpids(self, *_args):
                return 0
        monitor = object.__new__(bounded.MacProcesses)
        monitor.lib = Lib()
        with self.assertRaisesRegex(RuntimeError, "Cannot enumerate"):
            monitor.snapshot()


class CleanupTests(unittest.TestCase):
    def test_cleanup_discovers_child_spawned_during_term(self):
        m = Monitor([row(10)])
        tree = bounded.ProcessTree(m, 10)
        killed = []
        def kill(pid, sig):
            killed.append((pid, sig))
            m.rows[pid]["status"] = 5
            if pid == 10 and sig == signal.SIGTERM:
                m.rows[20] = row(20, 1)
        with patch.object(bounded.os, "kill", kill):
            remaining, errors = bounded.cleanup(tree, Root(m), grace=0, kill_grace=0)
        self.assertEqual(remaining, [])
        self.assertEqual(errors, [])
        self.assertIn((20, signal.SIGKILL), killed)

    def test_signal_error_does_not_skip_other_pids_and_cannot_report_success(self):
        m = Monitor([row(10), row(20, 10)])
        tree = bounded.ProcessTree(m, 10)
        tree.collect()
        killed = []
        def kill(pid, sig):
            if pid == 10:
                raise PermissionError("mock signal denied")
            killed.append(pid)
            m.rows[pid]["status"] = 5
        with patch.object(bounded.os, "kill", kill):
            remaining, errors = bounded.cleanup(tree, Root(m), grace=0, kill_grace=0)
        self.assertIn(20, killed)
        self.assertEqual(remaining, [10])
        self.assertTrue(errors)
        self.assertEqual(bounded.result_exit_code({"cleanup_ok": False, "reason": "command_exit", "command_exit": 0}), 125)

    def test_collection_exception_does_not_prevent_known_identity_cleanup(self):
        m = Monitor([row(10), row(20, 10)])
        tree = bounded.ProcessTree(m, 10)
        tree.collect()
        def broken():
            raise RuntimeError("mock enumeration loss")
        tree.collect = broken
        def kill(pid, _sig):
            m.rows[pid]["status"] = 5
        with patch.object(bounded.os, "kill", kill):
            remaining, errors = bounded.cleanup(tree, Root(m), grace=0, kill_grace=0)
        self.assertEqual(remaining, [])
        self.assertTrue(any("enumeration loss" in e for e in errors))


@unittest.skipUnless(sys.platform == "darwin", "real libproc fixtures require macOS")
class TinyRealFixtures(unittest.TestCase):
    def run_fixture(self, source, limit=64, seconds=1):
        with tempfile.TemporaryDirectory(prefix="rufren-bounded-test-") as directory:
            report = Path(directory) / "report"
            command = [sys.executable, str(SCRIPT), "--max-mib", str(limit), "--seconds", str(seconds),
                       "--report", str(report), "--", sys.executable, "-c", source]
            process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            (Path(directory) / "supervisor.pid").write_text(str(process.pid))
            try:
                stdout, stderr = process.communicate(timeout=5)
            except subprocess.TimeoutExpired:
                # This exact PID is the fixture supervisor we own. Payloads also
                # have intrinsic <=3second lifetimes if the guard itself fails.
                process.kill()
                process.wait(timeout=1)
                self.fail("Tiny supervisor fixture exceeded5seconds")
            self.assertTrue((report / "result.json").is_file(), stderr.decode())
            result = json.loads((report / "result.json").read_text())
            self.assertTrue(result["cleanup_ok"], result)
            self.assertEqual(result["remaining_pids"], [])
            return process.returncode, result, (report / "stdout.log").read_text()

    def test_tiny_allocation_crosses_sampled_limit_and_is_cleaned(self):
        code, result, _ = self.run_fixture("import time; a=bytearray(32*1024*1024); a[::4096]=b'x'*(len(a)//4096); time.sleep(2)", limit=24, seconds=2.5)
        self.assertEqual(code, 124)
        self.assertEqual(result["reason"], "memory_limit")
        self.assertGreater(result["peak_mib"], 24)

    def test_tiny_sleep_obeys_deadline(self):
        code, result, _ = self.run_fixture("import time; time.sleep(3)", seconds=0.35)
        self.assertEqual(code, 124)
        self.assertEqual(result["reason"], "time_limit")
        self.assertLess(result["elapsed_seconds"], 3)

    def test_normal_parent_exit_cleans_observed_child(self):
        source = "import subprocess,sys; p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(3)']); print(p.pid,flush=True)"
        code, result, out = self.run_fixture(source)
        self.assertEqual(code, 0)
        self.assertEqual(result["reason"], "command_exit")
        self.assertIn(int(out.strip()), result["observed_pids"])
        self.assertGreaterEqual(len(result["observed_pids"]), 2)


if __name__ == "__main__":
    unittest.main()
