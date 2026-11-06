# Rufren design

## Destination

Build the best Russian learning environment for one particular person. Ember begins with no Russian, has access to a willing human coach, wants to speak naturally, understand speech, read, and write, and loves science fiction and literature as well as mathematics and philosophy. Success means capabilities that persist outside the app. Prefer a Rust implementation that can grow from Apple devices to Android, Linux, Windows, and web.

Working memory is sometimes challenging. Treat that as a variable demand to accommodate, without inventing a fixed cognitive profile. Conceptual sophistication and the ability to retain an unfamiliar sound sequence are separate things. Offer rich explanations alongside short practice steps.

The evidence IDs below refer to [the evidence review](research/EVIDENCE.md). Product choices are hypotheses to test with Ember, rather than consequences automatically proved by the cited research.

## The central learning loop

1. **Encounter:** learn a useful utterance from a coach, a small conversation, or a reading passage. Preserve its meaning, situation, audio, and text.
2. **Understand:** make its sound and meaning clear. Offer a brief explanation, replay, phrase segmentation, and a relevant contrast. Show what an ending or stress pattern changes.
3. **Retrieve:** hear and interpret it, then produce it from a meaning or situation. Reading aloud, spelling from dictation, and writing an original answer are distinct exercises.
4. **Refine:** target one useful correction, try again, and listen to the difference. Return to a short meaningful exchange so the target is used rather than merely imitated.
5. **Revisit:** bring it back after a delay, including a changed speaker or context and an opportunity for independent production.

Learning content should be reusable across these activities. The app records what each attempt actually demonstrated, not a single global mastery score [E1–E4, E7].

### An initial session with the coach

The coach enters a short phrase, records two natural renditions, confirms meaning and stress, and adds an optional note. Ember hears it, can replay a short segment, records an attempt, and compares the recordings. The coach can annotate a specific span and give one actionable correction.

The same material supplies a listening question, a meaning-to-speech prompt, Cyrillic reading, and a short writing task. Later, a new utterance tests the same underlying feature. A phrase is not marked learned in all four skills because one exercise succeeded.

Coach proficiency, preferred pronunciation variety, and suitable reference voices remain to be established together. Being a helpful coach does not by itself make every acoustic feature of their voice a universal target.

## Working-memory support

- Keep the current context, instruction, and progress visible; reopening an explanation preserves the attempt.
- Let Ember replay short natural segments, repeat a whole utterance, and choose a smaller step. A slowed version is optional; keep normal-speed audio available.
- Introduce a sound contrast with familiar task mechanics before combining it with new spelling, vocabulary, and grammar.
- Allow untimed practice and an easy switch to more support. Response latency includes device, reading, retrieval, and motor demands; it is not a pure ability measure.
- Record assistance: replay, revealed text, translation, partial cue, model answer, and coach help. Prompted success and independent success are different observations.
- Fade cues within a skill when useful. During listening-only assessment, text that reveals the answer is a hint, not invisible assistance.

Personalization starts with observable confusions, self-reported effort, and performance over time. We should not diagnose a memory problem or estimate a permanent capacity from a handful of errors. A configurable session length is a convenience, not a scientifically optimal dose.

## Speech laboratory

The aim includes native-like pronunciation, fluent connected speech, and being readily understood. Track these separately. A spectrogram is an instrument for examining sound; it is not an accent verdict [E5–E6].

### Useful first capabilities

- Clean reference and learner recording, instant A/B playback, repeatable cropping, and waveform/spectrogram views.
- A syllable timeline and coach-confirmed stress location. Later, reliable duration and relative pitch tracks can help compare stress and intonation.
- Examples contrasting hard and soft consonants, vowel quality, voicing, and connected-speech patterns, chosen with the coach from observed difficulties.
- Coach annotations tied to time spans, with a short practice cue and a follow-up recording.

Keep the detailed instrument view optional. The ordinary practice screen should make the next useful action obvious. Raw spectrogram similarity is a poor objective: voices, microphones, room acoustics, vowel contexts, and speaking rates differ.

### Analysis pipeline

Capture original PCM audio with device/sample-rate metadata. Check clipping, silence, background noise, and recording interruptions before attempting pronunciation analysis. Preserve the original; derived displays and playback transformations must be reproducible.

Use windowed spectral analysis for visualization. For scored features, add pitch, duration, formants, or consonant cues only with a specified linguistic question and an evaluation set. For example, adjacent vowel formant transitions may help examine particular palatalization contrasts; no single F2 threshold establishes correct palatalization across all contexts [E6]. Normalize comparisons appropriately for the feature and speaker rather than demanding that Ember reproduce someone else's vocal tract.

An aligner can suggest where sounds occurred. A recognizer can suggest what was said. Neither proves that every expected sound was pronounced correctly. A forced aligner may assign the expected phoneme sequence to a mispronunciation. Any alignment must support uncertainty and manual correction. Time alignment for visual comparison must not erase duration differences from the measurement being evaluated.

An LLM can explain a supported observation and propose a drill. Feedback must name the relevant audio span and evidence. A transcript alone cannot support claims about tongue position, vowel quality, stress, or intonation. Use uncertain results to invite replay or coach review rather than to produce an authoritative correction.

### How automated feedback earns trust

Collect multiple attempts, conditions, and reference speakers over time. Label individual contrasts and stress with the coach; supplement with other proficient listeners when possible. Keep development clips separate from held-out words, sessions, and voices. Check false corrections as well as missed errors, including errors introduced by microphones and alignment.

Compare ordinary A/B listening plus coaching with the same practice plus a specific acoustic display, holding practice time roughly equal. Check delayed production of new items. A better-looking plot without an audible or functional benefit is insufficient. The Russian ultrasound experiment had 18 learners and did not establish general superiority over audio-only familiarization [E6]. This supports targeted experimentation, not abandoning visual tools.

## Russian curriculum

### Sound, script, and everyday use together

Teach Cyrillic through actual sound–word connections, while retaining separate listening-only practice. Letter names, letter shapes, spelling, and speech sounds are related but distinct skills. Use stress annotations as a scaffold and preserve ordinary spelling for later reading. Written marks alone do not establish auditory mastery [E8].

Begin with personally useful interactions: greetings, requests, asking what something means, asking for repetition, describing the immediate environment, and short exchanges with the coach. Add varied speakers and contexts gradually; the best number of voices for Ember is not known [E3].

### Grammar as meaning

Teach cases, agreement, aspect, and motion through contrasts in what is happening, who is doing what, where something is, and where it is going. Pair compact explanations with comprehension and production. Include items where the grammatical cue matters and word-order guessing cannot reliably succeed. Preserve useful paradigms as references, then test use in new sentences [E9–E10].

### Literature, science fiction, mathematics, and philosophy from early on

Let passages Ember wants to read shape part of the curriculum. Preserve original text and source/edition metadata alongside audio, glosses, and optional generated simplifications. Use short scenes and conversations as recurring contexts for vocabulary, grammar, and speaking. Select material for interest as well as linguistic usefulness; favorite authors and works remain open until Ember chooses them.

An experimental curriculum planner can identify which words, senses, constructions, and sound patterns would unlock the next passage, then choose a compact set of lessons and return to the original. This is a proposed application of learning theory, not a proven fastest-reading method. Keep general conversational usefulness in the selection objective so specialization does not crowd out everyday language.

Create a parallel reading strand: tiny supported excerpts and constructions such as “let,” “if … then,” “for every,” and “there exists,” growing into definitions, proofs, and arguments. These are curriculum proposals, not yet approved teaching content. Verify Russian wording, stress, terminology, and audio before treating generated material as a reference.

Support three distinct reading actions: hear/read the original, inspect a gloss or syntax explanation, and explain the argument in one's own words. Keep the original separate from simplified or generated paraphrases. Track understanding of unfamiliar passages; memorizing a translation does not establish reading competence.

Writing includes spelling, dictation, original sentences, and eventually explanations of ideas. Keyboard familiarity and optional iPad handwriting should be tracked separately from language errors. Reading remains available without requiring handwriting mastery.

## Learner model and scheduling

Start with a transparent event history. Each attempt records the content/version, target feature, input/output modality, context, cue level, response, correctness judgment and its source, time, audio reference, and optional effort. Corrections can supersede mistaken automated judgments without destroying history.

Maintain separate estimates for sound discrimination, listening comprehension, spoken retrieval, pronunciation, reading, spelling, and composition. Evidence may eventually transfer between related skills, but the initial model should not silently assume transfer.

Implement a simple, inspectable spaced retrieval scheduler as a comparison baseline. Alongside it, explore a model-based teaching policy that can choose explanations, examples, practice, cues, and later revisits. Prioritize useful delayed competence, observed confusion, and manageable effort. Allow user/coach overrides and record them. Do not prescribe a universal 70%, 80%, or 85% success target; a task-local challenge controller is a legitimate experiment. Do not maximize diagnostic information gain as a substitute for improving learning [E1–E2, A1].

Ember welcomes theoretically motivated approaches before they have direct empirical support for this app. Develop them with explicit assumptions and testable predictions, then compare delayed performance and effort with the baseline. Begin with small, interpretable models of memory and competing grammar generalizations. With one learner, a highly parameterized cognitive model may be less identifiable than its interface makes it appear. See [efficient learning models](research/OPTIMAL_LEARNING.md) for the research and proposed experiments.

## Cross-platform implementation direction

Use Rust for the curriculum, event model, teaching policies, and audio analysis. Ember selected Dioxus for Apple, Android, Linux, Windows, and browser targets. The foundation uses its WebView/native and browser routes; the experimental WGPU renderer is a separate choice. The [architecture](ARCHITECTURE.md) records the implemented boundaries and UI flows. The [platform assessment](research/RUST_PLATFORM.md) retains the text/audio/device feasibility criteria; compilation does not establish device audio quality.

Keep capture/playback, permissions, interruptions, storage, and networking behind platform adapters. A shared Rust codebase still uses platform APIs and may need small bindings or WebView bridges. Apple's audio examples remain useful references for an Apple adapter [T1–T2], while the analysis core should consume platform-independent PCM data.

Persist lesson metadata and events locally, with audio eventually in an app-managed file store. The foundation has export/import and manual sync through an optional private relay, with immutable attempts and explicit passage conflict resolution. Native SQLite and browser text storage implement the same event protocol. Audio sync and cross-device draft handoff remain to be built.

Use one replaceable provider interface for language tutoring/content preparation and distinct interfaces for speech recognition, speech generation, and acoustic assessment. Select speech providers by tests on Russian audio, including stress, numbers, symbols, and specialist vocabulary. No provider is selected merely because its chat model is strong.

Cloud processing should show what leaves the device. Keep secrets in appropriate local credential storage and out of builds/source control; decide per-provider whether a private gateway is necessary. Keep offline recording, replay, and existing lessons usable when an API is unavailable. Do not silently switch paid providers or enable auto-recharge.

## Evaluation for one person

Use short baseline and delayed probes across all four skills. Include unfamiliar utterances, new speakers, and unscripted exchanges; evaluate near-native pronunciation separately from fluency and intelligibility. Have the coach listen to shuffled recordings without seeing whether they came before or after training when practical. Familiarity with Ember's speech can bias judgments, so additional listeners would improve later evaluation.

For testing a feature, use matched item sets with randomized assignment or a staggered introduction. Account for cross-item learning, practice exposure, and time trends. Language learning is not readily reversible: an ordinary washout crossover is a poor default. Compare delayed outcomes and time/effort, not just immediate success. One person's results can guide this app without establishing a population-wide best method.

Do not turn every session into an experiment. Routine enjoyable learning is the product; lightweight evaluation helps it improve.
