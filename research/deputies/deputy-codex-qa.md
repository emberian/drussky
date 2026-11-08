# Drussky deputy review — 2026-11-08

**Source fixes are in this worktree. This is not a visual QA sign-off.** Simulator creation was blocked by this session's sandbox, and Xcode's Swift macro host was blocked independently. I could not launch the iPad or iPhone app, capture screenshots, test a microphone flow interactively, or certify a warning-free iOS build. No screenshot was captured or visually inspected. The findings below are from source and course-order inspection, not an invented walkthrough.

I read the app, course-schema and LearningCore documentation and all QA extensions. I inspected the view implementations for all 16 exercise kinds. No git commands were run; nothing was committed. No physical device, real notebook, saved credential, other worktree, or other session's cache was changed. Notebook Codable schemas are unchanged.

## Ranked findings

1. **P1 · Fixed in source: answering moved the paper and could start the next exercise's audio.** `PageView` scheduled a scroll 450 ms after an advance signal. `ExerciseSlot.finish`, completed speech analysis, diary corrections, Talk completion/skips and the end of a sound-pair comparison all emitted that signal. An answer or arriving correction is not an instruction to navigate. Those completion paths now stay in place; only the explicit sound-pair **Next** requests navigation. Removed the delayed scrolling task. Initial opening still resumes at the first unfinished exercise; explicit page turns still work. Fixed competing `-qa-top`/`-qa-bottom` positioning by giving bottom positioning precedence over resume. Writing controls retain a 44-point minimum space when completed, preventing the previous 34-to-zero collapse on a normal single-row strip. Regression test covers writing, speech, diary and Talk completion versus explicit Next. **Needs Pencil and simulator verification.**

2. **P1 · Fixed in source: Talk grabbed the microphone before the learner was ready, and could record its own replay.** `TalkSession.yourTurn()` immediately began capture after the tutor finished. It now waits with the microphone idle. Tap starts capture; typing and replay remain available. Replaying a tutor line stops capture, and replay work is cancelled when leaving or starting capture. The microphone button previously ignored an idle microphone after interruption, silence or capture failure; it now starts another attempt. Added a regression test that resuming after a tutor line leaves capture idle and replay cancels a posed capture.

3. **P1 · Fixed in source: Talk's “Try again” did nothing.** The failed-state button called `resume()`, which accepted only `.paused`. Resume now accepts failed and unavailable states. Added retry to the initial unavailable screen, a way to leave scene generation, **Pause** during a reply, and **Keep it unmarked** during marking. These give the learner control during slow/offline requests instead of relying on the 120-second network timeout. Renamed typed **Say** to **Send**. Added a missing-key retry regression test; it skips if a credential exists. Added Debug `-qa-no-key`, which masks key reads without deleting the credential, and `-qa-talk empty` for a genuine initial Talk page.

4. **P1 · Fixed in source: a diary pause opened an editor without being asked.** After 2.2 seconds without ink changes, `DiaryExercise` replaced its writing controls with the transcription editor. Removed this automatic transition; **Read my writing** opens it explicitly. Added **Keep as written** directly to typed and confirmed-handwriting composition, so keeping a diary line does not require a failed tutor request first. Existing uncorrected storage semantics remain ungraded.

5. **P2 · Fixed: an early diary prompt exceeded the useful vocabulary of its scene.** `u3-p2` asked for three objects in the actual room when the page's useful nouns included cat, cake and rocket. It now asks for those three taught nouns and permits imagined objects. Each is introduced before the diary: cat in `u1-p2`, cake by copying in `u3-p2#4`, rocket on the Р card and in `#5`. Changed the generator and regenerated `course.json`; the generated diff is exactly this one English prompt. No new audio is needed.

6. **P2 · Fixed: goal progress gauges overclaimed comprehension.** Removed the cover's percent-known display, predicted readable-sentence count and first-page threshold progress bar. Removed the same percentage/readability claims from the goal page preview. The useful text, vocabulary ladder, patterns and page actions remain. “You read its first page” was only backed by an opening timestamp; it now says **First page opened**. The first-page introduction invites reading with word help instead of declaring that the learner can read it.

7. **P2 · Fixed: the desk claimed fatigue and could claim a change that never happened.** Removed the sleepy icon and “you've been at it a while” judgment. The footer now reports actual changed/deferred review counts; it appears only if something changed. Removed the cover's inferred “mornings suit your…” claim. Updated the existing day test and added a regression for a tired estimate on a page with no modifiable reviews. **The underlying automatic shortening policy still exists; see unresolved concerns.**

8. **P2 · Fixed in selected teaching surfaces: English competed with Russian on the same line.** Meanings now sit below Russian in `RussianLine`, explanation examples, speaking targets and letter examples. Diary and Talk headers separate their language blocks; new diary/Talk page titles no longer concatenate Russian and English. Removed decorative emoji from basic meaning tags and translation prompts. This does not constitute a whole-app typography pass: morphology hints, goal vocabulary chips and some transcript feedback still mix languages.

9. **P2 · Fixed in selected controls: small targets and crowded action rows.** Shared speaker and pill controls have at least 44-point targets. Talk replay/English-reveal/take playback and narration speed/text controls are at least 44 points. Writing, diary and several speech/Talk action rows wrap with the existing flow layout. These are source-level corrections; small-phone wrapping and large Dynamic Type still need real inspection.

10. **P2 · Fixed: stress guidance was deliberately discarded in corrections.** Writing corrections and diary red-pen replacements now use the existing stress-display helper rather than stripping stress. Header labels, page praise, diary headings and Russian month names have stress. Removed exclamation marks from the page praise. The general tutor prompt now explicitly excludes acute marks on ё words. Original learner writing is preserved unchanged. The app's existing convention is an underlined/coloured stressed vowel in most teaching text, rather than a displayed combining acute.

11. **P2 · Fixed in source: narration could reflow while reading and reveal a hidden word's gloss.** Highlighting changed a word from regular to semibold, potentially changing its width and line break. It now changes colour/background with constant font weight. Hidden-word taps previously assigned `selected` before checking visibility, exposing the word/gloss below the passage. They now return before selection. Audio availability and actual line stability still need interactive verification.

12. **P3 · Fixed: validator counted an invisible introduction; gloss failure said only “—”.** The meaning-before-recall validator credited every letter-card example, including ones the view hides because their letters have not been introduced. It now applies that visibility gate. Added tests for hidden versus visible examples, earlier recall versus later copying, exact inflected forms and illegal ё stress on morpheme tiles. The complete current course passes the stronger checks. An unavailable fetched passage gloss now explains that its meaning is unavailable instead of displaying an unexplained dash.

## Not fixed / not verified

- **P1: no visual or iOS runtime sign-off.** Portrait/landscape iPad, iPhone, Pencil/touch movement, persistence after interactive edits, denied microphone, live offline requests and the new Talk transitions remain untested on iOS. Syntax parsing does not type-check SwiftUI. Run the app build and DrusskyTests before integration.
- **P2: read-aloud still has an automatic model-to-microphone handoff.** The initial microphone tap is explicit, but after the model plays `SpeakingTurn.run` starts capture without another readiness tap. Talk now waits. I would test the standalone speaking exercise with ember before changing its shared state machine; at minimum it needs a clear “ready” interval or separate listen/record controls.
- **P2: the inside cover is still a dashboard ahead of an index.** `ContentsView` places stats, desk/calibration notes, goals and the knowledge section before the alphabet and page list. A learner trying to find yesterday's page must pass through the model's account of them. I would put pages and the alphabet first, and put the calibration/knowledge plots behind an explicit secondary action. This is a larger navigation choice, left for root review.
- **P2: automatic fatigue adaptation still changes the page.** `Book+Day.shorten` replaces untouched recall exercises with choices and skips later reviews. The wording is now factual, but the learner still did not request the change. Recommend an explicit “Shorten this page” action rather than a hidden inference. Initial page resume positioning and automatic goal-page insertion also deserve real walkthrough scrutiny.
- **P2: typography/stress cleanup is incomplete outside the basic workbook.** `FamilyGuessExercise` shows a root, gloss and unstressed known-word keys in one row; `WordAnalysis` and goal chips mix English and Russian. Library originals and live tutor output are not guaranteed to contain stress. No automatic stress guesser was added. Source-backed text or an explicit annotation path is needed.
- **P2: narration playback failure can still be silent.** `NarrationPlayer.load/play` return without a visible error when a clip cannot load; `play()` also sets `isPlaying` without checking the AVAudioPlayer result. Bundled audio coverage passes, but a missing/corrupt imported narration could leave an apparently inert Play button. Needs a tested player-error state shared by passage and chunk controls.
- **P3: “away six days” is not the welcome-back branch.** The policy threshold is seven days. Six days produces a morning review composition. This is an intentional policy boundary in code, not evidence that the requested six-day screen was exercised.

## Screenshots and walkthrough matrix

**Actual screenshot paths: none.** The simulator could not be created. No layout in this report is described as visually verified.

`qa-artifacts/capture-matrix.json` contains exact launch arguments and **planned, nonexistent** screenshot names for 19 states, each to be inspected on iPad 13-inch portrait, iPad landscape and iPhone portrait. All are marked **NOT RUN**. The relevant expected checks are:

| Planned screenshot stem | State / what to inspect |
|---|---|
| `fresh` | First morning: honest instructions, first letters, dense teaching examples, no capture on arrival. |
| `day4` | Units 1–3 completed, next morning: orientation and a calm review opening. |
| `away6` | Same course progress after six days: manageable morning reviews, no fatigue judgment. |
| `contents` | Can the learner find their previous pages without dashboard hunting? |
| `early-fill` | Fill and pronouns: every answer form has been taught. |
| `diary-open`, `diary-marked` | Write, pause, explicitly read, keep without a tutor; inspect stress on corrections. |
| `talk-no-key`, `talk-scenes`, `talk-paused`, `talk-marked` | Useful key error, working retry/exit, explicit microphone start, legible transcript. |
| `goal-cover`, `goal-pull` | No progress gauges or comprehension promises; useful reading and word help. |
| `narration` | Stable lines at both speeds; hidden words stay hidden; Play and Pause work. |
| `speaking` | Explicit initial start, denied microphone path, honest empty-recognition feedback. |
| `sound-pairs` | Comparison stays in place; explicit Next navigates. |
| `passage` | Word help with no key, readable translation, finish/leave paths. |
| `morphology` | Build-word and family-guess tiles/hints on narrow screens. |
| `self-judge` | Check shows a usable comparison without moving the writing surface. |

All planned files follow `qa-artifacts/screenshots/<stem>-<orientation>.png`; these files do not exist.

Important fixture details: `-qa-complete 3` means three **pages**, not three units. The matrix uses `-qa-until u4-p1` to finish units 1–3. `-qa-day 1` means the work happened one day ago; it does not mean “course day one.” `-qa-diary` creates marked samples and is not a substitute for composing a new diary line. `-qa-talk mid` opens a saved/paused conversation. The new `-qa-talk empty -qa-no-key` exercises the actual initial missing-key state.

First authored occurrences covering the exercise kinds: `u1-p1` has meet, trace, explain, listen-pick, copy, read-aloud and dictation; `u1-p2` has translate and read-meaning; `u1-p3` has fill; `u3-p1` has diary; `u4-p4` has passage; `u23-p1` has build-word and family-guess. Sound-pair and Talk are generated kinds, covered by their QA flags. This is a source inventory, not exercised UI coverage.

## Validation and evidence

- **PASS:** `python3 apple/scripts/build_course.py`; generated diff is one English diary prompt.
- **PASS:** `python3 apple/scripts/validate_course.py --audio`: 31 units, 126 pages, 1,331 exercises, 709 lexicon entries. Log: `qa-artifacts/course-audio-validation.log`.
- **PASS:** five Python validator regression tests. Log: `qa-artifacts/validator-tests.log`.
- **PASS:** all 27 existing LearningCore tests, built as a Mac XCTest bundle against the actual LearningCore sources and run with `xcrun xctest`. Zero failures. Logs: `qa-artifacts/learningcore-build.log`, `learningcore-tests-build.log`, `learningcore-tests.log`. These are the existing tests, not substitute assertions; they do not cover the iOS app.
- **PASS:** Swift 6 syntax parsing of all 134 Drussky and DrusskyTests files. Log: `qa-artifacts/swift-parse.log`. **Not a type-check or link check.**
- **PASS:** audio status: 2,666 referenced clips present, zero missing. Log: `qa-artifacts/audio-status.log`.
- **BLOCKED:** own simulator creation. CoreSimulator connection refused and log access denied. Log: `qa-artifacts/simulator-create.log`.
- **BLOCKED:** initial Debug generic-simulator build after `xcodegen generate`. Xcode's macro process failed with `sandbox-exec: sandbox_apply: Operation not permitted`, followed by missing SwiftUI/Observation macro implementation errors. Log: `qa-artifacts/build.log`. It also emitted an AppIntents metadata-extraction warning; zero-warning completion is **not certified**.
- **BLOCKED:** requested DrusskyTests + LearningCoreTests simulator run, with recorded-speech and OnDeviceTutorTests exclusions. Destination unavailable; **no iOS tests executed**, so there is no app assertion-failure result. Log: `qa-artifacts/tests.log`.
- **ADDED, NOT RUN ON iOS:** Talk readiness/replay and no-key retry tests; completion-without-navigation test; no-false-shortening test; updated existing day and praise expectations.

Evidence/diffs are local review artifacts, not suggested committed binary assets. `qa-artifacts/changed-files.txt` lists changed sources, and `qa-artifacts/changes.diff` compares them with copies taken before editing. The added validator test is `apple/scripts/test_validate_course.py`. Derived data is isolated under `apple/.build/deputy-codex-qa`. Disk free space remained about 500 GiB.

## My three biggest “if I were ember” observations

1. **I would want the paper to wait for me.** A tick is enough. An answer, a pause, a recognizer result or a tutor reply should not silently become permission to move, record, or replace my work. The most valuable fixes here establish that distinction.
2. **I would want one place to find my writing, before seeing a model of myself.** The continuous page is the strongest part of the design. The cover's graphs, predictions and readiness machinery make it harder to return to the thing I actually came to do.
3. **I would want an achievable sentence before an invitation to improvise.** “Write about your room” sounds simple in English and can be impossible with today's Russian. A few taught forms, a visible example, and permission to describe an imagined scene make personal writing possible without turning it into a test of untaught vocabulary.

Next integration step: build and run the iOS tests in a session that can access CoreSimulator, then complete the saved screenshot matrix as an actual learner walkthrough. Root should review the source fixes independently of this missing visual sign-off.

## Integration and walkthrough — 2026-11-08 (Claude, branch `deputy-codex-qa`)

The source fixes above were merged with `main` (Talk ×2, devices, grammar, cursive, latency, the
seam-fix lane), built warning-free for the iOS 26.2 simulator, and walked through on an iPad Pro
13" (both orientations) and an iPhone 17 Pro. Screenshots: `research/artifacts/qa-walkthrough/`
(`ipad/<state>-portrait.png`, `ipad/<state>-landscape.png`, `iphone/<state>-portrait.png`), 21 states:
the matrix above plus `tired` (the desk line) and `diary-bottom` (the u3-p2 diary exercise itself).

Kept from the deputy: no completion path calls `requestAdvance` (only the sound-pair **Next**);
Talk waits for the microphone tap and replay never records itself; **Try again** resumes failed and
unavailable Talk pages (re-checking the reason first, so a still-missing key stays an honest note
rather than a request); the diary opens its editor only on **Read my writing** and offers **Keep as
written**; gauges, fatigue judgement and the hourly-preference claim are gone; meanings sit below
Russian; 44-pt controls; stress on corrections, headings, month names and praise; narration
highlights without reflow and hidden words stay hidden; the u3-p2 diary prompt names taught nouns.
Changed: one flag, `-qa-nokey` (main's name, the deputy's Keychain-level mechanism), replaces
`-qa-no-key` and `-qa-talk empty` is main's `-qa-talk fresh`; the missing-key Talk test asserts the
merged `.unavailable` semantics; the readiness test gives itself a placeholder key and restores the
learner's. Added from the walkthrough: stress on the cover's word chips and the goal's rungs
(`Book.stressedLabel`); long Talk scene titles wrap and `FlowRow` measures children at the row's
width so wrapped cards no longer collide with the row below; the `-qa-diary` fixture and the PDF
label use «Дневни́к»; `RotateUITests` turns the simulator for shell-driven landscape captures
(Xcode 27 ships no Simulator.app menu and `simctl` has no orientation command).

Still open, seen on the walkthrough: library words in the goal rungs and goal-pull options
(«добро», «сделаю») and narration passages carry no stress (no source data); the inside cover
remains statistics-first; read-aloud still hands the microphone over after the model plays;
`testEverySpokenStringHasARecording` is red on `main` too (23 grammar-lane sentences whose clips
and index entries are being generated in the main checkout, uncommitted).
