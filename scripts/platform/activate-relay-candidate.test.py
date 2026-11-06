#!/usr/bin/env python3
"""Same-protocol receipt and lifecycle delegation tests; no actual service access."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('same_protocol', Path(__file__).with_name('activate-relay-candidate.py'))
a = importlib.util.module_from_spec(spec)
spec.loader.exec_module(a)


class AcceptedCandidate(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.folder = Path(self.temp.name)
        def write(name, value):
            path = self.folder / name
            path.write_text(json.dumps(value))
            return path
        self.old = self.folder / 'old-relay'; self.old.write_bytes(b'old immutable binary')
        self.new = self.folder / 'new-relay'; self.new.write_bytes(b'new immutable binary')
        self.teacher = self.folder / 'teacher.rs'; self.teacher.write_text('frozen teacher')
        self.dialogue = self.folder / 'dialogue.rs'; self.dialogue.write_text('frozen dialogue')
        self.workers = {'scripts/speech/example.py': 'unchanged-worker'}
        self.previous = write('previous.json', {'accepted': True, 'data_protocol': 8, 'relay_executable': str(self.old),
            'relay_executable_sha256': a.shared.prior.digest(self.old), 'worker_source_sha256': self.workers})
        self.source_value = {'source_sha256': {'Cargo.toml': 'same-build', **self.workers},
            'server_prompt_source_sha256': a.shared.prior.digest(self.teacher), 'dialogue_prompt_source_sha256': a.shared.prior.digest(self.dialogue)}
        self.source = write('source.json', self.source_value)
        self.native = write('native.json', {'source_evidence': str(self.source)})
        self.build_value = {'binary': str(self.new), 'binary_sha256': a.shared.prior.digest(self.new),
            'event_version': 8, 'sync_version': 8, 'teacher_policy': 'teacher7', 'dialogue_policy': 'dialogue5',
            'source_sha256': self.source_value['source_sha256'], 'teacher_source': str(self.teacher),
            'teacher_source_sha256': a.shared.prior.digest(self.teacher), 'prompt_source': str(self.dialogue),
            'prompt_source_sha256': a.shared.prior.digest(self.dialogue)}
        self.build = write('build.json', self.build_value)
        self.evidence = write('actual-ui.json', {'passed': True})
        self.value = {'accepted': True, 'activation_kind': 'same-protocol', 'data_protocol': 8,
            'previous_accepted_relay_receipt': str(self.previous), 'previous_accepted_relay_receipt_sha256': a.shared.prior.digest(self.previous),
            'expected_previous_relay_sha256': a.shared.prior.digest(self.old), 'relay_executable': str(self.new),
            'relay_executable_sha256': a.shared.prior.digest(self.new), 'native_manifest_sha256': a.shared.prior.digest(self.native),
            'source_evidence_sha256': a.shared.prior.digest(self.source), 'relay_build_receipt': str(self.build),
            'relay_build_receipt_sha256': a.shared.prior.digest(self.build), 'relay_source_sha256': {'Cargo.toml': 'same-build'},
            'teacher_prompt_version': 'teacher7', 'dialogue_prompt_version': 'dialogue5',
            'teacher_prompt_source_sha256': a.shared.prior.digest(self.teacher), 'dialogue_prompt_source_sha256': a.shared.prior.digest(self.dialogue),
            'worker_sources_reviewed': True, 'worker_source_scope': 'files-only', 'worker_source_sha256': self.workers,
            'acceptance_evidence': [{'path': str(self.evidence), 'sha256': a.shared.prior.digest(self.evidence)}]}
        self.accepted = self.folder / 'accepted.json'

    def tearDown(self):
        self.temp.cleanup()

    def check(self):
        self.accepted.write_text(json.dumps(self.value))
        with patch.object(a.shared.preflight, 'artifact', return_value={'source_evidence': str(self.source)}), \
                patch.object(a.shared, 'worker_source_fingerprint', return_value=self.workers):
            return a.validate_candidate(self.accepted, self.native)

    def test_exact_candidate_and_unchanged_reviewed_workers_accept(self):
        self.assertEqual(self.check()[1], self.new.resolve())

    def test_schema_change_unaccepted_wrong_previous_or_missing_evidence_refuse(self):
        original = copy.deepcopy(self.value)
        for key, value in [('accepted', False), ('data_protocol', True), ('data_protocol', 9),
            ('activation_kind', 'migration'), ('expected_previous_relay_sha256', 'wrong'),
            ('worker_source_scope', 'loaded-runtime'), ('acceptance_evidence', [])]:
            with self.subTest(key=key, value=value):
                self.value = copy.deepcopy(original); self.value[key] = value
                with self.assertRaises(ValueError): self.check()

    def test_changed_binary_policy_evidence_and_native_bytes_refuse(self):
        self.check()
        for path in [self.old, self.new, self.teacher, self.dialogue, self.evidence, self.native]:
            with self.subTest(path=path.name):
                raw = path.read_bytes(); path.write_bytes(raw + b' changed')
                with self.assertRaises(ValueError): self.check()
                path.write_bytes(raw)

    def test_actual_build_protocol_and_shared_source_mismatch_refuse(self):
        original = copy.deepcopy(self.build_value)
        for key, value in [('sync_version', 9), ('event_version', True), ('source_sha256', {'Cargo.toml': 'different'}), ('teacher_policy', 'old')]:
            self.build_value = copy.deepcopy(original); self.build_value[key] = value
            self.build.write_text(json.dumps(self.build_value))
            self.value['relay_build_receipt_sha256'] = a.shared.prior.digest(self.build)
            with self.assertRaises(ValueError): self.check()

    def test_same_protocol_rollback_still_requires_exact_unchanged_history(self):
        snapshot = {'sha256': 'same', 'max_event_version': 8}
        self.assertTrue(a.shared.rollback_is_safe(snapshot, snapshot, 8))
        self.assertFalse(a.shared.rollback_is_safe(snapshot, {'sha256': 'new-history', 'max_event_version': 8}, 8))


if __name__ == '__main__':
    unittest.main()
