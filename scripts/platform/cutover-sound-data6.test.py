#!/usr/bin/env python3
"""Isolated receipt/history guards; never start services or access private data."""
import copy
import importlib.util
import json
from contextlib import closing
from pathlib import Path
import sqlite3
import subprocess
import sys
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

    def test_timing_rollback7_requires_unchanged_history_without8(self):
        with closing(sqlite3.connect(self.path)) as db, db:
            db.execute('UPDATE events SET json=?', (json.dumps({'version': 7}),))
        seven = cutover.logical_database(self.path, 8)
        self.assertTrue(cutover.rollback_is_safe(seven, seven, 7))
        with closing(sqlite3.connect(self.path)) as db, db:
            db.execute('INSERT INTO events VALUES(2,?,?)', ('new-eight', json.dumps({'version': 8})))
        eight = cutover.logical_database(self.path, 8)
        self.assertFalse(cutover.rollback_is_safe(seven, eight, 7))
        self.assertFalse(cutover.rollback_is_safe(eight, eight, 7))

    def test_pending_originals_have_actionable_safe_backup_failure(self):
        with closing(sqlite3.connect(self.path)) as db, db:
            fact = {'version': 6, 'fact': {'kind': 'sound_trial_prepared', 'data': {
                'references': [{'asset_id': 'synthetic-pending-original', 'asset': {'sha256': '0' * 64}}]}}}
            db.execute('UPDATE events SET json=?', (json.dumps(fact),))
        with self.assertRaises(cutover.IncompleteAudioBackup):
            cutover.run_backup('backup', self.path, self.path.with_name('pending.sqlite'))
        self.assertFalse(self.path.with_name('pending.sqlite.json').exists())

    def test_reader_rollback6_allows_only_exact_unchanged_history_without7(self):
        with closing(sqlite3.connect(self.path)) as db, db:
            db.execute('UPDATE events SET json=?', (json.dumps({'version': 6}),))
        six = cutover.logical_database(self.path, 7)
        self.assertTrue(cutover.rollback_is_safe(six, six, 6))
        with closing(sqlite3.connect(self.path)) as db, db:
            db.execute('INSERT INTO events VALUES(2,?,?)', ('new-seven', json.dumps({'version': 7})))
        seven = cutover.logical_database(self.path, 7)
        self.assertFalse(cutover.rollback_is_safe(six, seven, 6))
        self.assertFalse(cutover.rollback_is_safe(seven, seven, 6), 'even an unchanged7 snapshot cannot restart6')
        with self.assertRaises(ValueError):
            cutover.logical_database(self.path)  # Historical default still6.


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


class ReaderReceiptGuard(unittest.TestCase):
    def setUp(self):
        ReceiptGuard.setUp(self)
        self.prompt = self.root / 'dialogue.rs'
        self.prompt.write_text('synthetic frozen dialogue4 policy, never run')
        source = json.loads(self.source.read_text())
        source['dialogue_prompt_source_sha256'] = cutover.prior.digest(self.prompt)
        self.source.write_text(json.dumps(source))
        self.value.update(data_protocol=7, teacher_prompt_version='rufren-ai-teacher-v6',
                          dialogue_prompt_version='rufren-ai-dialogue-v4',
                          dialogue_prompt_source_sha256=cutover.prior.digest(self.prompt),
                          source_evidence_sha256=cutover.prior.digest(self.source))
        self.build = self.root / 'actual-relay-build.json'
        self.build_value = {'binary': str(self.binary), 'binary_sha256': cutover.prior.digest(self.binary),
                            'event_version': 7, 'sync_version': 7, 'teacher_policy': 'rufren-ai-teacher-v6',
                            'dialogue_policy': 'rufren-ai-dialogue-v4', 'source_sha256': source['source_sha256'],
                            'prompt_source': str(self.prompt), 'prompt_source_sha256': cutover.prior.digest(self.prompt)}
        self.value['relay_build_receipt'] = str(self.build)
        self.consumer_value.update(protocol=7, reader_documents=2, empty_app_reader={
            'documents': [{'document_id': 'authored', 'source_kind': 'ai_authored'},
                          {'document_id': 'custom', 'source_kind': 'learner_provided'}],
            'excerpts': [{'revision_id': 'saved-phrase'}], 'audio': [{'asset_id': 'saved-original'}],
            'producer_local_keys_absent_before_open': ['reader_state:custom'],
            'dialogues': [{'document_id': 'custom', 'chunk_id': 'part-3'},
                          {'document_id': 'custom', 'chunk_id': 'part-4'}]})

    def tearDown(self):
        ReceiptGuard.tearDown(self)

    def write(self):
        self.build.write_text(json.dumps(self.build_value))
        self.value['relay_build_receipt_sha256'] = cutover.prior.digest(self.build)
        ReceiptGuard.write(self)

    def check(self):
        with patch.object(cutover.preflight, 'artifact', return_value={'source_evidence': str(self.source)}), \
                patch.object(cutover, 'worker_source_fingerprint', return_value={'scripts/speech/synthetic.py': 'worker-source'}):
            return cutover.accepted_candidate(self.accepted, self.native, 'reader-data7')

    def test_exact_reader_pair_binds_actual_relay_build_and_preserves_default_profile(self):
        self.write()
        self.assertEqual(self.check()[1], self.binary.resolve())
        with patch.object(cutover.preflight, 'artifact') as artifact:
            with self.assertRaises(ValueError):
                cutover.accepted_candidate(self.accepted, self.native)
            artifact.assert_not_called()  # Default remains Sound6, never autodetects7.

    def test_reader_build_identity_policy_and_source_mismatches_refuse(self):
        for key, value in [('binary_sha256', 'different-binary'), ('sync_version', 6), ('event_version', True),
                           ('teacher_policy', 'wrong-policy'), ('dialogue_policy', 'rufren-ai-dialogue-v3'),
                           ('source_sha256', {'Cargo.toml': 'different-build-source'}),
                           ('prompt_source_sha256', 'different-policy-file')]:
            with self.subTest(key=key):
                previous = self.build_value[key]
                self.build_value[key] = value
                self.write()
                with self.assertRaises(ValueError):
                    self.check()
                self.build_value[key] = previous

    def test_reader_cannot_use_sound_only_or_wrong_consumer_and_chunk_proof(self):
        for key, value in [('protocol', 6), ('reader_documents', True), ('empty_app_reader', None)]:
            with self.subTest(key=key):
                previous = self.consumer_value[key]
                self.consumer_value[key] = value
                self.write()
                with self.assertRaises(ValueError):
                    self.check()
                self.consumer_value[key] = previous
        reader = self.consumer_value['empty_app_reader']
        for key, value in [('documents', [None, None]), ('audio', []), ('excerpts', []),
                           ('producer_local_keys_absent_before_open', []),
                           ('dialogues', [{'document_id': 'custom', 'chunk_id': 'part-3'}])]:
            with self.subTest(key=key):
                previous = reader[key]
                reader[key] = value
                self.write()
                with self.assertRaises(ValueError):
                    self.check()
                reader[key] = previous

    def test_reader_profile_requires_teacher6_dialogue4(self):
        self.value['dialogue_prompt_version'] = 'rufren-ai-dialogue-v3'
        self.write()
        with self.assertRaises(ValueError):
            self.check()


class TimingReceiptGuard(unittest.TestCase):
    def setUp(self):
        ReaderReceiptGuard.setUp(self)
        self.value['data_protocol'] = 8
        self.build_value.update(event_version=8, sync_version=8)
        self.producer = self.root / 'producer.json'
        target = {'kind': 'teacher', 'document_id': 'synthetic-document', 'practice_index': 0}
        def event(id, device, kind, data):
            return {'id': id, 'device': device, 'fact': {'kind': kind, 'data': data}}
        snapshot = {'session': 'original-session', 'target': target, 'evidence_ids': ['known-source']}
        self.producer_value = {'ok': True, 'errors': [], 'forbidden': [], 'checks': ['actual UI assertions'],
            'draft': 'unfinished original response', 'starts': [event('start', 'A', 'review_started', snapshot)],
            'events': [event('ca', 'A', 'review_cadence_changed', {'parents': [], 'cadence': 'more_often'}),
                       event('cb', 'B', 'review_cadence_changed', {'parents': [], 'cadence': 'more_space'}),
                       event('ta', 'A', 'review_timing_changed', {'parents': [], 'target': target, 'action': {'kind': 'pause'}}),
                       event('tb', 'B', 'review_timing_changed', {'parents': [], 'target': target, 'action': {'kind': 'available_at', 'at_ms': 50}}),
                       event('late', 'B', 'passage_revised', {}), event('start', 'A', 'review_started', snapshot)]}
        self.value['producer_receipt'] = str(self.producer)
        self.consumer_value.update(timing_source_events_sha256=cutover.hashlib.sha256(json.dumps(self.producer_value['events'], ensure_ascii=False, separators=(',', ':')).encode()).hexdigest(), protocol=8, mode='timing', original_take_records=0, played_original_asset_ids=[],
            inference_calls=0, forbidden_requests=0, errors=[], compact_event_json_identical=True,
            retry_cursor_unchanged=True, events=len(self.producer_value['events']), empty_app_timing={
                'starts': [{'event_id': 'start', **snapshot}],
                'cadence': {'heads': ['ca', 'cb'], 'conflicted': True, 'pending': False, 'selected': 'choose'},
                'tasks': [{'target': target, 'heads': ['ta', 'tb'], 'conflicted': True, 'pending': False, 'paused': True, 'effective_due_at_ms': None}],
                'producer_checks': {'divergent_cadence_ids': ['ca', 'cb'], 'divergent_timing_ids': ['ta', 'tb'],
                    'paused_draft_resumed': {'session': 'original-session', 'before': 'unfinished original response', 'after': 'unfinished original response'},
                    'late_arrival_ids': ['late'], 'prior_start_event_ids': ['start']}})

    def tearDown(self):
        ReceiptGuard.tearDown(self)

    def write(self):
        self.consumer_value['timing_source_events_sha256'] = cutover.hashlib.sha256(json.dumps(self.producer_value['events'], ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()
        self.producer.write_text(json.dumps(self.producer_value))
        self.value['producer_receipt_sha256'] = cutover.prior.digest(self.producer)
        ReaderReceiptGuard.write(self)

    def check(self):
        with patch.object(cutover.preflight, 'artifact', return_value={'source_evidence': str(self.source)}), \
                patch.object(cutover, 'worker_source_fingerprint', return_value={'scripts/speech/synthetic.py': 'worker-source'}):
            return cutover.accepted_candidate(self.accepted, self.native, 'review-timing-data8')

    def test_explicit_timing_profile_allows_text_only_with_exact_pinned_producer(self):
        self.write()
        self.assertEqual(self.check()[1], self.binary.resolve())
        self.producer.write_text('{}')
        with self.assertRaises(ValueError):
            self.check()

    def test_timing_cannot_reuse_reader7_or_sound6_profile(self):
        self.write()
        for profile in ['reader-data7', 'sound-data6']:
            with self.assertRaises(ValueError):
                cutover.accepted_candidate(self.accepted, self.native, profile)

    def test_missing_restore_starts_conflicts_or_changed_draft_refuse(self):
        original = copy.deepcopy(self.consumer_value)
        for key, value in [('starts', []), ('tasks', []), ('producer_checks', {}), ('cadence', {})]:
            with self.subTest(key=key):
                self.consumer_value = copy.deepcopy(original)
                self.consumer_value['empty_app_timing'][key] = value
                self.write()
                with self.assertRaises((ValueError, KeyError)):
                    self.check()
        self.consumer_value = copy.deepcopy(original)
        self.consumer_value['empty_app_timing']['starts'][0]['session'] = 'replacement-session'
        self.write()
        with self.assertRaises(ValueError):
            self.check()

    def test_same_device_or_late_evidence_rewrite_refuse(self):
        self.producer_value['events'][1]['device'] = 'A'
        self.write()
        with self.assertRaises(ValueError):
            self.check()
        self.producer_value['events'][1]['device'] = 'B'
        self.consumer_value['empty_app_timing']['starts'][0]['evidence_ids'].append('late')
        self.write()
        with self.assertRaises(ValueError):
            self.check()

    def test_later_generic_start_may_observe_the_late_source(self):
        event = copy.deepcopy(self.producer_value['starts'][0])
        event['id'] = 'later-generic-start'
        event['fact']['data'].update(session='generic-session', evidence_ids=['known-source', 'late'])
        self.producer_value['events'].append(event)
        self.producer_value['starts'].append(event)
        self.consumer_value['events'] += 1
        self.consumer_value['empty_app_timing']['starts'].append({'event_id': event['id'], **event['fact']['data']})
        self.write()
        self.assertEqual(self.check()[1], self.binary.resolve())

    def test_wrong_policy_or_build8_identity_refuses(self):
        for key, value in [('event_version', 7), ('sync_version', 7), ('teacher_policy', 'wrong-policy')]:
            previous = self.build_value[key]
            self.build_value[key] = value
            self.write()
            with self.assertRaises(ValueError):
                self.check()
            self.build_value[key] = previous


class TransitionPlan(unittest.TestCase):
    def test_both_plans_are_explicit_and_default_is_historical_sound(self):
        script = Path(__file__).with_name('cutover-sound-data6.py')
        for arguments, before, after in [([], 5, 6), (['--transition-profile', 'reader-data7'], 6, 7), (['--transition-profile', 'review-timing-data8'], 7, 8)]:
            result = subprocess.run([sys.executable, str(script), 'plan', *arguments], capture_output=True, text=True, check=True)
            value = json.loads(result.stdout)
            self.assertEqual((value['from_protocol'], value['to_protocol']), (before, after))
            self.assertEqual(value['action'], 'plan-only-no-service-change')
            self.assertEqual(value['model_calls'], 0)
        with self.assertRaises(ValueError):
            cutover.transition('arbitrary-unsupported-upgrade')


if __name__ == '__main__':
    unittest.main()
