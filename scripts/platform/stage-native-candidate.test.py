#!/usr/bin/env python3
"""Native staging guards with temporary fixtures; never build or sign anything."""
import argparse
import importlib.util
import json
from pathlib import Path
import plistlib
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('native_stage', Path(__file__).with_name('stage-native-candidate.py'))
stage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(stage)


class GuardTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()
        self.task = self.root / '.local/ipad-delivery'
        self.task.mkdir(parents=True)
        self.settings = patch.multiple(stage, ROOT=self.root, TASK=self.task)
        self.settings.start()

    def tearDown(self):
        self.settings.stop()
        self.temp.cleanup()

    def bundle(self, location, macos=False):
        app = self.root / location
        folder = app / 'Contents/MacOS' if macos else app
        folder.mkdir(parents=True)
        binary = folder / 'rufren-app'
        binary.write_bytes(b'synthetic never-executed native bytes')
        info = {'CFBundleIdentifier': 'town.lunar.rufren', 'CFBundleExecutable': 'rufren-app'}
        (app / ('Contents/Info.plist' if macos else 'Info.plist')).write_bytes(plistlib.dumps(info))
        return {'app': str(app), 'rust_executable_sha256': stage.digest(binary)}, binary

    def test_mutated_selected_executable_refuses(self):
        manifest, binary = self.bundle('frozen/Rufren.app')
        self.assertEqual(stage.verify_executable(manifest), binary)
        binary.write_bytes(b'changed')
        with self.assertRaises(ValueError):
            stage.verify_executable(manifest)

    def test_mutable_target_and_wrong_identity_refuse(self):
        manifest, _ = self.bundle('target/Rufren.app')
        with self.assertRaises(ValueError):
            stage.verify_executable(manifest)
        manifest, _ = self.bundle('frozen/Other.app')
        (Path(manifest['app']) / 'Info.plist').write_bytes(plistlib.dumps({
            'CFBundleIdentifier': 'other.app', 'CFBundleExecutable': 'rufren-app'}))
        with self.assertRaises(ValueError):
            stage.verify_executable(manifest)

    def test_mac_executable_escape_refuses(self):
        manifest, binary = self.bundle('frozen/Mac.app', True)
        outside = self.root / 'outside'
        outside.write_bytes(binary.read_bytes())
        binary.unlink()
        binary.symlink_to(outside)
        with self.assertRaises(ValueError):
            stage.verify_executable(manifest, True)

    def test_source_add_remove_and_mutation_all_refuse(self):
        baseline = {'old.rs': 'old'}
        for current in [{'old.rs': 'old', 'new.rs': 'new'}, {}, {'old.rs': 'changed'}]:
            with self.subTest(current=current), self.assertRaises(ValueError):
                stage.assert_equal_maps(baseline, current, 'Source inputs')

    def test_only_generic_and_numeric_aliases_are_guarded(self):
        for name in ['frozen-artifact.json', 'frozen-artifact-v8.json', 'frozen-artifact-old-candidate.json']:
            (self.task / name).write_text('{}')
        before = stage.alias_hashes()
        self.assertEqual(set(before), {'frozen-artifact.json', 'frozen-artifact-v8.json'})
        (self.task / 'frozen-artifact-v8.json').write_text('{"changed":true}')
        with self.assertRaises(ValueError):
            stage.assert_equal_maps(before, stage.alias_hashes(), 'Artifact aliases')

    def test_existing_candidate_and_unsafe_names_refuse_before_build(self):
        (self.task / 'frozen-existing').mkdir()
        for name in ['../escape', 'existing']:
            args = argparse.Namespace(name=name, web_build='0' * 20)
            with self.subTest(name=name), self.assertRaises(ValueError):
                stage.configuration(args)


if __name__ == '__main__':
    unittest.main()
