# One Russian narrator for Drussky: open-weight TTS in November 2026

2026-11-07. Goal: a single excellent, consistent Russian narrator for everything authored (sentences,
passages, explain examples), natural prosody, **correct stress from our `+`-marked text**, able to say
isolated syllables and lone vowels, generated offline on the M2 Max (the app only plays files); optionally
a second voice for dialogues. Today's ~630 synthetic clips are Silero v5 "xenia" (2023-era) next to ~700
human single-word recordings.

What was done tonight: a web survey of the current open-weight field (section 1), an installed audition of
the three feasible candidates against Silero on 29 diagnostic items with the **same designed narrator voice**
cloned into every model (section 2, listening sheet `research/artifacts/voice-2026/index.html`), a 60-sentence
shadow set from the real corpus (section 3), and a provider in `apple/scripts/audio_store.py` so the switch is
one command (section 4). Nothing in `Resources/Audio` changed.

## 0. Verdict (provisional, pending ears)

**VoxCPM2 is the narrator engine; ESpeech-TTS-1 is the stress engine; both render the same designed voice.**
VoxCPM2 (Apache-2.0, 2B, 48 kHz, MLX) was the most intelligible challenger on the diagnostic set (8 word errors
of 70, tied with OmniVoice), exact on every 2–15-word corpus sentence, the only one with a clear question rise on
«Э́то чай?», ~3× real time on the M2 Max, and its designed narrator clones cleanly. Its one disqualifying gap is
that it ignores stress marks (за́мок/замо́к pairs at chance), and it is weak on isolated single words. ESpeech-TTS-1
RL-V2 (Apache-2.0, Russian-only F5-TTS) is the only model that takes our `+` marks as input and the best at lone
sounds, but it is 7× real time, copies the reference's prosody (statement = question), and drops repeated words.
OmniVoice ties VoxCPM2 on intelligibility but is CC-BY-NC and skipped whole sentences inside passages. Silero
stays the most literal and the fallback. The pipeline (section 4) supports both engines per clip with one command,
so the real decision, which the listening sheet is for, is: does VoxCPM2's naturalness beat Silero's literalness
on sentences, and is the ESpeech join acceptable for the stress material.

## 1. The field, as of 2026-11-07

Facts below are from the model cards, READMEs, licence files and issue trackers fetched today; leaderboard
rows from Artificial Analysis' open-weights TTS arena and Hugging Face's Open TTS Leaderboard (2026-09-30).
"Apple Silicon" means a documented local path (MLX port or PyTorch MPS), not "probably runs".

| Model (release) | Size | Russian | Stress control | Cloning / design | Licence | Apple Silicon | Verdict for us |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **VoxCPM2** (OpenBMB, Apr 2026) | 2B, 48 kHz | Yes, 1 of 30 named; MiniMax-RU WER 3.6 (S2 2.4, Qwen3-TTS 3.2), speaker-sim 81 (best listed) | None for Russian (phonemes only for EN/ZH); **U+0301 ignored — measured** | Clone from 5–30 s ref; voice design from a text description | **Apache-2.0** | `mlx-community/VoxCPM2-bf16` (4.96 GB) via mlx-audio 0.6.4 | **Auditioned.** |
| **OmniVoice** (k2-fsa, 2026) | 0.8B, 24 kHz | 646 languages; RU WER 2.23 (MiniMax test) | U+0301 "works to some degree, sometimes ruins pronunciation" (issue #129, open; maintainers: fine-tune on stressed data) | Clone (3–10 s ref + transcript); design (EN/ZH-trained) | **CC-BY-NC** (no commercial checkpoint; issue #258) | `mlx-community/OmniVoice-bf16`, already in `.local/omnivoice-audition` | **Auditioned** (reuses the earlier local copy). |
| **ESpeech-TTS-1 RL-V2** (ESpeech, 2025) | F5-TTS Base DiT, ~1.3B f32 ckpt (2.7 GB) | Russian-only model, trained on Russian webinars | **Native `+` before the stressed vowel** (our corpus notation); RUAccent otherwise | Clone only (ref ≤ 12 s + transcript) | **Apache-2.0** | PyTorch MPS works (f5-tts 1.1.22, torch 2.15); ~12 s per clip | **Auditioned.** |
| Fish Audio **Drama 3** (Oct 2026) | 35B MoE / 3B active | 83 languages; RU WER 2.23 vs Eleven v4 2.32 in its report | Not documented | Clone; directed delivery tags; 16 speakers | Fish Community Licence: free under US$500k revenue, weights redistributable with notice, "Built with Fish Audio" attribution | mlx-community bf16 ≈ 67 GB (74 GB peak); 4-bit ≈ 26 GB | #1 on the open-weights arena (Elo 1275). Out of tonight's 15 GB budget; the quality ceiling to audition later at 4-bit. |
| Fish **S2-Pro** / OpenAudio S1-mini | 4B / 0.5B | yes | – | clone | S2-Pro: Research licence (non-commercial); S1-mini: CC-BY-NC | S1-mini via mlx-audio | S1-mini Elo 1043 (below Kokoro); S2-Pro non-commercial. Skip. |
| **Higgs TTS 3** (Boson, Jun 2026) | 4–5B | Russian in the 85 "production-quality" tier; clone WER 2.74 MiniMax | Not documented | Zero-shot clone | Research & Non-Commercial; redistribution needs agreement | mlx-audio `higgs_audio_v3` (~10 GB) | Licence rules it out for a released app; Elo 1039. |
| **Qwen3-TTS** 1.7B (Jan 2026; no later release) | 0.6B/1.7B | Yes (10 languages); MiniMax-RU WER 3.2 | None documented | Clone (3 s), VoiceDesign, 9 presets | Apache-2.0 | mlx-community ports | Already installed in `.local/qwen3-tts`; earlier audits found a gender-agreement slip (RQ07) and no stress reversal (RQ06). Not re-auditioned per ember. |
| **Fun-CosyVoice 3.0** (Dec 2025) | 0.5B | Russian is 1 of 9 named | Pinyin/CMU phonemes only | Clone, instruct | Apache-2.0 | Community 4-bit MLX port only; official stack is CUDA | Plausible fallback; no stress path; not auditioned. |
| **Chatterbox Multilingual** (Resemble) | 0.5B | `ru` listed; issue #360: "English accent, wrong stress, soft r" after a few generations, unfixed | None | Clone | MIT | mps flag | Repo "retired research, unmaintained as of Aug 2026". Skip. |
| **KugelAudio-0-open** | 7B | ru listed; "quality varies significantly by language", preference study is German | None | No cloning (4 preset voices) | MIT | mlx-audio | 19 GB VRAM, no cloning. Skip. |
| **Kokoro 82M** v1.0 | 82M | **No** (8 languages, no Russian) | – | No | Apache-2.0 | yes | Not applicable. |
| MOSS-TTS, Voxtral TTS, Ming-3.0, Breeze TTS 2, Maya2, Dia, Spark, IndexTTS-2.5, Step-Audio EditX | – | No Russian (IndexTTS: zh/en/ja/es/ar; Voxtral: 9 languages without ru; Breeze/Ming/Spark: en/zh) | – | – | – | – | Not applicable. |
| MiniMax / ElevenLabs v4 (API) | – | yes | ElevenLabs: no Russian stress markup | clone | paid | n/a | Not local; not needed on tonight's evidence. |

Sources: [VoxCPM2 README](https://github.com/OpenBMB/VoxCPM), [VoxCPM usage guide](https://voxcpm.readthedocs.io/en/latest/usage_guide.html),
[mlx-community/VoxCPM2-bf16](https://huggingface.co/mlx-community/VoxCPM2-bf16); [OmniVoice README](https://github.com/k2-fsa/OmniVoice),
[stress issue #129](https://github.com/k2-fsa/OmniVoice/issues/129), [licence issue #258](https://github.com/k2-fsa/OmniVoice/issues/258);
[ESpeech-TTS-1_RL-V2](https://huggingface.co/ESpeech/ESpeech-TTS-1_RL-V2); [Drama 3 card](https://huggingface.co/fishaudio/drama-3),
[Fish Community Licence](https://huggingface.co/fishaudio/drama-3/blob/main/LICENSE), [Drama 3 report](https://arxiv.org/abs/2610.18010);
[Higgs TTS 3](https://huggingface.co/bosonai/higgs-tts-3-4b); [Qwen3-TTS](https://github.com/QwenLM/Qwen3-TTS); [CosyVoice](https://github.com/QwenAudio/CosyVoice);
[Chatterbox](https://github.com/resemble-ai/chatterbox), [issue #360](https://github.com/resemble-ai/chatterbox/issues/360);
[KugelAudio](https://huggingface.co/kugelaudio/kugelaudio-0-open); [Kokoro](https://huggingface.co/hexgrad/Kokoro-82M);
[mlx-audio model list](https://github.com/Blaizzy/mlx-audio/blob/main/docs/models/tts/index.md) (0.6.4, 2026-11-02);
[Artificial Analysis open-weights TTS arena](https://artificialanalysis.ai/text-to-speech/leaderboard/provider-voice/open-weights);
[Open TTS Leaderboard](https://huggingface.co/blog/open-tts-leaderboard) (names OmniVoice, S2-Pro and Fun-CosyVoice3 as the strong multilingual entries).

Two structural facts drive the choice more than any MOS number:

1. **Only ESpeech reads our stress marks.** Russian stress is lexical and often unpredictable (за́мок/замо́к);
   every general model above learns it statistically and offers no Russian phoneme path. OmniVoice's maintainers
   say so outright; VoxCPM2 was shown tonight to produce byte-identical acoustics with and without U+0301
   (section 2). ESpeech-TTS is an F5-TTS fine-tune trained *on* `+`-marked Russian text, so the corpus notation
   is its input format.
2. **Licences.** VoxCPM2 and ESpeech are Apache-2.0 (a released app may ship their audio and even the weights).
   OmniVoice and Higgs are non-commercial; Drama 3 is fine for a personal or small release but carries an
   attribution duty.

## 2. Audition

**Setup.** `research/artifacts/voice-2026/items.json`: 13 core items (lone vowels у/е/ы, syllables га/ла, minimal
pairs мыл/мил and мат/мать, за́мок/замо́к in a frame, «Э́то чай.» vs «Э́то чай?», a 25-word sentence, a
numbers sentence) plus 8 two-syllable homograph pairs as bare words (16 clips), all in the corpus `+` notation.
The narrator is **one designed voice**: VoxCPM2 voice design from an English description and a Russian prompt
sentence (three seeds; all three transcribe exactly; candidate 1 chosen, the other two are on the sheet). That
clip is the cloning reference for VoxCPM2, OmniVoice and ESpeech, so the three challengers share timbre and
differ only in engine; Silero xenia is the production control. Every clip is first-take, seed 42, no retries.
Installs: `.local/tts-2026/{mlx,espeech}/venv` (uv, Python 3.12), models under `.local/tts-2026/models/`
(VoxCPM2-bf16 4.6 GB, ESpeech 2.5 GB, Vocos 52 MB); OmniVoice reused from `.local/omnivoice-audition/model`.
Total new downloads 7.2 GB.

**Intelligibility** (Qwen3-ASR 1.7B, expected text withheld, after the app's trim/level; `audition/*/asr.json`):

| | Silero xenia | VoxCPM2 | OmniVoice | ESpeech RL-V2 |
| --- | ---: | ---: | ---: | ---: |
| word errors / 70 words, 29 clips | 9 | 8 | 8 | 12 |
| lone vowels у, е, ы heard as | о, я, эй | ммм, ю, три | ну, ну, угу | у, е, эп |
| syllable га / ла | да / пла | да / ла | г / ла | г / ла |
| мыл. мил. | мил мил | мил мил | мыл мил | мыл (one word) |
| мат. мать. | mат мать | мать (one word) | мать мать | мать (one word) |
| long sentence (25 words), numbers | exact, exact | exact, exact | exact, drops «и» | exact, exact |

Reading the errors: an ASR model cannot name a lone vowel, so the V/S rows say only that ESpeech's isolated
sounds are the most recognisable (it is the one model trained on Russian alone). "полки → палки", "орган →
арган" count as ASR word errors but are correct reduced pronunciations of полки́/орга́н. The minimal-pair
collapses (мыл/мил, мат/мать) are the ones to listen for: three of four systems lose one member of a pair on
ASR, each a different one.

**Stress** (`audition/*/acoustics.json`; method in `tools/analyze.py`): the two vowel nuclei of each
two-syllable word are the peaks of the 300–3000 Hz band energy; the longer nucleus is called stressed. On
Silero, whose `+` control is known to work, this heuristic agrees with the authored stress on 12/16 words and
sees the pair-wise duration flip on 5/8 pairs, so it is a coarse instrument (final closed syllables like -мо́к
defeat it). Read the columns relative to Silero:

| | Silero | VoxCPM2 | OmniVoice | ESpeech |
| --- | ---: | ---: | ---: | ---: |
| words with authored stress heard, of measurable | 12/16 | 8/16 | 11/18 | 9/13 |
| pairs where the mark flips the duration ratio | 5/8 | 2/8 | 3/9 | 3/5 |
| marks change the waveform at all? | yes (by design) | yes (text tokens differ) but not the stress: `за́мок`/`замо́к` and unmarked `замок` are three near-identical readings (corr 0.99) | partly | yes (by design) |

VoxCPM2 at 2/8 and OmniVoice at 3/9 are chance: neither has a Russian stress control, as their maintainers
say. ESpeech honours the marks but produced only 5 measurable pairs because its bare-word clips are often cut
short (F5-TTS sizes the output from the reference-length ratio; "мука" came out as "м"). The honest summary:
**only ESpeech can place за́мок/замо́к on demand; on everything else the two newer models read with the stress
they learnt**, which for ordinary words is usually right and for homographs is a coin toss.

**Question intonation** («Э́то чай.» vs «Э́то чай?», pyin F0): VoxCPM2 raises the final third from 160 to
185 Hz for the question (a clear rise on чай); OmniVoice 162 → 171 Hz; Silero's question is a flatter, slightly
higher contour (range 1.9 st vs 6.0 st for the statement); **ESpeech's statement and question are identical**
(final third 164.4 vs 164.5 Hz, same range), the flow-matching clone copies the reference's prosody and the
question mark changes nothing. For teaching IK-3 that is a real limitation.

**Speed and footprint** (M2 Max, 96 GB): VoxCPM2 loads in ~2 s, ~2 s per short clip, 19 s for a 9 s sentence
(RTF ≈ 2.1), 48 kHz; OmniVoice loads in 2.5 s, 3.5–5 s per short clip but 6.4 s for the same 9 s sentence
(RTF 0.7), 24 kHz; ESpeech loads in 2.4 s and takes 10–13 s per clip regardless of length on MPS at 48 NFE
steps, 24 kHz; Silero 0.1–1 s per clip on CPU. Re-voicing the 683 authored clips: VoxCPM2 ≈ 40 min,
OmniVoice ≈ 45 min, ESpeech ≈ 2.3 h.

**Listening sheet**: `research/artifacts/voice-2026/index.html` (open the file from the checkout). Rows are
shuffled per item and labelled A–D; "reveal" shows the engine, the ASR transcript, the stress measurement and
the pitch numbers. Generation JSON next to every WAV records the exact input, notation, settings, seed, timings
and peak memory.

## 3. Shadow set

`research/artifacts/voice-2026/shadow/shadow.json`: 60 of the 627 xenia inputs, chosen evenly across length
buckets (8 single words, 18 of 2–3 words, 22 of 4–8, all 7 of 9–15, 5 of the 12 passages of 16+ words; 366 words),
rendered by all four systems with the same narrator reference (`shadow/<engine>/S01..S60.wav` + JSON, first take,
seed 42). These are raw engine outputs, not yet trimmed/levelled/AAC; the pipeline version of the same list is one
`revoice --dest` command (section 4). Not in `Resources/Audio`.

| | Silero xenia | VoxCPM2 | OmniVoice | ESpeech |
| --- | ---: | ---: | ---: | ---: |
| ASR word errors / 349 | 8 (2.3%) | 18 (5.2%) | 20 (5.7%) | 15 (4.3%) |
| clips with any error / 60 | 3 | 8 | 9 | 10 |
| generation time for 171–176 s of audio | 84 s (RTF 0.5, CPU) | 508 s (RTF 3.0) | 253 s (RTF 1.4) | 1215 s (RTF 6.9) |
| failure mode | «умом» → «у мам» (also VoxCPM2: likely the recogniser) | **single-word clips**: бил→бейл, едем→«мадина», съел→сью; the guide warns short inputs are weak | **drops whole clauses in passages** (S56, S59, S60 lose a sentence each; S25 loses «я») | **drops repeated words** («а где кот? кот тут.» → «а где кот»; «где хлеб? он тут» → «где хлеб»); S55 repeats a number |

On connected speech of 2–15 words all three challengers are essentially exact. The characteristic failures matter
more than the totals: OmniVoice skipping a sentence inside a passage is the kind of error nobody notices until a
learner reads along; ESpeech collapsing «кот? кот тут» is fatal for the question-and-answer micro-dialogues the
course is built on; VoxCPM2's weakness is isolated words, which is where the human recordings already do most of
the work (and where a sentence frame or the ESpeech engine can step in). Silero remains the most *literal* system;
the challengers are more natural. That trade is what the sheet is for.

## 4. Pipeline

`apple/scripts/audio_store.py` gained a `Narrator(engine)` provider next to `Silero`. It runs
`apple/scripts/narrator_worker.py` inside the engine's own venv as a subprocess (JSON lines on stdin/stdout, model
loaded once), so `audio.py` keeps working from any Python. Engines: `voxcpm2` (MLX) and `espeech` (PyTorch MPS);
both clone `research/artifacts/voice-2026/narrator/narrator-voxcpm2-1.wav` (in git, 1 MB) with its `+`-marked
transcript, so the timbre is one voice whichever engine renders a clip. Inputs stay in the corpus `+` notation
(the worker converts to U+0301 for VoxCPM2; ESpeech takes `+` natively); inputs over 160 characters are split at
sentence ends with 0.35 s pauses, as for Silero; loudness levelling and AAC encoding are the unchanged
`audio_store.encode`. Each clip's sources entry records `input`, `voice = narrator-2026-1`, `synth = <engine
settings string>`, `orig` (`.local/tts-2026/originals/<engine>/…wav`), `orig_sha1`, `pipeline`, so
`audio.py restore` regenerates a missing clip with the same engine (`audio_store.narrator_engine(synth)` picks it).
Silero stays the fallback for every entry that still names it.

Commands (from the repo root; the venvs are `.local/tts-2026/mlx/venv` and `.local/tts-2026/espeech/venv`,
built by `.local/tts-2026/setup-mlx.sh` / `setup-espeech.sh`):

    # shadow set: 60 corpus sentences through the real pipeline, nothing in Resources/Audio or the manifests touched
    python3 apple/scripts/audio.py revoice --engine voxcpm2 --list research/artifacts/voice-2026/shadow/shadow.json \
        --dest research/artifacts/voice-2026/shadow/pipeline-voxcpm2

    # THE switch: re-render all 683 authored clips (627 sentences/passages/words + 56 xenia pair clips) and
    # rewrite their sources*.json entries; `audio.py verify` afterwards
    python3 apple/scripts/audio.py revoice --engine voxcpm2        # or --engine espeech

    python3 apple/scripts/audio.py revoice --engine espeech --only eto-zamok-….m4a   # one clip, e.g. a homograph

Tested tonight: `revoice --dest` with both engines on three clips (a two-sentence question/answer, a bare word, a
six-line passage), then `status --plan`, `restore` from the cached original and `restore` by re-synthesis against
an empty cache root with a manifest holding one narrator entry, and `verify`. The pipeline shadow set for the
winner is in `research/artifacts/voice-2026/shadow/pipeline-voxcpm2/` (levelled .m4a + `sources-revoice.json`).

`revoice` only touches entries whose `voice` is `xenia` or `narrator-2026-1`; the aidar/baya pair variants
(deliberately other speakers), human recordings and the one macOS clip are left alone. Nothing was re-voiced
tonight: the shadow sets are in `research/artifacts/voice-2026/shadow/`.

**Mixed rendering is possible and may be the right answer**: because every engine clones the same reference,
passages and sentences can come from VoxCPM2 (prosody, questions, speed, 48 kHz) while the clips whose point is
stress (homograph pairs, за́мок/замо́к, stress-contrast words) come from ESpeech with `--only`; the voice stays
one voice. Whether the join is audible is a listening question for the sheet.

**Licences.** VoxCPM2 and ESpeech-TTS-1: Apache-2.0 (generated audio and even the weights may ship). OmniVoice:
CC-BY-NC, no commercial checkpoint exists (not wired into the pipeline for that reason). Silero v5 (today's
voice): CC-BY-NC-SA 4.0, the same problem the current corpus already carries. Fish Drama 3: Community Licence
(free under US$500k revenue, "Built with Fish Audio" attribution). The narrator reference itself is a VoxCPM2
voice-design output, not a person's recording, so there is no cloned human.

**Open items.** (1) ESpeech truncated very short inputs at speed 1.0; at speed 0.7 (`audition-speed07/espeech`)
«мука» and «атлас» come back whole, so the worker uses 0.7 for inputs under 12 letters. The two-word pairs
(«мыл. мил.») still collapse to one word at either speed: ESpeech should not render repeated or paired words.
(2) The stress heuristic is coarse; a forced aligner (or ten minutes of ember's ears on the 16
pair clips) would settle ESpeech's stress accuracy properly. (3) Fish Drama 3 at 4-bit (26 GB) is the remaining
quality-ceiling audition; it needs its own evening and has no stress control either. (4) A second voice for
dialogues is one more `design` run (`tools/gen_mlx.py --model voxcpm2 design` with a different instruct) and a
second reference constant.
