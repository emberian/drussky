import base64
import contextlib
import hashlib
import io
import json
import subprocess
import sys
import unittest
import wave
from unittest.mock import patch
sys.dont_write_bytecode = True

import silero_gateway as gateway
from silero_frontend import synthesis_input
from silero_model import PROJECT, STATE


def payload(source="Это за́мок.", mode="explicit_stress"):
    actual = synthesis_input(source)["synthesis_text"] if mode == "explicit_stress" else source
    return {"schema": gateway.SCHEMA, "text": actual, "request_id": "public-request", "revision_id": None,
            "source_text": source, "source_text_sha256": hashlib.sha256(source.encode()).hexdigest(), "mode": mode}


class FakeModel:
    def __init__(self):
        self.seen = []

    def samples(self, text, flags):
        self.seen.append((text, flags)); return [0.1, -0.1] * 24


class SileroGatewayContracts(unittest.TestCase):
    def test_explicit_preserves_source_binding_and_all_automatic_flags_off(self):
        model = FakeModel(); request = gateway.parse(json.dumps(payload(), ensure_ascii=False).encode())
        response, _ = gateway.synthesize(request, loader=lambda: model)
        self.assertEqual(model.seen, [("Это з+амок.", gateway.EXPLICIT_FLAGS)])
        self.assertEqual(response["source_text"], "Это за́мок.")
        self.assertEqual(response["provenance"]["synthesis_input"], synthesis_input("Это за́мок."))
        self.assertIn("put_accent=false", response["provenance"]["validation"])
        self.assertEqual(response["provenance"]["prompt_version"], "rufren-silero-explicit-v1")
        self.assertEqual(response["provenance"]["model"], gateway.MODEL_IDENTITY)
        self.assertIn("peak_amplitude=0.1;clipped_samples=0;clipped_fraction=0;", response["provenance"]["validation"])
        raw = base64.b64decode(response["wav_base64"], validate=True)
        self.assertEqual(response["sha256"], hashlib.sha256(raw).hexdigest())
        with wave.open(io.BytesIO(raw)) as audio:
            self.assertEqual((audio.getnchannels(), audio.getsampwidth(), audio.getframerate(), audio.getnframes()), (1, 2, 24000, 48))

    def test_automatic_requires_plain_identical_text_and_reports_enabled_flags(self):
        model = FakeModel(); request = gateway.validate(payload("Всё готово?", "automatic"))
        response, _ = gateway.synthesize(request, loader=lambda: model)
        self.assertEqual(model.seen, [("Всё готово?", gateway.AUTOMATIC_FLAGS)])
        self.assertEqual(response["provenance"]["synthesis_input"]["transform"], "verbatim")
        self.assertEqual(response["provenance"]["prompt_version"], "rufren-silero-automatic-v1")
        self.assertIn("put_stress_homo=true", response["provenance"]["validation"])

    def test_generation_preflight_before_model_preserves_explicit_per_token_route(self):
        model = FakeModel()
        for source in ["На двери висит замок, а на холме стоит старый замок.", "Приве́т! Это замок.", "Это за́мок, а это замок.", "У меня 21 свободная минута."]:
            with self.subTest(source=source), self.assertRaises(ValueError):
                gateway.validate(payload(source, "automatic"))
        self.assertEqual(model.seen, [])
        source = "На двери висит замо́к, а на холме стои́т старый за́мок."
        request = gateway.validate(payload(source))
        response, _ = gateway.synthesize(request, loader=lambda: model)
        self.assertEqual(response["source_text"], source)
        self.assertEqual(model.seen[0][0], "На двери висит зам+ок, а на холме сто+ит старый з+амок.")

    def test_schema_hash_bindings_and_literal_engine_syntax_fail_before_model(self):
        base = payload()
        cases = [{**base, "source_text_sha256": "0" * 64}, {**base, "schema": "future"}, {**base, "file": "/etc/passwd"},
                 {**base, "text": "Это зам+ок."}, {**base, "text": "<speak>Это</speak>"},
                 {**base, "request_id": "bad/path"}, payload("Это за́мок.", "automatic"), payload("З+амок", "automatic")]
        for value in cases:
            with self.subTest(value=value), self.assertRaises(ValueError):
                gateway.parse(json.dumps(value, ensure_ascii=False).encode())
        with self.assertRaises(gateway.InvalidInput):
            gateway.parse(b'{"schema":"a","schema":"b"}')
        with self.assertRaises(gateway.InvalidInput):
            gateway.parse(b"x" * 8193)

    def test_noisy_model_diagnostics_do_not_leak_text_or_pollute_stdout(self):
        class Noisy(FakeModel):
            def samples(self, text, flags):
                print("PRIVATE_SOURCE_TEXT " + text); print("PRIVATE_STDERR", file=sys.stderr)
                return super().samples(text, flags)
        source = io.TextIOWrapper(io.BytesIO(json.dumps(payload(), ensure_ascii=False).encode()), encoding="utf-8")
        result = io.TextIOWrapper(io.BytesIO(), encoding="utf-8")
        error = io.StringIO()
        with patch.object(sys, "argv", ["silero_gateway.py"]), patch.object(sys, "stdin", source), patch.object(sys, "stdout", result), patch.object(sys, "stderr", error), patch.object(gateway, "check_installation", lambda: {}), patch.object(gateway, "load_model", lambda: Noisy()), patch.object(gateway.os, "nice", lambda _: None), patch.object(gateway.resource, "setrlimit") as cpu_limit:
            gateway.main(); result.flush(); output = result.buffer.getvalue()
        cpu_limit.assert_called_once_with(gateway.resource.RLIMIT_CPU, (30, 30))
        value = json.loads(output)
        self.assertEqual(value["source_text"], "Это за́мок.")
        self.assertNotIn("PRIVATE_", output.decode()); self.assertNotIn("PRIVATE_", error.getvalue())
        self.assertGreater(json.loads(error.getvalue())["diagnostics_suppressed_bytes"], 0)

    def test_invalid_subprocess_output_is_redacted_without_numerical_import(self):
        child = subprocess.run([str(STATE / "venv/bin/python"), "-B", str(PROJECT / "scripts/speech/silero_gateway.py")],
                               input=b'{"source_text":"SECRET_MARKER"}', capture_output=True, timeout=5)
        self.assertEqual(child.returncode, 2); self.assertEqual(child.stdout, b"")
        self.assertNotIn(b"SECRET_MARKER", child.stderr); self.assertNotIn(b"Traceback", child.stderr)
        self.assertEqual(json.loads(child.stderr)["error"], "invalid_request")

    def test_check_is_read_only_and_does_not_import_torch(self):
        self.assertNotIn("torch", sys.modules)
        status = gateway.check_installation()
        self.assertTrue(status["enabled"]); self.assertFalse(status["model_loaded"]); self.assertFalse(status["inference_run"])
        self.assertNotIn("torch", sys.modules)
        self.assertEqual(status["schema"], gateway.STATUS_SCHEMA)

    def test_clipping_observations_survive_in_response_provenance(self):
        class Clipped(FakeModel):
            def samples(self, text, flags):
                return [1.2, -1.5, 0.1, -0.1]
        response, signals = gateway.synthesize(gateway.validate(payload()), loader=lambda: Clipped())
        self.assertEqual(signals["clipped_samples"], 2)
        self.assertIn("peak_amplitude=1.5;clipped_samples=2;clipped_fraction=0.5;", response["provenance"]["validation"])


if __name__ == "__main__":
    unittest.main()
