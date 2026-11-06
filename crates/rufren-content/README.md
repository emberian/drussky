# First contacts content pack

`rufren-content` embeds an original, offline Russian curriculum as typed JSON. It has
no dependency on the app, storage, scheduler, or core event model. The current pack
contains 6 units, 24 lessons, 169 phrases, 172 exercises, 50 target features, and a
33-letter reference. The naming lesson includes лук / люк and за́мок / замо́к
with distinct hearing, speaking, and written-stress activities. All Russian teaching content is **generated editorial draft**,
with source notes and no claim of human certification. The AI teacher supplies the
explanations, pronunciation models, feedback, and practice. Audio is generated on
demand by the host; no audio bytes are bundled. A friend is an optional casual
conversation partner, with no preparation, recording, or assessment duties.

The sequence starts with a real exchange and requests for help. Everyday branches
add preferences, quantities, company and topics; later encounters add relative
clauses, motion patterns, a second station scene, and quantifier scope. It moves through
script, introductions, location, objects, absence, agreement, actor/patient roles,
time/aspect, motion, and plans. Four early supported bridges explore an original
station scene, mathematical quantifiers, philosophical reasons, and narrator versus
character. These bridges preserve conceptual depth while keeping production short;
they do not imply the learner has already mastered every form they encounter.

## Integration contract

```rust
let pack = rufren_content::load_embedded()?;
let lesson = &pack.lessons[0];
for phrase in pack.lesson_phrases(lesson) {
    // Ordinary spelling and supported spelling are separate.
    show(&phrase.russian, &phrase.stressed, &phrase.translation);
}
```

`ContentPack::from_json` validates the schema and graph. Lookups are `lesson`,
`phrase`, `target`, `exercise`, and `unit`. Ordered unit membership and lesson
prerequisites describe a suggested encounter sequence; `available_lessons` consumes
an explicit set of encountered lesson IDs. A host should still allow exploration:
prerequisites are not mastery claims or a reason to lock an interested learner out.

IDs are stable editorial identities (`ru.lesson.roles`, `ru.target.roles`,
`ru.phrase.roles.04`). Persist **pack ID, pack revision, and item ID** with an attempt.
Editing the material increments `revision`. `previous_revisions` declares pack
edition ancestry (current revision `2026-11-05.4` lists `2026-11-05.1`, `2026-11-05.2`, and `2026-11-05.3` as ancestors).
A host can deterministically supersede prior bundled revisions for a stable phrase
without treating a human-edited head as superseded. The lineage is explicit content
metadata, never inferred from the receiving device’s current library. It contains
unique, nonempty revisions, excludes the current revision, and is bounded to 64
entries of at most 128 UTF-8 bytes each. Replacing an item's meaning or tested
feature requires a new item ID; never recycle an old ID for unrelated content.
Schema version 1 describes the representation, independently of the pack revision.

Phrase identity is independent of a packaging-only edition update. Optional paired
`phrase_revision` / `previous_phrase_revisions` fields pin immutable authored
provenance. `resolved_phrase_lineage` returns both values together; legacy phrases
without either field use the pack revision/ancestry, and an explicit empty parent
list never falls back. In `.4`, the 103 unchanged base payloads retain `.3` plus
their exact `.1/.2` parents; the 66 new phrases start at `.4` with no fake parents.
Consumers must derive the revision UUID, source suffix, and parent UUIDs from the
same resolution. Existing deterministic authored events retain envelope version 2
when the general event protocol advances. Session snapshots still record the
current pack edition separately.

The immutable `.3` regression fixture and `content_lineage` tests compare full core
Events and serialized bytes, all four skill ReviewKeys and spaced due/history
state, import-order convergence, and user-edit preservation. The staging tool also
checks unchanged stress, targets, and other phrase payloads. A future changed phrase
must receive a new phrase revision; do not reuse a historical ID with changed facts.


Every exercise has an explicit `reference_phrase_id` naming a within-lesson hidden
anchor for event linkage, recording, and requested support. It is separate from
`stimulus_phrase_ids`, which lists what Russian text or audio the activity actually
uses as its input. Production exercises keep that visible-input list empty; the
host must not reveal the hidden reference as an unrecorded cue. A composition
reference is a deliberate starting anchor, not a full expected free response.

An exercise names its input phrases, response modality, target features, hint,
feedback, and answer contract. `check_answer` returns `Accepted`, `NotMatched`, or
`NeedsReview`. An accepted text string establishes only a match to an authored
variant under that task's tolerances. `NotMatched` is deliberately not named
“wrong”: a finite list cannot reject every other grammatical free response.
Composition and speech always need review; a matching transcript is not a
pronunciation judgment. Choice answers use the option's **ID**, never its position.
Listening alternatives are parallel English meanings with balanced answer positions;
reading choices retain their authored language. Quantifier probes use an unknown
fictional situation so prior arithmetic facts cannot answer them. Hints address the
particular meaning or form and are explicit assistance. Option order varies. Preserve the original response when recording any judgment.

`normalize_answer` uses Unicode NFC, optional lowercasing, whitespace collapse,
optional removal of terminal `. ! ? …`, and optional removal of the pedagogical
acute accent. It does **not** fold ё into е, й into и, Latin lookalikes into Cyrillic,
or case endings into each other. Tasks that assess written stress set
`require_stress`; a punctuation task disables punctuation tolerance. This is
explicit task-local policy, not general linguistic equivalence.

## AI teaching, speech, and optional conversation

All phrases report `PendingGeneratedSpeech`: the host generates a reference model
when needed. `needs_audio` describes that asset need independently of its source;
`needs_recording` remains a compatibility alias. A generated model or an optional
human recording can provide the actual input. `MissingCoachRecording` remains
readable for legacy material and never imposes a human-recording requirement.
A listening or dictation attempt still needs actual audio playback: showing the
written stimulus is a different, explicitly supported activity.

The AI teacher carries teaching, vocabulary help, response interpretation, and
correction. The legacy `AnswerSpec::CoachReview` wire name means contextual review
by that teacher by default, not a request for a formal human coach. Speech feedback
still requires audio evidence for acoustic claims; a transcript alone cannot show
stress or consonant quality. Retain the generated voice/provider provenance and
label synthetic speech honestly.

`Lesson::conversation_invitation` is a short optional idea for an ordinary breakfast
or dinner conversation with a willing friend. It has no recording assignments,
review rubrics, or lesson preparation. The canonical JSON key is
`conversation_invitation`; serde also accepts the old `coach_activity` key. Practice
and fresh-context work remain available with the AI teacher whether or not a meal
conversation happens.

Keep visible context, phrase chunks, and replay available. Revealing a word, text,
or model is assistance to record. The lesson's transfer prompt offers a changed
message or a new situation for AI-led practice.

## Editorial quality and provenance

The pack remains generated editorial draft. Contributor review can improve stress,
naturalness, pragmatic fit, specialist notation, distractors, and sequencing; that
quality work is not homework for the learner's friend or a gate before beginning.
The learner chooses their name, address style, and past-tense self-description;
the AI teacher explains the available forms. Generated speech is a model to use
and inspect, without claiming it is a native human recording.

The alphabet is an optional reference with conditional cues, not a pronunciation
key or a prerequisite quiz. IPA appears only in a narrow word-specific note. The
pack deliberately gives no English transliteration that might masquerade as a
complete Russian sound model.

Original scenes are not literary quotations or simplifications of a named work.
No favorite author has been inferred. Future imported passages should preserve
edition, author, rights, original text, and distinctions between original, gloss,
and adaptation. The embedded pack and source records travel offline together.

See [SOURCES.md](SOURCES.md) for the scope and limitations of source checks.

Focused verification (coordinate with the workspace build owner):

```text
cargo test -p rufren-content --test content_lineage
cargo test -p rufren-content --test content_contract
```

## Sound Focus catalog

`src/sound_contrast.rs` supplies standalone, serde-compatible authored definitions
for hard/soft л and lexical stress, without a `.5` phrase-pack migration. Contrast,
member, exact utterance, and identify/produce task each have an immutable ID/version.
`SoundContrastDefinition` contains the safe English options, brief teaching notes,
source links, and the three legacy actions retired from new discovery.

Use `ContentPack::lesson_practice_actions(&lesson)` for **new** lesson actions and
new AI-teacher material. It returns `LessonPracticeAction::SoundFocus` and
`LessonPracticeAction::Exercise` through one projection. In the naming lesson,
Sound Focus replaces new starts of `hear-soft`, `hear-stress`, and `say-contrast`;
`mark-stress` remains a separate written task. Raw `exercise`, `lesson_exercises`,
lesson membership, old answer specifications, and saved snapshots stay intact for
history and resumption. The projection rejects a stale lesson or incompatible
lexical reference instead of falling back to the retired flow.

The new isolated utterances are exactly `Лук.` and `Люк.`. Their related `.07/.08`
phrase IDs at `.3` are lexical provenance, not permission to attach isolated WAVs
as recordings of `Это лук.` or `Это люк.`. The stress utterances are separately
versioned exact `Это за́мок.` and `Это замо́к.` strings, related to `.09/.10@.3`.
Core/app builders create sound-specific reference identities and bind actual
recording events, assets, hashes, and displayed options in durable trials. Ordinary
Russian text equality cannot distinguish the two stress members.

The catalog labels intended meanings; it does not certify an audio realization.
A first four-trial schedule, recorded intended meaning before production, playback
acknowledgment, assistance, and evidence projection are core/app responsibilities.
Tests for this seam are narrowly available with:

```text
cargo test -p rufren-content --test sound_contrast
```
