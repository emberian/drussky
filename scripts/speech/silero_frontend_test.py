import hashlib
import sys
import unittest
sys.dont_write_bytecode = True

from silero_frontend import synthesis_input, validate_request
from silero_worker import encode_wav


class SileroFrontendContracts(unittest.TestCase):
    def test_silero_stress_pair_preserves_source_and_distinguishes_actual_input(self):
        castle = synthesis_input("Это за́мок.")
        lock = synthesis_input("Это замо́к.")
        self.assertEqual(castle["synthesis_text"], "Это з+амок.")
        self.assertEqual(lock["synthesis_text"], "Это зам+ок.")
        self.assertNotEqual(castle["source_text_sha256"], lock["source_text_sha256"])
        self.assertNotEqual(castle["synthesis_text_sha256"], lock["synthesis_text_sha256"])
        self.assertEqual(castle["source_text_sha256"], hashlib.sha256("Это за́мок.".encode()).hexdigest())
        self.assertEqual(castle["transform"], "pedagogical_acute_to_silero_plus")
        self.assertEqual(castle["context"], "explicit_stress")

    def test_silero_frontend_preserves_yo_punctuation_and_other_spelling(self):
        value = synthesis_input("Всё гото́во?")
        self.assertEqual(value["synthesis_text"], "Всё гот+ово?")
        plain = synthesis_input("Всё готово?")
        self.assertEqual(plain["synthesis_text"], "Всё готово?")
        self.assertEqual(plain["transform"], "verbatim")
        self.assertEqual(plain["source_text_sha256"], plain["synthesis_text_sha256"])

    def test_silero_frontend_rejects_bad_marks_or_injected_engine_syntax(self):
        for text in ["́Замок", "Зам́ок", "За́́мок", "З+амок", "<speak>Замок</speak>", "[[rate 900]]замок", "за\u0300мок"]:
            with self.subTest(text=text), self.assertRaises(ValueError):
                synthesis_input(text)

    def test_silero_pcm_encoding_preserves_format_and_reports_clipping(self):
        import io, wave
        data, frames, stats = encode_wav([0.1, -0.1, 1.2])
        self.assertEqual(frames, 3); self.assertEqual(stats["clipped_samples"], 1)
        with wave.open(io.BytesIO(data)) as wav:
            self.assertEqual((wav.getnchannels(), wav.getsampwidth(), wav.getframerate(), wav.getnframes()), (1, 2, 24000, 3))

    def test_silero_pcm_rejects_empty_silent_and_nonfinite_output(self):
        for samples in [[], [0.0] * 10, [float("nan")], [float("inf")]]:
            with self.assertRaises(ValueError):
                encode_wav(samples)

    def test_silero_input_and_binding_bounds(self):
        for request in [
            {"text": "я" * 257, "request_id": "x"},
            {"text": "Замок", "request_id": "bad/path"},
            {"text": "Замок", "request_id": "x", "provider": "arbitrary"},
            {"text": "Замок", "request_id": "x", "revision_id": "bad/path"},
        ]:
            with self.assertRaises(ValueError):
                validate_request(request)


if __name__ == "__main__":
    unittest.main()
