# Listening curriculum — Codex deputy author

Authored 2026-11-08 in `deputy-codex-listening`. No git commands run. No Swift, generated course, project, audio manifest, or learner notebook changes.

## Delivered

- `apple/Drussky/Resources/listening.json`: version 1, 12 practice sets / **94 items**, plus 6 reserved weekly sets / **30 new messages**. Practice messages contain 38 comprehension questions; weekly messages contain 60 English free-response questions. The practice strand has 113 answer decisions in total.
- `apple/scripts/validate_listening.py`: standalone Python 3 / stdlib validator, importing the existing course stress checker and serial vocabulary helpers. Also exports and checks the spoken inventory.
- `research/artifacts/listening-spoken.txt`: **338 unique exact playback strings**, UTF-8, one per line, no headers or IDs. Includes main turns, both intonation forms, post-answer crucial spans, and Russian object/scene labels for optional audition after answering. It excludes shelf titles, English, and explanatory prose. Multiple items can use the same span or label; the inventory deduplicates exact strings, preserving stress and punctuation. Obtain item/voice/speaker context from the JSON, not this flat inventory.
- This report: decoding contract, curriculum counts, page behavior, validation, and integration priorities.

This is editable authored content, like `math.json`, not generated course material. The temporary authoring script is not a required source or build dependency. Edit the JSON and run:

```sh
python3 apple/scripts/validate_listening.py --write-inventory
python3 apple/scripts/validate_listening.py
```

## Curriculum and counts

Page order is **course array order**, not numerical sorting of unit IDs. In this checkout, table talk in units 13–14 precedes unit 8; unit 12 follows the location/accusative pages in unit 18. The set order follows that reality. `after_page` recommends when support is sufficient, as on the reading shelves; it need not prohibit opening earlier.

Abbreviations below: P object pick, W who needs, F follow, Q question/statement, M message, C correction, D disagreement. Course coverage counts exact taught Russian surface-form tokens in the main heard turns (both forms for Q), not repeated feedback, translations, or options. Glossed new forms supply the remainder. Titles are navigation, not assessment vocabulary.

| Set | After | Items | Kinds | Course coverage | Distinct new heard forms | Unfamiliar items |
|---|---|---:|---|---:|---:|---:|
| listen-01 — Asking or telling | u2-p3 | 6 | Q6 | 100.0% | 0 | 1 |
| listen-02 — Pass me the right thing | u4-p4 | 8 | P5 Q1 F2 | 64.3% | 4 | 2 |
| listen-03 — Small requests at home | u7-p4 | 8 | P3 Q1 F1 W3 | 65.9% | 4 | 2 |
| listen-04 — Whose request is it? | u14-p5 | 8 | W5 P1 Q1 C1 | 100.0% | 0 | 2 |
| listen-05 — What is missing? | u10-p4 | 8 | W3 F2 P1 Q1 C1 | 84.2% | 5 | 2 |
| listen-06 — Help without guessing the speaker | u12-p3 | 8 | W4 F1 M2 C1 | 96.9% | 2 | 2 |
| listen-07 — Messages about plans | u17-p5 | 8 | M4 C2 W1 Q1 | 94.4% | 4 | 2 |
| listen-08 — Where plans lead | u19-p4 | 8 | M3 C2 W2 Q1 | 88.6% | 7 | 2 |
| listen-09 — People, objects, recipients | u24-p5 | 8 | W3 M2 C2 F1 | 82.8% | 10 | 2 |
| listen-10 — Interruptions and revised plans | u25-p4 | 8 | C3 M3 W2 | 82.4% | 14 | 2 |
| listen-11 — Different preferences | u27-p5 | 8 | D4 M2 C1 W1 | 86.0% | 11 | 2 |
| listen-12 — Reasons, repairs, unresolved disagreement | u30-p4 | 8 | D4 M3 C1 | 87.7% | 17 | 2 |

Totals: **P10 W24 F7 Q12 M19 C14 D8**. Twenty-three practice items request an unfamiliar voice.

The apparently low early coverage is four deliberately recurring forms: **дай, мне, пожа́луйста, положи́**. They recur frequently in very short requests, so their token share is large. Every object-choice noun is already presented in the course; none needs an invented picture or an unknown written answer. The first set uses only the tiny vocabulary actually available at `u2-p3`: it does not pretend that “give me the book” is already known. Later gloss counts include exact inflections of familiar words, not just new dictionary lemmas. Each item remains short and carries its own glosses; prior listening exposure does not silently remove a gloss.

| Reserved set | After | Messages | English questions | Course coverage | Unfamiliar items |
|---|---|---:|---:|---:|---:|
| listen-week-01 | u14-p5 | 5 | 10 | 97.7% | 5 |
| listen-week-02 | u12-p3 | 5 | 10 | 92.7% | 5 |
| listen-week-03 | u17-p5 | 5 | 10 | 89.5% | 5 |
| listen-week-04 | u24-p5 | 5 | 10 | 85.7% | 5 |
| listen-week-05 | u27-p5 | 5 | 10 | 86.3% | 5 |
| listen-week-06 | u30-p4 | 5 | 10 | 84.8% | 5 |

These are six rising support levels, **not six psychometrically equated weekly forms**. Choose an unused set appropriate to the learner's support level. Compare the saved responses and assistance, not raw percentages across different levels as if difficulty were constant. The early baseline is after table talk, when a two-sentence household message is possible. The bank is finite: after using the suitable forms, author more; do not call a repeat unheard.

## Schema: exact version-1 decoding contract

All indices are **zero-based**. Strings are UTF-8. Every Russian polysyllable carries U+0301 except a word containing ё, which carries no acute; monosyllables are unmarked. Gloss keys alone are unstressed. IDs are globally unique lowercase ASCII kebab-case; scene IDs are local to one item. No numeric enums and no JSON-encoded strings.

The authoring validator requires the fields below within their applicable kind. A unified Swift `Decodable` item should make kind-specific fields Optional and switch on `kind`; a field inapplicable to an item is **absent**, not an empty placeholder. Existing notebook decoding should gain only Optional references/results if these become local exercise kinds. All `options` everywhere are `[String]`; scene transitions have their own Optional `outcomes` field. `answer` is always `Int`, never sometimes text. Unknown future fields should be safely ignored by the app; this version's authoring validator rejects typos/extra fields.

### Envelope and common item fields

| Field | Type | Meaning |
|---|---|---|
| Root `version` | Int | Exactly `1`. |
| Root `sets` | `[ListeningSet]` | Ordered teaching/practice content, 12 sets. |
| Root `weekly_samples` | `[ListeningSet]` | Reserved content, 6 sets; do not put it in normal shelf practice or review selection. |
| Set `id`, `title`, `title_en` | String each | Stable identity; stressed Russian navigation title; English title. Titles are not spoken. |
| Set `after_page` | String | Course page ID, compared by actual array order. |
| Set `items` | `[ListeningItem]` | 6–10 practice items or exactly 5 weekly messages. |
| Item `id` | String | Stable content identity. Suggested local source identity: `listening:<item-id>`. |
| Item `kind` | String | One of the seven kinds below. |
| Item `en` | String | Full translation of the main played stimulus; hidden before answering. For exchanges, A/B-labelled turns separated by newline; for intonation, translation of the selected form. |
| Item `after_page` | String | Repeated explicitly; never earlier than the parent set or preceding item. |
| Item `voice_hint` | String | `any` or `unfamiliar`; assignment policy below. |
| Item `glosses` | `[String:String]` | Lowercase unstressed **surface** form → contextual English; preserve ё and hyphens. May be `{}`. Applies to the item's Russian, including labels. |
| Item `crucial_span` | String | Exact stressed substring of one main turn or form; intonation uses the entire selected form. Reveal after a miss. No offsets to guess. |
| Item `why` | String | English explanation, sometimes with stressed Russian quotations. Post-answer only. |

### `pick_object`

| Field | Type | Meaning |
|---|---|---|
| `ru` | String | One complete spoken request, played without displaying it. |
| `question` | String | English task prompt shown before playback. |
| `options` | `[String]` | 3–4 distinct stressed Russian **noun forms already taught by this page**, presented as words. |
| `answer` | Int | Index of the requested object. |

The utterance may use an inflected form while the option is the dictionary form: «ло́жку» → «ло́жка». `why` identifies the deciding span and explains that relation. Distractors can be mentioned and rejected in the audio; matching any heard word is insufficient.

### `who_needs`

| Field | Type | Meaning |
|---|---|---|
| `lines` | `[{speaker:String, ru:String, en:String}]` | 2–4 alternating turns; speaker is exactly `A` or `B`, both appear. Play in array order with distinct stable voices. |
| `question` | String | English question about who needs something, requests help, or lacks information. |
| `options` | `[String]` | 3–4 English options; this bank uses `A`, `B`, `Both`. |
| `answer` | Int | Correct option index. |

No item-level `ru`. Identify A/B visually during playback with a neutral speaker badge; do not show their words or translations. Speaker A is not automatically the person in need: some A turns are offers or questions, and some B turns admit the same difficulty.

### `follow`

| Field | Type | Meaning |
|---|---|---|
| `ru` | String | One instruction or a short ordered sequence of instructions. |
| `question` | String | English prompt asking which result follows. |
| `scene` | `[{id:String, ru:String, en:String, role:String}]` | 3–5 named entities; role is `object` (movable) or `place` (destination). Russian labels are shown as words, with English available as support. |
| `options` | `[String]` | 3–4 English descriptions of possible results. Exactly one is licensed by the spoken instruction. |
| `outcomes` | `[{object:String, place:String?}]` | Parallel to `options`, same count/order. `object` references an object ID; `place` references a place ID. Explicit `null` means leave that object unchanged. |
| `answer` | Int | Index into both arrays. |

The minimal implementation presents word-labelled scene chips and outcome buttons. There are no pictures and no coordinates. Put objects in a neutral tray; places are named destinations. An outcome describes **one object's resulting location**, not the complete state of every object after a multi-part instruction. A future drag implementation can select the matching `(object, place)` outcome; record scene manipulation separately from button recognition. It must not pretend these records are interchangeable. “Unchanged” has its own button; it does not mean a missing destination caused by a decoding error.

### `question_or_statement`

| Field | Type | Meaning |
|---|---|---|
| `forms` | `[{ru:String, en:String}]` | Exactly two entries: index 0 is the statement ending `.`, index 1 the yes/no question ending `?`. They otherwise match exactly, including stress. |
| `played` | Int | Which form to play for this authored trial: 0 or 1. |
| `question` | String | English setting plus the task; displayed before playback. |
| `options` | `[String]` | Exactly `["Statement", "Question"]`. |
| `answer` | Int | Exactly equal to `played`. |

No item-level `ru`. `en` equals `forms[played].en`; `crucial_span` equals `forms[played].ru`. Both forms belong in the audio bank, but play only `played` until the response, then offer comparison. Save the selected form with the response. Do not display Russian punctuation before answering. Shuffle trial order where appropriate so the authored alternation is not a cue; never silently change `played` without updating the trial's answer/translation/reveal identity.

### `message`

| Field | Type | Meaning |
|---|---|---|
| `ru` | String | A complete 2–4 sentence voice message; play it as a connected message. |
| `questions` | `[ListeningQuestion]` | 2–3 English questions, including `gist` and `detail`. Each carries its own reveal span and explanation. |

No top-level `question`, `options`, or `answer`. The item-level `crucial_span`/`why` supply a whole-item fallback (in this bank the final detail's reveal). **Prefer the missed question's span**, not the fallback, when responding to a particular miss. Collect all answers before any feedback so one answer's reveal does not expose the next answer. Replay is the whole message.

Practice question fields:

| Field | Type | Meaning |
|---|---|---|
| `kind` | String | `gist` or `detail`. |
| `prompt` | String | English question, visible before answering. |
| `options` | `[String]` | 3–4 distinct English responses. |
| `answer` | Int | Correct option index. |
| `crucial_span` | String | Exact substring of the message, revealed for this question. |
| `why` | String | English feedback, hidden until all message answers are committed. |

Weekly question fields replace `options`/`answer` with the following; `kind`, `prompt`, `crucial_span`, `why` retain the same meanings:

| Field | Type | Meaning |
|---|---|---|
| `response_mode` | String | Exactly `english_text`: typed or handwritten English comprehension response. |
| `answer_en` | String | Model meaning, revealed only after the original response is sealed. |
| `accept_en` | `[String]` | One or more acceptable example paraphrases. They are semantic comparison examples, **not an exact-string whitelist**; further equivalent English answers count. |

Weekly responses are ungraded samples. Preserve the raw answer and let the learner compare it with `answer_en`; spelling/English style are not Russian listening errors. No model judge is required. A short Russian reply would be a different production task and a separate evidence record; this authored bank deliberately contains only English answers.

### `correction`

| Field | Type | Meaning |
|---|---|---|
| `ru` | String | One speaker's original statement followed by an explicit self-repair, played as one clip. |
| `question` | String | English question about the final intended version. |
| `options` | `[String]` | 3–4 English interpretations, including plausible pre-correction or wrong-scope alternatives. |
| `answer` | Int | Final meaning after the correction, not the first proposition. |

The last noun/number is not always the answer: late repairs change timing while preserving importance, or change the traveller while preserving the destination. `crucial_span` and `why` explain exactly what changed and what remained.

### `disagreement`

| Field | Type | Meaning |
|---|---|---|
| `lines` | `[{speaker:String, ru:String, en:String}]` | Same ordered two-speaker contract as `who_needs`, 2–4 turns. |
| `question` | String | English question about proposals, preferences, reasons, or what has actually been agreed. |
| `options` | `[String]` | 3–4 English interpretations, sometimes jointly describing A and B. |
| `answer` | Int | Correct interpretation. |

No item-level `ru`. Acknowledgment is not assent: “Yes, difficult, but I want this one” agrees about difficulty while rejecting replacement. The final dialogue has four turns so both speakers actually accept talking tomorrow; they do not agree to the outing.

## Playback and the listening page

One screen is a single listening situation. At the top: the set title, a short English task, and a large **Play** button. Under it, either noun choices, the small word-labelled scene, A/B speaker badges and English choices, or two English message questions. The Russian stimulus and translation are initially hidden. **Replay** and **Slower (0.75×)** remain beside Play; record their use. **Show text after answering** opens the stressed transcript and translation after the first response. A teaching session can offer an explicit “I need the text” escape earlier, recording assistance and finishing the independent attempt before the reveal.

After a miss, put the missed question's **crucial span** above a one-sentence **why** explanation. Highlight the same exact span in the full transcript when shown. A small span-play control can play its inventory clip; also replay the full utterance so the learner hears how the span fits into connected speech. A separately synthesized span is an isolated illustration, not an acoustically extracted segment; prefer aligned original audio if timings become available. Do not auto-play the wrong option as if that were the original message. For an intonation miss, play both forms in the same voice after the answer.

Retain first choice, later choices, actual clip/voice IDs, first-hearing result, replay count, slow playback, gloss/text reveals, and response timing from **audio completion**. A multi-turn exchange's timing starts after the last turn. Stop/cancel the sequence when the screen changes or another playback begins; use the existing Voice sequence ownership/cancellation behavior. Corrections should follow with another task probing the same distinction, but not by exposing reserved weekly material.

Teaching feedback can reveal after an item; weekly feedback waits until **all five messages and their responses are sealed**. Keep help and self-reported previous familiarity with the sample. Store reflection separately. Never award spelling or productive Russian recall credit for English choices. New comprehension observations should have their own listening-meaning modality; the weekly sample is an ungraded sample, not an FSRS vocabulary outcome.

### Voice identity is part of the task

`any` permits a familiar course voice. `unfamiliar` asks the app to select a voice/variant **not previously heard by this learner on course pages**, checking actual voice exposure when available. Aidar or baya are candidates if xenia was used on course pages, not guaranteed unfamiliar by name. Keep the selected voice stable across Replay/Slower; do not introduce a new voice on each retry. For an exchange, A and B must be audibly distinct and stable throughout; on an unfamiliar item prefer both voices unheard, and record each actual identity. With only a familiar voice available, mark that fact or defer the unheard sample; do not silently claim the hint was fulfilled.

The root audio lane needs whole-message clips, each separate dialogue turn with its assigned voice, both intonation forms, and optional reveal/label clips from the inventory. Preserve the existing rising-question audio key: statement and `?` forms must not collide. Do not synthesize literal `A:`/`B:` labels. Some strings repeat across different speaker contexts; the flat inventory is a text manifest, not a request for only one voice version per text.

The supplied `research/VOICE_2026.md` actually describes newer voice experiments as well as the current Silero material. Its audition found weak/flattened question contrasts for some engines. **Listen to every scored question/statement pair** in the final clips, verify both IK-1 and IK-3 and preserve the same speaker for both. If the contrast is not audible, use it as an ungraded teaching comparison until suitable audio exists. JSON punctuation and ASR agreement do not validate intonation. The first set depends on this check.

This delivery authors text and unfamiliar-voice requests; it does not supply or claim unheard human recordings. Commons single words cannot voice a connected original message, and LibriVox narration is not automatically conversational transfer. A later human-message bank, with actual held-out speakers and intact natural recordings, is still needed to test understanding people. Synthetic voice variation is useful practice and a narrower transfer sample.

## Validation and editorial checks

`python3 apple/scripts/validate_listening.py --write-inventory` and the default read-only validation pass. Zero missing glosses or schema/stress errors. The validator checks:

- Exact per-kind and nested field/type contracts, set sizes, unique IDs, valid page references, real course ordering within and across sets.
- The imported `validate_course.check_stress` for all Russian strings, including titles and quoted feedback, plus three audited placements (`нужна́`, `права́`, `пото́м`) found during editing. Gloss keys are excluded from stress checks.
- Answer ranges with booleans rejected as indices; fixed intonation answer/played consistency, full punctuation-only pairs and distinct rising-question audio keys.
- Exact reveal-span containment; scene references, roles, distinct outcomes and parallel outcome/option arrays; both alternating speakers.
- 2–4 message sentences, 2–3 questions with both gist/detail, and the distinct English free-response weekly contract.
- Exact surface vocabulary/gloss coverage including labels and Russian feedback quotations. Object choices must already be taught. Pre-answer Russian labels must use letters already introduced; hidden audio can use glossed request words before those letters are learned.
- Reserved message isolation against practice stimuli, whole exchanges, other weekly messages, and existing course spoken strings, ignoring punctuation/case/stress for reuse detection. New wording can of course share familiar words and short phrases; isolation is of the whole message.
- Per-set coverage and unfamiliar-item counts; exact freshness of the generated spoken inventory on a default run.

This checkout's 711 lexicon entries have **no `first_page` fields**. The validator uses `validate_series.build_course_snapshots`, the actual presentation sequence, and exact token/key helpers. If a lexicon entry has `first_page`, that page controls its eligibility. It never gives an unseen inflection automatic credit from its lemma or counts prior listening gloss exposure as course knowledge. These are curriculum exposure counts, not claims about an individual learner's mastery; the shared helper also treats presentation in an exercise as exposure, not proof of comprehension.

Twenty-three targeted mutation/metadata checks passed using a temporary harness: invalid/out-of-range/boolean answers, missing type-specific field, missing/uppercase gloss, nonexistent and backwards pages, missing/redundant/misplaced stress, intonation lexical and played/answer mismatches, invalid speaker and scene destination, invalid reveal span/question count, weekly mode/voice violations, duplicate and reused messages (including punctuation changes), duplicate IDs, untaught object choices, and explicit `first_page` behavior. A clean repeat passed after these checks, confirming the imported stress checker's global error list was not polluted. The harness is temporary; the delivered validator is the repeatable content gate.

An additional stress comparison against single-word course lexicon spellings exposed «по́том»; it was corrected to «пото́м». Editorial review also corrected «ну́жна» / «пра́ва», removed an unnatural shopping sentence, and made final agreement explicit in the four-turn dialogue. Automatic stress syntax checks alone would not have caught those placements.

No simulator, Swift build, or screenshots: this lane has authored resources and a validator; no listening view is implemented here to render or photograph. No audio generated or auditioned. These limits remain explicit so a passing content check is not mistaken for a working listening page or verified speech.

## What the course should teach earlier

1. Make **дай, мне, пожа́луйста, положи́** a tiny orally introduced request family as soon as there are three usable nouns. Teach the request's meaning through a response, not a spelling drill. The early sets demonstrate exactly why these four forms have high utility. Add «кни́гу» alongside «кни́га» when object requests begin; explain what changed without requiring an entire declension table first.
2. Bring asking for slower speech, repetition, and admitting non-understanding into the first household exchanges. «Я не понима́ю», «Ещё раз», «Поме́дленнее» are agency, not end-of-course repair vocabulary.
3. Teach useful home/work message forms before abstract readings: «купи́», «магазин/в магази́не», «её», «я дам / он даст», «сейча́с», «уже́», «встре́ча», «дава́й». Present each spoken surface form with meaning and then reuse it across requests and replies. The late gloss counts show how many everyday forms the current course still leaves implicit.
4. Export reliable `first_page` metadata from course construction. Keep form-specific presentation history, so shelves can distinguish a known lemma from a new inflection without guessing.
5. Put one meaning-first listening item among ordinary course activities, with a later variant in a different voice. Maintain separate comprehension and transcription evidence. Keep an unexposed assessment bank out of review generation and out of casual audio previews.

## Where I disagree with the existing listening design

`listen_pick` asks “Which one did you hear?” and shows written options containing the spoken syllable/word. This is useful sound-to-print recognition, but its response can be correct without understanding a request, resolving a pronoun, or remembering what a correction cancelled. Its current wrong-choice playback also changes the auditory stimulus during the decision; for comprehension I want the original message retained, then the decisive span explained after the response.

Dictation is valuable for hearing, spelling, segmentation, and verbal memory together. Making it the main listening check hides which of those demands caused a miss and withholds credit from someone who understood a message but cannot spell it. These tasks let a learner demonstrate meaning with an English answer or a scene result. They do not establish Russian production, and should not update that modality.

Sound-pair practice already offers something the broader listening strand needs: stable speaker identities, alternate recordings, and a post-answer comparison. Extend those good properties to whole meaningful turns. Avoid turning every conversation into isolated word discrimination, or treating voice variation alone as evidence of human-speech comprehension.

## Next integration steps

The root lane can generate and audition audio from the inventory plus the JSON speaker assignments. The Swift lane can load the versioned shelf and map each kind to a local exercise using the fields above, preserve first responses/help/clip identity, and implement the reserved weekly path separately. After adding the resource to the app, regenerate the project with xcodegen as needed and add the validator to the root content-check orchestration. Run scoped rendering and notebook compatibility checks there, including: wrong answer reveals only the appropriate span, all message answers precede reveal, both intonation clips remain distinct, replay uses the same assigned voice, and exhausted weekly content is not reused as new.
