# Drussky for iPad, iPhone and Mac

A Russian exercise book you write in. Open it and you are on today's page: numbered
exercises down a squared тетрадь page, answered in place — with the Pencil on ruled lines,
by ear, or aloud. Finished exercises get a red tick; a finished page gets a red mark
(5 — Отлично!). Turn the page with the arrow; turning back shows exactly what you wrote.

## What a page asks of you

| exercise | you |
|---|---|
| meet | see a letter in print and handwriting, hear it, trace it once, then write it freely |
| trace / copy | write over a faint model, or beside a shown one |
| listen and pick | hear a syllable or word, tap which it was |
| dictation | hear it, write it — no model |
| translate | see English, write the Russian from memory |
| fill | write the missing word in a sentence |
| read aloud | say it; your take is kept, played back, and its rhythm drawn against the model's |
| what does it mean | read Russian, choose its meaning |
| passage | read a short text (story, proof, dialogue, poem); tap any word for its meaning and sound |

Writing is read on-device by Apple's handwriting recognizer. It confirms a right answer by
itself after a short pause; **Check** asks it explicitly. When it misreads you, *I wrote it
right* overrides it — the recognizer is a helper, not the judge. *Show me* writes the
answer in red above your line and brings the item back soon.

Speech is transcribed by Apple's Russian recognizer (on-device where supported). It tells you
what it heard; it does not grade an accent. The rhythm strip shows loudness over time for the
model and for you: stressed syllables are the long, loud ones.

## Remembering

Everything you produce — letters you met, words you wrote or said — becomes a memory with a
due date. Right answers on due items push the next review out (×2.5); misses pull it in.
New pages weave up to three due items in; when ten or more are due the next page is a
review page (Повторение). Review switches modality: hear→write, English→write, see→say.

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

## Inside the cover

The book icon (top left) opens your alphabet — each letter in **your own handwriting** once
you've met it — and every page so far with its mark. Settings: finger drawing, cursive
models, stress marks, notebook export (pages, ink, recordings as a zip), credits.

## Building

```sh
cd apple
xcodegen generate
# simulator
xcodebuild -project Drussky.xcodeproj -scheme Drussky \
  -destination 'platform=iOS Simulator,name=Drussky iPad 13' -derivedDataPath .build/dd test
# iPad (team 992772WV7K, automatic signing)
xcodebuild -project Drussky.xcodeproj -scheme Drussky -configuration Release \
  -destination 'id=<device udid>' -derivedDataPath .build/device -allowProvisioningUpdates build
xcrun devicectl device install app --device <device udid> .build/device/Build/Products/Release-iphoneos/Drussky.app
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
`-qa-settings`.

## Content

`Drussky/Resources/course.json` is generated by `scripts/build_course.py` and checked by
`scripts/validate_course.py` (schema in `COURSE_SCHEMA.md`: letters before use, stress
marks, page sizes, answer integrity). Audio lives in `Drussky/Resources/Audio/` with
`index.json` (normalized text → file) and `sources.json` (provenance and licences); every
string the app can speak, including generated reviews, has a recording — a unit test
enforces it. `scripts/make_audio.py` (local Silero) generates missing ones.

Learner data lives in Application Support/Drussky (`book.json`, `ink/`, `voice/`).
