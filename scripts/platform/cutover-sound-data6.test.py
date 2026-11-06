#!/usr/bin/env python3
"""Isolated receipt/history guards; never start services or access private data."""
import importlib.util
import json
from contextlib import closing
from pathlib import Path
import sqlite3
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('sound_cutover', Path(__file__).with_name('cutover-sound-data6.py'))
cutover = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cutover)


class HistoryGuard(unittest.TestCase):
    def setUp(self):
        self.folder = tempfile.TemporaryDirectory()
        self.path = Path(self.folder.name) / 'history.sqlite'
        with closing(sqlite3.connect(self.path)) as db, db:
            db.executescript('PRAGMA user_version=1; CREATE TABLE events(seq INTEGER PRIMARY KEY,id TEXT UNIQUE,json TEXT); CREATE TABLE metadata(key TEXT PRIMARY KEY,json TEXT);')
            db.execute('INSERT INTO events VALUES(1,?,?)', ('old-five', json.dumps({'version': 5})))
            db.execute('INSERT INTO metadata VALUES(?,?)', ('relay', '"synthetic-library"'))
        self.snapshot = cutover.logical_database(self.path)

    def tearDown(self):
        self.folder.cleanup()

    def test_unchanged_five_can_restart_without_restore(self):
        self.assertTrue(cutover.rollback_is_safe(self.snapshot, cutover.logical_database(self.path)))

    def test_six_events_or_even_legacy_writes_prevent_rollback(self):
        for version in [5, 6]:
            with self.subTest(version=version):
                with closing(sqlite3.connect(self.path)) as db, db:
                    db.execute('INSERT INTO events VALUES(2,?,?)', ('new-write', json.dumps({'version': version})))
                self.assertFalse(cutover.rollback_is_safe(self.snapshot, cutover.logical_database(self.path)))
                with closing(sqlite3.connect(self.path)) as db, db:
                    db.execute('DELETE FROM events WHERE seq=2')

    def test_registry_only_change_prevents_rollback(self):
        with closing(sqlite3.connect(self.path)) as db, db:
            db.execute('INSERT INTO metadata VALUES(?,?)', ('asset.record.synthetic', '{"retain":"original"}'))
        self.assertFalse(cutover.rollback_is_safe(self.snapshot, cutover.logical_database(self.path)))

    def test_boolean_event_version_is_not_an_integer_protocol(self):
        with closing(sqlite3.connect(self.path)) as db, db:
            db.execute('UPDATE events SET json=?', (json.dumps({'version': True}),))
        with self.assertRaises(ValueError):
            cutover.logical_database(self.path)

    def test_pending_originals_have_actionable_safe_backup_failure(self):
        with closing(sqlite3.connect(self.path)) as db, db:
            fact = {'version': 6, 'fact': {'kind': 'sound_trial_prepared', 'data': {
                'references': [{'asset_id': 'synthetic-pending-original', 'asset': {'sha256': '0' * 64}}]}}}
            db.execute('UPDATE events SET json=?', (json.dumps(fact),))
        with self.assertRaises(cutover.IncompleteAudioBackup):
            cutover.run_backup('backup', self.path, self.path.with_name('pending.sqlite'))
        self.assertFalse(self.path.with_name('pending.sqlite.json').exists())


class ReceiptGuard(unittest.TestCase):
    def setUp(self):
        self.folder = tempfile.TemporaryDirectory()
        self.root = Path(self.folder.name)
        self.binary = self.root / 'immutable-relay'
        self.binary.write_bytes(b'synthetic executable identity, never launched')
        self.consumer = self.root / 'consumer.json'
        self.consumer_value = {'passed': True, 'protocol': 6, 'relay_executable_sha256': cutover.prior.digest(self.binary), 'preview_build': 'synthetic-web-build',
                               'coherent_snapshot_restored': True, 'legacy_protocol_refused': True,
                               'owned_service_stopped': True, 'sound_trials': 2, 'sound_completed_trials': 1,
                               'empty_app_sound_trials': [{'trial_id': 'synthetic-trial'}], 'original_take_records': 3,
                               'played_original_asset_ids': ['synthetic-original']}
        self.source = self.root / 'source.json'
        self.source.write_text(json.dumps({'server_prompt_source_sha256': 'teacher-source',
                                          'dialogue_prompt_source_sha256': 'dialogue-source',
                                          'source_sha256': {'Cargo.toml': 'shared-source', 'crates/rufren-speech/src/lib.rs': 'speech-source', 'crates/rufren-audio/src/lib.rs': 'audio-source'}, 'reference_web_build': 'synthetic-web-build'}))
        self.native = self.root / 'native.json'
        self.native.write_text(json.dumps({'source_evidence': str(self.source)}))
        self.accepted = self.root / 'accepted.json'
        self.value = {'accepted': True, 'data_protocol': 6, 'relay_executable': str(self.binary),
                      'relay_executable_sha256': cutover.prior.digest(self.binary),
                      'native_manifest_sha256': cutover.prior.digest(self.native),
                      'source_evidence_sha256': cutover.prior.digest(self.source),
                      'teacher_prompt_version': 'synthetic-teacher', 'dialogue_prompt_version': 'synthetic-dialogue',
                      'teacher_prompt_source_sha256': 'teacher-source', 'dialogue_prompt_source_sha256': 'dialogue-source',
                      'worker_sources_reviewed': True, 'worker_source_scope': 'files-only', 'worker_source_sha256': {'scripts/speech/synthetic.py': 'worker-source'},
                      'relay_source_sha256': {'Cargo.toml': 'shared-source', 'crates/rufren-speech/src/lib.rs': 'speech-source', 'crates/rufren-audio/src/lib.rs': 'audio-source'}, 'consumer_receipt': str(self.consumer)}

    def tearDown(self):
        self.folder.cleanup()

    def write(self):
        self.consumer.write_text(json.dumps(self.consumer_value))
        self.value['consumer_receipt_sha256'] = cutover.prior.digest(self.consumer)
        self.accepted.write_text(json.dumps(self.value))

    def check(self):
        with patch.object(cutover.preflight, 'artifact', return_value={'source_evidence': str(self.source)}), \
                patch.object(cutover, 'worker_source_fingerprint', return_value={'scripts/speech/synthetic.py': 'worker-source'}):
            return cutover.accepted_candidate(self.accepted, self.native)

    def test_exact_receipts_accept_and_consumer_mutation_refuses(self):
        self.write()
        self.assertEqual(self.check()[1], self.binary.resolve())
        self.consumer.write_text('{}')
        with self.assertRaises(ValueError):
            self.check()

    def test_incomplete_or_mistyped_consumer_claims_refuse(self):
        for key, value in [('protocol', 5), ('sound_trials', True), ('original_take_records', True),
                           ('played_original_asset_ids', []), ('coherent_snapshot_restored', False),
                           ('relay_executable_sha256', 'different-binary'), ('preview_build', 'different-web-build'),
                           ('sound_completed_trials', True), ('empty_app_sound_trials', [])]:
            with self.subTest(key=key):
                previous = self.consumer_value[key]
                self.consumer_value[key] = value
                self.write()
                with self.assertRaises(ValueError):
                    self.check()
                self.consumer_value[key] = previous

    def test_mismatched_shared_source_or_native_bytes_refuse(self):
        self.write()
        self.value['relay_source_sha256'] = {'Cargo.toml': 'different-source'}
        self.accepted.write_text(json.dumps(self.value))
        with self.assertRaises(ValueError):
            self.check()
        self.value['relay_source_sha256'] = {'Cargo.toml': 'shared-source', 'crates/rufren-speech/src/lib.rs': 'speech-source', 'crates/rufren-audio/src/lib.rs': 'audio-source'}
        self.accepted.write_text(json.dumps(self.value))
        self.native.write_text('{}')
        with self.assertRaises(ValueError):
            self.check()

    def test_health_validation_precedes_credentialed_request(self):
        with patch.object(cutover.preflight, 'relay_health', side_effect=ValueError('unsafe origin')), \
                patch.object(cutover.urllib.request, 'build_opener') as opener:
            with self.assertRaises(ValueError):
                cutover.authenticated_statuses(5)
            opener.assert_not_called()

    def test_worker_snapshot_requires_explicit_review_and_same_bytes(self):
        for key, value in [('worker_sources_reviewed', False), ('worker_source_scope', 'loaded-runtime'),
                           ('worker_source_sha256', {'scripts/speech/synthetic.py': 'changed-worker'})]:
            with self.subTest(key=key):
                previous = self.value[key]
                self.value[key] = value
                self.write()
                with self.assertRaises(ValueError):
                    self.check()
                self.value[key] = previous

    def test_authenticated_origin_revalidated_without_redirect_or_plaintext(self):
        for origin in ['http://localhost:9999', 'https://user:password@example.test', 'https://example.test/path']:
            with self.subTest(origin=origin):
                (self.root / 'bootstrap.json').write_text(json.dumps({'RUFREN_RELAY_URL': origin, 'RUFREN_SYNC_TOKEN': 'synthetic-test-token'}))
                with patch.object(cutover, 'FOLDER', self.root), \
                        patch.object(cutover.preflight, 'relay_health'), \
                        patch.object(cutover.urllib.request, 'build_opener') as opener:
                    with self.assertRaises(ValueError):
                        cutover.authenticated_statuses(5)
                    opener.assert_not_called()


if __name__ == '__main__':
    unittest.main()
