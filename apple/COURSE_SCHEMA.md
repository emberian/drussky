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
  "example": "ру́ка" }
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
  "pages": [Page, …] }
```

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
   should be the majority of exercises; every page mixes in hearing and speaking.

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

# Sound pairs — `Drussky/Resources/pairs.json`

Minimal pairs for listening (＋ → Sound pairs). `contrasts: [{id, name, note}]`,
`pairs: [{contrast, words: [{ru, en}, {ru, en}], same?}]`; `same: true` marks homophones
(луг / лук) that are shown on the teaching card but never asked. Words of 2+ syllables carry
stress marks. Audio is keyed by the stress-marked form in `Audio/index-pairs.json` /
`variants-pairs.json` / `sources-pairs.json` (`scripts/make_pair_audio.py`), so за́мок and
замо́к never share a clip. Pages are generated locally (`sound_pair` exercises, never authored
in course.json), from letters already met, weighted toward the contrasts missed most.

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
