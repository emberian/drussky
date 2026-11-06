# Handwriting recognition and feedback: comparison for Drussky

Research date: 2026-11-06. Seven targeted Kagi searches used `~/swarm/research.py`; raw results and primary documentation snapshots are in `.local/research-writing-2026/algorithms/`. This comparison concerns **Russian words and phrases written with a pen**, not merely a letter-tracing widget. No commercial account, model weights, paid inference or app dependency was added.

The practical recommendation is to keep the ink format independent of the recognizer, compare **ML Kit Russian digital ink** and **Apple's newly public PencilKit recognizer**, and retain **MyScript iink** as the serious commercial challenger. A raster Russian TrOCR model is a useful independent comparator, not an obvious replacement for temporal ink. No public, directly comparable benchmark found here establishes a winner on adult beginner Russian written with Apple Pencil.

## The important new Apple option

WWDC26 introduced `PKStrokeRecognizer` for iOS/iPadOS/macOS 27. It consumes `PKDrawing` strokes, runs locally, and permits recognition of the whole drawing or selected stroke IDs. A custom canvas can supply strokes; using the recognizer does not require replacing the entire Rust/Dioxus UI. The Swift actor is present in the installed Xcode 27.1 SDK. The simulator's language limitation makes it unsuitable for verifying Russian recognition. [Apple's session](https://developer.apple.com/videos/play/wwdc2026/203/)

Apple distinguishes one best transcription, multiple interpretations for indexing, and query-directed search. Only the first is a suitable starting point for an independent transcription. `search(expectedAnswer)` is **not independent evidence that the learner wrote the answer**. Keep the recognition version with derived text; submit the actual current drawing, and normalize its size to ordinary paper coordinates. [Recognition guide](https://developer.apple.com/documentation/PencilKit/recognizing-handwriting-and-converting-to-text)

Root's corrected Mac capability query on macOS 27.0.1 reports Russian (`ru`, expanded `ru-Cyrl-RU`) among 33 language identifiers and recognition version 9. The first probe's direct `Locale.Language` equality test returned a false negative despite `ru` in the list; that historical receipt is retained, and `pencilkit-capability-2.json` uses language-code matching. This is a capability observation, not a handwriting accuracy result. Saved iPad receipts report iPadOS 26.4.2, so the new API cannot be assumed available there. No OS update was performed.

## Candidate comparison

| Candidate | What is established | Fit and consequential limits |
| --- | --- | --- |
| **Apple PencilKit 27** | On-device temporal-stroke recognition; OS-supplied model; actual Mac SDK/runtime exposes Russian. | Strong Mac/Apple candidate with a small Swift bridge. Requires OS 27. First output is a single hypothesis, not a calibrated quality score. Current iPad receipts are OS 26. |
| **Google ML Kit Digital Ink** | Explicit Russian `ru` model, iOS and Android, recognition offline after model download. Input is ordered strokes of x/y/time. | Best immediate mobile baseline to test on the existing iPad OS. Native ObjC/Swift and Android bindings, not a portable Rust/WASM model. Needs model availability handling. [Overview](https://developers.google.com/ml-kit/vision/digital-ink-recognition), [language list](https://developers.google.com/ml-kit/vision/digital-ink-recognition/base-models) |
| **MyScript iink 4.5** | Russian listed for interactive and non-interactive recognition. Native iOS 15+, C++ on macOS/Linux/Windows, Java Android; WebSocket/REST for web. | Most complete documented cross-platform commercial option. Native operation avoids transmitting ink; browser API is a different networked deployment. Platform table's maximum versions are last qualified versions, not hard future incompatibility limits. [Languages](https://developer.myscript.com/docs/sdk/4.5/recognition-capabilities/text-languages/), [platforms](https://developer.myscript.com/docs/sdk/4.5/integration/platforms/) |
| **Russian TrOCR raster model** | `kazars24/trocr-base-handwritten-ru`, about 0.3B parameters, reports CER 0.048542 on its own validation. | Independent image-to-text comparator on local server compute. The model card does not establish writer-held-out testing, beginner print accuracy or a directly comparable commercial benchmark. Its listed split/counts are not sufficient to reproduce that result; license is not declared in the retrieved card. [Author model card](https://huggingface.co/kazars24/trocr-base-handwritten-ru) |
| **Cyrillic TrOCR extension** | Russian/Ukrainian/Church Slavonic finetune; card declares MIT and validation CER 0.253278. | Different data and label noise, so this number cannot rank it against the previous model. Historical/mixed-script distribution may be less relevant. October 2026 decoding correction removes repeated-ngram suppression that wrongly deleted legitimate recurring Cyrillic sequences. [Author model card](https://huggingface.co/cyrillic-trocr/trocr-handwritten-cyrillic) |
| **Apple Vision raster OCR** | Existing image-text request can run on rendered ink; runtime language/revision query is required. | Useful raster baseline, but discards temporal data and is not a handwriting-formation evaluator. Keep language correction off for a diagnostic spelling comparison. [Request documentation](https://developer.apple.com/documentation/vision/recognizetextrequest) |
| **Scribble** | Apple lists Russian among supported Scribble languages. | Useful alternative input for ordinary text fields. It converts writing into text, whereas this exercise needs original strokes, comparison and replay. Do not confuse a working Scribble field with a handwriting teaching system. [Feature availability](https://www.apple.com/ios/feature-availability/#apple-pencil-scribble) |

MyScript currently offers 25 device licenses for development/testing, with production terms requiring a contract; cloud testing has a separate 2,000-request allowance. These are not unrestricted perpetual deployment licenses. No purchase is recommended before comparison on actual ink. [MyScript pricing](https://developer.myscript.com/pricing/)

ML Kit input and output stay on device, but the SDK may contact Google for model/compatibility updates and sends performance/utilization metrics. Thus “offline recognizer” does not mean “SDK never networks.” Its models are proprietary software under Google's API terms; it is not an open model to extract and embed in WASM. [Terms and privacy](https://developers.google.com/ml-kit/terms)

## Recognition is one subsystem; feedback is another

For ML Kit, use actual stroke order and the correct writing-area scale. Its guidance assumes a single line for the writing-area height; edited words may need separate recognition rather than appending late correction strokes to a whole sentence. Supply only genuinely preceding learner text as pre-context. Supplying the target answer, or silently constraining the vocabulary to the exercise answer, would contaminate the evidence. The iOS guide says numeric scores are provided only by shape classifiers; text alternatives are ordered but should not become an invented confidence percentage. The candidate API's score is nullable. [iOS guidance](https://developers.google.com/ml-kit/vision/digital-ink-recognition/ios), [candidate API](https://developers.google.com/ml-kit/reference/ios/mlkitdigitalinkrecognition/api/reference/Classes/MLKDigitalInkRecognitionCandidate)

The app should separate four questions:

1. **What marks were made?** Immutable strokes, dimensions, timing, edits, and a rendered view.
2. **What text might they express?** Blind transcription, alternatives where available, engine/version, and any context used.
3. **Does that text answer this exercise?** Exact authored spelling/meaning criteria, after uncertainty is resolved.
4. **How can the shape or movement improve?** A specific observation about a taught form, not an arbitrary global handwriting score.

Language-model correction can conceal the very spelling error being taught. A Cyrillic post-OCR study reports word accuracy improving while character accuracy worsens on school-notebook images (raw student-text CAR 67.114% to 62.879%); its stronger in-distribution result does not erase that failure. This supports keeping original recognition and any corrected proposal separate. It does not establish that any particular current recognizer has the same error rate. [Davydkin et al., 2023](https://arxiv.org/abs/2311.15896)

## Which algorithms are worth trying?

**Whole-word/phrase transcription:** use a trained digital-ink recognizer first. Compare one raster model on the same rendered strokes to identify failure modes, rather than building a Cyrillic recognizer from scratch. A text recognizer may recognize untidy but readable writing; that is helpful for meaning and spelling, not proof of correct motor form.

**Letter formation support:** resampled trajectory alignment, dynamic time warping and geometric template distances can expose a local deviation. But one authored path is not the set of all valid Russian forms. Multi-stroke letters, alternative order, proportion, baseline and joins need different tolerances. Do not normalize away reflection, letter orientation or a meaningful proportion difference merely to improve a matching score. For our implementation, these should initially provide an optional visual overlay or a narrowly phrased hint, with no pass/fail gate.

The $P family is a useful transparent comparator for isolated forms: its point-cloud representation deliberately ignores stroke count, order and direction, and the reference implementation is BSD licensed. That makes it tolerant of some valid variations, but incapable of proving correct stroke order. It is a gesture recognizer, not a validated Russian word recognizer or educational scoring system. [Authors' $P project](https://depts.washington.edu/acelab/proj/dollar/pdollar.html)

**Keep temporal data instead of reconstructing it later.** A September 2026 trajectory-recovery paper explicitly treats recovery from an image as ambiguous and finds that sampling density changes measured performance. Its experiments concern Chinese, with English/Tamil extensions, not Russian learning. It reinforces the design choice to preserve original time-ordered ink; its impressive recovery results do not justify discarding our already available strokes. [Wang et al., 2026](https://arxiv.org/html/2609.02251v1)

**Do not call the app's method SOTA from these sources.** Newer architectures and commercial “best” claims lack a common, relevant test. The useful decision is which candidates to compare, what kinds of mistakes each makes, and whether feedback helps Ember read, write and say something later.

## Concrete comparison protocol

This is a proposed engineering evaluation, not yet run and not a validated psychometric test.

- Freeze a small development set of real exported ink with an independent transcription of the actual marks. Start with the taught sequence: letters, ма/ам, кот/там/да, Там кот., then он/она/оно/они/окно after their forms are introduced. Keep prompted target separate from transcribed response. Include deliberate wrong spellings, incomplete words, crossed-out attempts, spacing, uppercase/lowercase, and readable alternative forms.
- Add blank marks, non-letter doodles and visually confusable Cyrillic/Latin shapes. These expose an answer-conditioned recognizer that always returns the expected word. Synthetic strokes can validate integration, but must be reported separately from handwriting.
- Give every recognizer the same original, Russian language and neutral layout information. No expected answer, target vocabulary, teacher explanation, hidden target filename or prior corrected output. Save top transcription and returned alternatives before comparing with exercise criteria.
- Report character error rate, exact-word/phrase accuracy, false acceptance of a wrong answer, false rejection of a valid answer, no-result rate, latency and memory. Treat abstention and “please confirm what you wrote” as valid outcomes. Rank robustness to actual learner mistakes above a tiny average-CER difference.
- Hold out entire sessions, words and preferably writers from tuning; never choose thresholds on the same samples used to advertise performance. With one learner, later-session holdout is feasible but claims remain personal. Public scanned datasets help test raster recognition, not Pencil motor feedback.
- Collect later learning outcomes separately: writing from sound, saying from script, reading meaning, and novel combinations without a model. Improved recognition latency is not proof of improved Russian learning.

HKR is a useful public research lead: mostly Russian plus Kazakh, about 200 writers, with scanned forms. The Cyrillic Handwriting Dataset project describes image crops rather than temporal strokes, and its older project page reports 33,122 crops while a newer model card references 73,830. Pin the exact version and license before a benchmark; do not merge those numbers into one imaginary dataset. Neither collection supplies our adult-beginner Pencil learning outcomes. [HKR paper](https://arxiv.org/abs/2007.03579), [Cyrillic Handwriting Dataset project](https://zackpashkin.github.io/Cyrillic-Handwriting-Dataset/)

## Implementation decision this research supports

Build a provider-independent `InkRecognition` boundary with original hash, locale, engine/version, scale transform, output candidates and failure state. Retain both native PencilKit data, if captured, and portable strokes. Keep image rendering as a derived artifact. Native PencilKit is worth a real latency/palm-rejection comparison against the current web canvas; neither capture choice should dictate the exercise progression.

For the current iPad OS, evaluate ML Kit Russian as the first on-device baseline. For the current Mac, the new PencilKit recognizer is an immediately testable local comparator and could serve the existing private relay without an iPad OS change. MyScript is the cross-platform challenger if it materially improves actual Russian ink. A failed recognizer must leave the drawing, audio and lesson usable: show the original beside the model, allow transcription correction, and proceed to speaking without fabricating a grade.

The visible exercise should remain one flow: hear or see the word/message, write it, compare, say it, then return later under a different cue. Recognition should make that flow more responsive and informative; it should not turn the app into an OCR settings panel.

## Bounded native feasibility probe

After the capability query, root ran exactly three target-blind `recognizedText` calls on programmatically constructed uppercase block-print drawings, with expected strings held in a separate labels file. All three returned `nil`. Inspection then found that all three rendered PNGs were completely transparent. These are **invalid accuracy samples**: drawing construction/rendering must work before a recognition result can be interpreted. The original input, archives, images and null results remain in `.local/research-writing-2026/algorithms/pencilkit-synthetic-v1/`.

A zero-recognition geometry diagnostic reached an AppKit/PencilKit replica-manager crash while constructing a drawing after `NSApplication` initialization. The stack went through `CFEqual(NULL)` and bundle-ID preferences. This suggests that a bare Swift command-line process lacks application context needed by this API path; it is a hypothesis to test, not an established recognizer limitation. The next diagnostic used an isolated `.app` bundle with its own identifier and incremental geometry reports; it completed without that crash. The original archive had nonempty bounds, valid points and full visible path ranges, yet its thin synthetic glyphs remained transparent. A separate thick two-point line rendered correctly. A zero-recognition width/force comparison is therefore testing the synthetic construction parameters before any further recognition. Do not report the nulls as “Russian recognition failed,” or recommend a production relay solely from the capability list. A relay helper needs its application-context assumptions tested, rather than assuming a bare command-line process behaves like a native app.

## Acquisitions and remaining uncertainty

Two open papers are stored with source URLs and SHA-256 hashes in `~/paperbin/rufren/handwriting-algorithms/`: HKR (8,582,606 bytes) and Cyrillic post-OCR correction (4,417,473 bytes). Search results, fetched Apple Markdown/API documentation, SDK observations and the capability probe are retained under the ignored research directory. The initial DTW publisher page returned 403 and PubMed's automated view did not expose its abstract; no empirical DTW result is claimed from that inaccessible paper.

Still unmeasured: recognition of Ember's real writing; current iPad's exact live OS/capture performance; word-level feedback false positives; comparative latency; public dataset license/version resolution; and whether any particular shape feedback improves later independent writing. These are specific next comparisons, not grounds to ship another two-letter-only exercise.
