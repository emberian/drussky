#!/usr/bin/env python3
"""Client-only backend guards; no services, private data or alias writes."""
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('promotion', Path(__file__).with_name('promote-native-candidate.py'))
promotion = importlib.util.module_from_spec(spec)
spec.loader.exec_module(promotion)


class ClientOnlyBackend(unittest.TestCase):
    def setUp(self):
        self.accepted = {'data_protocol': 8, 'relay_executable_sha256': 'accepted-bytes',
                         'expected_previous_relay_sha256': 'accepted-bytes',
                         'private_config_sha256': {'ca.pem': 'same-ca', 'bootstrap.json': 'same-config'}}
        self.binary = Path('/synthetic/immutable-relay')
        self.running = {'running': True, 'ownership': 'recorded', 'pid': 321,
                        'executable': str(self.binary), 'recorded_sha256': 'accepted-bytes',
                        'on_disk_matches_receipt': True}

    def check(self, protocol=8, config=None, activation=None):
        with patch.object(promotion.activation_helper, 'validate_candidate', return_value=(self.accepted, self.binary)), \
                patch.object(promotion.shared.prior, 'config_fingerprint', return_value=config or self.accepted['private_config_sha256']), \
                patch.object(promotion.shared.services, 'service_status', return_value=self.running):
            return promotion.validate_backend(Path('/synthetic/ios.json'), protocol, activation, Path('/synthetic/accepted.json'))

    def test_same_running_backend_needs_no_activation_and_records_client_only(self):
        _, record = self.check()
        self.assertEqual(record['mode'], 'client-only')
        self.assertEqual(record['relay_pid'], 321)
        self.assertNotIn('accepted_relay_cutover', record)

    def test_backend_replacement_protocol_or_private_config_change_refuses(self):
        with self.assertRaises(ValueError):
            self.check(protocol=7)
        with self.assertRaises(ValueError):
            self.check(config={'ca.pem': 'changed'})
        self.accepted['relay_executable_sha256'] = 'new-backend'
        with self.assertRaises(ValueError):
            self.check()

    def test_actual_running_identity_or_disk_hash_mismatch_refuses(self):
        for key, value in [('running', False), ('ownership', 'legacy-command-and-cwd'),
                           ('executable', '/other/relay'), ('recorded_sha256', 'other'),
                           ('on_disk_matches_receipt', False)]:
            original = self.running[key]
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.running[key] = value
                self.check()
            self.running[key] = original

    def test_two_backend_inputs_refuse(self):
        with self.assertRaises(ValueError):
            self.check(activation=Path('/synthetic/cutover.json'))

    def test_existing_activation_path_retains_exact_pid_guard(self):
        activation = {'accepted': True, 'data_protocol': 8, 'transition_profile': 'same-protocol',
                      'stage': 'active-data8-awaiting-root-alias-promotion',
                      'native_candidate_receipt': '/synthetic/ios.json',
                      'private_config_preserved': True, 'artifact_aliases_preserved': True,
                      'accepted_relay_receipt': '/synthetic/accepted.json',
                      'relay_executable_sha256': 'accepted-bytes', 'relay_pid': 321,
                      'config_hashes_before': {}, 'worker_source_sha256': {}, 'artifact_alias_hashes_before': {}}
        with patch.object(promotion.native, 'read', return_value=activation), \
                patch.object(promotion.activation_helper, 'validate_candidate', return_value=(self.accepted, self.binary)), \
                patch.object(promotion.shared.prior, 'config_fingerprint', return_value={}), \
                patch.object(promotion.shared, 'worker_source_fingerprint', return_value={}), \
                patch.object(promotion.shared, 'alias_fingerprint', return_value={}), \
                patch.object(promotion.shared.services, 'service_status', return_value=self.running):
            _, record = promotion.validate_backend(Path('/synthetic/ios.json'), 8, Path('/synthetic/cutover.json'))
            self.assertEqual(record['mode'], 'after-relay-activation')
            self.assertIn('accepted_relay_cutover', record)
            self.running['pid'] = 322
            with self.assertRaises(ValueError):
                promotion.validate_backend(Path('/synthetic/ios.json'), 8, Path('/synthetic/cutover.json'))


if __name__ == '__main__':
    unittest.main()
