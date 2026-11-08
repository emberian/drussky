# An on-device tutor? Apple Foundation Models measured against Claude (2026-11-08)

Every tutor feature in Drussky (the ? beside exercises, page writing, diary correction, goal lemma
tables, passage glosses) needs the network and a Claude key. Apple's Foundation Models framework
(iOS 26 / macOS 26) puts a ~3B-parameter model on the iPad Pro M4 itself. This note says what it
can and cannot do in Russian, and what the app now routes to it: **glosses only, and only as the
offline fallback.** Everything else stays Claude's, and waits for the network.

## Availability

- SDK: Xcode 27.1, iOS SDK 27.1, `FoundationModels.framework` present for iphoneos and macosx.
  `@available(iOS 26.0, macOS 26.0, *)`. iOS 27 deprecates `LanguageModelSession.GenerationError`'s
  cases in favour of a `LanguageModelError` protocol; the app matches neither (errors are surfaced
  by `localizedDescription`), so it builds warning-free on the 27 SDK and runs on 26.
- Deployment target stays iOS 18: everything on-device lives under `#if canImport(FoundationModels)`
  + `if #available(iOS 26.0, macOS 26.0, *)` in `apple/Drussky/OnDevice/`, and
  `OnDeviceBackend.unavailableReason` says why the model is absent (`deviceNotEligible`,
  `appleIntelligenceNotEnabled`, `modelNotReady`, or no framework at all).
- `SystemLanguageModel.default.availability` on the development Mac (macOS 27.0.1, Apple Intelligence
  on): `.available`. **`supportedLanguages` does not include Russian** (24 locales: en, de, fr, es,
  it, pt, ja, ko, zh, nl, sv, da, nb, tr, vi). We ask in English about Russian text and the model
  answers; nothing refused us for language, but Apple makes no promise here, and the benchmark
  below is what that promise would have covered.
- Simulators: iOS 26.2 / 27.0 / 27.1 runtimes are installed, none of them usable for the model. The
  iOS 26.2 iPad Pro 13" (M4) simulator reports `.available` and then fails every generation
  (`com.apple.fm.language.instruct_300m.safety … promptTemplateNotFound`: its safety model's prompt
  template is missing). The iOS 27.0 simulator reports `.unavailable(.modelNotReady)` ("still
  downloading") and stayed there. Mac Catalyst would use the host model, but the app does not build
  for Catalyst today (an unrelated `NarrationPlayer` region-isolation error). So the honest
  measurement is the macOS CLI below; the XCTest harness skips on both simulators.
- Not run on the physical iPad (lane rule). The Mac's model is the same on-device Apple Foundation
  model family the iPad Pro M4 runs; latency on the M4 will be somewhat higher.

## Benchmark

`research/ondevice/` holds the harness:

- `make_cases.py` → `cases.json`: 50 glosses (the course's own passage glosses, the word with its
  sentence; the reference is the authored meaning in context), 20 "why is it this ending" questions
  with a key (authored here), 10 diary lines with planted slips and a gold correction (9 with one
  slip each, 1 clean), 40 word forms with hand-labelled lemmas (from `DrusskyTests/goal-lemmas.json`).
- `bench.swift` + `apple/Drussky/OnDevice/OnDeviceSchemas.swift` → `ondevice.json`: the on-device
  side, compiled from the **same prompts and @Generable schemas the app ships**, greedy sampling.
  Build: `swiftc -O -parse-as-library -swift-version 6 -target arm64-apple-macos26.0 -sdk $(xcrun
  --sdk macosx --show-sdk-path) research/ondevice/bench.swift apple/Drussky/OnDevice/OnDeviceSchemas.swift
  -o bench && ./bench research/ondevice/cases.json research/ondevice/ondevice.json`.
- `claude_side.py answer` → `claude.json`: what the app's `Tutor.stream` would say (`claude-opus-5-5`,
  effort low, server-side fallbacks; the diary and lemma prompts are the app's own).
  `claude_side.py score` → `scores.json`: glosses and ending answers graded by a Claude judge
  (one call, both sides' candidates mixed, blind to which is which; rubric in the script), diary
  edits and lemmas scored programmatically against the keys. Run with
  `uv run --isolated --no-project --env-file ~/dev/allgame/.env --with 'anthropic>=0.80' …`.

Numbers (Mac M-series, macOS 27.0.1; Claude over the network):

| task | on-device | Claude (Opus 5.5, low effort) |
|---|---|---|
| gloss, 50 unknown passage words in context | **40/50** right, 0.8 s mean | 49/50 right, 1.8 s mean |
| "why this ending", 20 questions | **0/20** correct, 5 partial, 15 wrong, 1.3 s | 20/20 correct, 3.7 s |
| diary correction, 10 lines (9 planted slips) | 6/9 slips caught, **6 false edits**, corrected text right 7/10, 5.1 s | 9/9 caught, 0 false edits, 10/10 right, 5.7 s |
| lemmatization, 40 forms in batches of 10 | **12/40** exact; 2 of 4 batches refused by the guardrail ("May contain unsafe content"), 0.3 s/form | 40/40 exact, 0.2 s/form |

What the misses look like (all in `research/ondevice/ondevice.json`):

- Glosses. The ten wrong: `станции → place` (×2), `ещё → yet`, `есть → to be` (for "there are"),
  `через → through` (for "in a month"), `нём → his`, `Сократ → short`, `Фёдора → name`,
  `Тютчева → poem`, `аршином → without a mistake`. Content words it knows, it knows (`живёт → lives`,
  `окне → window`, `возьмём → let's take`, `чётных → even`, `простые → simple` for "prime" was
  judged right). Names and function words are where it guesses. The `lemma` it returns alongside is
  unusable: Latin letters (`kakdy`, `často`), inflected forms copied back (`читала`), inventions
  (`станиция`); `OnDeviceBackend.cyrillic` drops anything not Cyrillic and the app never shows it.
- Endings. It calls direct objects nominative ("бра́та is in the nominative case because it is the
  subject"), puts «о Росси́и» and «в Петербу́рге» in the accusative, and fills the [[ru|en]] markup with
  invented grammar terms (`[[глагольное лицо]]`, `[[местоиме́нный падеж]]`). The five "partial"
  answers name the right case with a wrong or empty reason. A beginner cannot tell these from truth.
- Diary. It caught the plain endings (книгу, школу, университете, смотрят, другом) and left the
  clean line alone, but it also changed `папа → мама` ("you used father instead of mother"),
  `слово → вопрос`, marked `Москва` as a spelling slip with an empty replacement, invented a span
  (`учется`) that is not in the text, and wrote a teach item about "articles before nouns". Six
  false edits in ten lines is a red pen that marks right answers wrong.
- Lemmas. `слив → сливовать`, `обедом → объезд`, `мировое → мирный`, `потом → время`; and two of
  four batches of ten plain words were refused by the content guardrail. The rule-based
  `RussianLemmatizer` already covers the offline case; this would not improve on it.

Caveats, in fairness to the model: one prompt per task, no few-shot examples, no adapter; a Claude
model judged the gloss and ending answers (the same judge, blind, for both sides; the diary and lemma
scores need no judge). Russian is outside the model's supported languages. Sample sizes are small;
the ending and diary results are not close calls.

## What is routed where

`apple/Drussky/OnDevice/TutorBackend.swift`:

- `TutorBackend` protocol — `gloss`, `quickAnswer` ([[ru|en]] prose), `correctDiary`
  (→ `DiaryCorrection`), `lemmatize` (→ `Goal.LemmaEntry`); every answer carries a
  `TutorProvenance` (`.claude` / `.onDevice`). `ClaudeBackend` wraps the existing calls;
  `OnDeviceBackend` maps the @Generable answers (`OnDeviceSchemas.swift`) onto the contracts through
  the same lenient parsers Claude's replies use (`DiaryTutor.parse`, `GoalLemmaTable.entries`), so a
  diary correction from either tutor is the same struct, with `model` saying which.
- `TutorRouter.route(task, conditions)` with `TutorConditions(onDeviceAvailable, online, hasKey)`:
  Claude whenever it is reachable; the on-device model only for tasks in `onDeviceFallback`
  (= `[.gloss]`) when it isn't; otherwise `.unavailable(reason)` with the learner-facing reason
  (no key / offline). `onDeviceFirst` is empty. Changing the policy is editing those two sets.
- Hooks (kept to a few lines each): `PassageExercise` shows `GlossLine` under a tapped word, which
  asks the router only when the page has no gloss for it and shows a small "on device" tag when the
  device answered; `TutorSheet` consults the router before sending (offline now says so instead of
  failing), records provenance on each answer turn, and shows "on device" + **Ask Claude for more**
  under an on-device answer (dead code today, since quick answers never route there);
  `DiaryTutor.correct` and `GoalLemmaTable.fill` throw / wait when the router says unavailable.
  `Connectivity` (NWPathMonitor) is the network signal.

So, task by task:

1. **Glosses** — on-device as the **offline / no-key fallback**, labelled. Online with a key, a word
   the page cannot gloss is now asked of Claude (49/50) instead of showing "—"; offline, the device
   answers (40/50) with the tag. Course passages gloss every word they use, so this reaches goal and
   library texts whose lemma table is still unfilled.
2. **Quick "what does X mean / why this ending"** — not routed. 0/20. The plumbing (provenance tag,
   "Ask Claude for more") is in place for a future model.
3. **Diary correction offline** — not routed, not even labelled: a correction that changes right
   words is harmful, and a learner offline is better served by "the tutor needs the network".
4. **Goal-text lemmatization** — not routed. 12/40 and guardrail refusals; `RussianLemmatizer`
   already does the offline job.

## Tests

`apple/DrusskyTests/OnDeviceTutorTests.swift`: router decisions (online/offline/no key/no model ×
task), provenance on backends and on saved `Tutor.Turn`s (old notes decode), the Latin-lemma filter,
@Generable → `DiaryCorrection` mapping (empty and no-op edits dropped, categories spelled as the
contract spells them, teach capped at two), @Generable → `Goal.LemmaEntry` mapping, and
`testOnDeviceBenchmarkSlice`, which runs 20 glosses, 3 diary lines and 1 lemma batch against the live
model and skips when `OnDeviceBackend.isAvailable` is false. `cases.json` is bundled into the test
target from `research/ondevice/` (project.yml).

`apple/DrusskyUITests/OnDeviceUITests.swift`: with no key, opens the first page of the learner's
book (Tolstoy, lemma table unfilled), taps «пробежала», waits for the "on device" tag, attaches
the screenshot `ondevice-gloss`. Skips when the simulator has no model, which today is every
simulator (above), so there is no screenshot yet; the on-device quick answer is not routed at
all, so there is nothing to screenshot there. The first run on the iPad will produce the gloss one.

## Rough edges

- The content guardrail refuses lists of ordinary Russian words (2 of 4 lemma batches). Any
  on-device task fed raw learner text will meet this; the app treats it as "no answer".
- `supportedLanguages` excludes Russian; a future OS could start returning
  `unsupportedLanguageOrLocale` for these prompts. The router already treats any error as "no answer".
- iOS 27 renames the error enums; we avoid them, but the `GenerationOptions(samplingMode:)` spelling
  is the 27 SDK's (the 26 one is deprecated there). Building on a 26 SDK would need the old spelling.
- Not measured on the iPad itself. The M4 runs the same model; expect the same answers, slower.
- The benchmark is one session's prompts. Few-shot examples or a trained adapter could move the
  gloss number; nothing short of a different model moves the grammar numbers.
