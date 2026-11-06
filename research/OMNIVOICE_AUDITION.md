# OmniVoice first audition: number wording improves; one ending needs repair

The authorized three-item local audition completed **three first-take TTS calls and three independent ASR calls**, with no repeats. All intended words were recovered. On the exact numeric source that flagged Qwen, OmniVoice's independent transcript recovered the intended **двадцать одна свободная минута**. This is a concrete, narrow improvement worth pursuing; it does not establish an overall Russian quality winner.

The pronoun/reduction clip has a separate concern: it ends with substantial voicing under the runtime's automatically estimated fixed duration. Preserve it as a suspect ending rather than treating its exact transcript as complete acoustic approval. The working Qwen/Silero services and all existing teaching assets were unchanged. GPU/model processes were released after the calls.

## Actual outputs

| Case | Exact public source | Audio | Generation after load | Independent ASR |
| --- | --- | --- | --- | --- |
| OVA01 | `Он дома, она у окна, оно открыто, а они пьют молоко.` | 3.400 s | 3.589 s | Exact source words; 11.357 s |
| OVA02 | `Это лук, а это люк.` | 1.840 s | 1.739 s | Exact source words; 11.364 s |
| OVA03 | `У меня сегодня 2 вопроса, 5 идей и 21 свободная минута.` | 4.560 s | 2.235 s | `У меня сегодня два вопроса, пять идей и двадцать одна свободная минута.`; 9.278 s |

Measured model load was **7.792 s** before OVA01, separate from Python startup/imports and acquisition verification. OVA02/03 used the already loaded model. This MLX call emits one completed waveform, so first-playable timing equals whole-wave generation time; it is not streaming first-token latency. Generation/audio ratios were 1.056, 0.945 and 0.490. Process lifetime peak RSS was **1.946 GB**; Metal lifetime peak reached **2.681 GB**. These metrics overlap in unified memory accounting and should not be added. ASR runs began only after the TTS process exited.

All originals are 24 kHz mono PCM16; the original emitted float32 arrays are also retained. Peaks were 0.9180, 0.8164 and 0.8359, with no out-of-range float samples or full-scale PCM samples. Raw transcripts retain punctuation and spelling. Recognition received audio/hash/bindings and explicit Russian, **no expected text or hotwords**. Phonetic judgment, word timestamps and confidence were not invented.

## Conditions and provenance

The complete pinned model/codec inventory is **1,639,492,513 bytes** in `.local/omnivoice-audition/model`: `mlx-community/OmniVoice-bf16@8fb0b754cad788aaefec690cd55c207e8a628f85`. Each file's size and SHA-256 was checked; large files also matched pinned Hub LFS hashes. The observed upstream is `k2-fsa/OmniVoice@c5fdb5ccb189668d56333f77ba2629f4cd7535f4`; the exact conversion parent remains unverified. Upstream model/codec notices are retained. The model card describes noncommercial model use; this is a personal research audition, with no product/provider switch. [Upstream model](https://huggingface.co/k2-fsa/OmniVoice), [conversion](https://huggingface.co/mlx-community/OmniVoice-bf16).

The existing locked Python 3.12 / MLX-Audio **0.6.4** / MLX **0.32.4** environment was used **read-only**, with separate cache/model/output paths and offline inference. No package install or version mutation occurred. New benchmark code lives under `scripts/research/`, outside the live `scripts/speech` fingerprint. Numerical threads were limited to two, Metal allocation to 12 GiB, cache to 1 GiB; the supervisor bounded its child RSS at 16 GiB and the three-take process at 300 seconds.

Every take used language `ru`, supported attribute instruction `female`, seed **42**, **32** diffusion steps, guidance 2.0 and the installed remaining defaults. No human reference was used. This is **attribute voice design**, not voice cloning or a certified native voice. Upstream notes that voice design was trained on English/Chinese, whereas voice cloning is its more stable mode; Russian generalization therefore needs evaluation. The runtime's default `RuleDurationEstimator` chose each output length; no explicit duration override was supplied. Exact settings, runtime source hashes and the raw requests/responses are recorded. [Upstream voice-design documentation](https://github.com/k2-fsa/OmniVoice#voice-design).

## What improved, and what remains open

**Number wording:** OVA03 exactly matches the original RQ07 text, including numeric glyphs. The frozen Qwen RQ07 recording produced the independent hypothesis `двадцать один свободная минута`; OmniVoice produced `двадцать одна свободная минута`. Value 21 is unchanged, but the gender form differs. This is a useful source-matched lexical comparison, although model, voice and runtime differ. One recognizer may mishear, and no human listening verdict was supplied. The current production guard still rejects numeric glyphs; this authorized offline diagnostic did not change or bypass the live service's guard. [Retained baseline and controls](REFERENCE_QUALITY_SLOT2.md).

**Pronouns/reduction:** OVA01 recovered он/она/оно/они and the complete sentence. That establishes lexical recoverability under this recognizer, not correct stress or unstressed-о quality. Its whole-clip rate is approximately 194 words/minute, including silence; this is not an aligned articulation-rate measurement. A novice lesson may need shorter chunks even when a conversational voice is intelligible.

**Contextual softness:** OVA02 recovered both лук and люк in the intended order. The [shared-scale spectrogram and LPC inspection](artifacts/omnivoice-audition-v1/contextual-lateral.png) preserves their surrounding carriers. Resonance patterns differ through the later versus earlier word region, but precise phone boundaries were not established, and LPC pole identity is unstable in portions of the clip. This supports targeted listening/inspection, not a calibrated softness grade. The earlier Silero isolated-word pair is a useful different-context anchor, not an exact full-sentence baseline.

**Possible ending defect:** OVA01's final 20 ms has RMS **−13.6 dBFS** and peak **0.535**, versus approximately **−44.6/−45.1 dBFS** for OVA02/03. The [last-200-ms waveforms](artifacts/omnivoice-audition-v1/ending-inspection.png) show OVA01 still carrying strong periodic activity near the file boundary. Combined with its fixed estimated length, this makes a crowded/truncated final vowel a specific concern. It does not prove how much phonetic content is missing, and the final sample alone is near zero in all clips. Do not use ASR completeness to dismiss it.

The narrow next repair experiment, **not run here**, is the same OVA01 source/style/seed/steps with one explicit longer duration, for example 4.5 seconds. Retain it as a new candidate and compare complete words, repetitions and the ending against the original; do not pad silence onto the old file and call the vowel fixed. This tests the duration-setting concern without changing the model or selecting a flattering seed. No broad install, retry or voice search is warranted by this first result.

**Subsequent separate control:** the authorized [4.5-second request](OMNIVOICE_DURATION_CONTROL.md) produced a 4.52-second take, exact lexical recovery and a longer, lower-energy final periodic region. It still reaches the file boundary without clear trailing quiet, so the control is a partial improvement rather than a demonstrated truncation fix. This original three-take result set remains unchanged.

## Exact assets

| Case | WAV | SHA-256 |
| --- | --- | --- |
| OVA01 | [Pronouns/reduction](artifacts/omnivoice-audition-v1/OVA01.wav) | `043724bb0ec7ab206fa5a4769ad2e23d6cdb2f5a8e13ea3fe1d226c378618f3a` |
| OVA02 | [Contextual лук/люк](artifacts/omnivoice-audition-v1/OVA02.wav) | `989a08c41ff22b2716d09b90fc500f882f33c9caf46ccb4374c2be5beab3eb6f` |
| OVA03 | [Numeric agreement](artifacts/omnivoice-audition-v1/OVA03.wav) | `da2b9dffc21c8df4fcd98229fa0d93ae463f87a45e4c18c8ee6fa37923992048` |

Same-basename JSON retains source/synthesis hashes, actual settings, model/runtime provenance and measured resources. `*-review.json` retains full ASR evidence. [Evaluation](artifacts/omnivoice-audition-v1/evaluation.json), [measurements](artifacts/omnivoice-audition-v1/measurements.json) and [model provenance](artifacts/omnivoice-audition-v1/model-provenance.json) bind the result set. Raw requests, responses, original float arrays and stderr remain under `.local/omnivoice-audition/runs/`. Acquisition and execution scripts refuse accidental reuse of the same experiment directory; `audit_omnivoice.py` reproduces only offline analysis.

The broader [15-item plan](VOICE_QUALITY_NEXT.md) remains unexecuted. Actual Russian listening is still needed to compare naturalness, stress and vowel quality. The friend remains an optional conversation partner, not the benchmark annotator. Existing usable teaching material continues while the promising lexical result and concrete ending issue are investigated.
