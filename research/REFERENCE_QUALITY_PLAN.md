# Bounded Russian reference-quality evaluation

Accepted case plan, 2026-11-06, against baseline `8190096` and routing strategy `rufren-local-russian-v1`. This is the next speech batch, **not authorization to run inference during the current daily-session integration phase**. Root will grant the serial compute slot separately. Saving this plan performed no model inference, installation, Cargo build or app/provider change.

The purpose is to improve the Russian examples the learner actually hears: recoverable words and numbers, useful sound contrasts, and inspectable timing/spectral differences. Results attach to an exact recording and teaching target. The suite is not an accent scale or a general ranking of the two models. It complements [provider contracts](SPEECH_PROVIDERS.md), [the speech teacher](SPEECH_TEACHER.md), and [the actual Silero stress audit](SILERO_STRESS_AUDIT.md).

## Accepted cases

All texts fit the existing 256-character / 1,024-byte input bounds. Ordinary contextual text requires at least eight Russian word tokens and sentence-ending punctuation; authored combining acute marks select the explicit-stress branch before this count. Silero automatic and explicit-stress paths have different recorded frontend flags. A failed selected backend must not silently substitute another provider.

| ID | Exact public source text | Expected route | Specific question |
| --- | --- | --- | --- |
| RQ01 | Здравствуйте! | Silero automatic; reuse | Short-input baseline: intact greeting onset, complete wording and clean ending. |
| RQ02 | Лук. | Silero automatic; new | Is the intended hard lateral distinguishable from RQ03? Target meaning: onion. |
| RQ03 | Люк. | Silero automatic; new | Does the soft lateral and adjacent vowel transition supply the intended contrast? Target meaning: hatch. |
| RQ04 | Это за́мок. | Silero explicit stress; reuse | First-syllable stress reference; target meaning: castle. |
| RQ05 | Это замо́к. | Silero explicit stress; reuse | Second-syllable stress reference; target meaning: lock. |
| RQ06 | На двери висит замок, а на холме стоит старый замок. | Qwen contextual; new | Context requests lock first, castle second. Does it avoid the malformed-word behavior observed with combining accents, and supply the intended contrast? |
| RQ07 | У меня сегодня 2 вопроса, 5 идей и 21 свободная минута. | Qwen contextual; new | Complete number expansion and agreement, particularly **двадцать одна свободная минута**. |
| RQ08 | У меня сегодня два вопроса, пять идей и двадцать одна свободная минута. | Qwen contextual; new | Same intended spoken words without numeric glyphs; isolates a possible numeric-frontend problem. |
| RQ09 | У меня́ сего́дня два вопро́са, пять иде́й и два́дцать одна́ свобо́дная мину́та. | Silero explicit stress; new | Same intended wording through the authored-stress path; compare content and target realization. |
| RQ10 | Для каждого натурального числа существует следующее число, которое на единицу больше. | Qwen contextual; new | Complete mathematical statement and intelligible phrase grouping; no dropped/repeated connective or number word. |

RQ07 has exactly eight Cyrillic word tokens: digits do not count toward the router's threshold. RQ07/RQ08 compare frontend representations within Qwen. RQ08/RQ09 also change provider, voice and stress markup, so their difference cannot be attributed to a single cause. RQ06 is a newly scoped utterance; its occurrence inside an older long recording does not make that old audio an exact replacement.

## Existing artifacts to reuse

Paths are relative to the repository root; each WAV has a same-basename `.json` sidecar. These files were reopened and their exact-byte SHA-256 values checked against the sidecars while saving this plan. All are 24 kHz mono PCM16. Do not regenerate them to make the suite look uniform.

| Case | Existing WAV path | SHA-256 | Frames |
| --- | --- | --- | ---: |
| RQ01 | `.local/silero-tts/samples/hybrid-gateway-greeting-v1.wav` | `1f431043b464123eb9a023fe2cd52ecaccdca6f15f8a42d82b4328202ed2b138` | 23,400 |
| RQ04 | `.local/silero-tts/samples/silero-stress-castle-v1.wav` | `3ce677ebdad80003b514c177abdb2eb7efbc7674a92acbc2abf2f80b8d51ebe4` | 23,400 |
| RQ05 | `.local/silero-tts/samples/silero-stress-lock-v1.wav` | `3f0198c2415844291042e3161e445520e6d9aad7aec0c6956596a8c1e2d0ec76` | 23,100 |

RQ01's sidecar records all five automatic frontend flags enabled. RQ04/RQ05 record exact transformations to `Это з+амок.` / `Это зам+ок.` with automatic controls disabled. Their provenance pins Silero `v5_6_ru` to model SHA-256 `04cc9b1d08ce38bcaa9d02c488d80d3e6806d17f7eaa7c95982f36a3cbf50b2e`, voice Xenia. Preserve the returned provenance rather than replacing it with current descriptive copy.

Reuse [the stress measurements](artifacts/silero-stress-v1/measurements.json), [shared-scale plot](artifacts/silero-stress-v1/stress-comparison.png) and exact replay crops. The observed candidate vowel regions were approximately 140/50 ms for castle and 75/145 ms for lock. Those exploratory measurements support guided comparison; they are not fixed boundaries to transplant into RQ06 or learner speech.

The prior Qwen source/frontend control remains explanatory evidence in [the marked audit](fixtures/qwen_public_russian_v1.audit.json) and [unmarked audit](fixtures/qwen_public_russian_unaccented_v1.audit.json). Keep it separate from the new RQ06 result.

## Serial execution budget and order

The full accepted suite contains ten utterance records: three reused and **at most seven new TTS operations**—four Qwen (RQ06/07/08/10), three Silero (RQ02/03/09). Allow **at most eight independent ASR operations**: RQ01/02/03/06/07/08/09/10. No recognition call is needed to repeat the known spelling of the stress homographs in RQ04/05.

Recommended first slot: generate RQ02, RQ03 and RQ10, then run one independent ASR pass on each. This directly supplies the currently relevant sound pair and a mathematical reference. The remainder is four TTS operations and five ASR operations, including the reused greeting. If the first slot reveals a concrete blocker, retain its results and return the finding before extending the batch.

Run model work serially under root's resource handoff. Keep current worker deadlines, thread/memory limits and pinned environments; do not create concurrent GPU jobs or install an aligner. No automatic repeats, voice search, seed sweep, extra comparator downloads or regeneration of reused artifacts. A failed call still consumes its planned attempt; a follow-up experiment needs a specific question and an additional slot. Tiny offline analysis of the resulting audio can use the existing SciPy/gnuplot workflow.

## What each evidence channel can establish

**Contract evidence.** Check exact authored text, routed leaf provider, synthesis text/hash, frontend flags, returned WAV/hash, sample format and request bindings. A discrepancy directly refutes the declared software contract. Empty/truncated audio, malformed output or a demonstrably missing capture portion can also fail usability without a linguistic opinion.

**Independent lexical recognition.** ASR receives audio and the explicit Russian language selection, but no source text, intended answer, stress marks or hotwords. Store its literal response before comparison. RQ07's expected spoken sequence is RQ08; recognize admissible numeric versus word orthography without erasing the number value, word order or inflection. Preserve `ё/е` ambiguity separately. An ASR mismatch is a **lexical flag**, not a proved TTS sound error or a human listening verdict. Use RQ07/08/09 and selected spans to investigate a discrepancy rather than silently repairing its transcript. Recognition agreement supports lexical recoverability under this recognizer; it does not approve stress or every phoneme.

**Measured acoustic evidence.** Preserve original time and common frequency/level scales. For RQ04/05 and the two target words in RQ06, inspect candidate vowel-region duration, spectral envelope and contextual reduction jointly. Retain boundary sensitivity and exclude intervening consonants. For RQ02/03, inspect the lateral and the transition into the following vowel; preserve consonant context in replay. Track LPC sensitivity across settings and leave unstable formants unlabeled. Energy and F0 are supporting observations, not automatic stress labels. Identical decoded speech for intended contrasting references would refute the presence of that contrast; different hashes alone do not establish the intended contrast.

**Listening evidence.** If an actual listener evaluates a clip, record listener identity/type, whether the intended text was hidden, task, answer, replay count and exact audio/span. A human judgment exists only if a human actually supplied it. An audio-capable model's listening hypothesis is a separate machine evidence type; this plan does not add a cloud listener. Ordinary user perception trials can later contribute evidence without making Ember's friend a labeler or approval gate.

## Target-specific outcomes

Each case records separate outcomes for `contract`, `lexical_content`, `target_contrast` and `teaching_use`, with supporting evidence IDs/spans. Never collapse these into a global accent score.

| Outcome | Operational criterion | Resulting action |
| --- | --- | --- |
| **Usable for the stated target** | Contract passes and evidence supports the particular use. A lexical-reference screen can pass on complete recovered wording and no observed defect; a guided contrast additionally needs inspectable target-relevant acoustic differences and usable replay. Scope remains explicit. | Make that reference available for the supported teaching action. Preserve unresolved dimensions separately. |
| **Suspect** | Specific adverse evidence: malformed or missing audio, wrong routing/input transformation, anomalous lexical output, unsupported number expansion, identical speech for an intended contrast, or a target-region pattern requiring review. | Retain the original, identify the precise issue/span, and keep it out of automatic correct-answer use for the implicated target until the issue is resolved. Ordinary learning continues with already usable material. |
| **Unresolved** | Evidence cannot decide the target: ASR cannot distinguish homograph stress; segmentation is uncertain; LPC poles split/merge; recognizers/listeners disagree; or no relevant listening observation exists. | Offer demonstration/replay where useful, state the open target, and choose a narrowly discriminating next check. Do not invent an error probability or native-quality verdict. |

An artifact may therefore be usable for a timing demonstration while its precise vowel quality remains unresolved. “No lexical mismatch” is narrower than “correct pronunciation.” A blind trial scored against a generated example measures identification of that example; generalization to new voices/tokens must be observed separately.

## Primary sources and acquisition scope

- [Kochetov (2005), Russian laterals and rhotics](https://cla-acl.ca/pdfs/actes-2005/Kochetov.pdf): primary articulatory study of three Russian speakers with accompanying acoustic interpretation. It motivates examining lateral resonance patterns and adjacent transitions. Its materials and speaker-specific values do not provide a numerical threshold for `лук/люк` in Xenia's synthetic voice. Read through the public source; no model or corpus acquisition is proposed.
- [Padgett and Tabain (2005), Russian vowel reduction](https://pubmed.ncbi.nlm.nih.gov/16116302/), [paper](https://www.coli.uni-saarland.de/~andreeva/Courses/WS2023/Slavic/PadgettTabian.pdf): nine-speaker acoustic study, with trained segmentation and correction of mistracked formants. It supports joint duration/spectral investigation. Existing local acquisition: `~/paperbin/rufren/speech-teacher/padgett-tabain2005-russian-reduction.pdf`, with hash in that directory's `acquisition.json`.
- [Kocharov et al. (2019)](https://www.isca-archive.org/interspeech_2019/kocharov19_interspeech.pdf): Russian read-speech study separating effects of prominence and phrase-final lengthening. It explains why isolated versus contextual homographs cannot share a universal duration ratio. Existing acquisition: `~/paperbin/rufren/speech-teacher/kocharov2019-vowel-reduction.pdf`.
- [Qwen3-ASR technical report](https://arxiv.org/abs/2601.21337): original recognition/alignment methods. Its forced aligner uses an 80 ms timestamp grid; word navigation is a different requirement from measuring short vowel boundaries. Existing acquisition: `~/paperbin/rufren/speech-teacher/qwen-asr-report.pdf`. No aligner download is part of this batch.
- [Shi et al. (2023)](https://www.isca-archive.org/interspeech_2023/shi23_interspeech.html): experiment on acoustic training data, nonnative alignment and pronunciation assessment. It supports preserving the distinction between recognition/alignment and assessment. It does not validate Russian thresholds. Existing acquisition: `~/paperbin/rufren/speech-teacher/shi2023-alignment.pdf`.
- [Official Silero examples](https://github.com/snakers4/silero-models/blob/master/examples_tts.ipynb) and [Qwen model card](https://huggingface.co/Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign): provider/API provenance, not independent Russian quality evaluations. The already installed pinned models and runtimes are sufficient; no acquisition is requested.

## Artifacts and app consumer

For each executed case retain exact request/source, actual synthesis input and flags, immutable WAV and SHA-256, full response/provenance, run timing/resource observations, raw ASR output when run, and any measured original-frame spans. Store normalized comparisons as derivatives alongside the untouched transcript. Record unexecuted cases as unexecuted, never as passing fixtures.

Audio-studio coordination identified the consumer: teach both sound models, then a balanced blind choice with target text/meaning hidden until response, then explicitly commit the learner's intended meaning before a new take and self A/B comparison. The existing naming tasks reveal target meaning too early and repeatedly use the same target. RQ02/03 and reused RQ04/05 can supply exact reference pairs for correcting that task design. No UI/source changes belong to this case-planning step. The friend remains an optional casual conversation partner.
