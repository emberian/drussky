#!/usr/bin/env python3
"""Rollback decision tests use temporary SQLite only, never a service or restore."""
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import sqlite3
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('cutover', Path(__file__).with_name('cutover-dialogue-data5.py'))
cutover = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cutover)


class HistoryGuard(unittest.TestCase):
    def setUp(self):
        self.folder = tempfile.TemporaryDirectory()
        self.path = Path(self.folder.name) / 'library.sqlite'
        with sqlite3.connect(self.path) as db:
            db.executescript('PRAGMA user_version=1; CREATE TABLE events(seq INTEGER PRIMARY KEY,id TEXT UNIQUE,json TEXT); CREATE TABLE metadata(key TEXT PRIMARY KEY,json TEXT);')
            db.execute('INSERT INTO events VALUES(1,?,?)', ('old-four', json.dumps({'version': 4, 'fact': {'kind': 'synthetic_guard_fixture'}})))
            db.execute('INSERT INTO metadata VALUES(?,?)', ('relay', '"same-library"'))
        self.snapshot = cutover.logical_database(self.path)

    def tearDown(self):
        self.folder.cleanup()

    def test_real_backup_cli_manifest_and_verify_stdout_contract(self):
        # Real subprocess boundary, including a registered original audio blob.
        # No mocks, application process, private DB or service is involved.
        raw = b'synthetic original bytes for backup CLI boundary'
        sha = hashlib.sha256(raw).hexdigest()
        originals = Path(self.folder.name) / 'originals'
        originals.mkdir()
        (originals / (sha + '.blob')).write_bytes(raw)
        record = {'asset_id': 'test-original', 'content_hash': sha, 'byte_length': len(raw)}
        with sqlite3.connect(self.path) as db:
            db.execute('INSERT INTO metadata VALUES(?,?)', ('asset.record.test-original', json.dumps(record)))
        snapshot = cutover.logical_database(self.path)
        destination = Path(self.folder.name) / 'snapshot.sqlite'
        manifest = cutover.run_backup('backup', self.path, destination, originals)
        self.assertEqual(manifest['format'], 'rufren-relay-sqlite')
        self.assertEqual(manifest['assets'], {sha: len(raw)})
        self.assertEqual(manifest['sha256'], cutover.digest(destination))
        verified = cutover.run_backup('verify', destination)
        self.assertEqual(verified['event_count'], 1)
        self.assertEqual(verified['audio_blobs'], 1)
        self.assertEqual((destination.with_suffix('.sqlite.assets') / (sha + '.blob')).read_bytes(), raw)
        self.assertTrue(cutover.rollback_is_safe(snapshot, cutover.logical_database(destination)))

    def test_same_rows_ignore_sqlite_layout_and_allow_old_binary(self):
        copy = self.path.with_name('copy.sqlite')
        shutil.copy2(self.path, copy)
        with sqlite3.connect(copy) as db:
            db.execute('PRAGMA journal_mode=WAL')
        self.assertTrue(cutover.rollback_is_safe(self.snapshot, cutover.logical_database(copy)))

    def test_any_event5_makes_old_binary_rollback_unsafe(self):
        with sqlite3.connect(self.path) as db:
            db.execute('INSERT INTO events VALUES(2,?,?)', ('new-five', json.dumps({'version': 5, 'fact': {'kind': 'synthetic_guard_fixture'}})))
        current = cutover.logical_database(self.path)
        self.assertEqual(current['max_event_version'], 5)
        self.assertFalse(cutover.rollback_is_safe(self.snapshot, current))

    def test_even_new_legacy_writes_are_conservatively_preserved(self):
        with sqlite3.connect(self.path) as db:
            db.execute('INSERT INTO events VALUES(2,?,?)', ('new-four', json.dumps({'version': 4, 'fact': {'kind': 'synthetic_guard_fixture'}})))
        self.assertFalse(cutover.rollback_is_safe(self.snapshot, cutover.logical_database(self.path)))

    def test_registry_only_changes_are_not_discarded(self):
        with sqlite3.connect(self.path) as db:
            db.execute('INSERT INTO metadata VALUES(?,?)', ('asset.record.new-original', '{"synthetic":"preserve me"}'))
        current = cutover.logical_database(self.path)
        self.assertEqual(current['event_count'], self.snapshot['event_count'])
        self.assertFalse(cutover.rollback_is_safe(self.snapshot, current))


if __name__ == '__main__':
    unittest.main()
