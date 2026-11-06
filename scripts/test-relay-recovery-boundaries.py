#!/usr/bin/env python3
"""Recovery acceptance boundaries beyond the existing WAL/audio backup tests."""
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest
import uuid

TOOL = Path(__file__).with_name('relay-backup.py')


def invoke(*arguments):
    return subprocess.run([sys.executable, str(TOOL), *map(str, arguments)], capture_output=True, text=True)


def fixture(folder, register=True):
    database, assets = folder / 'source.sqlite', folder / 'originals'
    assets.mkdir()
    content = b'only this immutable synthetic original belongs in the backup'
    digest = hashlib.sha256(content).hexdigest()
    (assets / f'{digest}.blob').write_bytes(content)
    (assets / 'unrelated-operator-file.txt').write_text('synthetic excluded sentinel')
    event = dict(version=3, id=str(uuid.uuid4()), device=str(uuid.uuid4()), recorded_at_ms=0,
                 fact=dict(kind='recording_attached', data=dict(asset_id='take-a',
                           asset=dict(sha256=digest), target=dict(target='revision', id=str(uuid.uuid4())))))
    with sqlite3.connect(database) as connection:
        connection.executescript('PRAGMA user_version=1; CREATE TABLE events(seq INTEGER PRIMARY KEY,id TEXT UNIQUE,json TEXT); CREATE TABLE metadata(key TEXT PRIMARY KEY,json TEXT);')
        connection.execute('INSERT INTO events VALUES (1,?,?)', (event['id'], json.dumps(event)))
        connection.execute('INSERT INTO metadata VALUES (?,?)', ('relay', json.dumps(str(uuid.uuid4()))))
        if register:
            connection.execute('INSERT INTO metadata VALUES (?,?)', ('asset.record.take-a', json.dumps(dict(asset_id='take-a', content_hash=digest, byte_length=len(content)))))
    return database, assets, digest


class RecoveryBoundaries(unittest.TestCase):
    def test_fact_only_sync_reports_pending_audio_without_publishing_usable_backup(self):
        with tempfile.TemporaryDirectory() as temporary:
            folder = Path(temporary)
            database, assets, _ = fixture(folder, register=False)
            destination = folder / 'incomplete.sqlite'
            result = invoke('backup', database, destination, '--assets', assets)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('have not completed recording sync', result.stderr)
            self.assertIn('saved facts were kept', result.stderr)
            self.assertFalse(destination.with_suffix('.sqlite.json').exists())
            self.assertNotEqual(invoke('restore', destination, folder / 'refused.sqlite').returncode, 0)
            with sqlite3.connect(database) as source:
                self.assertEqual(source.execute('SELECT COUNT(*) FROM events').fetchone()[0], 1)

    def test_manifest_claims_require_exact_types_and_identity(self):
        with tempfile.TemporaryDirectory() as temporary:
            folder = Path(temporary)
            database, assets, digest = fixture(folder)
            backup = folder / 'backup.sqlite'
            result = invoke('backup', database, backup, '--assets', assets)
            self.assertEqual(result.returncode, 0, result.stderr)
            manifest_path = backup.with_suffix('.sqlite.json')
            original = json.loads(manifest_path.read_text())
            changes = [('version', True), ('schema', True), ('event_count', True), ('cursor', True),
                       ('cursor', 1.0), ('relay', str(uuid.uuid4())), ('event_count', 100)]
            for key, value in changes:
                with self.subTest(key=key, value=value):
                    modified = {**original, key:value}
                    manifest_path.write_text(json.dumps(modified))
                    self.assertNotEqual(invoke('verify', backup).returncode, 0)
                    target = folder / f'rejected-{key}-{type(value).__name__}.sqlite'
                    self.assertNotEqual(invoke('restore', backup, target).returncode, 0)
                    self.assertFalse(target.exists())
            manifest_path.write_text(json.dumps({**original, 'assets':{digest:True}}))
            self.assertNotEqual(invoke('verify', backup).returncode, 0)
            manifest_path.write_text(json.dumps(original))
            self.assertEqual(invoke('verify', backup).returncode, 0)
            self.assertEqual(len(list((folder / 'backup.sqlite.assets').iterdir())), 1)
            self.assertFalse(backup.with_suffix('.sqlite-wal').exists())
            self.assertFalse(backup.with_suffix('.sqlite-shm').exists())

    def test_missing_committed_original_and_conflicting_reference_fail_closed(self):
        with tempfile.TemporaryDirectory() as temporary:
            folder = Path(temporary)
            database, assets, digest = fixture(folder)
            (assets / f'{digest}.blob').unlink()  # This test's own synthetic file only.
            target = folder / 'missing.sqlite'
            self.assertNotEqual(invoke('backup', database, target, '--assets', assets).returncode, 0)
            self.assertFalse(target.with_suffix('.sqlite.json').exists())
            with sqlite3.connect(database) as connection:
                value = json.loads(connection.execute('SELECT json FROM events').fetchone()[0])
                value['fact']['data']['asset']['sha256'] = '0' * 64
                connection.execute('UPDATE events SET json=?', (json.dumps(value),))
            result = invoke('backup', database, folder / 'mismatch.sqlite', '--assets', assets)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('hash contradicts', result.stderr)


if __name__ == '__main__':
    unittest.main()
