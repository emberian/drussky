# Pronunciation engine

`PhoneticAnalyzer` (the `PronunciationAnalyzer` of `PronunciationTypes.swift`) says which sound
came out wrong and how, and where the stress fell. Everything runs on device.

```
take ──trim──► Fbank (lhotse-exact) ──► ZIPA CTC (Core ML, CPU) ──► posteriors, 50 frames/s
target ──► RussianG2P ──► expected sounds + syllables + "underlying" spelling
                     └──► PhoneSet.options: correct spellings + named mistakes per sound
Scorer: GOP margin per sound = local CTC log-likelihood of the expected sequence
        − best alternative with that one sound replaced (each named mistake) or deleted
Align (feature-weighted) free decode vs expected ──► extra sounds
Prosody: loudness peak × voiced length × pitch per syllable + vowel fullness ──► stress
Tips: ≤ 2, each with a pairs.json contrast
```

Get the model: `python3 apple/scripts/models.py fetch` (downloads 260 MB once, converts in a uv
venv under `.local/pronunciation/`, installs the 70 MB compiled model into the gitignored
`Resources/Models/`; in a worktree add `--cache-root <main checkout>/.local`). Without it the
analyzer compares words from `SpeechCheck.transcribe` sound by sound (`WordFallback`): same
report shape, coarser evidence, no timing or prosody.

## The recognizer: why ZIPA small

Measured on the bundled recordings (greedy decodes printed side by side, minimal pairs first):

| model | params | what it did with Russian |
|---|---|---|
| facebook/wav2vec2-xlsr-53-espeak-cv-ft | 300 M | ы almost never ([mu] for мы, [bʉt] быть, [naɪt] ныть); palatalisation rare and erratic (мать → [mɑt]) |
| lisakorolkova/ru-phoneme-mdd-v2 (xlsr fine-tune) | 300 M | clean ы ([y]) and soft consonants, but never [o]: every о comes out [a] (код [kad]), so reduction and stress-by-vowel are invisible; no licence |
| **anyspeech/zipa-small-crctc-ns-700k** | **64 M** | ы right (быть [bɨts], мыло [mɨla], шить [ʂɨts]), unstressed о reduced and stressed о full (за́мок [zɑmɑk] vs замо́к [zɑmok], до́ма/дома́), soft л as [ʎ]/[lʲ], hard as [l̴] |

ZIPA (Zhu et al. 2025, *ZIPA: a family of efficient models for multilingual phone recognition*,
trained on 17k h of IPA-transcribed speech in 88 languages) is the smallest and the most useful.
Licence CC BY-NC-SA 4.0 — fine for this personal app, not for a distributed one.

Conversion (`scripts/models.py`, `scripts/onnx_torch.py`): coremltools no longer reads ONNX, and
running the upstream icefall Python was not an option, so a small ONNX→PyTorch interpreter
folds every shape/`If` computation at a fixed 10 s window and traces the rest; the padding mask
stays a runtime function of the `length` input. Checked against ONNX Runtime fp32 on 90 bundled
clips: masked-padded decode = unpadded decode within 1% of symbols.

| | size | symbols differing from fp32 ONNX | 10 s window (M2 Max) |
|---|---|---|---|
| fp16 | 129 MB | 0.75 % (CPU), 0.86 % (GPU), **27 %** (Neural Engine) | 44 ms CPU, 14 ms GPU |
| **w8 per-channel (shipped)** | **70 MB** | 0.97 % | similar |

The Neural Engine's float16 overflows inside Zipformer (ZIPA's own fp16 export needed fixes),
and the GPU path once aborted the process compiling its MPS graph, so the model runs on the CPU:
whole analysis (features, inference, ~300 small CTC passes, prosody) median **112 ms**, p90
250 ms per take on an M2 Max; an M4 is faster. Takes longer than 10 s are analysed in windows.

## G2P (`RussianG2P`)

Moscow-standard, from a stress-marked target (`́` or ё; monosyllables are stressed; clitics
не, в, на, и… are not): reduction by distance from the stress (о/а → [ɐ] first pretonic and
word-initially, [ə] elsewhere; е/я/а after soft → [ɪ], final я → [ə]; е after ж ш ц → [ɨ]),
palatalisation before е ё и ю я ь, always-hard ж ш ц and always-soft ч щ й, iotation, final
devoicing and regressive voicing assimilation (proclitics join the next word, в is not a
trigger), and the usual exceptions: что, чтобы, конечно, -ого/-его, сегодня, -тся/-ться,
сч/зч → щ, здн, стн, стл, вств, рдц, дц/тц, гк/гч, солнце, здравствуйте, сейчас, пожалуйста.
Doubled consonants are one sound; final -ый/-ий й, initial в before a voiceless consonant and
the softness of a final labial are optional. Each sound keeps its letter range (Character offsets
in the target; a stress-marked letter is one Character), its syllable, and what the spelling
asked for before reduction/devoicing, which is how the learner's likely mistake is named.
Unmarked polysyllables accept both full and reduced vowels and are only judged for ы/и.
`PronunciationTests.g2pTable` checks 43 words.

## Scoring

For sound *i*, `Scorer` compares the CTC log-likelihood of sounds *i−2…i+2* over their aligned
frames with sound *i* as expected (best of its correct spellings: [l]/[ɫ], [tʲ]/[tsʲ]…) against
each named mistake (ы→и, soft→hard, щ→ш, a full unstressed о, a voiced final consonant…) and
deletion. Margin ≥ 2 nats: good; 0…2: close; below: the winning mistake (substituted, with the
heard phone) or missing. Calibrated thresholds where ZIPA's conventions differ from textbook IPA:
it rarely marks softness before a vowel (writes мил [mil]), so soft→hard needs −5 there and for
final л н с м, −2 for final тʲ, which it marks reliably; unnamed vowel swaps and a final voiced
consonant need −2; a deletion must beat the best mistake by 1.5 and be below −3.

Stress per word = argmax of 0.5·(z(length) + z(loudness) + ½·z(pitch)) + 1.5·fullness, where
fullness is the log-likelihood ratio of [o] vs [ɐ] (or я/е vs [ɪ]) at that vowel; a moved stress
needs a 1.5 lead. On these recordings vowel quality is the reliable cue: in the calibration run
the vowel margins of the stress-pair о-words separated own from swapped audio completely (own
≥ +3.7, swapped ≤ −1.7 nats); duration/loudness alone did not.

## Evaluation (`scripts/pronunciation-eval/run.sh`, bundled audio)

**Minimal pairs** (pairs.json, 41 pairs; each word's audio scored against its own target and
against its partner's). *Recall*: playing мил for target мыл flags the contrast's mistake at the
differing sound. *False alarm*: the same contrast flagged on the word's own audio. Recordings
are Wikimedia/Shtooka humans and some Silero voices.

| contrast | cross trials | recall | precision | false alarm (own audio) |
|---|---|---|---|---|
| ы / и | 16 | **0.94** | 0.94 | 0.06 |
| ж / ш (initial) | 8 | **0.88** | 0.88 | 0.12 |
| unstressed о (до́ма/дома́…) | 8 | **0.88** | 0.78 | 0.25 |
| stress (за́мок/замо́к…) | 12 | 0.50 | 0.75 | 0.17 |
| hard / soft | 24 | 0.50 | 0.75 | 0.17 |
| ш / щ | 8 | 0.38 | 1.00 | 0.00 |
| луг/лук, код/кот, плод/плот (homophones by final devoicing) | — | — | — | 0.08 (should be 0) |

Stress: 4/4 on за́мок/замо́к and хло́пок/хлопо́к, 7/8 on the reduction pairs, all via vowel
quality; 0/8 on му́ка/мука́, пла́чу/плачу́, а́тлас/атла́с, у́же/уже́, where only prosody can tell. Hard/soft: final тʲ (мать,
брать, шесть) is reliable both ways; final нʲ/лʲ and a soft consonant before a vowel (люк/лук,
ряд/рад) are often missed — ZIPA's training transcriptions rarely mark them, and the thresholds
were set to protect native takes. ш/щ: ZIPA writes both as [ʃ] often; only щ said as ш is caught.

**Native takes** — 531 human course/pair recordings (Shtooka, Wikimedia Lingua Libre, Tatoeba),
5 762 expected sounds; stress taken from course.json where the clip's text has it:

| | single words (275) | phrases (256) |
|---|---|---|
| sounds judged substituted or missing | 3.3 % | 3.6 % |
| takes given any tip | 14 % | 31 % |

400 Silero takes from the course: 3.0 % of sounds flagged, 28 % of takes given a tip.

False alarms by mistake (human takes), as a share of the sounds where it could be named: soft→hard 4.2 %,
ы/и 6.2 %, full unstressed о 3.3 %, reduced stressed о 6.5 %, voiced final 9.0 %, hard→soft 0.6 %,
р 0.5 %. Many phrase texts carry no stress marks and fast connected speech elides; real targets
are stress-marked.

## Rough edges

- Prosody alone (no reducible vowel) misplaces stress often; syllable timing comes from CTC
  spikes refined by the loudness peak.
- Soft consonants before vowels and final нь/ль are under-detected (see above); ш/щ is weak.
- Thresholds were tuned on the same bundled audio they are reported on (no held-out set yet);
  the learner's own takes, with an English accent and room noise, are the real test.
- Extra sounds are reported only for confident inserted vowels.
- The comparison recording (`model:` argument) is not used yet.
