#!/usr/bin/env python3
"""Ownership/executable-selection regressions; no real service or signal is used."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import MagicMock, patch

spec = importlib.util.spec_from_file_location('private_services', Path(__file__).with_name('private-relay-services.py'))
manager = importlib.util.module_from_spec(spec)
spec.loader.exec_module(manager)


class ProcessSelection(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name).resolve()
        self.folder = self.root / '.local/ipad-delivery'
        self.folder.mkdir(parents=True)
        self.settings = patch.multiple(manager, ROOT=self.root, FOLDER=self.folder)
        self.settings.start()
        self.executable = self.root / 'accepted/rufren-relay'
        self.executable.parent.mkdir()
        self.executable.write_bytes(b'synthetic executable bytes; never executed')
        self.executable.chmod(0o700)
        self.sha = hashlib.sha256(self.executable.read_bytes()).hexdigest()

    def tearDown(self):
        self.settings.stop()
        self.temporary.cleanup()

    def marker(self):
        (self.folder / 'relay.pid').write_text('321')

    def record(self):
        self.marker()
        manager.atomic_json(self.folder / 'relay-process.json', {
            'version': 1, 'service': 'relay', 'pid': 321, 'cwd': str(self.folder),
            'executable': str(self.executable), 'executable_sha256': self.sha})

    def test_legacy_exact_command_and_cwd_remain_manageable(self):
        self.marker()
        legacy = str(self.root / 'target/debug/rufren-relay')
        with patch.object(manager, 'process_command', return_value=legacy), patch.object(manager, 'process_cwd', return_value=str(self.folder)):
            self.assertEqual(manager.owned_pid('relay'), 321)
        with patch.object(manager, 'process_command', return_value=legacy + '-unrelated'), patch.object(manager, 'process_cwd', return_value=str(self.folder)):
            with self.assertRaises(RuntimeError):
                manager.owned_pid('relay')

    def test_recorded_immutable_path_owns_pid_without_mutable_target(self):
        self.record()
        with patch.object(manager, 'process_command', return_value=str(self.executable)), patch.object(manager, 'process_cwd', return_value=str(self.folder)):
            status = manager.service_status('relay')
            self.assertEqual(status['ownership'], 'recorded')
            self.assertTrue(status['on_disk_matches_receipt'])
            self.executable.write_bytes(b'changed on disk; running process identity stays recorded')
            self.assertFalse(manager.service_status('relay')['on_disk_matches_receipt'])

    def test_wrong_working_directory_never_owns_pid(self):
        self.record()
        with patch.object(manager, 'process_command', return_value=str(self.executable)), patch.object(manager, 'process_cwd', return_value=str(self.root)):
            with self.assertRaises(RuntimeError):
                manager.owned_pid('relay')

    def test_revalidate_before_signaling_rejects_changed_process(self):
        self.record()
        with patch.object(manager, 'process_command', side_effect=[str(self.executable), '/bin/unrelated']), patch.object(manager, 'process_cwd', return_value=str(self.folder)), patch.object(os, 'kill') as signal:
            with self.assertRaises(RuntimeError):
                manager.stop('relay')
            self.assertEqual(signal.call_count, 0)

    def test_bad_accepted_hash_never_starts_a_process(self):
        with patch.object(manager, 'owned_pid', return_value=None), patch.object(manager.subprocess, 'Popen') as spawn:
            with self.assertRaises(RuntimeError):
                manager.start('relay', relay_executable=self.executable, relay_sha256='0' * 64)
            self.assertEqual(spawn.call_count, 0)

    def test_explicit_start_records_identity_but_no_environment_values(self):
        provider = self.folder / 'provider.json'
        provider.write_text(json.dumps({'SYNTHETIC_PRIVATE_SETTING': 'do-not-record-value'}))
        (self.folder / 'relay-environment.json').write_text(json.dumps({'RUFREN_RELAY_BIND': '127.0.0.1:1', 'RUFREN_SYNC_TOKEN': 'synthetic-never-a-real-token'}))
        child = MagicMock(pid=321)
        child.poll.return_value = None
        with patch.object(manager, 'owned_pid', return_value=None), patch.object(manager.subprocess, 'Popen', return_value=child) as spawn, patch.object(manager.socket, 'create_connection', return_value=MagicMock()):
            manager.start('relay', str(provider), self.executable, self.sha)
            self.assertEqual(spawn.call_count, 1)
            self.assertEqual(spawn.call_args.args[0], [str(self.executable)])
        raw = (self.folder / 'relay-process.json').read_text()
        record = json.loads(raw)
        self.assertEqual(record['executable_sha256'], self.sha)
        self.assertEqual(record['provider_environment_sha256'], manager.digest(provider))
        self.assertNotIn('do-not-record-value', raw)
        self.assertNotIn('synthetic-never-a-real-token', raw)


if __name__ == '__main__':
    unittest.main()
