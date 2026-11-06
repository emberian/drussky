# Writing as language practice: product and platform comparison

2026-11-06. This comparison follows Ember's correction: the activity must connect hearing, handwriting and speaking across words and phrases. A two-letter drawing pad does not meet that requirement.

Four actual Kagi searches used `/Users/ember/swarm/research.py`; exact queries and results are under `.local/research-writing-2026/root/`. The sources below were opened and read. This is a comparison of documented capabilities, not a hands-on rating of the products or evidence of their educational effectiveness. Primary learning studies are compared separately in [Handwriting activities](HANDWRITING_ACTIVITIES.md); recognition and scheduling have separate reviews.

## What existing products establish

| Product and primary source | Documented pattern | Implication for Drussky |
| --- | --- | --- |
| [Write It! Russian](https://apps.apple.com/us/app/write-it-russian/id1400944429) | All 33 Cyrillic letters, stroke guidance, recognition, practice/test/review and offline use. The published description emphasizes alphabet practice and timed tests. | Letter formation is an existing baseline, not our destination. Don't substitute completing the alphabet or a speed score for meaningful language use. Its accuracy and learning claims are not independently established here. |
| [Skritter Learn/Test](https://docs.skritter.com/article/270-learn-mode-and-test-mode) and [review system](https://docs.skritter.com/article/250-spaced-repetition-system) | Guided character/vocabulary learning, interactive recall, and distinct extra practice versus scheduled reviews, for Chinese/Japanese. | Keep supported learning and later recall distinct, but let them feel like one coherent experience. Chinese character-specific stroke grading and scheduling heuristics are not automatically appropriate for Cyrillic. |
| [Writey](https://writey.app/) | Letters → connections → words → sentences; print and several cursive paths, zoom, guides and left-handed support. Its listed languages do not include Russian. | Whole words and comfortable writing size are ordinary product requirements. Cursive connections deserve their own models. Do not compress an entire sentence into a tiny fixed letter box. |
| [DictoEaso](https://apps.apple.com/us/app/dictoeaso-on-screen-dictation/id6760171418) | Audio words/phrases/sentences → handwriting → recognition/check → mistake practice; configurable timing and custom material; on-device recognition after model download. Russian is not in its listed practice languages. | The requested dictation loop is concrete and feasible. We still need Russian coverage, a beginner curriculum, useful uncertain-recognition handling and connected speech practice. Listing claims are not proof of handwriting accuracy or retention. |

No product above establishes the best combined Russian-learning policy. Their useful interactions inform our design; their marketing claims do not determine the learning model.

## Pencil interaction has a higher baseline than a canvas

Apple's [Pencil HIG](https://developer.apple.com/design/human-interface-guidelines/apple-pencil-and-scribble) calls for immediate marking, sufficient writing space, stable input while writing, usable controls with either hand, and expected Pencil interactions. It recommends PencilKit for native low-latency drawing and discusses undo/redo. A fetched Markdown copy is retained with the search records.

Those requirements change the implementation comparison:

- A custom WebView canvas is a useful portable adapter and test surface. Pen events alone do not establish low latency, palm handling, system gesture support, native erasing or comfortable zoom on the physical iPad.
- A native `PKCanvasView` is the leading iPad capture candidate. Rust can still own activities, persistence, cue policy and the curriculum. A small platform adapter is preferable to making cross-platform purity the learner's problem.
- Full-screen native writing is being assessed against a DOM-aligned native overlay. The latter adds two coordinate/scroll systems, clipping and hit-testing synchronization. The simpler boundary should win if it supports the complete exercise.
- Words and phrases need stable paper geometry, legible models and user-controlled size. The model and learner ink must share geometry during comparison. Resizing the display must not rewrite saved strokes or interrupt a word to make room for controls.

PencilKit capture and handwriting recognition are separate decisions. [Apple's WWDC26 session](https://developer.apple.com/videos/play/wwdc2026/203/) introduces public `PKStrokeRecognizer` on OS 27. Root ran a capability-only probe on this Mac (macOS 27.0.1): recognition version 9 advertises Russian `ru-Cyrl-RU`. This is runtime capability evidence, not an accuracy test. The first probe's direct `Locale.Language` equality check was misleading; the corrected language-code query is preserved as `pencilkit-capability-2.json` under the algorithms research directory. The last physical iPad receipt is OS 26.4.2, so the new recognizer cannot simply be assumed available there. No OS update has been performed.

The technical comparison therefore includes recognition on the existing private Mac, an earlier-iOS on-device recognizer, and newer native recognition when supported. No provider has yet won a Russian beginner-handwriting benchmark.

## What we will evaluate

The user-visible unit is a message, not a tool panel. A first useful example is **кот**, followed by **Там кот.** after the relevant letters and meaning have been taught. The same activity should support hearing → writing → comparison → saying, or seeing → copying → hearing → saying. Optional formation help belongs near the troublesome letter. A later return can remove answer cues; it must still say what response is wanted.

Evaluate three different contracts:

1. **Interaction:** can Ember write a whole phrase comfortably, undo, listen, speak, pause and resume without fighting the surface? Physical Pencil behavior needs physical testing; injected browser pen events cannot settle it.
2. **Recognition and feedback:** can a provider transcribe ordinary, mistaken and ambiguous Cyrillic ink without seeing the expected answer? Measure exact text/character errors and misleading acceptance/rejection. Preserve the original and separate transcription from spelling and formation advice. Do not use language-model correction toward the known answer as evidence that the learner wrote it correctly.
3. **Learning:** does later sound-to-writing, meaning-to-writing or meaning-to-speech improve for the time spent, and can the learner use a new combination? Immediate copied output, recognition accuracy and completing a screen are insufficient substitutes.

The first two contracts can be tested during implementation. The third requires real later practice. The app should support that comparison without turning the learner's experience into a laboratory dashboard.

## Current decision

Retain the durable-ink work and replace the A/М-only activity resolver with authored word-and-phrase activities. Do not release the narrow pad as the completed writing experience. Compare native capture and the available recognition paths before committing to a default. Use the explicit task/cue policy in the empirical review as the baseline, then test a more adaptive policy against it rather than calling a generic scheduler optimal.
