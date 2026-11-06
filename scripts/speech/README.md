# Pinned local Russian Qwen3 worker

This is a parallel local provider prepared while the initial iPad delivery uses the existing gateway. The Rust client is now integrated behind explicit `RUFREN_SPEECH_PROVIDER=qwen` in source and focused tests; the main private gateway now uses the accepted `local_russian` router after coordinated activation and HTTPS status checks. Milena remains explicitly selectable as a baseline. Qwen passed a disposable generated-reference UI call, including cached provenance/hash, playback, reload and offline replay. No app build is changed by starting these scripts.

The approved project-owned installation lives under `.local/qwen3-tts/`: Python 3.12 virtual environment, isolated uv/Hugging Face caches, pinned BF16 model, private worker token, PID, logs and generated samples. The 4,520,194,992-byte MLX artifact is pinned to `7d3824abff87e49756bb0f83fb5411de75d160c4`, with upstream model/revision recorded in its manifest. Setup verifies all downloaded file sizes and SHA-256 hashes, including published LFS IDs. Weights use Apache-2.0; MLX-Audio uses MIT. `qwen-requirements.lock` records the installed runtime dependencies.

Explicit setup, only when a model installation is intended:

```sh
UV_CACHE_DIR="$PWD/.local/qwen3-tts/uv-cache" uv venv --python /opt/homebrew/bin/python3.12 .local/qwen3-tts/venv
UV_CACHE_DIR="$PWD/.local/qwen3-tts/uv-cache" uv pip sync --python .local/qwen3-tts/venv/bin/python scripts/speech/qwen-requirements.lock
.local/qwen3-tts/venv/bin/python scripts/speech/setup-qwen.py
```

Start/status/stop never generate speech:

```sh
.local/qwen3-tts/venv/bin/python scripts/speech/qwen_service.py start
.local/qwen3-tts/venv/bin/python scripts/speech/qwen_service.py status
.local/qwen3-tts/venv/bin/python scripts/speech/qwen_service.py stop
```

The worker binds only `127.0.0.1:8790` by default. Both `/status` and `/generate` require the task-owned token; it is never printed. Numerical imports/model loading are deferred to explicit operator preparation. Before preparation succeeds, status is disabled and ordinary `/generate` requests fail without loading the model. The launcher records its exact child PID and append-only log. Stop verifies that PID's command immediately before signaling it; it never searches or kills by process name. It does not manage another preview/relay/process.

Operator preparation is a separately scheduled compute operation. The explicit `qwen_client.py prepare` command uses a fixed public phrase, produces no new sample file, and enables readiness only after a structurally valid generation. It may be repeated after an intentional worker restart without overwriting audit artifacts.

Generation is a separately scheduled compute operation. During a shared iOS build, wait for the build owner's explicit slot. The public fixture client is:

```sh
.local/qwen3-tts/venv/bin/python scripts/speech/qwen_client.py public-fixture
```

It writes `.local/qwen3-tts/samples/public-russian-v1.wav` exclusively, with source/provenance JSON beside it. It does not overwrite an earlier sample. The Russian text covers natural questions, ё, explicit combining-acute за́мок/замо́к contrasts and a short mathematical statement. Actual timing/RSS/Metal-peak and source-peak/clipping metrics appear in the worker log and returned status. This is a useful listening sample, not proof that the voice is natural or that its stress is correct.

The request `{text, request_id, revision_id?}` stays aligned with `SpeechRequest`; output uses unchanged `SpeechResponse` fields. Provenance is `ai_generated_speech` / `local_qwen3_tts`, the exact model snapshot, and a versioned designed voice. No human reference audio or voice cloning is used. SHA-256 verification runs during explicit setup; runtime startup checks the pinned manifest identity and each file’s size, not a fresh full multi-gigabyte hash pass. Runtime loading is offline and cannot download another model. Fixed Russian VoiceDesign settings and seed 42 produce 24 kHz mono PCM, encoded as canonical 16-bit WAV with exact-byte SHA-256 and exact request/text/revision bindings.

Bounds: 1,024 UTF-8 bytes / 256 characters of text; 8 KiB JSON request; 4 MiB JSON response; 45 seconds of generated audio; one generation, no queue; four HTTP handlers; two-thread numerical-library environment caps and one persistent MLX inference-owner thread; 16 GiB Metal memory ceiling. Operator `/prepare` checks a 120-second deadline between chunks and has a 150-second process watchdog; its client allows 145 seconds. Ordinary `/generate` requires readiness, checks a 50-second deadline inside the inference owner, has a 53-second process watchdog and a 55-second client limit, fitting the existing 60-second relay. A timeout does not release admission while a native job continues: the HTTP thread waits on the actual owner future, and the watchdog stays armed until it finishes or exits the owned worker. The client rejects redirects/proxies. No API token billing is involved.

Fast tests avoid model loading/inference:

```sh
PYTHONDONTWRITEBYTECODE=1 .local/qwen3-tts/venv/bin/python -m unittest discover -s scripts/speech -p test_qwen_worker.py
```

The tests check authentication before decoding, idle status, exact WAV/hash/text binding, request limits, unqueued concurrency and invalid/silent audio. The focused Qwen Rust client/provenance checks and no-server WASM build passed; the later hybrid route/subprocess/provenance tests, server-disabled WASM check and fresh relay build also passed in their serialized slot. Compare speech errors, stress and naturalness against other references before treating Qwen as a quality improvement. Keep original mathematical notation separate from a deliberately written Russian spoken rendering.

Observed public-fixture evidence on this M2 Max: cold preparation 62.137 seconds; one approved warm repeat 19.973 seconds; both generated the exact same 13.36-second WAV (SHA-256 `9b4f568b66665996c75b840fbd09e843e98f29d0addd3f2ac7f18309437bacf0`). Lifetime process RSS peak was 4.79 GB and Metal peak 5.14 GB. Emitted peak amplitude was 0.806 with no full-scale or additional conversion clipping. The upstream streaming decoder already clips its emitted waveform, so these measurements do not reveal pre-decoder latent peaks. This is structural, latency and memory evidence; lexical/prosodic quality remains a separate audit.

The scoped frontend control produced `public-russian-unaccented-v1.wav` by removing only U+0301 combining acute marks, preserving `Всё`, punctuation, seed and voice settings. It is 15.84 seconds, generated in 9.508 seconds, SHA-256 `1bf0293b6f296f8e2a9cf4064a6cd26bb989bb86cecbd66cdfd17d5865988fb4`. The independent BF16 ASR screen then recovered both `замок` tokens rather than the accented sample’s `самоек`/`заямок` anomalies. This supports ordinary spelling as Qwen input for this case; it does not verify the intended homograph stresses, ё sound, naturalness or superiority. Both samples remain audit artifacts, not certified stress references. Operator `prepare` uses the ordinary-spelling public text. Application integration should send the ordinary `russian` field while preserving stressed display separately; it must not silently erase an isolated contrast’s intended stress or falsely claim that ASR verified it.

The source-integrated Qwen worker contract is `rufren-qwen-worker-v2`. The gateway reads its private token from server-only `RUFREN_QWEN_WORKER_TOKEN_FILE` (default `.local/qwen3-tts/worker.token`) and a loopback-only port from `RUFREN_QWEN_WORKER_PORT` (default 8790). `SpeechService::checked_status()` verifies readiness without loading or generating; synchronous `status()` remains configuration-only and pessimistic for Qwen; use `checked_status()` for live readiness. The Rust client rejects incompatible worker versions; the Python worker emits this fixed version and both sides enforce exact actual-input metadata.

For Qwen, `source_text` remains the exact authored input. Optional `provenance.synthesis_input` records the original SHA-256, actual `synthesis_text`, its SHA-256, declared transform and input category. Ordinary spelling passes verbatim. Combining acute marks are removed only by the versioned conservative contextual policy: at least eight Russian word tokens and a sentence ending, valid post-vowel marks, no known stress homographs or repeated base forms with different marked positions. Marked short phrases and contrasts, including both authored «Это за́мок.» / «Это замо́к.» references, fail actionably before inference. The input category is a heuristic, not proof of disambiguation or acoustic accuracy. The shared WASM-capable `SpeechSynthesisInput::validate_for(source_text)` validates those exact operations and also the explicit-stress Silero acute-to-before-vowel-plus transform; it does not certify either voice's pronunciation.


### Declared local Russian routing

Select `RUFREN_SPEECH_PROVIDER=local_russian` only after the pinned Silero install is present and the Qwen worker is prepared. The server uses Silero for authored acute stress, Qwen verbatim for ordinary full-utterance shape (eight Russian words plus sentence ending), and Silero automatic frontend for other short ordinary text. Short unmarked known homographs ask for stress/context before inference. Selected-engine failures never fall back. The fixed Silero worker command is `.local/silero-tts/venv/bin/python -B scripts/speech/silero_gateway.py`; its exact private JSON protocol is called by the Rust adapter, not by the browser. `--check` verifies the installed model SHA without Torch loading or inference. CPU threads are capped at two, CPU time at 30 seconds and the Rust operation deadline at 35 seconds; stdout/stderr are bounded.

A separate disposable QA directory can be started only against a freshly built relay, without touching the working service:

```sh
PYTHONDONTWRITEBYTECODE=1 .local/qwen3-tts/venv/bin/python -B scripts/speech/qwen_gateway_qa.py start --provider local_russian --web-port 8091
PYTHONDONTWRITEBYTECODE=1 .local/qwen3-tts/venv/bin/python -B scripts/speech/qwen_gateway_qa.py reject-ambiguous --provider local_russian
PYTHONDONTWRITEBYTECODE=1 .local/qwen3-tts/venv/bin/python -B scripts/speech/qwen_gateway_qa.py stop --provider local_russian
```

These commands start/check/stop the declared provider and reject an ambiguous request; they do not generate a sample. Real generation remains a separately coordinated compute action. The model's CC-BY-NC-SA-4.0 terms apply to the personal-use Silero path. Do not treat structural success, distinct audio hashes or an ASR transcript as acoustic stress verification.

Hybrid acceptance completed one public marked request through the isolated gateway in 4.059 seconds, with exact source/actual-input metadata, canonical WAV/hash and zero conversion clipping; its ambiguous unmarked counterpart returned HTTP 400 before consuming allowance. The sample and sidecar are `.local/silero-tts/samples/hybrid-gateway-castle-v1.wav` and `.json`. No acoustic correctness is inferred from that integration result. Root accepted this result and the storage owner activated `local_russian` on the main private relay. A separate authorized automatic «Здравствуйте!» request also passed in 3.831 seconds with all five automatic flags true, exact verbatim input hashes and zero conversion clipping; its artifact is `.local/silero-tts/samples/hybrid-gateway-greeting-v1.wav` plus sidecar. Lexical/acoustic correctness remains unassessed. The owned isolated QA relay has been stopped, retaining its logs and samples; the main relay and warm Qwen worker remain running.
