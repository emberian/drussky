#!/usr/bin/env python3
"""Ownership/budget regressions with fake processes; never launches an app."""
import importlib.util
import json
from pathlib import Path
import plistlib
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("watch", Path(__file__).with_name("watch-qa-app.py"))
watch = importlib.util.module_from_spec(spec)
spec.loader.exec_module(watch)


class Fake:
    def __init__(self, executable, shared):
        self.calls, self.dead, self.killed = 0, set(), []
        self.exe, self.shared = str(executable.resolve()), shared
        self.rows = {p: {"pid": p, "ppid": 1, "uid": 501, "start": [p, 1], "status": 2}
                     for p in (11, 22, 33)}

    def pressure(self):
        return 1

    def snapshot(self):
        self.calls += 1
        return {p: r for p, r in self.rows.items()
                if p not in self.dead and (self.calls > 1 or p == 33)}

    def identity(self, pid):
        return None if pid in self.dead else self.rows.get(pid)

    def executable(self, pid):
        if pid == 11:
            return self.exe
        return "/System/Library/Frameworks/WebKit.framework/Versions/A/XPCServices/com.apple.WebKit.WebContent.xpc/Contents/MacOS/com.apple.WebKit.WebContent"

    def coalition(self, pid):
        return 101 if pid == 33 or self.shared else 202

    def attribute(self, row):
        return {**row, "path": self.executable(row["pid"]),
                "coalition": self.coalition(row["pid"])}

    def usage(self, pid):
        return {"footprint": 2 * 1048576}

    def kill(self, pid, _sig):
        self.killed.append(pid)
        self.dead.add(pid)


class WatcherTests(unittest.TestCase):
    def scenario(self, shared, missing_usage=False):
        with tempfile.TemporaryDirectory(prefix="rufren-watch-test-") as folder:
            root = Path(folder)
            bundle = root / "Rufren QA.app"
            (bundle / "Contents/MacOS").mkdir(parents=True)
            exe = bundle / "Contents/MacOS/rufren-app"
            exe.write_text("non-executable test fixture")
            (bundle / "Contents/Info.plist").write_bytes(plistlib.dumps({
                "CFBundleIdentifier": "town.lunar.rufren.qa.fixture-test"}))
            report = root / "report"
            fake = Fake(exe, shared)
            if missing_usage:
                fake.usage = lambda pid: None
            arguments = ["watch", "--executable", str(exe), "--report", str(report),
                         "--max-mib", "1", "--seconds", "1"]
            with patch.object(watch, "Inspector", return_value=fake), \
                 patch.object(watch.os, "kill", side_effect=fake.kill), \
                 patch.object(watch.time, "sleep"), patch.object(watch.signal, "signal"), \
                 patch.object(sys, "argv", arguments):
                self.assertEqual(watch.main(), 124)
            result = json.loads((report / "result.json").read_text())
            self.assertNotIn(33, fake.killed, "unrelated app's WebKit must remain untouched")
            self.assertEqual(result["remaining_pids"], [])
            return result, set(fake.killed)

    def test_combines_host_and_its_helper_and_cleans_both(self):
        result, killed = self.scenario(False)
        self.assertEqual(result["reason"], "memory_limit")
        self.assertEqual(result["peak_mib"], 4)
        self.assertEqual(killed, {11, 22})

    def test_shared_tool_coalition_is_refused_without_claiming_helpers(self):
        result, killed = self.scenario(True)
        self.assertEqual(result["reason"], "watcher_error")
        self.assertEqual(killed, {11})

    def test_live_measurement_failure_still_stops_host_and_helpers(self):
        result, killed = self.scenario(False, missing_usage=True)
        self.assertEqual(result["reason"], "watcher_error")
        self.assertEqual(killed, {11, 22})


if __name__ == "__main__":
    unittest.main()
