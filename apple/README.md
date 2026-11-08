# Drussky for iPad, iPhone and Mac

A Russian exercise book you write in. Open it and you are on today's page: numbered
exercises down a squared тетрадь page, answered in place — with the Pencil on ruled lines,
by ear, or aloud. Finished exercises get a red tick; a finished page gets a red mark
(5 — Отлично!). Turn the page with the arrow; turning back shows exactly what you wrote.

## What a page asks of you

| exercise | you |
|---|---|
| meet | see a letter in print and handwriting, hear it, *watch it printed* stroke by stroke, trace it once, then write it freely; one formation hint (start, order, direction, size, baseline) compares it with the print model (no hints against cursive guides) |
| trace / copy | write over a faint model, or beside a shown one |
| listen and pick | hear a syllable or word, tap which it was |
| dictation | hear it, write it — no model |
| translate | see English, write the Russian from memory |
| fill | write the missing word in a sentence |
| read aloud | one tap: hear the model, a soft cue, then just say it (it stops when you do); every take is kept, compared with the model syllable by syllable, with a tip or two; *Skip for now* (ungraded) is always there |
| what does it mean | read Russian, choose its meaning |
| passage | read a short text (story, proof, dialogue, poem); tap any word for its meaning and sound |
| diary (Дневник) | write a few words of *your own* Russian about a prompt, on ruled lines (or type); confirm or fix the recognizer's transcript, then the tutor marks it like homework — wrong spans underlined, the right form in red above, the corrected line, a word of praise, the pattern behind the biggest slip. Your ink stays. Every edit becomes evidence (own-writing modality, a confusion edge, the pattern), and corrected forms join your words for review. From unit 3 every page ends with one; ＋ → *Write in my diary* adds one now; *Not today* skips it |

Writing is read on-device by Apple's handwriting recognizer. It confirms a right answer by
itself after a short pause; **Check** asks it explicitly. When it misreads you, *I wrote it
right* overrides it — the recognizer is a helper, not the judge. *Show me* writes the
answer in red above your line and brings the item back soon.

Saying something aloud is hands-free (`Speaking/`): the model plays (Settings: 0–3 times,
tap to skip), a soft tone says it's your turn, and the microphone (AVAudioEngine) measures the
room for a quarter second, starts the take at your first sound and ends it after 0.7 s of quiet
(6 s at most, longer for long sentences); a tap stops it early. A voice is confirmed only by
periodic sound, so a knock or a breath never starts a take, but a leading с is kept. The take is
trimmed (0.12 s before, 0.18 s after) and kept on the page.

Each take is analysed by a `PronunciationAnalyzer` (Pronunciation/PronunciationTypes.swift). Until
phone-level analysis is wired in (`SpeakingTurn.analyzer`), the word-level analyzer stands in: the
recognizer's words aligned letter by letter with the target (a letter heard as another is marked
red and named, with the contrast's advice and a *Practise* link that opens a Sound pairs page for
it), and the model's syllables found from its loudness dips and spectral changes, carried onto
your take through a time warp (DTW on cepstra) so rhythm, loudness and melody compare syllable by
syllable. Stress is judged against the model, so a voice's own habits cancel. Tap any syllable to
hear just that part of the model or of you. Letters a word-level report couldn't hear aren't
marked right. If recognition can't confirm a take you can mark it *Sounded right to me* (counted,
as unverified) or skip it (not counted). Letters heard as others are logged as speech confusions
on their sound contrast.

## Remembering

Everything you produce — letters you met, words you wrote or said — becomes something to recall.
When it comes back is decided by one model: LearningCore's FSRS memory per item *and modality*
(hearing → writing, English → writing, reading aloud, choosing by ear or meaning), folded from
the answer log (`events.jsonl`). A miss brings the item back within minutes (relearning), not on
its old interval; skipping or leaving an item unanswered never postpones it. Grades are honest:
*right first time* (good) only when the checker confirmed it with no earlier failed check;
right after a failed check, or by your own judgement, is hard; a wrong pick then the right one,
or being shown the answer, is again; rewriting an answer after it was shown is practice and
isn't graded. New pages weave up to three due items in; when ten or more are due the next page
is a review page (Повторение). Scaffolds fade (trace → copy → write from hearing) while recall
of a word is strong now, and come back if it decays.

The book also knows what day it is. The first page of a new day opens with a short warm-up (the
easiest due items), the day's reviews up to a cap (25 a day by default; the overflow waits, most
urgent first), then the next course page. After a week or more away, the first page re-tests a
sample of what you knew rather than everything. When several things the next page leans on have
gone weak, a short *Getting ready* page brings them back first; when much is due but the next
page's material is ready, the course goes on with reviews woven in. Now and then a few things the
book believes you know are slipped onto pages one, seven and thirty days later; inside the cover,
*How well the book knows you* sets its predictions against what you actually recalled, and the
memory model's intervals move a little, slowly, towards the evidence. When answers slow and misses
pile up late in a long page, the page shortens (two remaining reviews become listening or reading
choices, the rest wait) and says so in one line. Settings: retention aim (80–95%), reviews per day,
probes on or off. (`LearningCore/README.md`, "Time and the learner".)

## The tutor

The **?** beside each exercise (and *Ask about this page* at the bottom) asks Claude
(`claude-opus-5-5`) about that exact exercise and your attempt. Questions and answers are
written onto the page under *Вопросы*. The key lives in the device Keychain: type it in
Settings, or drop it into the app's `Documents/tutor-key.txt` with
`xcrun devicectl device copy to --domain-type appDataContainer --domain-identifier software.ember.drussky`
and the next launch imports it and deletes the file.

## Pages that write themselves

The **＋** in the top bar asks the tutor for a new page, placed right after the current one:
*A page from my mistakes* (built from what you've missed and find hardest) or *Read my own
text…* (paste or Scan Text any Russian; it becomes a glossed passage with questions, and its
new words join your reviews). Pages arrive in the course's own format and every exercise is
checked before it goes in. The tutor's answers mark Russian as tappable phrases you can
**Keep**. The **pencil** button turns the whole page into paper for margin notes (graphite;
double-tap the Pencil to erase). Speakers with a small number cycle through several human
voices.

## Your book

The inside cover asks for one text you want to read for real — from the Library, pasted or
scanned, or a ru.wikisource link (fetched through the MediaWiki parse API and cleaned to its
paragraphs). It becomes *your book*, kept in the notebook (several allowed; one current), and
the cover shows **Distance to your book**: the share of its words you know, how many new words
stand between you and its first page, how many of its sentences you could read now, a ladder
of the next twenty words worth knowing (occurrences in the text, weighted toward the first
page) and the grammar patterns it leans on. The text's words are lemmatized by the course's
tables, then by the tutor (`claude-sonnet-5-5`, 300 forms per call, cached in the goal with a
short gloss), with a rule-based stemmer standing in until that arrives.

**Toward my book** (＋, or the panel) has the tutor write a page from the goal's own sentences:
a passage of 2–4 consecutive sentences that are now ≥80% known (glossed), then exercises on the
most valuable unknown words taken from the text. Every Russian string on the page must be a
contiguous stretch of the text; anything invented is dropped. Without a key, an offline page
teaches the words through their own sentences. The book also writes one by itself when a
brand-new course page would teach nothing toward the goal while a readable stretch waits (at
most one in three pages). When the first five sentences are each ≥90% known, the book opens
them as a page of their own — *You can read the first page of your book* — with tap-to-gloss
and a red «Пе́рвая страни́ца» stamp. Policies and thresholds: `Drussky/Goal/Goal.swift`;
the distance arithmetic is LearningCore's `Target`.

## Inside the cover

The book icon (top left) opens your alphabet — each letter in **your own handwriting** once
you've met it — and every page so far with its mark. **My Russian** collects every diary line
you have ever written, newest first, with its red pen, and what the pen keeps finding this
week; it exports as Markdown. Settings: finger drawing, cursive
models, stress marks, notebook export (pages, ink, recordings as a zip), credits.

## Audio

The clips (`Drussky/Resources/Audio/*.m4a`) are not in git; their manifests are. After cloning,
before building:

```sh
.local/silero-tts/venv/bin/python apple/scripts/audio.py restore   # from the repo root
```

It rebuilds every clip named in `Audio/index*.json` / `variants*.json`: from the local original
cache in `.local/` if present, else by downloading the recorded original (Wikimedia Commons,
Tatoeba), else by re-synthesizing the recorded Silero input; then trims, levels and encodes it as
the generators did. Plain `python3` works when nothing needs synthesis. `audio.py status --plan`
shows what is missing and how it would come back; `audio.py verify` checks that every clip exists,
decodes and has provenance.

## Building

```sh
cd apple
xcodegen generate
# simulator
xcodebuild -project Drussky.xcodeproj -scheme Drussky \
  -destination 'platform=iOS Simulator,name=Drussky iPad 13' -derivedDataPath .build/dd test
# ember's iPad (team 992772WV7K, automatic signing). `Personal` is Release plus PrivatePack
# (personal-study audio); plain Release and archives never contain PrivatePack.
xcodebuild -project Drussky.xcodeproj -scheme Drussky -configuration Personal \
  -destination 'id=<device udid>' -derivedDataPath .build/device -allowProvisioningUpdates build
xcrun devicectl device install app --device <device udid> .build/device/Build/Products/Personal-iphoneos/Drussky.app
# Mac (native Mac Catalyst app, sandboxed with microphone + network)
xcodebuild -project Drussky.xcodeproj -scheme Drussky -configuration Release \
  -destination 'platform=macOS,variant=Mac Catalyst' -derivedDataPath .build/mac -allowProvisioningUpdates build
cp -R .build/mac/Build/Products/Release-maccatalyst/Drussky.app ~/Applications/
```

Tests: `DrusskyTests` covers text normalization, course integrity, audio coverage (every
string the app or its reviews can speak), handwriting recognition on synthetic print and cursive
ink and on 156 single-line pen-stroke samples with hand-like slant and wobble (on the iPad
Pro M4: pen 155/156, cursive 12/12, print 12/13), scheduling and persistence, notebook-recovery and redo
regressions, speech recognition on recorded Russian (runs once speech recognition is
allowed), and one live tutor answer (runs when `TEST_RUNNER_DRUSSKY_TUTOR_KEY` is set).
`DrusskyUITests` writes on the paper with real touch strokes, completes the exercise, and
checks it survives a relaunch.

QA launch arguments (simulator): `-qa-reset`, `-qa-complete N` (work through N pages with
synthetic ink), `-qa-partial K`, `-qa-age D`, `-qa-top`, `-qa-bottom`, `-qa-contents`,
`-qa-settings`, `-qa-diary` (three diary pages with canned red-pen corrections, no key
needed), `-qa-myrussian` (with `-qa-contents`: opens My Russian);
`-qa-settings`; `-qa-day N` / `-qa-away D` (the pages were done N or D days ago, then the page is
turned: a morning or welcome-back page), `-qa-probes` (a synthetic answered probe set for the cover
chart), `-qa-tired` (the current page marked shortened);
`-qa-settings`, `-qa-goal <library id>` (make that text the learner's book), `-qa-goal-pull` (an
offline page toward it), `-qa-goal-first` (open its first page now).

## Content

`Drussky/Resources/course.json` is generated by `scripts/build_course.py` and checked by
`scripts/validate_course.py` (schema in `COURSE_SCHEMA.md`: letters before use, stress
marks, page sizes, answer integrity). Hand-printed stroke models for all 66 letter forms
(`Drussky/Resources/strokes.json`) are built by `scripts/build_strokes.py`. Audio lives in `Drussky/Resources/Audio/` with
`index.json` (audio key → file) and `sources.json` (provenance and licences). The audio key is
the normalized text plus `?` for a rising (yes/no) question, so «Э́то чай?» and «Э́то чай.» are
different clips (`RussianText.audioKey`, `validate_course.audio_key`; `scripts/audio_identity.py`
files every clip by what its own provenance says it says). Every
string the app can speak, including generated reviews, has a recording — a unit test
enforces it. `scripts/make_audio.py` (local Silero) generates missing ones.

Learner data lives in Application Support/Drussky (`book.json`, `ink/`, `voice/`).
