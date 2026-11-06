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

    def test_dialogue_audio_and_transcript_originals_share_complete_backup_boundary(self):
        for kind in ('dialogue_audio_attached', 'dialogue_speech_transcribed'):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as temporary:
                folder = Path(temporary)
                database, assets, digest = fixture(folder, register=False)
                with sqlite3.connect(database) as connection:
                    event = json.loads(connection.execute('SELECT json FROM events').fetchone()[0])
                    event['version'] = 5
                    event['fact']['kind'] = kind
                    event['fact']['data'] = (dict(asset_id='take-a', asset=dict(sha256=digest))
                                            if kind == 'dialogue_audio_attached'
                                            else dict(asset_id='take-a', audio_sha256=digest))
                    connection.execute('UPDATE events SET json=?', (json.dumps(event),))
                pending = invoke('backup', database, folder / 'pending.sqlite', '--assets', assets)
                self.assertNotEqual(pending.returncode, 0)
                self.assertIn('have not completed recording sync', pending.stderr)
                self.assertFalse((folder / 'pending.sqlite.json').exists())
                content = (assets / f'{digest}.blob').read_bytes()
                with sqlite3.connect(database) as connection:
                    connection.execute('INSERT INTO metadata VALUES (?,?)', ('asset.record.take-a',
                        json.dumps(dict(asset_id='take-a', content_hash=digest, byte_length=len(content)))))
                backup, restored = folder / 'complete.sqlite', folder / 'restored.sqlite'
                copied = invoke('backup', database, backup, '--assets', assets)
                self.assertEqual(copied.returncode, 0, copied.stderr)
                self.assertEqual(invoke('verify', backup).returncode, 0)
                self.assertEqual(invoke('restore', backup, restored).returncode, 0)
                self.assertEqual((folder / 'restored.sqlite.assets' / f'{digest}.blob').read_bytes(), content)
                with sqlite3.connect(restored) as connection:
                    self.assertEqual(json.loads(connection.execute('SELECT json FROM events').fetchone()[0]), event)

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

    def test_sound_nested_sources_and_spoken_originals_require_complete_verified_audio(self):
        # These fixtures intentionally exercise only fields read by the backup
        # tool; the separate real-client/Rust-reader acceptance validates full6 facts.
        for kind in ('sound_trial_prepared', 'sound_reference_played', 'sound_trial_completed'):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as temporary:
                folder = Path(temporary)
                database, assets, digest = fixture(folder, register=False)
                reference = dict(asset_id='take-a', asset=dict(sha256=digest))
                second = dict(asset_id='take-b', asset=dict(sha256=digest))
                data = ({'references': [reference, second]} if kind == 'sound_trial_prepared' else
                        {'reference': reference} if kind == 'sound_reference_played' else
                        {'response': {'kind': 'speaking', **reference}})
                with sqlite3.connect(database) as connection:
                    event = json.loads(connection.execute('SELECT json FROM events').fetchone()[0])
                    event.update(version=6, fact=dict(kind=kind, data=data))
                    connection.execute('UPDATE events SET json=?', (json.dumps(event),))
                pending = invoke('backup', database, folder / 'pending.sqlite', '--assets', assets)
                self.assertNotEqual(pending.returncode, 0)
                self.assertIn('have not completed recording sync', pending.stderr)
                self.assertFalse((folder / 'pending.sqlite.json').exists())
                content = (assets / f'{digest}.blob').read_bytes()
                with sqlite3.connect(database) as connection:
                    for asset_id in ['take-a', 'take-b'] if kind == 'sound_trial_prepared' else ['take-a']:
                        connection.execute('INSERT INTO metadata VALUES (?,?)', ('asset.record.' + asset_id,
                            json.dumps(dict(asset_id=asset_id, content_hash=digest, byte_length=len(content)))))
                backup, restored = folder / 'complete.sqlite', folder / 'restored.sqlite'
                copied = invoke('backup', database, backup, '--assets', assets)
                self.assertEqual(copied.returncode, 0, copied.stderr)
                self.assertEqual(invoke('restore', backup, restored).returncode, 0)
                self.assertEqual((folder / 'restored.sqlite.assets' / f'{digest}.blob').read_bytes(), content)
                with sqlite3.connect(restored) as connection:
                    self.assertEqual(json.loads(connection.execute('SELECT json FROM events').fetchone()[0]), event)
                target = (data['references'][1] if kind == 'sound_trial_prepared' else
                          data['reference'] if kind == 'sound_reference_played' else data['response'])
                target['asset']['sha256'] = '0' * 64
                with sqlite3.connect(database) as connection:
                    connection.execute('UPDATE events SET json=?', (json.dumps(event),))
                mismatch = invoke('backup', database, folder / 'mismatch.sqlite', '--assets', assets)
                self.assertNotEqual(mismatch.returncode, 0)
                self.assertIn('hash contradicts', mismatch.stderr)
                self.assertFalse((folder / 'mismatch.sqlite.json').exists())

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
