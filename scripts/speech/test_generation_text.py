"""No models, network, downloads or numerical-runtime imports."""
import sys
sys.dont_write_bytecode = True
import unittest

from generation_text import AMBIGUOUS, validate_generation_text
from qwen_frontend import synthesis_input


class GenerationTextContracts(unittest.TestCase):
    def test_exact_finite_lexicon_and_each_occurrence_at_any_length(self):
        self.assertEqual(AMBIGUOUS, frozenset(("замок", "мука", "плачу", "атлас", "орган", "духи", "ирис", "белки", "стрелки", "кружки", "стоит", "дороги", "вести", "пропасть", "село")))
        for word in AMBIGUOUS:
            for source in [word, f"Сего́дня мы вместе читаем длинное предложение про {word}.", f"{word}, {word.upper()}!"]:
                with self.subTest(source=source), self.assertRaises(ValueError):
                    validate_generation_text(source)
        for source in ["Это за́мок, а это замок.", "За́мок — ЗАМОК", "Приве́т! Это замок.", "Это з́амок.", "Это за́мо́к.", "́замок"]:
            with self.subTest(source=source), self.assertRaises(ValueError):
                validate_generation_text(source)

    def test_marked_occurrences_can_have_different_stress_and_forms_are_not_stemmed(self):
        for source in ["Это за́мок, а это замо́к.", "ЗА́МОК!", "О замках.", "Здравствуйте!"]:
            validate_generation_text(source)

    def test_numeric_readout_requires_authored_words_not_a_toy_normalizer(self):
        for source in ["У меня 21 свободная минута.", "У меня ２ вопроса.", "У меня ² вопроса.", "У меня ½ минуты.", "У меня́ 2 вопроса."]:
            with self.subTest(source=source), self.assertRaises(ValueError):
                validate_generation_text(source)
        validate_generation_text("У меня двадцать одна свободная минута.")
        validate_generation_text("Chapter 21")

    def test_historical_provenance_remains_readable_and_source_unchanged(self):
        for source in ["На двери висит замок, а на холме стоит старый замок.", "У меня 21 свободная минута."]:
            self.assertEqual(synthesis_input(source)["synthesis_text"], source)
            with self.assertRaises(ValueError):
                validate_generation_text(source)


if __name__ == "__main__":
    unittest.main()
