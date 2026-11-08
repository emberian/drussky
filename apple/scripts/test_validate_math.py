"""Failure-oriented checks for the math shelf's validation contract."""
import copy
import json
import tempfile
import unittest
from pathlib import Path

import validate_math as math


class MathValidationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.shelf = json.loads(math.DEFAULT.read_text())

    def errors_after(self, mutate):
        data = copy.deepcopy(self.shelf)
        mutate(data)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'math.json'
            path.write_text(json.dumps(data, ensure_ascii=False))
            errors, _, _ = math.validate(path)
        return '\n'.join(errors)

    def test_delivered_shelf(self):
        errors, _, rows = math.validate()
        self.assertEqual(errors, [])
        self.assertEqual(len(rows), 24)
        self.assertTrue(all(row[-1] == 0 for row in rows))

    def test_future_course_word_still_needs_gloss(self):
        # множество is in the course lexicon, but has not been taught at rung 1.
        errors = self.errors_after(lambda d: d['texts'][0]['glosses'].pop('множество'))
        self.assertIn('missing glosses: множество', errors)

    def test_stress_syntax_and_audited_mobile_stress(self):
        for bad in ('множество', 'мно́жество́', 'ё́лка', 'мн́ожество', 'положи́м', 'получи́м'):
            with self.subTest(bad=bad):
                self.assertTrue(math.stress_errors(bad))
        for good in ('мно́жество', 'чи́сла', 'числа́', 'ёлка', 'поло́жим', 'полу́чим'):
            with self.subTest(good=good):
                self.assertEqual(math.stress_errors(good), [])

    def test_hyphenated_words_match_reader_gloss_lookup(self):
        self.assertEqual(math.tokens('по-пре́жнему'), ['по-пре́жнему'])
        errors = self.errors_after(lambda d: d['texts'][17]['glosses'].pop('по-прежнему'))
        self.assertIn('missing glosses: по-прежнему', errors)

    def test_answer_is_zero_based_integer_not_bool(self):
        for invalid in (-1, 3, True, '1'):
            errors = self.errors_after(lambda d: d['texts'][0]['questions'][0].update(answer=invalid))
            self.assertIn('zero-based integer', errors)

    def test_page_order_is_course_order_not_unit_number(self):
        course = json.loads(math.COURSE.read_text())
        order = list(math.course_exposure(course))
        self.assertLess(order.index('u14-p5'), order.index('u8-p1'))
        errors = self.errors_after(lambda d: d['texts'][1].update(after_page='u1-p1'))
        self.assertIn('must not move backwards', errors)

    def test_nested_schema_reports_errors_without_crashing(self):
        for field, value, message in (
            ('how_read', None, '3–5 frames'),
            ('notation', [None], 'symbol/ru/en'),
            ('questions', [None]*3, 'question must be an object'),
            ('after_page', [], 'not a course page id'),
        ):
            with self.subTest(field=field):
                self.assertIn(message, self.errors_after(lambda d: d['texts'][0].update({field: value})))
        self.assertIn('every text must be an object', self.errors_after(lambda d: d['texts'].append(None)))

    def test_notation_is_allowed_but_html_is_rejected(self):
        good = self.errors_after(lambda d: d['texts'][0].update(text=d['texts'][0]['text'] + ' |x| < 1.'))
        self.assertNotIn('leftover markup', good)
        bad = self.errors_after(lambda d: d['texts'][0].update(text=d['texts'][0]['text'] + ' <script>'))
        self.assertIn('leftover markup', bad)

    def test_raw_options_do_not_teach_a_word(self):
        # Letter examples and presented phrases do teach; distractors do not.
        course = {'letters': [], 'lexicon': [{'ru':'мно́жество'}, {'ru':'я зна́ю'}], 'units':[
            {'pages':[
                {'id':'first','exercises':[{'kind':'listen_pick','say':'я зна́ю','options':['мно́жество','я зна́ю']}]},
                {'id':'second','exercises':[{'kind':'copy','text':'мно́жество'}]},
            ]}
        ]}
        pages = math.course_exposure(course)
        self.assertEqual(pages['first'], {'я', 'знаю'})
        self.assertEqual(pages['second'], {'я', 'знаю', 'множество'})


if __name__ == '__main__':
    unittest.main()
