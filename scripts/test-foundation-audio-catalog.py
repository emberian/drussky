#!/usr/bin/env python3
"""Pure, bounded integrity checks; no playback, builds, or speech generation."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest
import tempfile
import hashlib
import wave

spec = importlib.util.spec_from_file_location('catalog', Path(__file__).with_name('generate-foundation-audio-catalog.py'))
catalog = importlib.util.module_from_spec(spec)
spec.loader.exec_module(catalog)


class CatalogTests(unittest.TestCase):
    def setUp(self):
        self.manifest = json.loads((catalog.ASSETS / 'manifest.json').read_text())

    def test_checked_in_catalog_matches_every_asset(self):
        self.assertEqual(catalog.generate(self.manifest, catalog.ASSETS), catalog.OUTPUT.read_text())

    def test_original_twenty_durations(self):
        expected = {'a': 41472/44100, 'ma': .325, 'am': .3625, 'da': .325, 'net': .375,
                    'o': 1, 'ka': .2875, 'ta': .275, 'kot': 43776/44100, 'tam': .325,
                    'tam-kot': .7125, 'i': 40320/44100, 'na': 1.1, 'okno': 46080/44100,
                    'on': 40320/44100, 'ona': 1.25, 'oni': 1.3, 'ono': 21888/44100,
                    'ty': 1, 'ya': 1.15}
        records = {r[0]: r for r in (catalog.record(a, catalog.ASSETS) for a in self.manifest['assets'])}
        for suffix, duration in expected.items():
            row = records['foundation-' + suffix]
            self.assertAlmostEqual(row[2] / row[3], duration)

    def test_rejects_duplicate_ids(self):
        self.manifest['assets'].append(copy.deepcopy(self.manifest['assets'][0]))
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            catalog.generate(self.manifest, catalog.ASSETS)

    def test_rejects_inconsistent_identity_and_missing_provenance(self):
        cases = [(['source_response', 'sha256'], '0' * 64),
                 (['source_response', 'frames'], 1), (['source_response', 'sample_rate'], 1),
                 (['source_response', 'channels'], 2), (['byte_length'], 1),
                 (['measurements', 'duration_seconds'], 0),
                 (['provenance', 'credit'], ''), (['provenance', 'license_url'], 'http://invalid'),
                 (['provenance', 'original_sha256'], '0' * 64),
                 (['provenance', 'original_file'], '../escape.ogg'),
                 (['provenance', 'source_metadata'], '/tmp/no.json'),
                 (['provenance', 'kind'], 'unknown'), (['transforms'], [])]
        for keys, value in cases:
            asset = copy.deepcopy(self.manifest['assets'][0])
            parent = asset
            for key in keys[:-1]:
                parent = parent[key]
            parent[keys[-1]] = value
            with self.subTest(keys=keys), self.assertRaises((ValueError, KeyError, FileNotFoundError)):
                catalog.record(asset, catalog.ASSETS)

    def test_real_gateway_explicit_stress_envelope(self):
        # Real prepared first-take metadata; create tiny PCM locally, no runtime import.
        asset = {'id': 'foundation-a9-this', 'file': 'foundation-a9-this.wav', 'spoken': 'это', 'display_text': 'э́то', 'teaching_span': 'whole meaningful word or message', 'encoding': 'RIFF/WAVE PCM16 little-endian mono', 'byte_length': 21044, 'source_response': {'request_id': 'foundation-a9-this', 'revision_id': 'foundation-2026-11-06.3', 'source_text': 'э́то', 'sha256': '845d8ff8a8117b4ec8e0b26ba2db95e4a4fd23c2fe42b25b14603f024657e93e', 'sample_rate': 24000, 'channels': 1, 'frames': 10500, 'provenance': {'kind': 'ai_generated_speech', 'provider': 'local_silero_tts', 'model': 'v5_6_ru@04cc9b1d08ce38bcaa9d02c488d80d3e6806d17f7eaa7c95982f36a3cbf50b2e;repo=e382298387332f2bf461fe6eadfde99104611b0c', 'voice': 'xenia', 'prompt_version': 'rufren-silero-explicit-v1', 'validation': 'CPU2;seed=42;put_accent=false,put_stress_homo=false,put_yo=false,put_yo_homo=false,stress_single_vowel=false;peak_amplitude=0.505883634;clipped_samples=0;clipped_fraction=0;full_scale_fraction=0; actual API input recorded; acoustic correctness/stress not certified by ASR', 'synthesis_input': {'source_text_sha256': 'ed7301bf2bd7dd4c3996ad40b12376395f5f411e19c620d5519596ee1a3cca11', 'synthesis_text': '+это', 'synthesis_text_sha256': 'cd7b051c164e0de4899a926225eab5b38f853111f94f2774c91c2966224befe9', 'transform': 'pedagogical_acute_to_silero_plus', 'context': 'explicit_stress'}}, 'usage': {'input_tokens': None, 'output_tokens': None, 'estimated_micro_usd': 0, 'reserved_micro_usd': 0}}, 'measurements': {'peak_amplitude': 0.5058836340904236, 'clipped_samples': 0, 'clipped_fraction': 0.0, 'full_scale_fraction': 0.0, 'duration_seconds': 0.4375, 'generation_seconds': 0.1318}, 'runtime': {'runtime': 'torch==2.15.0', 'device': 'cpu', 'threads': 2, 'model_license': 'CC-BY-NC-SA-4.0', 'model_source_revision': 'e382298387332f2bf461fe6eadfde99104611b0c'}, 'transforms': [], 'review': {'status': 'prepared; lexical and acoustic review pending', 'requested_checks': ['Exact target and no added words', 'Correct intended stress, complete onset and ending', 'Whole model is intelligible without reading its spelling', 'Lexical stress: э́то: syllable 1 of 2. Stress annotations specify lexical position, not equal sentence emphasis on every word.', 'Initial Э has no glide; reduce the unstressed final written О naturally.']}, 'prepared_at': '2026-11-06T22:02:46.980523+00:00'}
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            wav = root / asset['file']
            with wave.open(str(wav), 'wb') as audio:
                audio.setnchannels(1)
                audio.setsampwidth(2)
                audio.setframerate(24000)
                audio.writeframes(b'\x01\x00' * 24)
            asset['byte_length'] = wav.stat().st_size
            asset['source_response']['sha256'] = hashlib.sha256(wav.read_bytes()).hexdigest()
            asset['source_response']['frames'] = 24
            asset['measurements']['duration_seconds'] = .001
            catalog.record(asset, root)
            cases = [(['source_response', 'source_text'], 'э\u0300то'),
                     (['spoken'], 'эта'),
                     (['source_response', 'provenance', 'model'], 'v5_6_ru@' + '0' * 64),
                     (['runtime', 'model_source_revision'], '0' * 40),
                     (['source_response', 'provenance', 'synthesis_input', 'transform'], 'verbatim'),
                     (['source_response', 'provenance', 'synthesis_input', 'synthesis_text'], 'эт+о'),
                     (['source_response', 'provenance', 'synthesis_input', 'source_text_sha256'], '0' * 64),
                     (['source_response', 'provenance', 'synthesis_input', 'synthesis_text_sha256'], '0' * 64)]
            for keys, value in cases:
                changed = copy.deepcopy(asset)
                parent = changed
                for key in keys[:-1]:
                    parent = parent[key]
                parent[keys[-1]] = value
                with self.subTest(keys=keys), self.assertRaises(ValueError):
                    catalog.record(changed, root)

    def test_missing_and_unsafe_wav_paths_fail(self):
        for filename in ('../bad.wav', '/tmp/bad.wav', 'missing.wav'):
            asset = copy.deepcopy(self.manifest['assets'][0])
            asset['file'] = filename
            with self.subTest(filename=filename), self.assertRaises(ValueError):
                catalog.record(asset, catalog.ASSETS)


if __name__ == '__main__':
    unittest.main()
