# Rufren design

## Destination

Build the best Russian learning environment for one particular person. Ember begins with no Russian, wants to speak naturally, understand speech, read, and write, and loves science fiction and literature as well as mathematics and philosophy. The AI teacher carries everyday instruction: explanations, examples, corrections, pronunciation demonstrations, practice and review. Ember’s friend is a willing conversation partner at breakfast and dinner, not a formal coach or a source of assigned recording, grading or curriculum-review work. Success means capabilities that persist outside the app. Prefer a Rust implementation that can grow from Apple devices to Android, Linux, Windows, and web.

Working memory is sometimes challenging. Treat that as a variable demand to accommodate, without inventing a fixed cognitive profile. Conceptual sophistication and the ability to retain an unfamiliar sound sequence are separate things. Offer rich explanations alongside short practice steps.

The evidence IDs below refer to [the evidence review](research/EVIDENCE.md). Product choices are hypotheses to test with Ember, rather than consequences automatically proved by the cited research.

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

The teacher should use the learner history to choose a manageable next step, explain the choice when useful, and preserve drafts and assistance. Cached lessons, examples and audio support ordinary offline use; unavailable cloud services should leave a useful local path and a recoverable request rather than send Ember to find a human grader.

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

A useful next instrument is an editable two-region comparison for teacher references and learner-selected spans: duration, shared-scale spectra and original-time A/B playback. The [existing Silero pair audit](research/SILERO_STRESS_AUDIT.md) supplies exact crops and exploratory duration/spectral differences; it does not justify a universal stress threshold or confident LPC formants when poles are unstable. Existing plots/cropping are a base for this view, not a claim that dual-region comparison is implemented.

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

Create a parallel reading strand: tiny supported excerpts and constructions such as “let,” “if … then,” “for every,” and “there exists,” growing into definitions, proofs, and arguments. The teacher develops these materials with explicit provenance and ongoing quality checks for Russian wording, stress, terminology and audio. Use trustworthy linguistic sources and curated examples where appropriate, identify generated adaptations, and preserve uncertainty. Routine lessons do not wait for individual human approval of every phrase.

Support three distinct reading actions: hear/read the original, inspect a gloss or syntax explanation, and explain the argument in one's own words. Keep the original separate from simplified or generated paraphrases. Track understanding of unfamiliar passages; memorizing a translation does not establish reading competence.

Writing includes spelling, dictation, original sentences, and eventually explanations of ideas. Keyboard familiarity and optional iPad handwriting should be tracked separately from language errors. Reading remains available without requiring handwriting mastery.

## Learner model and scheduling

Start with a transparent event history. Each attempt records the content/version, target feature, input/output modality, context, cue level, response, any judgment and its source, time, audio reference and optional effort. Missing self-report is explicitly NotReported rather than fabricated recall success or failure. Corrections can supersede mistaken automated judgments without destroying history. Whole-response grades remain attempt-level evidence; they do not establish mastery of every tagged feature.

Record actual opportunities to see or hear material, rather than counting imported definitions or hidden/offscreen UI as exposure. Preserve instruction, task input, answers and optional references separately. A first probe input is not previous restudy. No prior recorded presentation means novelty only within the available app history; pre-log and outside-app familiarity remain uncertain. Presentation is not proof of attention or learning.

Learning-content identity is per phrase, with explicit ancestry across pack editions. Repackaging unchanged material must preserve prior attempts, due state and audio links; changed stress or meaning must retain separate explainable evidence. Keep the original immutable events while extending the curriculum.

Maintain separate estimates for sound discrimination, listening comprehension, spoken retrieval, pronunciation, reading, spelling, and composition. Evidence may eventually transfer between related skills, but the initial model should not silently assume transfer.

Implement a simple, inspectable spaced retrieval scheduler as a comparison baseline. Alongside it, explore a model-based teaching policy that can choose explanations, examples, practice, cues, and later revisits. Prioritize useful delayed competence, observed confusion, and manageable effort. Allow learner overrides and optional human suggestions, and record them. Do not prescribe a universal 70%, 80%, or 85% success target; a task-local challenge controller is a legitimate experiment. Do not maximize diagnostic information gain as a substitute for improving learning [E1–E2, A1].

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
