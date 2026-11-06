# Rufren design

## Destination

Build the best Russian learning environment for one particular person. Ember begins with no Russian, wants to speak naturally, understand speech, read, and write, and loves science fiction and literature as well as mathematics and philosophy. The AI teacher carries everyday instruction: explanations, examples, corrections, pronunciation demonstrations, practice and review. Ember’s friend is a willing conversation partner at breakfast and dinner, not a formal coach or a source of assigned recording, grading or curriculum-review work. Success means capabilities that persist outside the app. Prefer a Rust implementation that can grow from Apple devices to Android, Linux, Windows, and web.

Working memory is sometimes challenging. Treat that as a variable demand to accommodate, without inventing a fixed cognitive profile. Conceptual sophistication and the ability to retain an unfamiliar sound sequence are separate things. Offer rich explanations alongside short practice steps.

The evidence IDs below refer to [the evidence review](research/EVIDENCE.md). Product choices are hypotheses to test with Ember, rather than consequences automatically proved by the cited research.

## Product navigation and hierarchy

The interface keeps five primary destinations available: **Today, Learn, Talk, Read and Notebook**. Phone layouts use bottom tabs; tablet and desktop layouts use a sidebar with the same destinations. Practice and Speech studio belong under Learn; Progress and Settings belong under Notebook. Secondary tools should be discoverable without competing with the learner’s main destinations.

Today resumes the exact unfinished lesson and shows useful next actions; empty return sections disappear. Learn has a searchable lesson catalog and a compact catalog-to-detail path. Inside a lesson, an explicit outline exposes the steps instead of hiding navigation in a selection menu. Saved passage practice searches the complete available history and reveals rows in groups of twelve with “Show more”; an eighty-item cap must not make older material unreachable.

Feedback puts the question, original answer, concrete corrections and next step first. Extra explanation, examples and practice use disclosures. Explanation opens by default when there is no concrete correction, so a meaningful response does not disappear behind an empty feedback summary. Support remains available without forcing every detail into the initial view.

These choices follow Apple’s guidance on [tab bars](https://developer.apple.com/design/human-interface-guidelines/tab-bars), [sidebars](https://developer.apple.com/design/human-interface-guidelines/sidebars), [pop-up buttons](https://developer.apple.com/design/human-interface-guidelines/pop-up-buttons), [lists and tables](https://developer.apple.com/design/human-interface-guidelines/lists-and-tables), and [disclosure controls](https://developer.apple.com/design/human-interface-guidelines/disclosure-controls). They are product decisions, not a claim of native control implementation or completed HIG/device acceptance. The current revision passes scoped browser navigation/catalog checks; actual iPad use and further usability work remain.

The project has spent too much attention on foundational machinery before establishing a coherent everyday product. Better navigation is a necessary correction, while the destination remains fluent four-skill Russian, native-like pronunciation and independent literary/scientific reading. Advanced curriculum is still thin, learning outcomes and pronunciation assessment remain unvalidated, and broader device/OS runtime work remains open.

## The central learning loop

1. **Encounter:** the AI teacher introduces a useful utterance in a small conversation or reading passage, with generated or curated audio ready to play. Preserve meaning, situation, exact text and audio provenance. A phrase picked up over a meal can also become material when Ember wants.
2. **Understand:** make its sound and meaning clear. Offer a brief explanation, replay, phrase segmentation, and a relevant contrast. Show what an ending or stress pattern changes.
3. **Retrieve:** hear and interpret it, then produce it from a meaning or situation. Reading aloud, spelling from dictation, and writing an original answer are distinct exercises.
4. **Refine:** target one useful correction, try again, and listen to the difference. Return to a short meaningful exchange so the target is used rather than merely imitated.
5. **Revisit:** bring it back after a delay, including a changed speaker or context and an opportunity for independent production.

Learning content should be reusable across these activities. The app records what each attempt actually demonstrated, not a single global mastery score [E1–E4, E7].

### An initial session with the AI teacher

The teacher starts with one short useful exchange, explains its meaning and an important sound or spelling feature, and supplies a playable pronunciation model. Use generated speech or suitable curated recordings, cached for reuse, rather than asking another person to record the curriculum. Let Ember hear normal speed, replay a small span, see stress or chunks when useful, and try a short response. The teacher gives one useful correction tied to the actual task and response, then offers another meaningful turn.

The same material supplies listening, meaning-to-speech, Cyrillic reading and writing tasks. Listening-only retrieval hides answer-bearing text and tracks replay support. A scheduled delayed review attempts retrieval before showing the answer again. New utterances later test whether the useful feature transfers. A phrase is not learned in all four skills because one exercise succeeded.

Feedback keeps the exact question and saved answer beside the explanation. Reading input is distinct from instructions; oral feedback shows the confirmed words and exact original recording, and discusses language without claiming to assess pronunciation. An optional “Try this question again” opens supported practice on that original task inline. Earlier answers and critique remain available, unfinished work resumes, and a fresh oral response requires a new recording. This is a repair opportunity, not evidence of independent delayed recall or a clean response before help.

The teacher should use the learner history to choose a manageable next step, explain the choice when useful, and preserve drafts and assistance. Cached lessons, examples and audio support ordinary offline use; unavailable cloud services should leave a useful local path and a recoverable request rather than send Ember to find a human grader.

### Zero-beginner foundation: implementation specification

**Status:** the first three episodes are implemented, browser-reviewed and installed on iPad; learning outcomes and the continuing beginner course are not established. The current dialogue-first first lesson is unsuitable for a learner who knows no Russian. It asks for decoding, phrase retention and output before teaching those prerequisites. Preparing its dialogue audio does not repair that ordering. This section governs the first-run path and subsequent beginner progression; the broader conversational loop above remains the destination.

The foundation has three short, untimed episodes. Each screen has one immediate action, an English instruction and a persistent replay/help control. A compact progress control replaces a competing lesson catalog inside the activity. Introduce one mapping at a time; completing an episode means encountering and trying its activities, not mastering its letters. “Smaller step”, “Show me”, “Skip for now” and stopping remain available. A skipped prerequisite gets a supported introduction when next needed rather than a permanent curriculum lock.

#### Episode 1 — Hear a sound; recognize its letters

Entry assumptions: English instructions are accessible; no Russian letters, words, keyboard or pronunciation terminology are known. Introduce **А а**, then **М м**. Do not display the complete alphabet or a Russian dialogue as required input.

1. In English, explain: “Russian uses a different alphabet. We’ll connect two letters to sounds, then try two tiny sound patterns. You can replay anything.” Show **А а**, play its prepared vowel model, and explain the model with a short English cue. The sound is the authority; an English approximation is only a cue. Learner taps Replay, then Continue; this is exposure, not a scored response.
2. Introduce **М м** with prepared **ма** audio: “Listen to the beginning, before the vowel.” Highlight **м** and then **а** while replaying their syllable. Do not ask TTS to pronounce the isolated glyph м and call the resulting letter name a consonant model. Offer optional imitation without scoring it.
3. With both taught shapes visible, play the prepared **а** vowel or the **м**-initial **ма** example and ask which highlighted letter represents the target sound. State whether the question concerns the whole vowel or the beginning of the syllable. An error says, for example, “This example begins with the sound represented by М. Listen to the beginning of ма again.” Replay, highlight the target and offer Try again. A retry after that demonstration is assisted practice.
4. Show **ма**, then **ам** separately. Say these are **practice syllables, not new vocabulary**. Demonstrate blending with prepared whole-syllable audio and synchronized shape highlighting; do not splice consonant letter names into a word. Learner taps the two letters in spoken order. During a later reading turn, present one syllable without its model; let the learner attempt it aloud, then reveal/play the model for self-comparison. Without a reliable acoustic judgment this is an attempted reading, not automatically correct pronunciation.
5. Add a useful meaning without requiring its spelling: play **да** (“yes”) and **нет** (“no”), one at a time, with English meaning visible. These are explicitly **whole spoken chunks whose letters have not been taught**; default display has English meaning and replay, not answer-bearing Cyrillic. Then hide the meaning, play one chunk and offer English Yes / No choices. A wrong choice names the actual meaning, replays the chunk and invites a retry. Change answer positions. A tiny English-framed radio exchange can make this adult and meaningful: a character answers whether the signal arrived; the learner interprets the Russian reply. No extra Russian sentence is smuggled into the task.
6. End with “You practiced А, М, two sound patterns, and hearing yes/no.” Offer repeat, stop, or next episode. Do not report that listening, reading and speaking are mastered. No typing, recording permission, account configuration, friend or live AI response is required to finish this episode.

#### Episode 2 — Read one real word

Entry inventory: А а, М м introduced; ма/ам demonstrated; да/нет heard as whole chunks. First offer one short retrieval of a taught mapping and a replay/reteach branch. No failure threshold locks continuation. Introduce **О о**, **К к**, **Т т**, in separate sound/shape cards, using prepared vowel and syllable examples (**о**, **ка**, **та**) and pointing explicitly to the target beginning where relevant. All written example letters are already taught or introduced on that card. Warn only when useful: О in this stressed example has this sound; its sound can change in other words. Do not introduce vowel reduction as another test here.

Teach **кот** as “cat” with a picture or clear English gloss and a prepared whole-word model. All three graphemes are now introduced. Show к → о → т alongside the whole word; avoid pretending synthetic letter-name concatenation is natural speech. Learner builds **кот** from a small tray of taught letters, with the gloss and whole-word model available. A misplaced letter gets a specific position cue and model, not a generic “Wrong.” Then show **кот** without its gloss/model and ask the learner to try reading it before revealing meaning and audio. Copying/building after hearing is supported spelling; selecting its picture measures meaning recognition; neither alone proves independent decoding.

Return briefly to the audio-only yes/no task after an intervening letter activity. The learner may also record кот and compare it with the reference if recording is available. This is optional speaking practice; the episode stays useful without microphone permission. End inventory: **А М О К Т** and the meaning **кот**. Do not count practice syllables as acquired vocabulary.

#### Episode 3 — Send a small message

Entry inventory: А М О К Т introduced; кот taught; да/нет audio meanings introduced. Begin with an optional supported revisit of кот. Introduce only **Д д**, using prepared **да** as the sound-in-syllable model, then connect the already-heard “yes” chunk to its now-decodable spelling **да**. **Нет** remains an audio-first chunk: Н and Е are not yet introduced. It must not appear in an independent reading or dictation prompt.

Teach **там** (“there”) with a visible scene showing the same cat at a distant indicated location, English meaning, prepared audio and the three already-taught letters. Keep the referent and location visible; do not introduce here/there discrimination without teaching a second location word. Demonstrate **Там кот.** (“There is a cat there.”), including word spacing, capitalization and the period; the same lower/upper letter pairs were introduced previously. Explain in one optional sentence that this Russian message needs no separate word for “is”.

Learner assembles **Там кот.** from persistent word tiles to describe the visible scene, then optionally builds one of its words from letter tiles or types with an available Cyrillic keyboard. Keep the task instruction visible. Word tiles are supported message construction, not free writing. For an error, name the particular word/position, show its meaning and replay the short model; preserve the original attempt before retry. Offer reference/learner A/B playback if recording is available; never convert a matching transcript into a pronunciation score.

After an intervening task, show **да**, **кот** or **там** without its answer-bearing model and invite an attempted reading before self-comparison; separately offer a sound-to-English-meaning choice. Record each task and support independently. End with a small useful message and cumulative inventory **А М О К Т Д**, not a claim to have learned the alphabet. Offer continued sound/script work and an explicitly supported first exchange.

#### Prepared sound and offline delivery contract

Episode 1 ships with playable local assets for а, ма, ам, да and нет, plus any distinct guided-blending model actually used. Episode 2 adds о, ка, та and кот; episode 3 adds там and Там кот. Reuse the same да reference when appropriate. Do not silently depend on on-demand TTS, a configured cloud account or an external recording task. Each required asset has an ID, exact spoken target, model/source provenance, encoding identity and an explicit relationship to its displayed teaching span. Asset generation must check actual audio, not merely a successful provider response. A prepared synthetic model is labeled synthetic; preparation does not establish native naturalness or pedagogical efficacy.

Consonant **names** and consonant **sounds in syllables** are distinct. Synthesizing М may yield эм, К may yield ка, Т may yield тэ, and Д may yield дэ; these must never be passed off as isolated consonant sounds. In ка the model includes the vowel: the card explicitly asks the learner to notice its beginning. Do not automatically crop stop consonants or promise their isolated playback is a natural pronounceable unit. Keep a normal whole-syllable/word model alongside any pedagogical segmentation. Check stress, syllable identity and unintended carrier words in every shipped clip.

A required Play control starts its existing clip in one action; “Create spoken example” is not Play. If an asset is absent or playback fails, identify the unavailable sound and offer retry or an explicit skip. Do not silently turn listening into a text task or award listening evidence. An episode with missing required audio has failed offline acceptance, even if skipping allows exploration.

#### Small prerequisite and resume state

Use authored data rather than a new inference engine: each step declares stable episode/step IDs and revision, introduced graphemes/meanings, required graphemes/meanings, displayed text, prepared asset IDs, response mode and feedback. Give whole audio chunks an explicit `audio_first_chunk` treatment with script hidden by default until introduced; optional script revelation is recorded support, not a decoding requirement. Check required input/output against prior introductions. Capital/lowercase pairs are introduced together; punctuation/spacing used in a response are demonstrated explicitly. Instructions remain English. Optional reference text may exceed the inventory only when clearly marked as untaught and never necessary to act.

Introduced is not mastered. A learner may explore later lessons; the host supplies missing prerequisites or labels the encounter as supported exploration. It must not fabricate prior knowledge from a prerequisite checkbox, importing phrases or completing unrelated tasks.

Persist current episode/step/version, unfinished answer/tiles, completed encounters, first response and later retries, revealed aids and playback state sufficient to restart safely. Resume restores the exact task, visible supports and draft; audio remains stopped until requested. “Show me” preserves the draft and records assistance. Do not advance a failed/unsubmitted answer when closing, reopening or navigating away. Keep prior wrong responses as observations rather than replacing them with the successful retry. Recording state resumes as a saved clip or an explicit interrupted recording, never an apparently running microphone.

#### Continue through the course, not back into an unchanged catalog

The existing 24-lesson pack is reusable editorial material, not a suitable default sequence in its current broad lesson units. Follow these episodes with recurring short sound/script encounters: introduce a few mappings at a time, then use cumulative words; explicitly teach visual traps such as Р, Н and С when they enter. Introduce stress, vowel reduction, softness, clusters and connected speech in separate manageable encounters tied to words already given meaning. The exact letter order beyond these episodes remains an implementation decision, not a research result.

Keep a parallel useful-exchange strand. **Привет** can be heard, understood and imitated as a whole chunk before every letter is taught; flag its untaught script and supply meaning/audio. A first exchange begins with one demonstrated turn and a supported reply, not six phrases plus a general explanation. Break the current hello/repair material into such encounters; retain broader explanations as optional references. The current naming lesson mixes object identification, letter mappings, hard/soft consonants and stress minimal pairs: split these into separate later encounters. Do not place it after two phrase lessons and call the collapsed alphabet reference sufficient instruction.

Each subsequent practice step declares its small actual prerequisites. The next-step picker chooses supported decoding from the cumulative inventory and a short communicative use; conceptual-interest scenes can appear early with English context and limited Russian. The full alphabet reference stays available with introduced/next status, but neither memorizing 33 names nor passing a gate is required to speak. Preserve stable content identities and attempts when repackaging existing phrases. Reuse the phrase library, presentation/audio infrastructure and attempt history; replace the initial dialogue-first route and generic read/write/say/listen sequence where their prerequisites are absent.

#### Learner-level release checks and evidence limits

These gates concern the intended experience; route tests alone cannot satisfy them:

- With fresh storage, network disabled and no configured provider, complete episode 1 using its actual prepared audio. A novice can understand every instruction without decoding untaught Russian or opening another tool.
- Audit every required written response and reading stimulus against prior introductions. Whole chunks with unknown script are explicitly identified and do not become accidental reading tests. Audit meanings too: a picture or English label must establish what a new word refers to before meaning retrieval.
- Deliberately choose a wrong response on each task type. Receive task-specific model/correction, retain the original answer, and retry without being credited as independently correct. A reveal or replay is visible assistance.
- At the narrowest supported layout, one immediate action remains obvious, instruction/context stays visible, and navigation does not take half the usable activity width. A paragraph or full dialogue must not become the prerequisite to finding the next action.
- Stop mid-answer and during optional recording; resume the exact safe state without lost tiles/text, false completion or an unexpectedly live microphone. Deny microphone permission and still complete the core episode.
- Separately observe audio meaning recognition, sound/shape work, attempted decoding and supported spelling. No one response produces all-four-skills credit; neither transcript matching nor self-comparison certifies pronunciation.

A simulated novice walkthrough can falsify hidden prerequisites, inoperable audio and poor feedback. It cannot show that Ember or another human learned. Later human use should check whether instructions make sense, whether taught material can be attempted before cues, and what persists or transfers after a delay; report assistance and uncertainty. Do not promise a validated dose or impose a research session on every ordinary visit.

This design follows the existing evidence's bounded implications: orthography and auditory knowledge can diverge, so teach script with sound and assess listening separately ([E8](research/EVIDENCE.md); local primary review `~/paperbin/rufren/orthography-phonology2021.txt`). Demonstration and specific repair are preferable to repeatedly demanding an unknown answer; the classroom feedback synthesis supports feedback opportunities but does not validate this app's judgments ([E5](research/EVIDENCE.md); local primary text `~/paperbin/rufren/lyster-saito2010-oral-feedback.txt`). Retrieval and later revisits motivate preserving pre-help attempts ([E1–E2](research/EVIDENCE.md)). The selected letters, three-episode sequence, interface supports and learning benefit remain design hypotheses, not findings established by those papers. See also the already-written executable teaching policy in [AI teacher pedagogy](research/AI_TEACHER_PEDAGOGY.md).

### Conversations over meals

Offer an optional invitation such as asking a real question, understanding a reply, or trying a phrase at breakfast. Ember can keep a note or a phrase afterward, and the teacher can help unpack it. The friend does not need to use the app, prepare lessons, approve generated material, label recordings or complete a review queue. Recording another person is a separate explicit choice. Optional human feedback remains welcome and retains its source, without becoming a prerequisite to everyday learning.

## Working-memory support

- Keep the current context, instruction, and progress visible; reopening an explanation preserves the attempt.
- Let Ember replay short natural segments, repeat a whole utterance, and choose a smaller step. A slowed version is optional; keep normal-speed audio available.
- Introduce a sound contrast with familiar task mechanics before combining it with new spelling, vocabulary, and grammar.
- Allow untimed practice and an easy switch to more support. Response latency includes device, reading, retrieval, and motor demands; it is not a pure ability measure.
- Record assistance: replay, revealed text, translation, partial cue, model answer, teacher explanation, and any human help. Prompted success and independent success are different observations.
- Fade cues within a skill when useful. During listening-only assessment, text that reveals the answer is a hint, not invisible assistance.

Personalization starts with observable confusions, self-reported effort, and performance over time. We should not diagnose a memory problem or estimate a permanent capacity from a handful of errors. A configurable session length is a convenience, not a scientifically optimal dose.

## Speech laboratory

The aim includes native-like pronunciation, fluent connected speech, and being readily understood. Track these separately. A spectrogram is an instrument for examining sound; it is not an accent verdict [E5–E6].

### Pronunciation models supplied by the product

Generated and curated audio are the default teaching references. Preserve exact input text, stress/disambiguation instructions, provider/model/voice or source/licensing details, generation settings, and encoded-audio identity. Label synthetic speech as synthetic, retain ordinary spelling separately from pronunciation scaffolds, and cache successful results. Check homographs, stress, numbers, symbols and specialist vocabulary; a plausible voice can still pronounce the wrong word or stress.

The teacher can explain and correct language content from written text or a transcript, while naming that evidence. Acoustic claims about stress, vowel quality, intonation or an accent require actual audio and an appropriate method. Generated examples and helpful provisional feedback are usable without a blanket human-signoff gate. Their availability is separate from evidence that an automatic assessor is reliable or that Ember has achieved the pronunciation goal.

### Useful first capabilities

- Clean reference and learner recording, instant A/B playback, repeatable cropping, and waveform/spectrogram views.
- A syllable timeline and stress locations with a stated linguistic source and confidence. Later, reliable duration and relative pitch tracks can help compare stress and intonation.
- Examples contrasting hard and soft consonants, vowel quality, voicing, and connected-speech patterns, chosen by the teacher from observed difficulties and the current linguistic target.
- Evidence-linked teacher feedback tied to time spans where audio supports it, with a short practice cue and a follow-up recording. Optional human observations retain separate provenance.

A useful next instrument is an editable two-region comparison for teacher references and learner-selected spans: duration, shared-scale spectra and original-time A/B playback. The [existing Silero pair audit](research/SILERO_STRESS_AUDIT.md) supplies exact crops and exploratory duration/spectral differences; it does not justify a universal stress threshold or confident LPC formants when poles are unstable. The original-frame two-region view is now implemented and browser-accepted, with exact identity/boundary/export/replay and input-race checks. Native use and the interpretation of a learner’s sounds still require separate evidence.

Keep the detailed instrument view optional. The ordinary practice screen should make the next useful action obvious. Raw spectrogram similarity is a poor objective: voices, microphones, room acoustics, vowel contexts, and speaking rates differ.

### Analysis pipeline

Capture original PCM audio with device/sample-rate metadata. Check clipping, silence, background noise, and recording interruptions before attempting pronunciation analysis. Preserve the original; derived displays and playback transformations must be reproducible.

Use windowed spectral analysis for visualization. For scored features, add pitch, duration, formants, or consonant cues only with a specified linguistic question and an evaluation set. For example, adjacent vowel formant transitions may help examine particular palatalization contrasts; no single F2 threshold establishes correct palatalization across all contexts [E6]. Normalize comparisons appropriately for the feature and speaker rather than demanding that Ember reproduce someone else's vocal tract.

An aligner can suggest where sounds occurred. A recognizer can suggest what was said. Neither proves that every expected sound was pronounced correctly. A forced aligner may assign the expected phoneme sequence to a mispronunciation. Keep an ASR hypothesis tied to the exact original recording, span and derivation; let Ember confirm or edit the words before a text teacher discusses them. This adoption preserves the original audio and is not a phonetic certification. Any alignment must support uncertainty and manual correction. Time alignment for visual comparison must not erase duration differences from the measurement being evaluated.

An LLM can explain a supported observation and propose a drill. Feedback must name the relevant audio span and evidence. A transcript alone cannot support claims about tongue position, vowel quality, stress, or intonation. Use uncertain results to invite replay, another example or a narrower question. Seek optional independent evaluation when needed to test a feature; do not require a friend to adjudicate routine responses.

### How automated feedback earns trust

Collect multiple attempts, conditions and reference speakers over time. Evaluate individual contrasts and stress against suitable held-out Russian examples and, where the question requires perceptual judgment, qualified independent listeners. A licensed labeled corpus or a deliberately recruited evaluation session can supply evidence; the breakfast companion is not the default labeling workforce. Keep development clips separate from held-out words, sessions and voices. Check false corrections as well as missed errors, including errors introduced by microphones and alignment. This evaluation is required before presenting a feature as a reliable pronunciation assessor, not before Ember can use generated examples or ordinary teaching feedback.

Compare ordinary A/B listening with the same teacher-guided practice plus a specific acoustic display, holding practice time roughly equal. Check delayed production of new items. A better-looking plot without an audible or functional benefit is insufficient. The Russian ultrasound experiment had 18 learners and did not establish general superiority over audio-only familiarization [E6]. This supports targeted experimentation, not abandoning visual tools.

## Russian curriculum

### Sound, script, and everyday use together

Teach Cyrillic through actual sound–word connections, while retaining separate listening-only practice. Letter names, letter shapes, spelling, and speech sounds are related but distinct skills. Use stress annotations as a scaffold and preserve ordinary spelling for later reading. Written marks alone do not establish auditory mastery [E8].

Begin with personally useful interactions: greetings, requests, asking what something means, asking for repetition, describing the immediate environment, and short exchanges with the AI teacher and opportunities to try them in real conversation. Add varied speakers and contexts gradually; the best number of voices for Ember is not known [E3].

### Grammar as meaning

Teach cases, agreement, aspect, and motion through contrasts in what is happening, who is doing what, where something is, and where it is going. Pair compact explanations with comprehension and production. Include items where the grammatical cue matters and word-order guessing cannot reliably succeed. Preserve useful paradigms as references, then test use in new sentences [E9–E10].

### Literature, science fiction, mathematics, and philosophy from early on

Let passages Ember wants to read shape part of the curriculum. Preserve original text and source/edition metadata alongside audio, glosses, and optional generated simplifications. Use short scenes and conversations as recurring contexts for vocabulary, grammar, and speaking. Select material for interest as well as linguistic usefulness; favorite authors and works remain open until Ember chooses them.

An experimental curriculum planner can identify which words, senses, constructions, and sound patterns would unlock the next passage, then choose a compact set of lessons and return to the original. This is a proposed application of learning theory, not a proven fastest-reading method. Keep general conversational usefulness in the selection objective so specialization does not crowd out everyday language.

The implemented Reader gives connected passages their own workspace. It preserves exact originals, source/edition metadata and chunk boundaries, with separately attributed AI translations, contextual glosses, grammar notes, stress scaffolds and optional questions. Continuous text and a focused part are alternative views of the same source. Personal pasted text keeps its wording and line breaks; missing source information is not invented. The original remains visible when stress support is shown, and original-text audio is distinct from scaffold audio.

Reading place, open help and own-word notes persist locally. Failed-save guards retain visible edits and prevent guarded actions from silently discarding them; unsaved edits are not promised to survive process loss. Ember can read without a quiz, write an idea in English or Russian, or open a Teacher question anchored to the exact source part. Sending that question is a separate action. Using translation or discussing an idea can support understanding without demonstrating independent Russian reading.

Keeping a phrase is an explicit choice that retains its source span. Later edits create child revisions with navigable source ancestry; reading and extraction do not manufacture attempts or mastery. The Reader's immutable documents, links and sent discussion were introduced in event/sync version 7 and remain supported by current version 8, while its reading state stays local and physical SQLite remains schema 1. These are source contracts, not rollout or learning-quality claims; [ARCHITECTURE.md](ARCHITECTURE.md) details the boundaries.

The next reading strand should grow into definitions, proofs and arguments, including constructions such as “let,” “if … then,” “for every,” and “there exists.” General simplified adaptations remain a proposal, distinct from the current stress/translation support. Preserve their original sources and identify generated changes when implemented. Routine teaching has no per-item human approval requirement. [Reading Studio](research/READING_STUDIO.md) describes the evidence and future evaluation: supported access, understanding unfamiliar passages and later phrase use are different outcomes, and memorizing a translation does not establish reading competence.

Writing includes spelling, dictation, original sentences, and eventually explanations of ideas. Keyboard familiarity and optional iPad handwriting should be tracked separately from language errors. Reading remains available without requiring handwriting mastery.

## Learner model and scheduling

Start with a transparent event history. Each attempt records the content/version, target feature, input/output modality, context, cue level, response, any judgment and its source, time, audio reference and optional effort. Missing self-report is explicitly NotReported rather than fabricated recall success or failure. Corrections can supersede mistaken automated judgments without destroying history. Whole-response grades remain attempt-level evidence; they do not establish mastery of every tagged feature.

Record actual opportunities to see or hear material, rather than counting imported definitions or hidden/offscreen UI as exposure. Preserve instruction, task input, answers and optional references separately. A first probe input is not previous restudy. No prior recorded presentation means novelty only within the available app history; pre-log and outside-app familiarity remain uncertain. Presentation is not proof of attention or learning.

Learning-content identity is per phrase, with explicit ancestry across pack editions. Repackaging unchanged material must preserve prior attempts, due state and audio links; changed stress or meaning must retain separate explainable evidence. Keep the original immutable events while extending the curriculum.

Maintain separate estimates for sound discrimination, listening comprehension, spoken retrieval, pronunciation, reading, spelling, and composition. Evidence may eventually transfer between related skills, but the initial model should not silently assume transfer.

Implement a simple, inspectable spaced retrieval scheduler as a comparison baseline. Alongside it, explore a model-based teaching policy that can choose explanations, examples, practice, cues, and later revisits. Prioritize useful delayed competence, observed confusion, and manageable effort. Allow learner overrides and optional human suggestions, and record them. Do not prescribe a universal 70%, 80%, or 85% success target; a task-local challenge controller is a legitimate experiment. Do not maximize diagnostic information gain as a substitute for improving learning [E1–E2, A1].

Learner timing controls are available without interpreting preference as ability. MoreOften, Standard and MoreSpace scale the baseline interval by 0.5, 1 and 2 within ten minutes–120 days; these are convenience settings, not established optimal doses. New responses pin the chosen cadence at actual start. A later preference change leaves existing dates and legacy drafts alone. Exact tasks can be paused, resumed or given a one-shot availability date; completing a matching session consumes only the date it observed when starting. Pauses persist, and competing offline choices stay visible for reconciliation. These choices never manufacture a recall score or mastery claim.

Keep the issued timing reason and availability separate from later evidence qualification. An early start does not become delayed retrieval because the form stayed open. The current immutable start audit preserves timing inputs with a bounded evidence-ID sample and full-set digest; it does not implement the full future forecast/policy experiment. [Personal review policy](research/PERSONAL_REVIEW_POLICY.md) specifies clean pre-help boundaries, shadow-model comparisons and transfer evaluation still needed.

Ember welcomes theoretically motivated approaches before they have direct empirical support for this app. Develop them with explicit assumptions and testable predictions, then compare delayed performance and effort with the baseline. Begin with small, interpretable models of memory and competing grammar generalizations. With one learner, a highly parameterized cognitive model may be less identifiable than its interface makes it appear. See [efficient learning models](research/OPTIMAL_LEARNING.md) for the research and proposed experiments.

## Cross-platform implementation direction

Use Rust for the curriculum, event model, teaching policies, and audio analysis. Ember selected Dioxus for Apple, Android, Linux, Windows, and browser targets. The foundation uses its WebView/native and browser routes; the experimental WGPU renderer is a separate choice. The [architecture](ARCHITECTURE.md) records the implemented boundaries and UI flows. The [platform assessment](research/RUST_PLATFORM.md) retains the text/audio/device feasibility criteria; compilation does not establish device audio quality.

Keep capture/playback, permissions, interruptions, storage, and networking behind platform adapters. A shared Rust codebase still uses platform APIs and may need small bindings or WebView bridges. Apple's audio examples remain useful references for an Apple adapter [T1–T2], while the analysis core should consume platform-independent PCM data.

Persist lesson metadata and events locally. The current foundation has native SQLite, browser IndexedDB with a recovery journal, versioned notebook export/import, separate original-audio backup and explicitly enabled audio sync through an optional private relay. Immutable attempts and competing passage edits retain their history. Active/paused practice and passage-editor drafts are local metadata; deliberate cross-device draft handoff remains to be built. Generated references should join the same durable audio-asset and provenance workflow, with replay available offline after caching.

Make the AI teacher a primary application flow, integrated into lessons, response feedback and the practice/review loop. Use replaceable provider interfaces for language teaching/content preparation, speech recognition, speech generation and acoustic assessment. These operations have different evidence and privacy contracts; a successful text call does not establish a working speech teacher. Select speech providers by tests on Russian audio, including stress, numbers, symbols, and specialist vocabulary. No provider is selected merely because its chat model is strong.

Cloud processing should show what leaves the device. Keep secrets in appropriate local credential storage and out of builds/source control; decide per-provider whether a private gateway is necessary. Keep offline recording, replay, and existing lessons usable when an API is unavailable. Do not silently switch paid providers or enable auto-recharge.

## Evaluation for one person

Use short baseline and delayed probes across all four skills. Include unfamiliar utterances, new speakers, and unscripted exchanges; evaluate near-native pronunciation separately from fluency and intelligibility. When evaluating pronunciation, use shuffled recordings and independent proficient listeners when practical, without revealing whether a recording came before or after training. Familiarity with Ember’s speech can bias judgments. Arrange evaluation deliberately rather than assigning ongoing grading to a friend. Automated feedback can guide practice while its reliability is being tested; a model’s opinion is not proof of native-like speech.

For testing a feature, use matched item sets with randomized assignment or a staggered introduction. Account for cross-item learning, practice exposure, and time trends. Language learning is not readily reversible: an ordinary washout crossover is a poor default. Compare delayed outcomes and time/effort, not just immediate success. One person's results can guide this app without establishing a population-wide best method.

Do not turn every session into an experiment. Routine enjoyable learning is the product; lightweight evaluation helps it improve.
