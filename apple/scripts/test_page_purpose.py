"""Purpose, receptive vocabulary and reserved baseline content regressions (no audio synthesis)."""
import json
from pathlib import Path
import re
import unittest
import build_course
import validate_course as validate


class PagePurposeTests(unittest.TestCase):
    def errors(self, purpose, kinds):
        validate.errors.clear()
        validate.check_page_purpose('test', {'purpose': purpose, 'exercises': [{'kind': k} for k in kinds]})
        return list(validate.errors)

    def test_purpose_changes_composition_not_other_validation(self):
        reading = ['explain'] + ['read_meaning'] * 6 + ['diary']
        self.assertFalse(self.errors('reading', reading))
        self.assertTrue(self.errors('writing', reading))
        self.assertTrue(self.errors('oral', reading))
        oral = ['explain', 'read_aloud', 'read_aloud', 'read_aloud', 'listen_pick', 'listen_pick']
        self.assertFalse(self.errors('oral', oral))
        self.assertTrue(self.errors('oral', ['listen_pick'] * 6))
        self.assertTrue(self.errors('whatever', reading))

    def test_participles_are_reading_not_production_and_keep_diary(self):
        course = build_course.build()
        page = next(p for u in course['units'] for p in u['pages'] if p['id'] == 'u28-p2')
        self.assertEqual(page['purpose'], 'reading')
        self.assertEqual(page['exercises'][-1]['kind'], 'diary')
        body = page['exercises'][:-1]
        # Recognition, hearing and reading aloud: nothing on it demands producing a participle with the Pencil.
        self.assertTrue(all(e['kind'] in ('explain', 'read_meaning', 'listen_pick', 'read_aloud') for e in body))
        self.assertFalse(any(validate.is_writing(e) for e in body))
        self.assertGreater(sum(e['kind'] == 'read_meaning' for e in body) * 2, len(body))
        self.assertTrue(6 <= len(body) <= 10)
        # English option order rotates through the authored helper, so position is not the answer.
        self.assertGreater(len({e['answer'] for e in body if e['kind'] == 'read_meaning'}), 1)

    def test_baseline_russian_has_stress_and_uses_existing_audio(self):
        root = Path(__file__).resolve().parents[1]
        source = (root / 'Drussky/Baseline/Baseline.swift').read_text()
        construction = (root / 'Drussky/Constructions/ConstructionChoice.swift').read_text()
        validate.errors.clear()
        for text in re.findall(r'"([^"\n]*[А-Яа-яЁё][^"\n]*)"', source + construction):
            validate.check_stress('baseline', text)
        self.assertEqual(validate.errors, [])
        files = re.findall(r'"([a-z0-9-]+\.m4a)"', source)
        self.assertEqual(len(files), 10)
        library = json.loads((root / 'Drussky/Resources/library.json').read_text())
        licensed = {s['audio']: s for s in library['sentences']}
        for file in files:
            self.assertTrue((root / 'Drussky/Resources/Audio' / file).is_file(), file)
            self.assertTrue(licensed[file]['license'])
            self.assertTrue(licensed[file]['source'])


if __name__ == '__main__':
    unittest.main()
