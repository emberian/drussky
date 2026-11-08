# Narration: human readings of library texts, aligned word by word

2026-11-07. Lane: connected speech for Drussky's library (`apple/scripts/narration.py`,
`apple/Drussky/Narration/`). Working files in `/Users/ember/dev/rufren/.local/drussky-librivox/`
(originals, aligner venv + ONNX model, alignments, Whisper cross-checks, logs; 2.9 GB of which 1.3 GB
is the aligner model and 1.4 GB Whisper medium).

## What was found

LibriVox's Russian catalogue (76 items tagged `rus` on archive.org, plus the Russian sections of the
multilingual short-works collections) overlaps our 48 library texts in four places. Wikimedia Commons
adds three more readings under free licences. Everything else in the library (Tolstoy's reader stories,
Krylov, Ushinsky, Chekhov's other stories, Zamyatin, the science and philosophy pieces) has no
public-domain or CC human reading we could find on LibriVox, archive.org or Commons; those texts keep the
synthetic voice (they simply lack the `narration` fields).

| text | recording | reader | licence | excerpt in recording | clip |
|---|---|---|---|---|---|
| pushkin-vystrel | LibriVox `belkintales_1406_librivox` §01 | Xenium5 | public domain (LibriVox) | 31.1–186.2 s of 28 min | 155 s |
| pushkin-stantsionnyi-smotritel | LibriVox `belkintales_1406_librivox` §04 | Xenium5 | public domain | 221.1–396.8 s of 26 min | 177 s |
| dostoevsky-zapiski-iz-podpolya-2x2 | LibriVox `notes_underground_russian` §06 (part 1, ch. 9–10) | Yakovlev Valery | public domain | 207.2–329.4 s of 12 min | 123 s |
| tolstoy-muravei-i-golubka | LibriVox Multilingual Short Works 012 §11 | Luiza Karimova | public domain | 13.6–46.0 s of 58 s | 33 s |
| chekhov-tolstyi-i-tonkii | Commons `Ru-Чехов-Толстый и тонкий.ogg` | Pogrebnoj-Alexandroff | CC BY 3.0 | 11.1–254.1 s of 261 s | 244 s |
| pushkin-zimnee-utro | Commons `Ru-Зимнее утро.ogg` (spoken Wikisource) | Шантопик | CC BY-SA 4.0 | 2.1–60.4 s of 62 s | 60 s |
| turgenev-russkii-yazyk | Commons, phonograph cylinder 1894 | А. П. Ленский | public domain | 4.1–34.8 s of 41 s | 32 s |

Downloads: 44 MB, sequential, `User-Agent: Drussky narration lane (https://ember.software)`.
Provenance (URL, SHA-1 of the original, cut offsets, reader, licence statement) is in
`Audio/sources-narration.json`; `audio.py restore` re-downloads and re-cuts (checked: the restored
clip decodes to the same PCM, md5 `6ab06f55…` before and after).

Recording quality (noise floor of the excerpt relative to full scale, speech levelled to −20 dB RMS):
Chekhov −82 dB, Tolstoy −72 dB, Smotritel −64, Vystrel −63, Dostoevsky −55, Turgenev/Lensky 1894 −51
(surprisingly usable after whoever restored it), **Zimnee utro −32 dB** (a home recording with audible
hiss; kept because it is the only reading of the poem, flagged as rough).

## Aligner

`ctc-forced-aligner` 1.0.2 (MMS-300M forced-alignment model, ONNX, CPU; no torch) in a uv venv,
with `unidecode` romanisation of the Cyrillic words. Chosen over Apple Speech (an ASR, whose output
would still need aligning to the text) and WhisperX (needs torch + a 1.2 GB wav2vec2 Russian model;
its word times come from the same CTC family). Throughput about 3× real time on this Mac; the two
28-minute Pushkin chapters took ~9 minutes each.

Two passes were needed:

1. **Per-word wildcards** (the package default: `<star>` between every word). Fine on the four
   recordings that contain only the text, wrong on the two long chapters: a `<star>` has log-probability
   0 everywhere, so a run of short words after a long wildcard wanders to look-alike sounds minutes
   away. Dostoevsky came out as 271 words spread over 122–647 s with eight gaps over 3 s
   (3.4 characters/s); in Станционный смотритель the lone «В» of «В 1816 году» landed 72 s early.
2. **Edge wildcards** (what `narration.py align` does now): one `<star>` before the first word, one
   after the last, one before each word that follows a token the text spells in digits or `***`
   (the reader says «тысяча восемьсот шестнадцатом» or nothing). Everything else must be
   contiguous. Dostoevsky: 207.2–329.4 s, 14.5 chars/s, no gaps; the other six unchanged to within
   10 ms. Word spans are the first-to-last letter spikes (the package pads each span with half the
   neighbouring pause, which would light a word up to half a second before it is said); then every
   word gets ≥ 60 ms and a 60 ms tail, and a one- or two-letter word stranded > 1.5 s from both
   neighbours is put back just before the next word (`tidy`).

## Accuracy

I cannot listen, so the check is two independent systems plus structural tests, not a human ear.

**Against Whisper `medium` word timestamps** (`whisper --word_timestamps True`, DTW over
cross-attention — a different model family and method; its own word starts are known to drift
early into the preceding pause). Words matched by sequence alignment of the two transcripts.

| clip | words matched | onset median (whisper − aligner) | within 100 ms | within 200 ms | worst |
|---|---|---|---|---|---|
| tolstoy-muravei-i-golubka (59 words, clean studio) | 56 | +20 ms | 86 % | 96 % | 1.0 s on «Муравей» after a 1.2 s pause — Whisper starts the word at the pause |
| pushkin-zimnee-utro (121 words, noisy) | 116 | +40 ms | 86 % | 99 % | 0.22 s |
| turgenev-russkii-yazyk (50 words, 1894 cylinder) | 35 | +40 ms | 71 % | 97 % | 1.1 s on «не», again a pause absorbed by Whisper |
| chekhov-tolstyi-i-tonkii (425 words, 4 min, dialogue) | 395 | +20 ms | 85 % | 96 % | 0.92 s on «я» after a pause (Whisper again) |

Over the four clips: 602 matched words, median offset +20…+40 ms, 85 % within 100 ms, 96–99 %
within 200 ms; every outlier over 0.3 s is a word that follows a long pause, where Whisper's
onset is the start of the pause and the aligner's is the start of the word.

The aligner is consistently 20–40 ms *earlier* than Whisper's onsets, which is the direction one
wants for highlighting (a word lights as it begins). Word *ends* agree less (70 % within 100 ms):
the aligner's end is the last letter spike, Whisper's includes the vowel decay; the 60 ms tail is
a compromise. On the 60-word Tolstoy sample every word is in order, every sentence boundary
coincides with a pause of 0.9–1.5 s in the alignment, and single-letter words («к», «и», «в»)
sit 60–80 ms before the word they lean on.

**Structural checks** (`narration.py check`, `validate_library.py`, `NarrationTests`): word list
equals the text's letter-bearing tokens; times monotone and inside the clip; 6–20 characters per
second of clip for every text; manifest, library and timing file name the same clip; provenance
fields present; restore reproduces the audio.

Speech rate of the excerpts: 9.6 (Lensky, declaiming) to 15.0 (Xenium5) characters per second —
real narration, 2–3× faster than the course's isolated clips, which is the point.

## The app

`passage` + `narration` + `words` → `ReadAlongView` (play/pause, current word lit, tap a word to hear
from it, ¾× with pitch kept — `AVAudioPlayer.enableRate`, which is the time-pitch unit under the hood —
and *listen first*: text hidden, words appear as they are read). `Narration.page(for:)` makes the
whole page locally — no tutor call: the passage, then two **chunk dictations** (a 3–9 word sentence
of the reading, written from the reader's voice) and two **shadowing** read-alouds (the existing
speaking flow; the model clip is the reader's sentence, cut on demand for the rhythm strip).
Library rows show a headphone for narrated texts; the text header offers *Listen and read along*.
`-qa-narration <id>` / `-qa-narration-at <ms>` open that page for QA.

Screenshots (`research/artifacts/narration-v1/`, from `NarrationUITests` on the iPad Pro 13" simulator):
`read-along-playing.png` (Муравей и голубка 12 s in, «бросила» lit, the chunk exercises under it),
`read-along-listen-first.png` (text hidden, revealed up to «Потом»), `read-along-chunks.png`
(two chunk dictations «Write what the reader says», two shadowing lines with the Reader button),
`library-headphones.png` (the headphone mark on the narrated row), `library-text-read-along.png`
(the text header with *Listen and read along*).

## Rough edges / next

- Only 7 of 48 texts have a human reading; the rest need recordings that do not exist in the free
  corpora. A session with a native speaker reading the A1–A2 Tolstoy stories would cover the beginner
  end of the library in an hour.
- Word ends are letter spikes, so a long final vowel is "lit" slightly short; a per-word energy
  refinement (extend the end to the energy drop) would fix it without another model.
- `Зимнее утро` is hissy. Lensky 1894 is a historical curiosity more than a model to shadow.
- Dictation of a whole sentence is checked by the existing handwriting matcher, which was tuned for
  words and short phrases; punctuation is ignored but a 9-word line is a long strip.
- `ctc-forced-aligner` depends on `librosa`/`scipy`/`scikit-learn`; the venv is 400 MB. The model
  URL is a Hugging Face file (`deskpai/ctc_forced_aligner`); pin a copy if it ever matters.
