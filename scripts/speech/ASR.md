# Local Russian transcription worker

`asr_worker.py` is the local CLI boundary now connected to the speech-teacher gateway and app. It accepts one JSON request on stdin and returns one JSON response on stdout. The worker itself performs no HTTP serving or runtime model download. It uses its own `.local/qwen3-asr` environment/cache, leaving the TTS worker's versions unchanged. Root accepted the complete app journey through transcription, explicit learner confirmation and Claude language feedback; core/sync v3 keeps the ASR observation and confirmation separate.

Set up explicitly from the repository root:

```sh
uv venv --python /opt/homebrew/opt/python@3.12/bin/python3.12 .local/qwen3-asr/venv
uv pip install --python .local/qwen3-asr/venv/bin/python --cache-dir .local/qwen3-asr/uv-cache -r scripts/speech/asr-requirements.lock
.local/qwen3-asr/venv/bin/python scripts/speech/setup-asr.py
```

This downloads the pinned 4.08 GB BF16 model. Setup verifies every file's size and SHA-256, checking published LFS object hashes when supplied. `asr_model.py` pins the MLX conversion and records an observed upstream revision. The card identifies the upstream model and conversion by mlx-audio 0.3.1, but does not establish which upstream commit was converted; the manifest keeps that distinction explicit. The runtime used here is locked to 0.6.4. Upstream-equivalent numerical behavior has not been demonstrated.

Request fields:

```json
{
  "request_id": "request-1",
  "asset_id": "recording-1",
  "attempt_id": "optional-exact-attempt-id",
  "revision_id": "optional-exact-revision-id",
  "language": "ru",
  "audio_sha256": "64 lowercase hexadecimal characters hashing exact WAV bytes",
  "wav_base64": "base64 of the exact original WAV",
  "span": {"start_frame": 0, "end_frame": 320640}
}
```

`attempt_id` and `revision_id` may be omitted/null for a public fixture. `span` may be omitted for the full recording. The supported capture formats are RIFF/WAVE PCM16 and IEEE float32, one/two channels, 8–48 kHz, at most 45 seconds. WAVE_FORMAT_EXTENSIBLE, compressed WAV, RF64, other PCM bit depths and rates are rejected explicitly. Float samples must be finite and in [-1,1]. This subset needs deliberate conversion at the future gateway for other `rufren-audio` formats; do not silently relabel a converted WAV as the original asset.

Spans use half-open **original-frame** coordinates and must be contained in the exact hashed file. The worker does not receive the lesson text, stress target, hotwords or intended answer. Russian is explicitly selected, not independently detected. It averages channels, preserves amplitude, and uses SciPy's polyphase resampler to produce 16 kHz mono; the derivative records its method/version, hash and frame count. The source PCM hash is interleaved little-endian float32 without a metadata header and is **not** asserted to equal `rufren-audio`'s canonical identity.

Run one operation:

```sh
.local/qwen3-asr/venv/bin/python scripts/speech/asr_worker.py < request.json > response.json
```

The response binds the request/asset/attempt/revision, exact source WAV hash and span; it includes literal `transcript`, `evidence_kind: transcript_hypothesis`, model/runtime/worker/manifest provenance, derivative metadata and processing timestamps. `word_timestamps`, `confidence` and `acoustic_assessment` are null. No aligner is installed by this worker. Its span identifies the audio supplied, not the start/end of recognized words. Timestamps must not be synthesized from the transcript.

`asr_review.plan_next_action(expected_text, evidence, mode=..., replacement_candidates_used=...)` is a pure post-recognition helper. It computes token differences after removing pedagogical acute accents while preserving `й` and `ё`. In `generated_reference` mode a mismatch requests at most one alternative candidate; after that it returns replay/meaning-confirmation/retry. A difference involving only `ё/е` is flagged as spelling ambiguity and does not automatically regenerate: ASR orthography can omit the dots without implying a sound error. In `learner_attempt` mode a mismatch is a listening hypothesis and asks for confirmation/retry. The caller must persist/increment the replacement count when actually requesting a candidate. This helper does not generate replacements or grade pronunciation. Replay uses the supplied original-frame crop, explicitly not word alignment. The gateway can connect these actions without waiting for a phonetic classifier.

The supervisor allows one local ASR process at a time with a file lock, uses a 120-second hard operation deadline, monitors its own child's RSS against 16 GiB, and limits Metal allocation to 12 GiB and its cache to 1 GiB. Numerical/tokenizer thread settings are two. The child has a 1,024 generated-token bound. SIGTERM/cancellation terminates its owned child process group. It verifies model hashes before loading and sets Hugging Face/Transformers offline flags. Errors are nonzero exits on stderr; there is no partial successful response. The future gateway should bound request transmission and retain its own cancellation/deadline as well.

CPU-only boundary checks:

```sh
.local/qwen3-asr/venv/bin/python scripts/speech/test_asr_worker.py
```

These checks cover hash mismatch, wrong language, prohibited target-text fields, invalid crop coordinates, float capture validation, truncated/duplicate WAV chunks and inconsistent byte metadata. They do not evaluate speech recognition. Actual public-sample results belong in the sample audit, not in an asserted pronunciation score. Read [the speech-teacher design](../../research/SPEECH_TEACHER.md) before attaching this transcript to Claude feedback: unconfirmed ASR text is a hypothesis and cannot justify a definite grammar or sound correction.

First actual Mac run (2026-11-05 local date): the 13.36-second Qwen-generated public sample completed in 11.737 seconds including model verification/loading, with 4,355,702,784 bytes peak RSS and 5,191,502,392 bytes peak Metal allocation. The independently produced transcript recovered most text but rendered the intended lock/castle words as `самоек` and `заямок`, and rendered `Всё` as `Все`. This flags the sample for targeted listening and a controlled stress-mark comparison; it cannot distinguish TTS error from ASR error. Exact evidence is in [the public audit](../../research/fixtures/qwen_public_russian_v1.audit.json). No word alignment or phonetic grading was run.

The approved control changed only the TTS input's combining acute marks: the new 15.84-second sample decoded in 7.783 seconds, recovering both `замок` tokens. `Всё/Все` ambiguity remained. [The control audit](../../research/fixtures/qwen_public_russian_unaccented_v1.audit.json) retains exact source/hash/provenance and the second transcript. This is evidence about lexical recovery in two generated samples, not verified Russian stress or a novice-recognition benchmark. Both one-shot processes exited; there is no resident ASR allocation.
