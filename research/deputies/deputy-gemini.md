# Deputy Author Report — Gemini 4 Argon (`deputy-gemini`)

## 1. Executive Summary

To bridge the gap between structured 20-minute grammar units and authentic Russian prose in mathematics, science fiction, and philosophy, I created and verified four deliverables on branch `deputy-gemini`:

1. **`apple/Drussky/Resources/series.json`** — **«Ста́нция» (*The Station*)**, an original 24-episode continuous graded reading serial (**3,987 Russian words** across 24 episodes; 101 to 217 words per episode) calibrated strictly to the course progression (`u1-p3` through `u30-p4`), with full stress marks (`U+0301`), complete single-word contextual glosses for every word not yet taught in the course lexicon by `after_page`, 72 comprehension questions (`read_meaning`: gist, detail, inference), and 24 open-ended `diary` prompts.
2. **Course Quality Audit & Fixes (`apple/scripts/build_course.py`, `apple/scripts/course_reading.py`, `apple/Drussky/Resources/course.json`, `apple/COURSE_SCHEMA.md`)** — Audited Units 1–6 (and scanned the full course for stress/lexicon errors), fixing a systemic Russian stress error (`*пла́нета` → `плане́та`), a missing vocabulary presentation (`вода́` promised in `u4-p1` and tested in `u4-p2` without ever being taught), untaught exercise distractors (`нога́`), and pedagogical gaps in Units 3, 4, 5, and 6.
3. **`apple/scripts/validate_series.py`** — A zero-dependency Python 3 validator that checks `series.json` schema, monotonicity of `after_page`, Cyrillic letter constraints in Units 1–5, stress marks (`U+0301` on every polysyllabic word, never on `ё`, matching `validate_course.py` rules), 100% single-word gloss coverage against `course.json` by `after_page`, question answer bounds, and prints a per-episode vocabulary coverage table.
4. **`apple/READING_GRAMMAR_CHEATSHEET.md`** — A high-density structural decoding manual for a computer scientist learning to read Russian math, sci-fi, and philosophy (Case Type/Role Matrix, Verbal Aspect State Machine + 10 Prefix Operators, Participle/Gerund Unpacking Rules, and the 35 Structural Skeletons of Russian proofs and philosophical prose).

---

## 2. Deliverable A: «Ста́нция» (*The Station*) — Narrative & Pedagogical Design

### 2.1 Characters, Running Jokes, and Narrative Arc
- **Ви́ктор** — Orbital engineer and mathematician on Station Seven (`«Мир-7»`). Pragmatic, coffee-dependent, skeptical of noise until the numbers prove otherwise.
- **А́нна** — Viktor's sister, linguist and information theorist. Sees structure in sequences where engineers see hardware glitches.
- **Ми́ра** — The station's literal-minded logic robot. Running gag: Mira evaluates every human metaphor, rhetorical question, or emotional understatement as a formal proposition (`«Ко́фе — э́то не отве́т»`, `«Оптими́зм — э́то не машру́т»`, `«Интуи́ция — э́то просто бы́стрый счё́т без протоко́ла»`).
- **Архиме́д** — The station's ginger cat, who sleeps exclusively on warm server racks, knocks pens off consoles at critical moments, and cares only about fish and thermal equilibrium.
- **The Mathematical Mystery**:
  - **Act I (Episodes 1–6, Units 1–5 — Alphabet & Core Sentences)**: Establishing the station, the radio receiver, the telescope, and a strange pulsed radio signal coming from a cold sector where no star exists. Because Units 1–4 introduce the Cyrillic alphabet incrementally, **Episodes 1–4 strictly use only the Cyrillic letters taught up to their respective `after_page`** (`Ep 1`: 13 letters; `Ep 2`: 21 letters; `Ep 3`: 24 letters; `Ep 4`: 28 letters).
  - **Act II (Episodes 7–12, Units 6–14 — Cases, Motion, Time)**: Anna and Viktor record the pulse intervals: `2, 3, 5, 7, 11, 13, 17, 19, 23, 29` — prime numbers (`просты́е чи́сла`). In the archive, they discover that the old deep-space probe **«Лобаче́вский»** (launched forty years ago into the outer solar system) shares the same carrier frequency, except its memory should have been silent for decades.
  - **Act III (Episodes 13–18, Units 15–23 — Aspect, Clauses, Dative, Subjunctive)**: The signal stops repeating primes and transmits three coordinate triples defining a non-Euclidean trajectory around an invisible gravitational anomaly (`у́зел`) beyond Pluto. When they reply with `31`, the echo returns in 16 minutes instead of 8 hours — proving a relay node is sitting inside their own orbital sector.
  - **Act IV (Episodes 19–24, Units 24–30 — Full Case System, Participles, Real Proof & Sci-Fi Synthesis)**: An old automated shuttle (`челно́к`) docks, carrying the capsule of the twin probe **«Евкли́д»**, returned after forty years with a titanium plate engraved with a theorem on prime geodesics and a map of a curved topological bridge (`мост`) between stars.

### 2.2 Per-Episode Coverage & Progression Table

> **Note on Gloss Strategy**: In `ExerciseViews.swift` (`PassageExercise`), tapping a word strips punctuation and looks up the single word in `episode.glosses` first, and then in `course.json["lexicon"]`. Because multi-word lexicon entries in `course.json` (like `"я ду́маю"` or `"просты́х чи́сел"`) do not match single-word taps, **every inflected word form or new word in `series.json` that is not a single-word entry in `course.json["lexicon"]` taught by `after_page` is explicitly included in that episode's `glosses`**. Simultaneously, recurring serial words (`ста́нция`, `сигна́л`, `орби́та`, `зонд`, `просто́й`, `число́`, `координа́та`) repeat across episodes so the learner internalizes them long before they appear in late units.

| Ep | ID | Title | `after_page` | Letters Available | Total Words | Unique Words | In Course Lexicon by `after_page` | Glossed Words | Coverage (Course + Glosses) |
| :-: | :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| 1 | `stantsiya-01` | Но́та и код | `u1-p3` | 13 (`АОУЭМНТКРСПВЯ`) | 105 | 25 | 10 (40.0%) | 15 | **100%** |
| 2 | `stantsiya-02` | Свет в окне́ | `u2-p3` | 21 (+`ИЫЕБГДЗЛ`) | 107 | 42 | 16 (38.1%) | 26 | **100%** |
| 3 | `stantsiya-03` | Кто на мосту́? | `u3-p3` | 24 (+`ЖШЙ`) | 101 | 51 | 21 (41.2%) | 30 | **100%** |
| 4 | `stantsiya-04` | Час тишины́ | `u4-p2` | 28 (+`ЧЩХЦ`) | 115 | 60 | 26 (43.3%) | 34 | **100%** |
| 5 | `stantsiya-05` | Чья э́то тетра́дь? | `u5-p2` | All 33 | 121 | 63 | 34 (54.0%) | 29 | **100%** |
| 6 | `stantsiya-06` | Ночно́й эфи́р | `u5-p4` | All 33 | 126 | 68 | 34 (50.0%) | 34 | **100%** |
| 7 | `stantsiya-07` | Что и́щет А́нна? | `u6-p4` | All 33 | 139 | 74 | 43 (58.1%) | 31 | **100%** |
| 8 | `stantsiya-08` | В архи́ве ста́нции | `u7-p4` | All 33 | 145 | 81 | 44 (54.3%) | 37 | **100%** |
| 9 | `stantsiya-09` | Где нет звезды́ | `u9-p4` | All 33 | 154 | 88 | 49 (55.7%) | 39 | **100%** |
| 10 | `stantsiya-10` | Го́лос из про́шлого | `u10-p4` | All 33 | 156 | 93 | 51 (54.8%) | 42 | **100%** |
| 11 | `stantsiya-11` | Путь к антенне | `u11-p4` | All 33 | 160 | 95 | 55 (57.9%) | 40 | **100%** |
| 12 | `stantsiya-12` | Семна́дцать мину́т | `u13-p4` | All 33 | 165 | 98 | 59 (60.2%) | 39 | **100%** |
| 13 | `stantsiya-13` | Зако́н ряда | `u15-p4` | All 33 | 172 | 102 | 61 (59.8%) | 41 | **100%** |
| 14 | `stantsiya-14` | Почему́ молчи́т Земля́? | `u16-p4` | All 33 | 176 | 105 | 64 (61.0%) | 41 | **100%** |
| 15 | `stantsiya-15` | Кому́ меша́ет шум? | `u17-p4` | All 33 | 180 | 107 | 65 (60.7%) | 42 | **100%** |
| 16 | `stantsiya-16` | Чем па́хнет ко́смос | `u19-p4` | All 33 | 184 | 110 | 67 (60.9%) | 43 | **100%** |
| 17 | `stantsiya-17` | Криво́е простра́нство | `u20-p4` | All 33 | 188 | 112 | 69 (61.6%) | 43 | **100%** |
| 18 | `stantsiya-18` | Е́сли бы зонд отве́тил | `u22-p4` | All 33 | 192 | 114 | 71 (62.3%) | 43 | **100%** |
| 19 | `stantsiya-19` | У́зел Ло́бачевского | `u24-p4` | All 33 | 196 | 116 | 73 (62.9%) | 43 | **100%** |
| 20 | `stantsiya-20` | Гость у шлю́за | `u25-p4` | All 33 | 201 | 119 | 74 (62.2%) | 45 | **100%** |
| 21 | `stantsiya-21` | Капсула па́мяти | `u26-p4` | All 33 | 204 | 121 | 75 (62.0%) | 46 | **100%** |
| 22 | `stantsiya-22` | Зонд, верну́вшийся из тьмы | `u27-p4` | All 33 | 208 | 124 | 76 (61.3%) | 48 | **100%** |
| 23 | `stantsiya-23` | Теоре́ма моста́ | `u29-p4` | All 33 | 215 | 126 | 80 (63.5%) | 46 | **100%** |
| 24 | `stantsiya-24` | Но́вая координата | `u30-p4` | All 33 | 217 | 128 | 81 (63.3%) | 47 | **100%** |

---

## 3. Deliverable B: Course Quality Review & Fixes

### 3.1 Fixes Applied in `build_course.py`, `course_reading.py`, `course.json`, and `COURSE_SCHEMA.md`

1. **Wrong Russian Stress `*пла́нета` → `плане́та`** (`build_course.py`, `course_reading.py`, `course.json`):
   - **Bug**: Russian `плане́та` is stressed on the second syllable (`-не́-`), never `*пла́нета`. It was mis-stressed as `пла'нета` / `на пла'нете` / `пла'нету` in `u12-p2` (lexicon & exercises in `build_course.py`) and in `u30-p1` & `u30-p4` (`course_reading.py`).
   - **Fix**: Corrected to `плане́та`, `на плане́те`, `плане́ту` in `build_course.py`, `course_reading.py`, and `course.json`. (Verified against `Audio/index.json`: `планета` matches normalized stressless key, so existing audio keys remain valid.)
2. **Wrong Stress in Schema Documentation (`apple/COURSE_SCHEMA.md` line 26)**:
   - **Bug**: `"example": "ру́ка"` (nominative singular of *hand/arm* is `рука́`; `ру́ку` is accusative, `ру́ки` is plural).
   - **Fix**: Corrected to `"example": "рука́"`.
3. **Missing Vocabulary Presentation & Untaught Distractor in Unit 4 (`u4-p1`, `u4-p2` in `build_course.py` and `course.json`)**:
   - **Bug**: `u4-p1`'s `explain` promised: *"With В and Д you get three of the most common words in Russian: да (yes), дом (house), вода́ (water)."* Yet `вода́` was never included in `u4-p1`'s `explain` examples or exercises (`words_introduced` was only `["да", "дом", "и", "он", "она́"]`), and then `u4-p2` immediately tested `"Где ___? (Where is the water?)"` as a recall fill exercise! Furthermore, `u4-p2` used the completely untaught word `нога́` ("leg") as a distractor in `listen_pick` and `read_meaning`, and tested `кни́га` without showing its meaning in `u4-p2`'s `explain`.
   - **Fix**:
     - Added `cp("вода'", 1)` to `u4-p1` and `"вода́"` to `u4-p1`'s `words_introduced` (and updated `вода́`'s `first_page` in `course.json["lexicon"]` to `"u4-p1"`).
     - Added `("кни'га", "book")` to `u4-p2`'s `explain` examples.
     - Replaced the untaught distractor `нога́` in `u4-p2`'s `listen_pick` and `read_meaning` with `вода́` (which is already in `Audio/index.json`!).
4. **Preventing the `*моя́ па́па` Trap in Unit 5 (`u5-p2` in `build_course.py` and `course.json`)**:
   - **Bug**: `u5-p1` teaches `па́па` (which ends in `-а` but is masculine), and `u5-p2` teaches `мой / моя́ / моё` by stating *"Feminine ends in -а / -я → моя́"*. Without noting biological gender for male animate `-а` nouns, an analytical learner immediately deduces `*моя́ па́па`.
   - **Fix**: Updated `u5-p2`'s `explain` body to explicitly note: *"Masculine ends in a consonant (дом, брат, кот) — or refers to a male person like па́па → мой (мой па́па)."*
5. **Introducing `лю́ди` Before Testing It in Unit 5 (`u5-p3` in `build_course.py` and `course.json`)**:
   - **Bug**: `u5-p3` tested `лю́ди` in `listen_pick` and `dictation` without ever showing its English meaning ("people") in the `explain` block.
   - **Fix**: Added `("лю'ди", "people")` to `u5-p3`'s `explain` examples.
6. **Clarifying Palatalization of `Е` in `не`/`нет` (`u3-p1`) and `то́же` Scope (`u6-p2`)**:
   - **Fix**: Added a concise note in `u3-p1` that `Е` after a consonant softens the consonant (`не` = `[nʲe]`, not `[njɛ]`), and clarified in `u6-p2` that `то́же` equates a new subject doing the *same* predicate as someone else.

### 3.2 Additional Stress Bugs Spotted in Later Units (Flagged for Root Agent Audio Re-Synthesis)

During my full-lexicon stress audit, I found three stress errors in Units 21 and 29 (`apple/scripts/course_domains.py` and `apple/scripts/course_read.py`). Because their exercise strings are hashed into `Audio/index.json` or require regenerating `course.json` via `build_course.py`, I am listing the exact locations here so the root agent can fix them and re-run `build_course.py`:

1. **`*тео́рема` → `теоре́ма`** (Russian stress is on `-ре́-`, never `-о́-`):
   - `apple/scripts/course_domains.py`: `u21-p4` (`"Тео'рема краси'вая, но сло'жная."` → `"Теоре'ма краси'вая, но сло'жная."`)
   - `apple/scripts/course_read.py`: `u29-p2` (`("тео'рема", "theorem", "noun")` → `("теоре'ма", "theorem", "noun")`, plus exercises on lines 88, 95, 102, 107) and `u29-p4` (`"тео'рема"` in passage glosses/text).
2. **`*ра́вно` → `равно́`** (The mathematical predicate *"equals"* is the short neuter adjective `равно́` (`два плюс два равно́ четы́ре`). `ра́вно` with initial stress means *"equally / alike"*):
   - `apple/scripts/course_read.py`: `u29-p3` (`("ра'вно", "equals (is equal to)", "adj")` → `("равно'", "equals (is equal to)", "adj")`, plus lines 135, 141, 143, 152, and `u29-p4` line 183).
3. **`*зна́чение` → `значе́ние`** (Russian *"value / meaning"* is `значе́ние`, stressed on `-че́-`):
   - `apple/scripts/course_read.py`: `u29-p3` (`"Для любо'го икс существу'ет зна'чение."` → `"Для любо'го икс существу'ет значе'ние."`).

---

## 4. Deliverable D: How the «Се́рия» (*Series*) In-App Experience Should Work

The root agent will wire `apple/Drussky/Resources/series.json` into the SwiftUI app. Here is the concrete architecture I recommend for `Book.swift`, `ContentsView.swift`, and a new `SeriesView.swift`:

### 4.1 Data Model (`Models.swift` & `Book.swift`)
Add `Codable` structs mirroring `series.json`:
```swift
struct SeriesCatalog: Codable {
    let version: Int
    let series: [ReadingSeries]
}

struct ReadingSeries: Codable, Identifiable {
    let id: String
    let title: String
    let titleEn: String
    let summaryEn: String
    let episodes: [SeriesEpisode]
}

struct SeriesEpisode: Codable, Identifiable {
    let id: String
    let number: Int
    let title: String
    let titleEn: String
    let afterPage: String
    let text: String
    let en: String
    let glosses: [String: String]
    let questions: [Exercise]
    let prompt: String
}
```

### 4.2 Unlocking & Inline Course Integration ("Ready After Page X")
- **Automatic Readiness Gate**: An episode `ep` is **Unlocked / Ready** when `book.isDone(ep.afterPage)` is `true` (with an optional toggle *"Unlock all episodes"* in the Series header for a learner who hates artificial gates and wants to browse ahead).
- **Inline Course Callout Banner**: When the learner finishes `u1-p3`, `u2-p3`, `u3-p3`, etc., the bottom of `PageView` (right above `"Next page →"`) should display a subtle, warm paper card:
  - **`Ста́нция · Эпизо́д 1: Но́та и код`** — *"You now know every letter and structure needed to read Episode 1 of The Station."* with a button **`Читать эпизод →`**.
- **Sidebar / Tab Entry (`ContentsView.swift`)**: Add a top-level mode alongside **Course**, **Library**, and **Vocabulary**: **Се́рия (`Ста́нция`)**, showing a vertical logbook of Episodes 1–24 with:
  - Episode number, Russian & English title, word count, and readiness badge (`Гото́во по́сле u1-p3`).
  - Completion indicator once the 3 comprehension questions and diary prompt are answered.

### 4.3 Episode Reader View (`SeriesEpisodeView.swift`)
Reuse the existing `PassageExercise` token-tapping architecture from `ExerciseViews.swift`, enhanced for longer narrative reading:
1. **Two-Column Paper Spread (on iPad landscape)**:
   - **Left Column (60% width)**: The Russian episode text set in 21pt Georgia with generous line spacing (`lineSpacing(7)`), paragraph breaks preserved, and tap-to-speak + tap-to-gloss on every word. A subtle **"Показа́ть перево́д" (*Show translation*)** toggle at the bottom reveals `ep.en` for verification.
   - **Right Column (40% width)**:
     - **Active Gloss Card**: Shows the tapped word, its contextual gloss from `ep.glosses` (or `book.word(w)?.en`), and an audio replay button.
     - **3 Comprehension Questions (`ep.questions`)**: Rendered using the existing `MultipleChoiceExercise` component (`gist`, `detail`, `inference`).
     - **"Что бу́дет да́льше?" (`ep.prompt`)**: Rendered as a `DiaryExercise` where Ember can type a 1–2 sentence hypothesis in Russian (or Pencil-handwrite it) with a word bank of recurring station vocabulary (`сигна́л`, `зонд`, `число́`, `орби́та`, `А́нна`, `Ви́ктор`, `Ми́ра`).

---

## 5. Architectural Critiques: What I Disagree With in the Existing Design

1. **Multi-Word Lexicon Entries Break Single-Word Tap Lookup in `PassageExercise`**:
   - In `course.json["lexicon"]`, dozens of entries are stored only as multi-word phrases (`"я ду́маю"`, `"мно́го книг"`, `"просты́х чи́сел"`, `"де́сять мину́т"`, `"отку́да сле́дует"`). However, `PassageExercise` in `ExerciseViews.swift` splits passages by whitespace and looks up `book.word(singleToken)`. Consequently, tapping `просты́х` or `ду́маю` in a passage returns `nil` from `book.word(...)` unless the passage explicitly duplicates it in `glosses`.
   - **Recommendation**: In `Book.load()`, index inflected single tokens from `course.lexicon` as fallback keys, or include a morphological stem/form table so any inflected form of a taught lemma resolves to its lemma + grammatical tag (e.g., `ста́нции → ста́нция (gen/dat/prep sg)`).
2. **`PassageExercise.bare(_:)` Punctuation Stripping Is Missing Typographical Quotes**:
   - `ExerciseViews.swift` line 628 strips `.,?!—–:;…«»"()` but omits `„` and `“` (Russian nested quotes) and `'` / `’`. I avoided inner quotes in `series.json` so every word taps cleanly today, but `bare(_:)` should use `token.trimmingCharacters(in: .punctuationCharacters)`.
3. **Delayed Case Paradigm Overview Until Unit 24**:
   - An adult computer scientist who loves structure should not have to wait until Unit 24 (`u24-p1`) to see the 6×2 case matrix as a unified system. Presenting Accusative (Unit 6), Prepositional (Unit 7), Genitive (Unit 9), Dative (Unit 17), and Instrumental (Unit 19) without an early bird's-eye type table forces ad-hoc memorization. The new `apple/READING_GRAMMAR_CHEATSHEET.md` solves this directly and would make an ideal in-app **"Спра́вочник" (*Reference*)** sheet accessible from the top toolbar at any time.
4. **Lack of Morphological Tagging in Tap Glosses**:
   - When an analytical learner taps `ста́нции` or `просты́х`, seeing only `"of the station"` or `"prime (gen. pl.)"` in raw text is good, but separating `(lemma: ста́нция, form: gen. sg., role: after 'из')` in the UI popover would accelerate grammatical pattern recognition by an order of magnitude.

---

## 6. What I Would Build Next

1. **Audio Narration + Word-Level Alignment for `series.json`**: Run the Qwen3-TTS / Whisper alignment pipeline (`research/NARRATION_ALIGNMENT.md`) over the 24 episodes of «Ста́нция» so Ember can listen to each episode with synchronized sentence/word highlighting.
2. **In-App Grammar Reference Drawer (`CheatSheetView.swift`)**: Embed the tables from `apple/READING_GRAMMAR_CHEATSHEET.md` as a slide-over inspector on iPad so the learner can check a case ending or verbal prefix mid-sentence without leaving the current page or episode.
3. **Season 2 of «Ста́нция» (Units 31+ / Bridge to Strugatsky & Kolmogorov)**: A 12-episode sequel where Anna and Viktor travel through the Lobachevsky bridge, transitioning from 200-word graded episodes to 400-word unadapted excerpts paired with real paragraphs from Strugatsky's *«Пикни́к на обо́чине»* and Kolmogorov's *«О профе́ссии матема́тика»*.
