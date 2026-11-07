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
