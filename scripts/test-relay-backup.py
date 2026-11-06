#!/usr/bin/env python3
"""Focused live-WAL backup / restore / corruption / no-overwrite regression test."""
from pathlib import Path
import json
import hashlib
import sqlite3
import subprocess
import sys
import tempfile
import unittest
import uuid

SCRIPT = Path(__file__).with_name('relay-backup.py')


class BackupContinuity(unittest.TestCase):
    def test_wal_snapshot_restore_digest_and_no_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            original, backup, restored = [folder / name for name in ['live.sqlite', 'backup.sqlite', 'restored.sqlite']]
            connection = sqlite3.connect(original)
            connection.execute('PRAGMA journal_mode=WAL')
            connection.executescript('''PRAGMA user_version=1;
                CREATE TABLE events(seq INTEGER PRIMARY KEY AUTOINCREMENT,id TEXT UNIQUE NOT NULL,json TEXT NOT NULL);
                CREATE TABLE metadata(key TEXT PRIMARY KEY,json TEXT NOT NULL);
                CREATE TABLE acknowledgements(relay TEXT NOT NULL,event TEXT NOT NULL,PRIMARY KEY(relay,event));''')
            relay = str(uuid.uuid4())
            connection.execute('INSERT INTO metadata VALUES (?,?)', ('relay', json.dumps(relay)))
            event_id = str(uuid.uuid4())
            event = dict(version=1, id=event_id, device=str(uuid.uuid4()), recorded_at_ms=0,
                fact=dict(kind='passage_revised', data=dict(passage=dict(id=str(uuid.uuid4()), title='Legacy',
                    russian='Привет', translation='Hello', source='Original fixture', coach_reviewed=False), parents=[])))
            connection.execute('INSERT INTO events(id,json) VALUES (?,?)', (event_id, json.dumps(event)))
            connection.commit()
            connection.execute('INSERT INTO events(id,json) VALUES (?,?)', (str(uuid.uuid4()), '{}')) # deliberately uncommitted
            def run(*arguments, success=True):
                result = subprocess.run([sys.executable, str(SCRIPT), *map(str, arguments)], capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
                return result
            run('backup', original, backup)
            manifest = json.loads(backup.with_suffix('.sqlite.json').read_text())
            self.assertEqual(manifest['relay'], relay)
            self.assertEqual(manifest['event_count'], 1)
            self.assertEqual(manifest['cursor'], 1)
            run('restore', backup, restored)
            run('verify', restored)
            with sqlite3.connect(restored) as copied:
                self.assertEqual(json.loads(copied.execute('SELECT json FROM events').fetchone()[0]), event)
            run('restore', backup, restored, success=False)
            self.assertEqual(json.loads(restored.with_suffix('.sqlite.json').read_text())['event_count'], 1)
            with backup.open('ab') as output:
                output.write(b'corruption')
            run('verify', backup, success=False)
            run('restore', backup, folder / 'corrupt.sqlite', success=False)
            self.assertFalse((folder / 'corrupt.sqlite').exists())
            connection.rollback()
            connection.close()

    def test_audio_bundle_keeps_distinct_takes_and_rejects_corrupt_original(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original, backup, restored = [root / name for name in ['live.sqlite', 'bundle.sqlite', 'restored.sqlite']]
            assets = root / 'original-assets'
            assets.mkdir()
            content = b'original audio bytes'
            key = hashlib.sha256(content).hexdigest()
            (assets / f'{key}.blob').write_bytes(content)
            with sqlite3.connect(original) as db:
                db.executescript('PRAGMA user_version=1; CREATE TABLE events(seq INTEGER PRIMARY KEY,id TEXT UNIQUE,json TEXT); CREATE TABLE metadata(key TEXT PRIMARY KEY,json TEXT);')
                for take in ['take-a', 'take-b']:
                    db.execute('INSERT INTO metadata VALUES (?,?)', (f'asset.record.{take}', json.dumps(dict(asset_id=take, content_hash=key, byte_length=len(content)))))
            def run(*arguments, success=True):
                result = subprocess.run([sys.executable, str(SCRIPT), *map(str, arguments)], capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
            run('backup', original, backup, '--assets', assets)
            manifest = json.loads(backup.with_suffix('.sqlite.json').read_text())
            self.assertEqual(manifest['assets'], {key:len(content)})
            run('restore', backup, restored)
            self.assertEqual((root / 'restored.sqlite.assets' / f'{key}.blob').read_bytes(), content)
            with sqlite3.connect(restored) as db:
                self.assertEqual(db.execute('SELECT COUNT(*) FROM metadata').fetchone()[0], 2)
            (root / 'bundle.sqlite.assets' / f'{key}.blob').write_bytes(b'damaged original')
            run('verify', backup, success=False)
            run('restore', backup, root / 'bad.sqlite', success=False)
            self.assertFalse((root / 'bad.sqlite').exists())


if __name__ == '__main__':
    unittest.main()
