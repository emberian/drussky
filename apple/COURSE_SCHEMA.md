# Drussky course file — `apple/Drussky/Resources/course.json`

The iPad app is a **workbook**: the learner turns pages, and each page is a column of
numbered exercises answered *in place* — mostly by writing with the Pencil on ruled paper,
sometimes by speaking or tapping. Their ink and recordings stay on the page forever.
The app interleaves spaced-review exercises into pages on its own; this file holds the
authored new material only.

## Top level

```json
{
  "version": 1,
  "letters": [Letter, …],     // all 33, alphabet order
  "lexicon": [Word, …],       // every word/phrase the course teaches with a meaning
  "units":   [Unit, …]
}
```

### Letter
```json
{ "upper": "Р", "lower": "р", "say": "эр",
  "sound": "a tapped or rolled r",
  "family": "familiar | false_friend | new | sign",
  "tip": "Looks like P, sounds like r.",
  "example": "рука́" }
```
`say` is what the app speaks when the learner taps the letter: the letter's *sound* as a
syllable or vowel (e.g. "ра" or "а"), not necessarily its alphabet name. `tip` is one short,
memorable line in the tutor's voice.

### Word (lexicon)
```json
{ "ru": "кни́га", "en": "book", "emoji": "📕", "note": "feminine: ends in -а" }
```
`note` is optional. `emoji` optional but welcome for concrete nouns.

### Unit
```json
{ "id": "u3", "title": "Pronouns", "subtitle": "я ты он она оно мы вы они",
  "teaches_cursive": true,
  "pages": [Page, …] }
```
`subtitle` and `teaches_cursive` (boolean, `true` on the cursive unit) are optional.

### Page
```json
{ "id": "u3-p2", "title": "He, she, it",
  "intro": "One or two sentences, warm and direct, saying what this page is for.",
  "exercises": [Exercise, …] }
```

## Exercises (`kind` selects the shape)

| kind | fields | what the learner does |
|---|---|---|
| `meet` | `letter` | Letter card: big print + model, tap to hear, the tip; then traces it on ruled paper. |
| `explain` | `text`, optional `examples:[{ru,en}]` | Reads a short tutor note (≤ 3 sentences). Examples are tappable to hear. |
| `trace` | `text` | Writes over a faint model of `text` on the ruled line. |
| `copy` | `text`, `times` (1–3) | Copies `text` beside the model. |
| `listen_pick` | `say`, `options:[…]` | Hears `say`, taps which option it was. `say` must be one of `options`. |
| `dictation` | `text` | Hears `text` (model hidden), writes it. Checked by handwriting recognition. |
| `read_aloud` | `text` | Sees `text`, says it aloud. Checked by speech recognition; model playable after. |
| `translate` | `en`, `answer` | Sees English, writes the Russian from memory. `answer` must be in the lexicon. |
| `fill` | `before`, `after`, `hint`, `answer` | Writes the missing word: `before` + ___ + `after`. `hint` is the English of the whole sentence. |
| `read_meaning` | `text`, `options:[en…]`, `answer` (index) | Reads Russian, picks its meaning. |
| `passage` | `text`, `en`, optional `glosses:{word: meaning}` | Reads a short text (2–8 sentences); taps any word to see its meaning and hear it; can play the whole text and reveal `en`. Words missing from `glosses` fall back to the lexicon. Gloss keys are the word as written, lowercase, without stress. |
| `build_word` | `en`, `answer`, `parts:[tile…]`, optional `options:[tile…]`, `alternatives:[ru…]` | Sees the English, taps morpheme tiles into order (`пере-`, `пис`, `-а́ть`), checks. `parts` spell `answer` in order (hyphens and stress dropped); `options` is every tile offered, in display order (defaults to `parts`); `alternatives` are other words the tiles may spell that also count. `answer` must be in the lexicon and be a lemma or cognate of `morphology.json`. Right at the first check is recall; two misses show the word. |
| `family_guess` | `text`, `word`, `root`, `options:[en…]`, `answer` (index), `en` | Reads a sentence with one new word (`word`, a `morphology.json` lemma built on `root`, a root the learner knows), guesses its meaning before being told (the pretesting effect), then sees the word taken apart. Every other word of `text` must be in the lexicon. |
| `paradigm_fill` | `word`, `paradigm`, `forms:{cell: form}`, `cells:[cell…]`, optional `options:[form…]` | Sees a `paradigms.json` table of `word` with every cell of `forms` drawn (its ending shown small in each); the cells in `cells` are empty and to be produced — written with the Pencil in a small ruled cell each, or, when `options` is present, filled by tapping tiles (every asked form, plus tiles that don't belong). One **Check** grades the table; all right at the first check is recall, a later success practice, a second miss shows the forms in red with the paradigm row. `forms` must equal the word's forms in `paradigms.json`; `paradigm` is its table (`past`/`imp` for a verb's past and imperative cells). |
| `transform` | `word`, `cue`, `answer`, optional `paradigm`, `before`, `after`, `hint` | Sees `word` and a grammatical cue («кни́га» → genitive singular), or a sentence with a gap that forces the form (`before` ___ `after`, `hint` its English), and writes the form. `answer` is `word`'s `cue` cell in `paradigms.json`; the checker accepts it with or without stress marks and with ё written е. A miss shows the paradigm row with the right cell marked. |
| `cue_sort` | `options:[bucket…]`, `items:[{text, answer}]`, optional `paradigm` | Sorts phrases into the case use (or table) they show: tap a phrase, then its bucket. Buckets are `paradigms.json` use titles (у меня́ есть, в / на + where …) or table titles. A wrong bucket sends the phrase back once; twice places it in red. Flawless is recall, slips are practice, a phrase placed for you is a miss. Every word of every item must be in the lexicon. |
| `read_handwriting` | `text`, optional `options:[ru…]`, optional `seed` | Reads `text` (a word or short line) rendered in somebody's handwriting — one of the bundled OFL handwriting fonts or the cursive stroke models written through, slanted, sized and inked by `seed` (a non-negative integer; absent, one is derived from the text). Without `options` the learner prints the word on the ruled line (recognized, counts as writing); with `options` (2+, `text` among them, the others look-alikes such as ма́ша for ма́ма) they pick it. Knowledge modality `read-cursive`. Pages from ＋ → *Read handwriting* are built locally from known words. |
| `diary` | `prompt` | Дневник: writes a few words of their *own* Russian on ruled lines about `prompt` (English, or Russian obeying rules 1–2). The recognizer proposes a transcript they confirm or fix; the tutor returns a red-pen correction (corrected text, edits with a category and a reason, one line of praise, 0–2 patterns to teach), drawn over their ink, which stays. The prompt is never spoken. At most one per page, always last, and not counted in rule 5. From unit 3 on, `build_course.py` adds one to every page (`DIARY_PROMPTS`). |

A `passage` may carry a human reading: `narration` (an `Audio/` clip) and `words` (its word-timing
file, `Audio/narr-<id>.words.json`); the app then plays the reader with the word being said lit up,
replays from a tapped word, slows to ¾ speed, and can hide the text until it is read. A `dictation`
or `read_aloud` built from one sentence of that reading carries the same two fields plus
`span: [first, last]` — the sentence's word indices in the timings — so the clip plays just that
stretch (chunk dictation, shadowing). These exercises are made locally from a narrated library text
(*Listen and read along*), never authored in course.json. The timing file is
`{"version": 1, "text": "<library id>", "file": "<clip>", "duration": s, "reader": "…",
"words": [["слово", start, end], …]}`, where `words` are, in order, every whitespace token of the
text whose bare form (outer punctuation and stress marks off) contains a letter; digits, `***` and
dashes have no entry. `scripts/narration.py` writes and checks it; `Narration.swift` applies the
same rule.

A `trace` or `copy` may carry `cursive: true`: its ghost is then the cursive stroke model
(`Resources/strokes-cursive.json`, built by `scripts/build_strokes_cursive.py`, joined up letter to
letter) whatever the learner's cursive setting, and the one-hint coach compares with the cursive
models. A cursive `copy` shows the hand once on the line (the print word above it) and leaves room
to copy; neither fades (trace → copy → dictation) however secure the word, since they teach the
hand, not the word. The cursive unit (`scripts/course_cursive.py`) is made of these; it introduces
no words.

Any exercise may also carry an optional `skills: ["past", "negation"]`: the grammar patterns
it practises, by construction id from `Resources/lemmas.json` (`gram:` prefix optional). When it is
absent, the knowledge model tags the exercise from its words (lemma map plus a few word-order
rules), so authoring it is only needed where the tagger would be wrong.

## Rules

1. **Letters before use.** A Russian string in any exercise may only use letters already met
   (in an earlier page, or by a `meet` earlier on the same page). Space and `.,?!—-` are free.
   `scripts/validate_course.py` enforces this.
2. **Stress marks.** In words of 2+ syllables mark the stressed vowel with U+0301
   (`ма́ма`, `кни́га`), except when it is `ё`. The app strips marks for audio lookup and
   handwriting checks, and the learner never has to write them.
3. **Words have meanings.** Anything used in `dictation`, `translate`, `fill`, `read_meaning`
   should be a real word or sentence in the lexicon (or a sentence built from lexicon words),
   not a nonsense syllable. Syllables like `ма` are fine in `trace`/`copy`/`listen_pick`/`read_aloud`
   while letters are brand new.
4. **Voice.** A good private tutor sitting beside a smart adult. One line of instruction at
   most; exercise kinds already say what to do. No hedging, no disclaimers, no "optional",
   no talk about recordings, models, accuracy or the app itself. No mention of friends or coaches.
5. **Size.** A page is 6–10 exercises, ~8–12 minutes. Writing (trace/copy/dictation/translate/fill)
   is the majority on a `purpose: "writing"` page (the default for old pages), which also
   mixes in hearing and speaking. A `purpose: "reading"` page has a majority of
   passage/read_meaning/family_guess tasks and need not demand production. A `purpose: "oral"`
   page has mostly read_aloud/listen_pick tasks with at least one spoken response.
   Diary lines remain last, outside these counts. Lexicon entries may be receptive vocabulary.

# Library — `Drussky/Resources/library.json` (and `PrivatePack/library.json`)

Real texts to read beyond the course, at several levels. The app shows them on the inside
cover by level; opening one makes a reading page from it — either the original (glossed) or
rescaled by the tutor to the learner's current vocabulary.

```json
{
  "version": 1,
  "texts": [
    { "id": "tolstoy-filippok", "title": "Филиппок", "title_en": "Filipok",
      "author": "Лев Толстой", "year": 1875, "level": "A2",
      "topic": "classic | scifi | math | science | philosophy | everyday",
      "text": "the Russian text or excerpt, ≤ 3000 characters, paragraphs separated by \n",
      "summary_en": "one line saying what it is and why it's worth reading",
      "license": "public domain | CC BY 2.0 FR | …", "source": "https://…" }
  ],
  "sentences": [
    { "ru": "…", "en": "…", "audio": "file.m4a", "speaker": "…", "license": "…", "source": "https://…" }
  ]
}
```

Levels follow CEFR (A1 A2 B1 B2 C1). `PrivatePack/` (git-ignored, never in a release build)
has the same layout for material that is fine for personal study but not redistributable.

A text read by a human (LibriVox, Wikimedia Commons) also has `narration` (its clip in `Audio/`,
registered in `Audio/index-narration.json` under the key `narration:<id>` with provenance in
`Audio/sources-narration.json`), `words` (the word-timing file) and `reader`. The library shows a
headphone icon for it and offers *Listen and read along*, which builds the read-along page above
without the tutor. Texts without the three fields are read by the synthetic voice. `scripts/narration.py`
(`align`, `build`, `check`) produces all of it from `scripts/narration_sources.json`;
`validate_library.py` checks that the three fields, the manifests and the timings agree.

## Original reading collections — `Drussky/Resources/math.json`

A collection uses the same version-1 `texts` / `sentences` envelope. It may additionally
carry `collection: {id, title, title_en, description_en}` (all strings). Mathematics has
24 original CC0 readings, ordered by `rung`. `math.json` is the editable source; it is
not generated by `build_course.py`. Its `source` identifies this repository as the place
of original publication; `references`, where present, identify consulted mathematical
sources, not the source of copied wording.

All of the following text fields are **optional extensions** to the Library contract
(required within this particular mathematics collection):

| Field | Shape and meaning |
|---|---|
| `rung` | Integer, starting at 1, defining collection order. |
| `after_page` | Existing course page **id**, such as `u18-p2`; support recommendation after completing that page, not an access gate. Compare course array order, not numeric unit labels. |
| `en` | Faithful full English translation with the same paragraph breaks as `text`. |
| `glosses` | `{surface_form: contextual English meaning}`; lowercase, unstressed Russian keys, preserving ё and internal hyphens (`по-прежнему`). Inflections are separate keys. Like course passage glosses, a key applies to every occurrence of that form in the reading. Where two senses or cases occur, describe both explicitly. |
| `questions` | Three objects `{kind, prompt, options, answer, explanation}`. `kind` is `gist`, `detail`, or `inference`; `options` has three English strings; `answer` is a zero-based integer. Prompt and explanation are English, with stressed Russian quotations where useful. Inference questions test scope, agreement, hypotheses, or dependencies. |
| `how_read` | 3–5 objects `{ru, en, note}`: a stress-marked Russian frame, its contextual English reading, and an English note about case, aspect, agreement, or argument structure. |
| `notation` | Objects `{symbol, ru, en}`: untouched mathematical notation, a stress-marked Russian reading, and an English explanation. Latin variables remain placeholders; the audio renderer must supply their spoken names. This is separate from prose and from prose word counts. |
| `references` | Optional objects `{url, note}`: HTTPS reference and English explanation. No borrowed prose is implied. |

Russian in titles, passages, frames, notation readings, and quotations carries U+0301
on every polysyllable without ё; ё carries no acute. Gloss keys and symbolic notation
are deliberately exempt. `⊂` means **proper** subset here; `⊆` permits equality.
Natural numbers in these readings start at 1. `level` describes the supported reading
register, not the learner's general CEFR competence. Short predicate adjectives such
as `равно́` and `откры́то` are distinguished from short passive participles such as
`определена́` rather than being labelled participles indiscriminately.

`python3 apple/scripts/validate_math.py` checks the base Library schema, these nested
fields, lengths (60–220 **Russian word tokens**, excluding variables and numbers),
stress syntax, answer indices, course-page order, and gloss coverage. It reports exact
surface-form course coverage and, separately, earlier shelf exposure. A lexicon entry
counts only after a meaning-presentation event, following the policy in
`validate_course.py`; multiword entries expose their component forms only when the
whole entry is presented. Cognate recognition and automatic inflection inference do
not inflate the counts. Missing glosses include Russian in reading frames, notation
readings and question quotations, not just the passage. The tool detects missing or
malformed stress marks (and a small regression list of audited stress placements),
not all incorrect stress placements or mathematical errors;
those still need editorial review. Run the general checks with:

```sh
python3 apple/scripts/validate_library.py apple/Drussky/Resources/math.json
python3 -m unittest discover -s apple/scripts -p 'test_validate_math.py'
```

**In the app:** `Library.load()` merges the files named in `Library.shelfFiles` (`math`)
behind `library.json`; each text remembers its `shelf` (the collection id) and
`Library.Text` decodes the fields above as Optionals (`rung`, `after_page`, `en`, `glosses`,
`questions`, `how_read`, `notation`). `Shelves.page(for:on:)` (Model/Shelves.swift) turns
a reading into a page: a `passage` carrying `text`, `en`, `glosses` and `frames` (the
`how_read` frames then the `notation` readings, each `{ru, en, note?, symbol?}` — an
Optional field of `Exercise`, shown under the text as *How it is read*), then the three
questions as `read_meaning` exercises whose `text` is the English prompt and `hint` the
explanation. The page's `sourceID` is `reading:<id>`.

# The serial — `Drussky/Resources/series.json`

```json
{ "version": 1, "series": [{ "id": "stantsiya", "title": "Ста́нция", "title_en": "The Station", "summary_en": "…",
    "episodes": [{ "id": "stantsiya-01", "number": 1, "title": "Се́ктор Три", "title_en": "Sector Three",
                   "after_page": "u3-p3", "text": "…", "en": "…", "glosses": { "сектор": "sector" },
                   "questions": [{ "kind": "read_meaning", "text": "…", "options": ["…", "…", "…"], "answer": 0 }],
                   "prompt": "What happens next …" }] }] }
```

`Resources/kitchen.json` («Ку́хня», 30 overheard dialogues) has the same shape; its series
object adds `episode_word: "диало́г"` (what one item is called; absent means «эпизо́д») and
each episode may carry `notice`: one to three stressed strings, Russian first and the English
after, listed under the passage once it has been read (*What to hear*). Dialogue lines start
with «— » and run 25–80 words; a serial episode 100–250. `Serial.files` names the files.

Episodes are numbered from 1 and their `after_page`s are non-decreasing in course order.
`text` and the question texts carry stress on every polysyllable without ё; episodes before
the alphabet is complete use only letters met by `after_page`; every word of `text` not in the
course lexicon by `after_page` is in `glosses` (lowercase, unstressed keys); `en`, options and
glosses are English (Russian only inside a gloss's parenthesis). `questions` are three
`read_meaning` exercises in the course's own format. `scripts/validate_series.py [file]` checks
all of this and prints per-episode coverage. In the app (`Serial.load()`, `Shelves.page(for:in:)`)
an episode is a page — `passage` (text, en, glosses), the three questions, a `diary` with the
prompt — with `sourceID` `episode:<id>`; a page's Optional `offers` lists the shelf items its
footer offered when it was composed.

# Listening — `Drussky/Resources/listening.json`

```json
{ "version": 1,
  "sets": [{ "id": "listen-02", "title": "Дай, пожа́луйста", "title_en": "Pass me the right thing", "after_page": "u4-p4",
             "items": [{ "id": "listen-02-01", "kind": "pick_object", "en": "Give me the cheese.", "after_page": "u4-p4",
                         "voice_hint": "any", "glosses": { "дай": "give (informal singular request)" },
                         "ru": "Дай сыр.", "question": "Which thing is wanted?", "options": ["сок", "сыр", "торт"], "answer": 1,
                         "crucial_span": "сыр", "why": "сыр names the cheese." }] }],
  "weekly_samples": [{ "id": "listen-week-01", "title": "Пе́рвые сообще́ния", "title_en": "Unheard messages 1", "after_page": "u14-p5",
                       "items": [ /* five `message` items whose questions have response_mode, answer_en, accept_en */ ] }] }
```

Authored content (editable like `math.json`), the exact contract in
`research/deputies/deputy-codex-listening.md`. Every item has `id`, `kind`, `en` (the translation
of what is played; A/B-labelled lines for an exchange), `after_page`, `voice_hint` (`any` |
`unfamiliar`), `glosses` (lowercase unstressed surface form → meaning; may be `{}`),
`crucial_span` (an exact stressed substring of a heard turn) and `why`. The rest depends on
`kind`, and a field a kind does not use is **absent**, never an empty placeholder; indices are
zero-based:

| kind | fields |
|---|---|
| `pick_object` | `ru`, `question`, `options` (3–4 taught nouns, stressed), `answer` |
| `who_needs`, `disagreement` | `lines: [{speaker: "A"\|"B", ru, en}]` (2–4, both speakers), `question`, `options` (English), `answer` |
| `follow` | `ru`, `question`, `scene: [{id, ru, en, role: "object"\|"place"}]`, `options` (English results), `outcomes: [{object, place\|null}]` parallel to `options`, `answer` |
| `question_or_statement` | `forms: [{ru, en}]` (exactly two: the statement ending `.`, the question ending `?`), `played` (0\|1), `question`, `options: ["Statement", "Question"]`, `answer` = `played` |
| `message` | `ru` (2–4 sentences), `questions: [{kind: gist\|detail, prompt, options, answer, crucial_span, why}]` (2–3); the item-level span/why are a fallback |
| `correction` | `ru` (a statement and its self-repair in one clip), `question`, `options` (English), `answer` |

Weekly messages replace a question's `options`/`answer` with `response_mode: "english_text"`,
`answer_en` and `accept_en` (examples, not a whitelist). `scripts/validate_listening.py` checks
the schema, stress, exact surface-form coverage by `after_page`, course order, answers, spans,
scene references, intonation pairs (distinct audio keys) and the isolation of the reserved
messages from everything else, and `--write-inventory` writes the 338 playback strings to
`research/artifacts/listening-spoken.txt`.

In the app (`Listening.load()`, `ListeningItem` with Optional kind-specific fields) a set is a
page of `listening` exercises — each carries its item in the Optional `listening` field, in a
fixed order that is not the authored one — with `sourceID` `listening:<set-id>`, offered once like
an episode. A result keeps one choice in `chosen`, or a message's choices in `recognized`
("1,2") committed together. Reserved sets are never pages; `openBaseline()` takes a reached set's
five messages as `BaselineTask`s (kind `listening`, the new Optional `text` played by text in the
voice `speaker` names).

# Structure — `Drussky/Resources/library-structure.json`

The skeleton of every library sentence, written offline by `scripts/library_structure.py` (Claude,
`claude-opus-5-5`, structured JSON, one paragraph-sized request at a time, resumable) and checked by
`scripts/validate_structure.py`. The app lays it over any passage made from a library text — as written,
read along, the goal's first page — by the sentence's text alone (`Structure.swift`), and the
«Структу́ра» toggle on the page draws it as a teacher's pencil marks (`StructureViews.swift`).

```json
{
  "version": 1, "model": "claude-opus-5-5",
  "roles": { "subject": { "ru": "подлежа́щее", "en": "subject", "tier": "member" }, "participial_clause": { "…": "…", "tier": "clause" } },
  "texts": {
    "tolstoy-lev-i-mysh": { "sentences": [
      { "index": 1, "text": "Мышь пробежала ему по телу.", "words": [[0, 4], [5, 14], [15, 18], [19, 21], [22, 26]],
        "chunks": [{ "words": [0, 0], "span": [0, 4], "role": "subject" }, { "words": [3, 4], "span": [19, 26], "role": "prepositional_phrase" }],
        "referents": [{ "word": 2, "head": 0, "headSentence": 0, "kind": "pronoun" }],
        "desugar": [{ "words": [3, 7], "span": [19, 50], "kind": "participle", "ru": "лев, кото́рый спал", "en": "the lion that was sleeping" }],
        "roots": [{ "word": 1, "prefix": "про", "root": "беж", "suffix": "а", "inflection": "ла", "rootID": "бег", "gloss": "run",
                    "relatives": ["бе́гать", "побежа́ть"], "source": "morphology" }],
        "governance": [{ "head": 3, "dependents": [4], "frame": "по + Dat", "case": "dat" }] } ] } }
}
```

`index` is the sentence's position in the text (paragraphs split on `\n`, sentences by
`GoalAnalysis.splitSentences`, which the script ports); `text` is the exact source sentence, so an edited
text invalidates its entry visibly. `words` are the sentence's words as `[start, end)` Unicode-scalar
offsets — runs of letters, digits and stress marks, hyphenated compounds as one word (`Structure.words(in:)`
is the same rule) — and every other index refers to them. Member chunks (tier `member`: subject, predicate,
object, attribute, adverbial, prepositional_phrase, …) cover the words without overlapping; clause chunks
(subordinate, relative, participial, gerund, coordinate, direct speech, parenthetical) nest. A referent's
head may be in an earlier sentence of the same text (`headSentence`). Paraphrases and frames carry stress
marks on polysyllables and none on ё. An x-ray's four pieces spell the word as written; when
`morphology.json` knows the word, its root, gloss and taught relatives come from there (`source:
"morphology"`), otherwise from the model, relatives restricted to words the course teaches.

The `structure` exercise is generated from it (never authored): `{"kind":"structure","text":"<sentence>",
"probe":<word index of the pronoun or participle>,"answer":<word index of its head>,"word":"<probe as
written>","skills":["anaphora"|"participle"]}` — the learner taps the head; right first time is a
read-pick `good` on the construction (and the pronoun), right after a wrong tap `again`.

# Stress — `Drussky/Resources/library-stress.json`

Stress marks for the Russian that came from outside the course: every sentence of every library text and
every Tatoeba sentence (the pseudo-text `sentences`, one entry per sentence in file order), written
offline by `scripts/library_stress.py` (Claude, `claude-opus-5-5`, structured JSON, 14 sentences a
request, resumable) and checked by `scripts/validate_stress.py`. The authored shelves (math.json,
series.json, kitchen.json) are scanned too and appear only where a sentence lacked a mark (none do).

```json
{
  "version": 1, "model": "claude-opus-5-5",
  "texts": { "tolstoy-lev-i-mysh": { "sentences": [
      { "index": 1, "text": "Мышь пробежала ему по телу.", "stressed": "Мышь пробежа́ла ему́ по те́лу." } ] } },
  "homographs": [{ "text": "tolstoy-filipok", "index": 13, "word": "больша́я", "alternatives": ["бо́льшая"], "reason": "…" }],
  "disagreements": [{ "text": "pushkin-zimnee-utro", "index": 26, "word": "сне́гу", "course": ["снегу́"] }]
}
```

`text` is the exact source sentence (an edited text invalidates its entry visibly); `stressed` is the same
sentence with U+0301 after the stressed vowel of every word of two or more syllables and nothing else
changed. The rule both scripts and the app (`RussianStress.problems`) enforce: removing the marks gives
the source back exactly (a mark the source already carries stays); one mark per Cyrillic word of two or
more syllables, after a vowel; none on a one-syllable word or a word with ё; in a hyphenated compound each
part follows the rule, except that a compound of one-syllable parts may take one mark in all (кто́-то).
Homographs are the one place the model's judgement decides, listed for review. Every stressed word is
compared with the course's own spellings (course.json, morphology.json, paradigms.json); a word the course
marks differently is kept as the model wrote it and listed under `disagreements`
(`library_stress.py --recheck` recomputes that list when the course changes).

The app (`Model/LibraryStress.swift`) finds a sentence by its unstressed text and shows the stressed form
through `Theme.stressed` — the coloured, underlined vowel, with a faded underline in passages — only
while «Show stress» is on; ink, audio and gloss keys keep the bare text. A goal text (the learner's own
book) gets the same pass at runtime from the tutor (`Goal/GoalStress.swift`, `claude-sonnet-5-5`), each
form checked by the same rule and cached in the notebook as `goals/<id>.stress.json`; offline it reads bare.

# Sound pairs — `Drussky/Resources/pairs.json`

Minimal pairs for listening (＋ → Sound pairs). `contrasts: [{id, name, note}]`,
`pairs: [{contrast, words: [{ru, en}, {ru, en}], same?}]`; `same: true` marks homophones
(луг / лук) that are shown on the teaching card but never asked. Words of 2+ syllables carry
stress marks. Audio is keyed by the stress-marked form in `Audio/index-pairs.json` /
`variants-pairs.json` / `sources-pairs.json` (`scripts/make_pair_audio.py`), so за́мок and
замо́к never share a clip. Pages are generated locally (`sound_pair` exercises, never authored
in course.json), from letters already met, weighted toward the contrasts missed most.

# Paradigms — `Drussky/Resources/paradigms.json`

Russian inflection as a visible system, built by `scripts/build_paradigms.py` from its author
tables and checked by `scripts/validate_paradigms.py` against hand-copied Wiktionary tables
(sources and policy: `scripts/paradigm_sources.md`).

```json
{
  "version": 1,
  "components": [{ "id": "decl1-gen-sg", "name": "I gen sg", "note": "Nouns I, genitive singular: -ы / -и / -ии", "requires": ["decl1-nom-sg"] }],
  "tables": [{ "id": "decl1", "kind": "noun", "title": "Nouns I · -а / -я", "subtitle": "…", "note": "the rule in two sentences",
               "rows": [{ "id": "gen", "label": "genitive", "gloss": "of; after нет, у, для, numbers" }], "cols": [{ "id": "sg", "label": "singular" }],
               "cells": [{ "id": "gen-sg", "row": "gen", "col": "sg", "ending": "-ы / -и / -ии", "components": ["gram:decl1-gen-sg", "gram:genitive"], "words": ["книга", "…"] }],
               "words": ["книга", "…"] }],
  "words": { "книга": { "ru": "кни́га", "en": "book", "table": "decl1", "pos": "noun", "gender": "f", "animate": false,
                        "forms": { "nom-sg": "кни́га", "gen-sg": "кни́ги", "…": "…", "prep-pl": "кни́гах" } },
             "читать": { "ru": "чита́ть", "table": "conj1", "pos": "verb", "forms": { "inf": "чита́ть", "1sg": "чита́ю", "…": "…", "past-f": "чита́ла", "imp-sg": "чита́й" } } },
  "uses": [{ "id": "have", "case": "gen", "title": "у меня́ есть", "component": "gram:have", "legacy": [], "rule": "…",
             "examples": [{ "ru": "У меня́ есть кни́га.", "en": "I have a book.", "word": "я", "cell": "gen" }],
             "gaps": [{ "before": "У ", "answer": "меня́", "after": " есть кни́га.", "hint": "I have a book.", "word": "я", "cell": "gen" }] }],
  "exceptions": { "мать": "the stem grows: ма́тери, ма́терью", "кофе": "indeclinable" },
  "forms": { "книги": [["книга", "gen-sg"], ["книга", "nom-pl"], ["книга", "acc-pl"]] }
}
```

Thirteen tables: Nouns I, II, III (six cases × two numbers), adjectives (six cases × m f n pl),
personal pronouns, мой and наш (твой and ваш follow them), Verbs I and II (present), the
irregular verbs (хоте́ть, есть, дать, мочь, идти́, е́хать), past, imperative, and the case uses
the course teaches. A cell names the component it exercises first (`gram:decl1-gen-sg`, unit
"Grammar", with prerequisite edges from `requires`) and the course's older coarse pattern after
it (`gram:genitive`); pronoun and irregular-verb cells are the forms themselves (`form:меня`).
`forms` indexes every written form to the cells it fills; the lemma map's tags narrow homographs
(книги as `plural` is the nominative plural, not the genitive singular). Every inflected token of
course.json and library.json whose lemma has a table is a cell of it or a listed exception;
`validate_paradigms.py` enforces that, and `validate_course.py` checks the three grammar exercise
kinds against the same file.

In the knowledge model, every inflected token fills its cell; `transform` and `paradigm_fill`
name their cells outright (modality `cue-write`, or `cue-pick` for tiles and `cue_sort`), and a
phrase put in the wrong bucket is a confusion between two uses. The inside cover's *Grammar*
entry is a chip per table (cells met, tinted by mean recall) opening the table, cells coloured by
the recall of their pattern, a dot for cells the learner has produced; ＋ → *Grammar page* builds
a page from the weakest cells, cases interleaved, asking only words the learner has produced (the
headword, or two of its forms) and only cells whose pattern they have met (the cell, or the same
case in another table of the kind) — unmet rows are left for the course to introduce.
`scripts/course_grammar.py` adds one authored page at the end of each unit that introduces a
system (g1–g8).

# Morphology — `Drussky/Resources/morphology.json`

Roots and word families, built by `scripts/build_morphology.py` from its author tables and
checked by `scripts/validate_morphology.py` (sources and the segmentation policy:
`scripts/morphology_sources.md`).

```json
{
  "version": 1,
  "roots":    [{ "id": "пис", "forms": ["пис", "пиш"], "gloss": "write", "label": "пис-/пиш-", "note": "…" }],
  "prefixes": [{ "id": "пере", "forms": ["пере"], "gloss": "across; again, re-", "examples": ["перево́д", "…"] }],
  "suffixes": [{ "id": "тель", "forms": ["тель"], "gloss": "one who does", "pos": "noun", "examples": ["учи́тель"] }],
  "lemmas":   { "переписать": { "ru": "переписа́ть", "morphemes": ["пере", "пис", "а́", "ть"], "roots": ["пис"],
                               "prefixes": ["пере"], "suffixes": ["ть"], "pos": "verb", "en": "to rewrite, to copy out",
                               "family": "пис", "base": "писать", "example": { "ru": "…", "en": "…" }, "note": "…", "loan": "…" } },
  "forms":    { "книгу": "книга" },
  "cognates": [{ "ru": "филосо́фия", "en": "philosophy", "source": "Greek φιλοσοφία …", "rules": ["ph-f", "ia-iya"],
                 "parts": [{ "tile": "фило-", "gloss": "love (φιλο-)" }, { "tile": "-со́фия", "gloss": "wisdom (σοφία)" }] }],
  "rules":    [{ "id": "th-t", "pattern": "th → т", "note": "…", "examples": ["тео́рия", "…"] }],
  "names":    ["филипок", "…"]
}
```

`lemmas` is keyed by the headword without stress; `morphemes` spell `ru`; every root named is in
`roots` and one morpheme is one of its `forms`; `family` is one of the lemma's roots (members of a
family all carry its root), and a lemma with no root (да, здесь) has no family. `base` names the
lemma it is derived from. Every course lexicon word (through `lemmas.json`), every lemma-map lemma
and the 300 most frequent library words resolve to an entry (or a cognate, or a name).

In the knowledge model (`RussianDomain`), roots are `root:<id>` components and affixes are
`affix:пере-` / `affix:-тель`; a word has `contains-root`, `contains-affix` and `derived-from`
edges. Producing a word right is evidence for its root, in the exercise's modality; a miss never
is (missing one word is not forgetting the root). `build_word` has its own modality,
`meaning-build`. The inside cover's *Word families* shows every root with a known member; the
＋ menu's *Word families* and *Cognates* pages are built locally from roots with two known members
and from cognates in letters already met.

# Lemmas — `Drussky/Resources/lemmas.json`

The knowledge model's inflection data: `forms` maps each inflected word as written (lowercase,
unstressed) to `[lemma, construction…]`, so `"книгу": ["книга", "acc-u"]`. `constructions` lists
the grammar patterns (`{id, name, note, requires?}`). `requires` gives the prerequisite edges, and
from those the map's "ready next". Add a form here when the course or library introduces one;
words absent from the map are their own lemma.

# Local sample and construction pages

These are prepared in code, outside generated `course.json`. `construction_choice` has `text`
(a sentence with a gap), `options` (plausible Russian forms), integer `answer`, `hint` (feedback
shown afterward), and `word` (the complete sentence identity). It finishes on the first choice,
uses the separate `construction-choice` modality, and credits only its construction decision.
It deliberately has no spoken string.

A local `explain` exercise may have an optional `baseline` task; the view routes this to weekly
sample capture instead of an explanation. `ExerciseResult.baseline` is optional too. The original
response, help/familiarity flags and timestamps are kept alongside the normal ink/voice files.
A sealed response cannot be reopened; reflection is a separate field. These are ungraded samples,
not evidence of lexical recall. Old pages and results decode without either field.
