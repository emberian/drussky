"""Regression checks for what the learner actually saw before recall.

Run with python3 -m unittest discover -s apple/scripts -p 'test_validate_course.py'.
"""
import unittest

import validate_course as validator


class TeachingOrderTests(unittest.TestCase):
    def setUp(self):
        validator.errors.clear()

    def course(self, exercises):
        return {
            "letters": [{"upper": "К", "example": "кот"}, {"upper": "О"}, {"upper": "Т"}],
            "units": [{"pages": [{"id": "first", "exercises": exercises}]}],
        }

    def test_hidden_letter_example_does_not_teach_a_word(self):
        course = self.course([
            {"kind": "meet", "letter": "К"},  # кот is hidden: О and Т are still unknown
            {"kind": "meet", "letter": "О"},
            {"kind": "meet", "letter": "Т"},
            {"kind": "translate", "answer": "кот"},
        ])
        validator.check_meaning_before_recall(course, {"кот"})
        self.assertEqual(len(validator.errors), 1)
        self.assertIn("recalls ['кот']", validator.errors[0])

    def test_visible_letter_example_teaches_the_word(self):
        course = self.course([
            {"kind": "meet", "letter": "О"},
            {"kind": "meet", "letter": "Т"},
            {"kind": "meet", "letter": "К"},
            {"kind": "translate", "answer": "кот"},
        ])
        validator.check_meaning_before_recall(course, {"кот"})
        self.assertEqual(validator.errors, [])

    def test_later_copy_does_not_license_earlier_recall(self):
        course = self.course([
            {"kind": "dictation", "text": "кот"},
            {"kind": "copy", "text": "кот"},
            {"kind": "dictation", "text": "кот"},
        ])
        validator.check_meaning_before_recall(course, {"кот"})
        self.assertEqual(len(validator.errors), 1)
        self.assertTrue(validator.errors[0].startswith("first#1"))

    def test_untaught_inflection_is_not_a_known_lemma(self):
        course = self.course([
            {"kind": "copy", "text": "кни́га"},
            {"kind": "fill", "answer": "кни́гу"},
        ])
        validator.check_meaning_before_recall(course, {"книга", "книгу"})
        self.assertEqual(len(validator.errors), 1)
        self.assertIn("книгу", validator.errors[0])

    def test_yo_tiles_never_take_an_acute(self):
        validator.check_tile("tile", "-ё́т")
        self.assertEqual(len(validator.errors), 1)
        validator.errors.clear()
        validator.check_tile("tile", "-ёт")
        validator.check_tile("tile", "-а́ть")
        self.assertEqual(validator.errors, [])


if __name__ == "__main__":
    unittest.main()
