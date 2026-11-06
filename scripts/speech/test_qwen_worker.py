#!/usr/bin/env python3
"""Fast contract tests use a fake engine, never load weights or execute inference."""
import base64
import hashlib
import json
import threading
import time
import unittest
import urllib.error
import urllib.request

import sys
sys.dont_write_bytecode = True

from qwen_model import MAX_REQUEST_BYTES, MODEL_ID, MODEL_REVISION
from qwen_worker import Engine, Server, canonical_wav, validate
from qwen_client import verify_response
from qwen_frontend import synthesis_input, UnsupportedStress

TOKEN = "public-fake-worker-test-token-0123456789"


class FakeEngine:
    model = object()
    prepared = True
    last_stats = None
    delay = 0
    calls = 0

    def generate(self, request, preparing=False):
        self.calls += 1
        self.model = object()
        self.prepared = True
        time.sleep(self.delay)
        wav, frames, _, _ = canonical_wav([0.1, -0.1] * 12)
        self.last_stats = {"elapsed_seconds": 0.01}
        return {"request_id": request["request_id"], "revision_id": request["revision_id"], "source_text": request["text"],
                "sample_rate": 24000, "channels": 1, "frames": frames, "wav_base64": base64.b64encode(wav).decode(),
                "sha256": hashlib.sha256(wav).hexdigest(),
                "provenance": {"provider": "local_qwen3_tts", "kind": "ai_generated_speech", "model": f"{MODEL_ID}@{MODEL_REVISION}","synthesis_input":synthesis_input(request["text"])}}


class FrontendContracts(unittest.TestCase):
    def test_worker_explicit_model_input_preserves_source_and_rejects_stress_contrast(self):
        for text in ["Это за́мок.", "Это замо́к.", "Это долгая фраза про старый за́мок, который стоит на холме."]:
            with self.assertRaises(UnsupportedStress):
                synthesis_input(text)
        source="Сего́дня я изуча́ю ру́сский язы́к и чита́ю интере́сную кни́гу о звёздах."
        actual=synthesis_input(source)
        self.assertEqual(actual["synthesis_text"], "Сегодня я изучаю русский язык и читаю интересную книгу о звёздах.")
        self.assertEqual(actual["source_text_sha256"],hashlib.sha256(source.encode()).hexdigest())
        self.assertEqual(actual["context"], "contextual_utterance")


class OwnerThreadContracts(unittest.TestCase):
    def test_worker_model_jobs_keep_one_owner_thread_and_failed_state(self):
        observed = []
        class Owner(Engine):
            def _generate(self, request, deadline):
                observed.append(threading.get_ident())
                if request.get("fail"):
                    raise ValueError("Public injected failure")
                self.model = object(); self.prepared = True
                return {"deadline": deadline}
        engine = Owner()
        self.assertEqual(engine.generate({}, preparing=True), {"deadline": 120})
        self.assertEqual(engine.generate({}), {"deadline": 50})
        self.assertEqual(len(set(observed)), 1)
        self.assertNotEqual(observed[0], threading.get_ident())
        with self.assertRaises(ValueError):
            engine.generate({"fail": True})
        self.assertEqual(engine.phase, "failed"); self.assertFalse(engine.prepared)
        with self.assertRaises(RuntimeError):
            engine.generate({})


class WorkerContracts(unittest.TestCase):
    def setUp(self):
        self.engine = FakeEngine()
        self.server = Server(("127.0.0.1", 0), self.engine, TOKEN)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.base = f"http://127.0.0.1:{self.server.server_port}"
        self.request = {"text": "Привет!", "request_id": "public-fixture", "revision_id": "revision-2"}

    def tearDown(self):
        self.server.shutdown(); self.server.server_close(); self.thread.join()

    def call(self, path="/generate", data=None, token=TOKEN, headers=None):
        body = json.dumps(self.request if data is None else data).encode()
        request = urllib.request.Request(self.base + path, data=body if path != "/status" else None,
                                         headers={"Authorization": "Bearer " + token, "Content-Type": "application/json", **(headers or {})})
        try:
            with urllib.request.urlopen(request, timeout=2) as response:
                return response.status, json.load(response)
        except urllib.error.HTTPError as error:
            return error.code, json.load(error)

    def test_worker_auth_before_body_decode_and_status_does_not_load(self):
        self.engine.model = None; self.engine.prepared = False
        status, _ = self.call(data="invalid", token="wrong")
        self.assertEqual(status, 401)
        status, result = self.call(path="/status")
        self.assertEqual(status, 200); self.assertFalse(result["loaded"]); self.assertEqual(self.engine.calls, 0)

    def test_worker_cold_generation_requires_explicit_operator_prepare(self):
        self.engine.model = None; self.engine.prepared = False
        self.assertEqual(self.call()[0], 503)
        self.assertEqual(self.engine.calls, 0)
        self.assertEqual(self.call(path="/prepare")[0], 200)
        self.assertTrue(self.call(path="/status")[1]["enabled"])
        self.assertEqual(self.call()[0], 200)

    def test_worker_binds_text_and_hash_to_canonical_wav(self):
        status, value = self.call()
        self.assertEqual(status, 200); self.assertEqual(len(verify_response(self.request, value)), 92)
        value["source_text"] = "Другая фраза"
        with self.assertRaises(ValueError):
            verify_response(self.request, value)

    def test_worker_request_size_and_structured_input_limits(self):
        for data in [{**self.request, "text": "я" * 257}, {**self.request, "request_id": "bad/path"},
                     {**self.request, "provider": "arbitrary"}, {**self.request, "text": "[[rate 900]]Привет"},
                     {**self.request, "text": "<laugh>Привет"}, {**self.request, "text": "abc\x7f"}]:
            self.assertEqual(self.call(data=data)[0], 400)
        self.assertEqual(self.call(data={**self.request, "text": "x" * MAX_REQUEST_BYTES})[0], 413)
        self.assertEqual(self.engine.calls, 0)

    def test_worker_rejects_marked_short_input_before_inference(self):
        status,_=self.call(data={**self.request,"text":"Это за́мок."})
        self.assertEqual(status,422);self.assertEqual(self.engine.calls,0)

    def test_worker_one_generation_no_queue(self):
        self.engine.delay = 0.2
        result = []
        thread = threading.Thread(target=lambda: result.append(self.call()))
        thread.start()
        for _ in range(100):
            if self.engine.calls:
                break
            time.sleep(0.001)
        self.assertEqual(self.call()[0], 429)
        thread.join(); self.assertEqual(result[0][0], 200)

    def test_worker_audio_reports_clipping_instead_of_hiding_it(self):
        wav, frames, peak, clipped = canonical_wav([1.2, -0.8])
        self.assertEqual((frames, peak, clipped), (2, 1.2, 1))

    def test_worker_audio_validates_finite_silence_and_duration(self):
        for audio in [[], [0.0] * 100, [float("nan")], [float("inf")]]:
            with self.assertRaises(ValueError):
                canonical_wav(audio)


if __name__ == "__main__":
    unittest.main()
