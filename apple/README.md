# Drussky for iPad, iPhone and Mac

A Russian exercise book you write in. Open it and you are on today's page: numbered
exercises down a squared тетрадь page, answered in place — with the Pencil on ruled lines,
by ear, or aloud. Finished exercises stay in place and get a red tick; a finished page gets a red mark
(5 — Отли́чно), or a red tick (Гото́во) when nothing on it was graded — practice, skips, speaking
you judged yourself, a diary line nobody corrected. A diary line the tutor marked counts, and three
or more corrections on it count as a miss. Turn the page with the arrow; turning back shows exactly
what you wrote.

## What a page asks of you

| exercise | you |
|---|---|
| meet | see a letter in print and handwriting, hear it, *watch it printed* stroke by stroke, trace it once, then write it freely; one formation hint (start, order, direction, size, baseline) compares it with the print model (no hints against cursive guides) |
| trace / copy | write over a faint model, or beside a shown one |
| listen and pick | hear a syllable or word, tap which it was |
| dictation | hear it, write it — no model |
| translate | see English, write the Russian from memory |
| fill | write the missing word in a sentence |
| read aloud | tap: hear the model (the microphone stays closed), then *Your turn: tap to say it*; on that tap a soft cue, then just say it (it stops when you do); every take is kept, compared with the model syllable by syllable, with a tip or two; *Skip for now* (ungraded) is always there |
| what does it mean | read Russian, choose its meaning |
| diary (Дневник) | write a few words of *your own* Russian about a prompt, on ruled lines (or type); confirm or fix the recognizer's transcript, then the tutor marks it like homework — wrong spans underlined, the right form in red above, the corrected line, a word of praise, the pattern behind the biggest slip. Your ink stays. Every edit becomes evidence (own-writing modality, a confusion edge, the pattern), and corrected forms join your words for review. From unit 3 every page ends with one; ＋ → *Write in my diary* adds one now; *Not today* skips it |
| read along | a library text read by a human (LibriVox volunteers, Commons readers): the word being said lights up, tap a word to hear from there, ¾ speed, or listen first with the text appearing as it is read; then write one of its sentences from the reader's voice and say one after the reader |
| passage | read a short text (story, proof, dialogue, poem); tap any word for its meaning and sound; a word you don't know yet on a root you do is shown taken apart first (root зна- (know) + -ние). On a library text, «Структу́ра» in the margin shows its skeleton (below) |
| who is meant (structure) | a library sentence with one pronoun or participle marked: tap the word it points to or agrees with; a wrong tap gets a second try, then the answer |
| build a word | see the English, tap morpheme tiles (пере- · пис · -а́ть) into order |
| talk (Разгово́р) | a spoken conversation with the tutor: pick one of three scenes made from your words, then 6–10 turns — the tutor speaks (short Russian, stress-marked, at most two new words a turn, a gentle «Вы хоте́ли сказа́ть…?» when you slip), you tap the microphone when ready and answer out loud, an eye icon whispers the English; afterwards every line of yours gets the red pen and at most one pronunciation tip. ＋ → *Talk* |
| guess from the root | a sentence with one new word on a root you know: guess its meaning, then see how it is built |
| fill the table | a declension or conjugation table with some cells given; write the missing forms in small ruled cells (or tap tiles into them); a miss shows the row with the right cell marked |
| the form | «кни́га» → genitive singular, or a sentence with a gap that forces the case: write the form |
| sort by case | phrases sorted into the job their case is doing (у меня́ есть · в до́ме · чита́ю кни́гу) |
| read handwriting | a word or line in somebody's hand (one of four bundled OFL handwriting fonts, or the school cursive models written through with a different slant, size and ink each time): print it on the line, or pick it among look-alikes (ма́ма / ла́ма / ма́ша). ＋ → *Read handwriting* makes a page of them from words you know |

**Cursive** (`Cursive/`): with *Handwritten (cursive) models* on — or on the cursive unit's own pages —
the ghost on the ruled line is the school cursive (прописи) stroke model written through, letter
joined to letter (`Resources/strokes-cursive.json`, 66 forms with their entries, exits and join
models, from `scripts/build_strokes_cursive.py --proof sheet.png`), *Watch it written* plays it
stroke by stroke, and the one formation hint speaks cursive ("м starts with the hook on the
baseline", "т is three humps in one stroke"). The cursive unit sits after unit 10: letter families
by shape, then words, then a note to copy. Reading handwriting is its own memory modality
(*read-cursive*); a word taken for a look-alike is logged as a shape confusion. Once the unit's
first page is finished, words never yet read in another hand come back as read-handwriting
reviews after everything due (two per page at most) — picked among look-alikes until read twice,
then printed back — and from then on FSRS schedules that memory like any other.

Writing is read on-device by Apple's handwriting recognizer. For tracing and copying it ticks
the line by itself after a short pause. For recall (dictation, translate, fill) **you are the
judge** by default (Settings → *Judge my writing*): **Check** writes the model answer beside
your line and you say *Right* or *Not quite*; the recognizer reads silently, pre-selects the
likely verdict with one small hint ("The reader saw «кат». You decide."), and its reading goes
into the answer log beside your verdict — a right you gave over a reading it disputed is marked
*worth another look* on the word's history, never overruled. With *The reader* as judge it
confirms a right answer itself, *I wrote it right* overrides it, and *Show me* writes the answer
in red above your line and brings the item back soon. (`LearningCore/README.md`, "Judging one's
own writing".)

**Hooks** (`Hooks/`): on a word's history card, *Give me a hook* asks the tutor for a one-line
keyword mnemonic (мы́ло → "MEAL-o — wash your hands before the meal" 🧼); keep it, reword it or
write your own. It is shown faintly above the line before the next few recalls of that word, fainter
each time, then not at all. Once a day, with a key on the device, the tutor writes hooks for the
~60 words you miss most in one call; *Export hooks* gives them as Markdown.

**Fluency** (`Fluency/`, `metrics.jsonl`): a passage has *Read it with a finger* — follow the lines
as you read, lift at the end; nothing ticks while you read, and afterwards one line gives words a
minute against last time. Every word written from memory records strokes a second and how legibly
the recognizer read it; a sentence said aloud records words a minute from its syllable onsets; a
sentence written from the reader's voice records whether you caught it at 1× or ¾×. Small sparklines
on the word's history and one line inside the cover ("You read about 20% faster than last week").

Saying something aloud is hands-free once you have said you are ready (`Speaking/`): the model
plays with the microphone closed (Settings: 0–3 times, tap to skip), the page waits for your tap
(`SpeakingTurn.Phase.ready`; with no model to play, the tap that started the turn is that tap), then
a soft tone says it's your turn, and the microphone (AVAudioEngine) measures the
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
as unverified) or skip it (not counted). A word-level report's letter differences are the
recognizer's spelling, not a measurement of a sound: they are shown in amber as what it heard, with
the contrast's advice as something to try, and leave no evidence. Only the phone-level analyzer
(`PhoneticAnalyzer`, when its model is bundled) asserts sound verdicts and logs speech confusions
on their contrast; each report carries its `provenance`.

## Show the structure

Every library text carries a structure layer (`Resources/library-structure.json`, written offline by
`scripts/library_structure.py` with Claude and checked by `scripts/validate_structure.py`; format in
COURSE_SCHEMA.md). On any passage made from a library text — as written, read along, the first page of
your book — «Структу́ра» in the text's margin reveals it without moving a word, as a teacher's pencil
marks: the members of each sentence underlined the way a Russian school exercise book does it
(подлежа́щее one line, сказу́емое two, дополне́ние dashed, определе́ние wavy, обстоя́тельство and
prepositional phrases dash-dot), light brackets round subordinate, participial and gerund clauses and
direct speech, a small ring on every pronoun, participle or relative word that points at something on
the page. Tap a ringed word and its head pulses (even in the sentence before). Tap inside a participial
or gerund construction and its flat Russian appears under the text («лев, кото́рый спал»); tap it again
for the English. Tap a preposition or a governing verb and its case frame appears (по + Dat) with its
dependents pulsed. Hold a word down for its roles (подлежа́щее · in the прида́точное) and its root x-ray
— приста́вка + ко́рень + су́ффикс + оконча́ние, the root's meaning, and one or two words you already
know on the same root. No English until you ask for it. The toggle stays as you left it.

Reading pages made from a library text end with one or two *who is meant* questions: a pronoun or
participle of one of its sentences is marked and you tap its head. The outcome is reading evidence on
the `anaphora` / `participle` construction (and the pronoun), never a word to recall.

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
book expects you to recall are slipped onto pages one, seven and thirty days later. The back page,
*Practice evidence and estimates*, compares review predictions with observed answers, and the
memory model's intervals move a little, slowly, towards the evidence. When answers slow and misses
pile up late in a long page, one line appears where you are — *Make the rest of this page lighter* —
and nothing changes unless you tap it: then two remaining reviews become listening or reading
choices, the rest wait for a later page, and the line says what changed. Turning the page leaves it
as it was; the page is asked once. The page never shortens itself. Settings: retention aim
(80–95%), reviews per day, probes on or off. (`LearningCore/README.md`, "Time and the learner".)

## The tutor

The **?** beside each exercise (and *Ask about this page* at the bottom) asks Claude
(`claude-opus-5-5`) about that exact exercise and your attempt. Questions and answers are
written onto the page under *Вопросы*. The key lives in the device Keychain: type it in
Settings, or drop it into the app's `Documents/tutor-key.txt` with
`xcrun devicectl device copy to --domain-type appDataContainer --domain-identifier software.ember.drussky`
and the next launch imports it and deletes the file.

## Pages that write themselves

The **＋** in the top bar asks the tutor for a new page, placed right after the current one:
*Word families* (roots you already have two words of, with their other members to guess and
build; `Resources/morphology.json`, see COURSE_SCHEMA.md), *Cognates* (the Greek and Latin of
mathematics and philosophy, read as Russian) and *Grammar page* (the weakest cells of the
declension and conjugation tables whose words you know, cases interleaved; `Resources/paradigms.json`)
are built on the device without the tutor. The tutor
writes *A page from my mistakes* (built from what you've missed and find hardest) or *Read my own
text…* (paste or Scan Text any Russian; it becomes a glossed passage with questions, and its
new words join your reviews). Pages arrive in the course's own format and every exercise is
checked before it goes in. The tutor's answers mark Russian as tappable phrases you can
**Keep**. The **pencil** button turns the whole page into paper for margin notes (graphite;
double-tap the Pencil to erase). Speakers with a small number cycle through several human
voices.

## Talk

＋ → *Talk* (`Drussky/Talk/`) is a spoken conversation with the tutor on the page, and every
conversation has something to accomplish. The tutor proposes three scenes built from the words
you know; each is an information gap: the partner holds facts you don't (where the keys are, what
the shop has, which film they prefer) and you have a goal, told to you in English on the scene
card with two or three phrase frames you may need (from your own words first). The partner
answers the meaning you actually express from its facts, asks you to clarify when a line is
unclear, says no when its facts say no, and is not obliged to agree. It speaks one or two short
stress-marked sentences within your vocabulary plus at most two new words; then it is your turn
and the microphone stays idle until you tap it — time to read the line, replay it or think.
Your tap starts the same hands-free capture as *read aloud*, with a conversation profile
(`TalkSession.endpointing`: a pause of 1.5 s ends the turn rather than 0.7 s, up to 12 s of
speech, 10 s to start; "Still listening… a pause is fine; tap when you've finished"; nothing is
captured while the tutor speaks, and replaying a line closes the microphone, which waits for
your next tap).
Speech recognition transcribes it, the eye beside each line whispers the English and the new
words (each keepable); typing a line is always there. After 6–10 turns the tutor says goodbye and
the transcript is marked: first whether you reached your goal and what came across, judged from
your words alone (the marker is told the goal, never the partner's facts, which are then shown to
you), then your lines as heard with the red pen over them (`DiaryTutor`'s JSON contract per line,
lenient about what the recognizer could have misheard), one word of praise, the pattern behind
the biggest slip, and, where the phone-level analyzer is bundled, one pronunciation tip per take.
Every turn the tutor understood is `good` on the words you said (modality `own-speech`); each
edit is `again` with a speech confusion; new words and corrected forms join your words.
Conversations are saved inside the exercise's result and listed under *My Russian*.

The tutor's reply is a line-based format (`RU:` / `EN:` / `NEW:` / `END`) rather than JSON so the
voice can start on the first sentence while the rest is still arriving (`TalkLatency` is kept on
each tutor line). Models, measured 2026-11-08 from this machine with the app's own prompts: the
scenes call is `claude-sonnet-5-5` (first token 0.63–0.81 s against 8.7–12.7 s for Opus 5.5, with
the full state prompt or the words-only one — the prompt's size made no difference, so it sends
only the words); turns are Sonnet too: on ten sample turns (an opener, a clean order, a garbled
«С молоко и сахар», noise words, silence, an English line, a last turn) both models were 0/10
flagged on format, length (≤ 12 words), Latin letters and `END`; Sonnet missed one stress mark
in 30 multi-syllable words, Opus none; mean first token Sonnet 1.9 s, Opus 2.9 s (in-app Opus
turns ranged 1.9–9.4 s). The marking stays on Opus 5.5. The system prompt carries a prompt-cache
breakpoint. The lines are spoken by `TutorVoice`: the system voice is the first implementation
(`SystemTutorVoice`, the best installed Russian voice — Premium over Enhanced over default;
Settings → Talk says which one and how to download a better one in iOS Settings → Accessibility →
Spoken Content); a narrator engine slots in behind the same three calls. Offline, or without a
key, the page says so in one line and can be left (*Not today*, or turn the page); nothing is
started. A page turn cancels whatever is live; the lines said so far are already saved and the
page offers to continue.

## Shelves: «Ста́нция» and «Матема́тика»

Two shelves stand beside the course (`Drussky/Model/Shelves.swift`, `Views/ShelfViews.swift`).
**«Ста́нция»** (`Resources/series.json`) is a serial: 24 graded science-fiction episodes, each
written to follow one course page (`after_page`), fully stressed, every word not yet taught
glossed. **«Матема́тика»** (`Resources/math.json`) is a shelf of 24 original mathematical readings
in rungs, each with a translation, glosses, three questions and *how it is read*: the phrases
and notation of the text with their stressed Russian reading and the grammar behind them.

An episode or reading is an ordinary page: the passage (tap any word; *Show the translation*
when you want it), its three questions as *what does it mean* exercises (a question in English
asks about the text just read; a miss shows the answer and why), and, for an episode, a diary
line about what happens next — so everything done there is evidence like anything else. The
course never inserts one: when the page an item follows is finished, the next composed page
carries one calm card at its foot (*You can read this episode now · Read it*), once. Everything
is also open from ＋ → *Library* (one entry per shelf; the serial lists its episodes in order
with what each is ready after, every one openable ahead of time), from ＋ → *Ста́нция · эпизо́д
N* (the next episode), and from one row per shelf inside the cover. Nothing on the shelves is
voiced yet; the speaker falls back to the synthetic voice as it does for any unrecorded string.

**«Ку́хня»** (`Resources/kitchen.json`) is a second serial in the same format: 30 overheard
domestic dialogues of 25–80 words, one per course page, each with one to three things to hear
(«Дава́й + perfective future = let's …») listed under the passage once it is read.

QA: `-qa-series N` / `-qa-kitchen N` / `-qa-math N` open episode N / dialogue N / rung N as a
page (`-qa-partial 1` then marks its passage read).

## Listening: «На слух»

A third shelf (`Resources/listening.json`, `Model/Listening.swift`, `Views/ListeningView.swift`):
twelve sets of six to eight short situations said once, each set ready after one course page.
Seven kinds, one calm card each — the English task, **Play** (nothing plays until you tap),
**Replay**, **Slower** (¾ speed, same voice), the choices, and only after the answer the words:

| kind | you hear | you answer |
|---|---|---|
| which thing (`pick_object`) | a request («Дай сыр.») | the noun asked for, among taught nouns (Russian, stressed) |
| who needs what (`who_needs`) | two people, A then B, two voices; badges light as each speaks | A, B or both |
| follow (`follow`) | an instruction over a small scene — things on a tray, places to put them, as word chips | which result follows (one object's new place, or unchanged) |
| asking or telling (`question_or_statement`) | one of two identical sentences, as a statement (IK-1) or a yes/no question (IK-3) | Statement / Question; afterwards both forms, same voice, side by side |
| voice message (`message`) | two to four sentences | two or three English questions (gist and detail), judged together once all are answered |
| a correction (`correction`) | a sentence with a self-repair («Я хочу́ чай. Нет, ко́фе.») | the final meaning |
| who wants what (`disagreement`) | two people who do not agree | what each wants or what was actually agreed |

After the answer the stressed transcript appears in place with the deciding span underlined in
red and one line of *why* under it (for a message, the missed question's span, not the item's);
the right choice is marked, the translation and the item's glosses follow. Nothing scrolls or
moves. A miss offers *Try it again* (ungraded). Every answer is `hear-pick` evidence on the words
and constructions of the heard turn (a heard word is not evidence of its letters); a miss is
`again` on the words of the crucial span only; *asking or telling* is one skill of its own,
`sound:intonation`, whatever the words. A card is never a thing to recall later, and the tutor
never writes one.

A set is offered once at the foot of the next page after its course page is finished (the same
calm card as an episode: *You can listen to this now · Listen*), is listed in ＋ → *Library*
and on the cover's Shelves row («На слух · 12 sets · what is ready next»), and ＋ → *На слух · N*
opens the next one. Six reserved sets of five unheard messages (`weekly_samples`) never appear
there: once the course has finished a reserved set's page, **What I could do this week** takes
its five messages as the sample's listening part — played by text in a voice the course never
uses (aidar), answered in English on the ruled lines, the Russian and the model meanings shown
only after the first response is sealed, never graded — and falls back to the bank of human
recordings until then. Each set is used once.

Voices: `voice_hint: any` keeps the course voice (xenia); `unfamiliar` moves to aidar; an
exchange is always two voices (xenia/aidar, or aidar/baya when unfamiliar), and a replay keeps
the speaker. `scripts/make_listening_audio.py` makes the clips with the same rule
(`ListeningItem.voice(forLine:)`): 288 Silero clips for the 338 strings (the other 50 already had
course clips), filed as `listening-<voice>-…` in `Audio/index.json` or, where the text already
has a course clip in another voice, in `Audio/variants-listening.json` — the mechanism
`variants.json` uses for other speakers of one word; `Voice.recordingURL(for:voice:)` picks the
speaker by that prefix. `make_morphology_audio.py --voice=xenia|aidar|baya` is the shared
generator. `scripts/validate_listening.py` is the content gate (in `check_content.py`) and
writes the spoken inventory `research/artifacts/listening-spoken.txt`. QA: `-qa-listening N`
opens set N; `-qa-listening-week N` opens the weekly sample with reserved set N's messages.

## Your book

The inside cover asks for one text you want to read for real — from the Library, pasted or
scanned, or a ru.wikisource link (fetched through the MediaWiki parse API and cleaned to its
paragraphs). It becomes *your book*, kept in the notebook (several allowed; one current), and
the cover keeps **Your book** with a ladder
of the next twenty words worth knowing (occurrences in the text, weighted toward the first
page) and the grammar patterns it leans on. The text's words are lemmatized by the course's
tables, then by the tutor (`claude-sonnet-5-5`, 300 forms per call, cached in the goal with a
short gloss), with a rule-based stemmer standing in until that arrives.

**Toward my book** (＋, or the panel) has the tutor write a page from the goal's own sentences:
a passage of 2–4 consecutive sentences that are at ≥95% production-backed form coverage (glossed), then exercises on the
most valuable unknown words taken from the text. Every Russian string on the page must be a
contiguous stretch of the text; anything invented is dropped. Without a key, an offline page
teaches the words through their own sentences. The book also writes one by itself when a
brand-new course page would teach nothing toward the goal while a readable stretch waits (at
most one in three pages). When the first five sentences are each at ≥98% form coverage, the book opens
them as a page of their own — *Try the first page of your book* — with tap-to-gloss
and a red «Пе́рвая страни́ца» stamp. Policies and thresholds: `Drussky/Goal/Goal.swift`;
the distance arithmetic is LearningCore's `Target`.

## Inside the cover

The book icon (top left) opens your alphabet — each letter in **your own handwriting** once
you've met it — and every page so far with its mark. **Grammar** is one row of chips, one per
table of the system behind the words (the three noun declensions, adjectives, pronouns, the two
conjugations and the irregular verbs, past and imperative, what each case does), each counting
the cells you have met; a chip opens its table, *All tables* opens them all. The tables start
empty and fill in as you meet and produce each cell (a cell glows by the recall of its pattern, a
dot marks one you have written yourself); tap a cell for its examples with audio and history. **My Russian** collects every diary line
you have ever written, newest first, with its red pen, and what the pen keeps finding this
week; it exports as Markdown. Settings: finger drawing, cursive
models, stress marks, notebook export (pages, ink, recordings as a zip), the notebook as a PDF, credits.

## On paper, in a pocket, at a keyboard (`Drussky/Devices/`)

**Print your тетрадь.** Settings → *Export notebook as PDF* (A4 or Letter; the local one is
picked) lays every page out as it is in the book — squared paper, red margin, dated header,
numbered exercises with your ink on their ruled lines, red ticks and corrections, the diary's red
pen, the tutor's questions, the mark — and hands it to the share sheet. *Print this page* at the
foot of any page does one page. Ink is zoomed from its strip so nothing written is cut off, and a
page is laid out as blocks that are never split across sheets (`NotebookPDF.paginate`).

**Pocket (iPhone).** On a phone, ＋ → *Pocket: listen and speak* writes a page with nothing to
write: due items as *listen and pick* or *say it* (a dictation or translation that is due is said
instead), real-voice sentences of known words to pick or repeat, sound pairs on the weakest
contrasts, and one passage already read, to hear again — sized for five to ten minutes
(`PocketPage`). The full page is still there behind it.

**Typing Russian.** With a hardware keyboard (always on the Mac; an iPad with a keyboard), every
writing exercise offers *Type it instead*: a field that is checked exactly like handwriting, with a
small ЙЦУКЕН map under it lighting the next key you need (a wrong key lights *delete*). Type on the
system Russian layout, or switch on the **phonetic** scheme and type Latin
(`Transliteration`): `a b v g d e zh z i j k l m n o p r s t u f h c ch sh sch y`, `yo yu ya ' ''`
for `ё ю я ь ъ`, `eh` for э, `x` for х — `ya chitayu` → я читаю, `novyj` → новый, `zhizn'` → жизнь.
A typed answer is recorded under its own `type` modality: knowing how to type a word is not
knowing how to write it. ＋ → *Typing drill* (keyboard devices) runs home row → all letters → your
own words, one line at a time, with words per minute shown after each line and the median speed
of each day as a thin line (kept in `typing.json` in the notebook folder). The Pencil stays
first on the iPad: the strip is always there and typing is a button under it.

QA: `-qa-pocket` inserts a pocket page, `-qa-typing` opens the drill, `-qa-keyboard` pretends a
keyboard is attached (simulator).

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

Narrated library texts (`Audio/narr-*.m4a`, registered in `index-narration.json` /
`sources-narration.json` with their `*.words.json` word timings) are excerpts cut out of whole
public-domain chapters at offsets recorded in the manifest, so `restore` rebuilds them from the
download alone. Timing a new text needs the aligner venv once:
`cd .local/drussky-librivox && uv venv venv --python 3.12 && uv pip install --python venv/bin/python ctc-forced-aligner unidecode`,
then `.local/drussky-librivox/venv/bin/python apple/scripts/narration.py align <id>` and
`python3 apple/scripts/narration.py build <id>` (see `research/NARRATION_ALIGNMENT.md`).

## Building

```sh
cd apple
xcodegen generate
# simulator tests: the fast loop (DrusskyTests + LearningCoreTests in one xcodebuild, ~70 s of test
# time), the slow target, or all three unit-test targets; DRUSSKY_SIM picks the simulator
scripts/test.sh            # fast
scripts/test.sh slow       # DrusskySlowTests: Vision over hundreds of ink samples (~80 s), when ink or recognition code changes
scripts/test.sh all
# the same by hand (-collect-test-diagnostics never: otherwise a run can end with ten minutes of `simctl diagnose`)
xcodebuild -project Drussky.xcodeproj -scheme Drussky \
  -destination 'platform=iOS Simulator,name=Drussky iPad 13' -derivedDataPath .build/dd \
  -collect-test-diagnostics never -only-testing:DrusskyTests -only-testing:LearningCoreTests test
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
string the app or its reviews can speak), handwriting recognition on a dozen words of synthetic
print and cursive ink (cursive 12/12, print 12/13), scheduling and persistence, notebook-recovery and redo
regressions, speech recognition on recorded Russian (runs once speech recognition is
allowed), self-judgement evidence (verdict × reader → grade and confidence), hooks (parsing,
storage, fading) and fluency (words a minute from onsets, strokes a second from ink, the metric
log, the cover line), and live tutor calls — one answer, one diary correction, one hook and a
small hook batch (run when `TEST_RUNNER_DRUSSKY_TUTOR_KEY` is set). `DrusskySlowTests` holds the
recognition measurements that take as long as they have samples: 156 single-line pen-stroke
samples with hand-like slant and wobble (on the iPad Pro M4: pen 155/156; simulator 153/156) and every
bundled cursive hand × ten words with and without the sheared pass. Vision runs one recognition at a time
whatever the concurrency, 0.15–0.5 s each on the simulator, so these are not made fast, only kept apart.
`DrusskyUITests` writes on the paper with real touch strokes, completes the exercise, and
checks it survives a relaunch.

QA launch arguments (simulator): `-qa-reset`, `-qa-no-key` (Debug: hides the tutor key for this launch without changing Keychain), `-qa-complete N` (work through N pages with
synthetic ink), `-qa-partial K`, `-qa-age D`, `-qa-top`, `-qa-bottom`, `-qa-contents`,
`-qa-settings`, `-qa-diary` (three diary pages with canned red-pen corrections, no key
needed), `-qa-myrussian` (with `-qa-contents`: opens My Russian);
`-qa-talk fresh|scene|mid|marked` (a Talk page: untouched, the scene choice, the middle of a dialogue, the
marked transcript; `-qa-nokey` hides the tutor key for the launch without touching the Keychain, `-qa-offline` poses the offline note);
`-qa-settings`; `-qa-day N` / `-qa-away D` (the pages were done N or D days ago, then the page is
turned: a morning or welcome-back page), `-qa-probes` (a synthetic answered probe set for the cover
chart), `-qa-tired` (the offer to make the rest of the page lighter, standing on the current page);
`-qa-settings`, `-qa-goal <library id>` (make that text the learner's book), `-qa-goal-pull` (an
offline page toward it), `-qa-goal-first` (open its first page now); `-qa-judge` (ink on the first
open recall exercise with Check tapped: the self-judgement panel), `-qa-hooks` (canned hooks on a few
words; `-qa-contents -qa-node lex:кот` opens one), `-qa-structure <library id>` (that text as written with
«Структу́ра» on, then its *who is meant* questions), `-qa-fluency` (three weeks of synthetic fluency
samples for the cover line and sparklines); `-qa-grammar` (a grammar page from the tables;
`-qa-grammar-partial K` works through its first K exercises), `-qa-grammar-table decl1` /
`-qa-grammar-cell decl1/gen-sg` / `-qa-grammar-all` (with `-qa-contents`: open that table, that
cell, or every table of the Grammar section).

## Content

`Drussky/Resources/course.json` is generated by `scripts/build_course.py` and checked by
`scripts/validate_course.py` (schema in `COURSE_SCHEMA.md`: letters before use, stress
marks, page sizes, answer integrity). `scripts/check_content.py` runs every content validator
at once: the course, the library (`validate_library.py`), the mathematics shelf
(`validate_math.py` and its unit tests), the serials (`validate_series.py`), morphology and
listening (`validate_listening.py`). Hand-printed stroke models for all 66 letter forms
(`Drussky/Resources/strokes.json`) are built by `scripts/build_strokes.py`. Audio lives in `Drussky/Resources/Audio/` with
`index.json` (audio key → file) and `sources.json` (provenance and licences). The audio key is
the normalized text plus `?` for a rising (yes/no) question, so «Э́то чай?» and «Э́то чай.» are
different clips (`RussianText.audioKey`, `validate_course.audio_key`; `scripts/audio_identity.py`
files every clip by what its own provenance says it says). Every
string the app can speak, including generated reviews, has a recording — a unit test
enforces it. `scripts/make_audio.py` (local Silero) generates missing ones.

Learner data lives in Application Support/Drussky (`book.json`, `ink/`, `voice/`).

## Weekly samples and reading evidence

The inside cover's **What I could do this week** opens a dated sample page: one new beginner
scene with gist/detail/inference questions, five existing human recordings from five speakers,
an unscripted spoken task, and an uncorrected paragraph. Responses can be written or typed;
the spoken task keeps its first recording. Saving locks the first response and reveals comparison
notes, with a separate reflection field. Help, previous familiarity, replay starts and rough weekly
exposure remain with the sample. These tasks do not grade vocabulary or change recall traces.
`-qa-baseline` opens the flow in the QA notebook; `-qa-contents -qa-evidence` opens the estimates back page. There are currently two parallel reading sets;
after those, the app asks for new material rather than presenting a repeat as unseen. The five
messages come from the listening strand's reserved sets once the course has reached one
(«На слух», above), else from the bank of human recordings; familiarity is self-reported, not
inferred.

Goal and library vocabulary coverage require successful production of the **actual written form**
in hear→write, English→write or see→say, with current predicted recall at least 0.85. Copying,
recognition, inferred lemma credit and root credit do not count. Supported-passage selection uses
95% coverage and the first-page invitation uses 98%. Neither estimates comprehension; text remains
available to study below those thresholds. Percentages count running forms; the first-page bonus
only affects the vocabulary teaching ladder.

After the accusative and genitive have been encountered, **Which ending fits?** offers a short
concentrated introduction followed by decisions in new sentences. `construction-choice` has its
own scheduling trace and no production credit. The finite bank never recycles a sentence as new;
two kinds of context are reserved from ordinary review for checks scheduled at 7 and 30 days.
Probe records retain the first-attempt flag, help/disputes, context and last logged exposure, so the
scheduled delay is distinguishable from elapsed time without logged practice. Weekly pages show
these later examples in prose. The knowledge map, root estimates, hourly observations and
calibration chart are under **Practice evidence and estimates**, at the back of the inside cover.
