"""CPU-only adversarial checks for the unhooked ASR evidence boundary."""
import base64
import hashlib
import io
import math
import struct
import unittest
import wave
import sys
sys.dont_write_bytecode = True
from asr_worker import decode_wav, validate
from asr_review import lexical_tokens, plan_next_action, token_differences


def pcm_wav():
    output = io.BytesIO()
    with wave.open(output, "wb") as wav:
        wav.setnchannels(1); wav.setsampwidth(2); wav.setframerate(16000)
        wav.writeframes(struct.pack("<hhhh", 0, 1000, -1000, 0))
    return output.getvalue()


def request(data=None):
    data = pcm_wav() if data is None else data
    return {"request_id": "public-fixture", "asset_id": "asset-1", "language": "ru",
            "audio_sha256": hashlib.sha256(data).hexdigest(), "wav_base64": base64.b64encode(data).decode()}


class EvidenceBoundary(unittest.TestCase):
    def test_original_frame_crop(self):
        req = request(); req["span"] = {"start_frame": 1, "end_frame": 3}
        shape, payload, span = validate(req)
        self.assertEqual((shape["frames"], span, len(payload)), (4, req["span"], 8))

    def test_no_expected_text_prompt(self):
        req = request(); req["expected_text"] = "замок"
        with self.assertRaises(ValueError): validate(req)

    def test_wrong_recording_hash(self):
        req = request(); req["audio_sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "hash mismatch"): validate(req)

    def test_wrong_language(self):
        req = request(); req["language"] = "en"
        with self.assertRaises(ValueError): validate(req)

    def test_no_out_of_bounds_or_boolean_spans(self):
        for span in ({"start_frame": -1, "end_frame": 3}, {"start_frame": 0, "end_frame": 5},
                     {"start_frame": True, "end_frame": 3}, {"start_frame": 2, "end_frame": 2}):
            req = request(); req["span"] = span
            with self.assertRaises(ValueError): validate(req)

    def test_truncated_or_extra_container(self):
        for data in (pcm_wav()[:-1], pcm_wav() + b"junk"):
            with self.assertRaises(ValueError): validate(request(data))

    def test_duplicate_data(self):
        data = bytearray(pcm_wav()); data.extend(b"data" + struct.pack("<I", 2) + b"\0\0")
        struct.pack_into("<I", data, 4, len(data) - 8)
        with self.assertRaisesRegex(ValueError, "Duplicate WAV data"): validate(request(bytes(data)))

    def test_float_capture_and_nonfinite_rejection(self):
        for sample in (0.25, math.nan, math.inf, 1.1):
            fmt = struct.pack("<HHIIHH", 3, 1, 16000, 64000, 4, 32)
            body = b"WAVEfmt " + struct.pack("<I", 16) + fmt + b"data" + struct.pack("<I", 4) + struct.pack("<f", sample)
            data = b"RIFF" + struct.pack("<I", len(body)) + body
            if sample == 0.25:
                self.assertEqual(validate(request(data))[0]["encoding"], "ieee_f32le")
            else:
                with self.assertRaises(ValueError): validate(request(data))

    def test_decoder_metadata_mismatch(self):
        data = bytearray(pcm_wav()); struct.pack_into("<I", data, 28, 1)
        with self.assertRaises(ValueError): decode_wav(data)

    def test_stress_marks_are_not_acoustic_observations(self):
        self.assertEqual(lexical_tokens("за́мок"), lexical_tokens("замо́к"))
        self.assertNotEqual(lexical_tokens("всё"), lexical_tokens("все"))
        self.assertEqual(lexical_tokens("мой"), ["мой"])

    def test_lexical_replacement_budget_and_replay(self):
        evidence = {"schema_version": "rufren-speech-transcript-v1", "evidence_kind": "transcript_hypothesis",
                    "target_conditioned": False, "language": "ru", "transcript": "самоек",
                    "asset_id": "sample", "audio_sha256": "a" * 64, "original_audio": {"frames": 100},
                    "span": {"start_frame": 0, "end_frame": 100}}
        first = plan_next_action("замо́к", evidence, mode="generated_reference")
        second = plan_next_action("замо́к", evidence, mode="generated_reference", replacement_candidates_used=1)
        learner = plan_next_action("замо́к", evidence, mode="learner_attempt")
        self.assertEqual(first["primary_action"], "request_one_alternative_reference_candidate")
        self.assertEqual(second["primary_action"], "listen_confirm_meaning_and_retry")
        self.assertEqual(learner["primary_action"], "listen_confirm_meaning_and_retry")
        self.assertEqual(first["replay"]["localization"], "selected_crop_not_word_alignment")
        self.assertIsNone(first["pronunciation_verdict"])
        evidence["target_conditioned"] = True
        with self.assertRaises(ValueError): plan_next_action("замок", evidence, mode="learner_attempt")

    def test_repeated_words_and_omissions(self):
        diff = token_differences("да да нет", "да нет")
        self.assertEqual(len(diff), 1)
        self.assertEqual((diff[0]["kind"], diff[0]["expected"]), ("omission", "да"))

    def test_yo_spelling_is_flagged_without_automatic_regeneration(self):
        evidence = {"schema_version": "rufren-speech-transcript-v1", "evidence_kind": "transcript_hypothesis",
                    "target_conditioned": False, "language": "ru", "transcript": "Все готово?",
                    "asset_id": "sample", "audio_sha256": "a" * 64, "original_audio": {"frames": 100},
                    "span": {"start_frame": 0, "end_frame": 100}}
        plan = plan_next_action("Всё готово?", evidence, mode="generated_reference")
        self.assertEqual(plan["primary_action"], "listen_confirm_meaning_and_retry")
        self.assertTrue(plan["lexical_differences"][0]["yo_spelling_ambiguity"])


if __name__ == "__main__":
    unittest.main()
