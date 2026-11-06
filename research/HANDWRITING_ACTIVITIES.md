# Writing words and messages: activity contract and comparison review

2026-11-06. Proposed integration, based on the current foundation pack and Pencil implementation. This note responds to Ember's request for **hearing something, writing it, then saying it**, and **seeing a word, copying it, then learning to say it**. It supplements [the script-learning evidence](HANDWRITING_FOUNDATIONS.md); it is not a claim that these activities are already implemented.

## The change that matters

The inspected `HandwritingActivity` permits only А and М at `f1-a` / `f1-m`. Its Copy → Sound flow finishes with saved ink; speaking is incidental instruction, not an explicit teaching stage. That is a letter pad, not yet the requested writing-and-language lesson.

Ship a complete word-and-message route using existing audio through **Там кот.**, with no additional TTS. Keep the motor model and durable ink, but make the target a meaningful utterance when the course reaches words. A learner should be able to finish an activity by having heard, written, compared and said the same message. The final speaking invitation is visible and explicit, with **I said it**, **Replay**, and **Skip speaking**; it does not require recording, certify pronunciation, or turn silence into success.

Do not require repetitive copying at every return. Teach the mapping first; subsequently let the learner choose “Hear and write” directly. Fresh retrieval starts on blank ink, while prior copying remains in the history. Immediate hidden-model performance after copying is practice with recent exposure, not delayed independent mastery.

## Exact authored registry proposal

Every row needs an immutable activity ID/revision, an exact teaching-source step/revision, target spelling, existing audio ID, input focus, meaning and supported sequence(s). Entry follows actual presentation of the source and its prerequisites. Being later in a JSON file is not evidence that the learner saw it. All listed source revisions are currently `1`.

| Target | Exact source step | Audio ID | Meaning / important constraint |
| --- | --- | --- | --- |
| А / а | `f1-a` | `foundation-a` | Vowel model. Specify the requested case visually/in English; audio does not identify capitalization. |
| М / м | `f1-m` | `foundation-ma` | Write the letter for the beginning of **ма**; Say uses the whole syllable. Audio is not isolated /m/ or its letter name. |
| ма | `f1-model-ma` | `foundation-ma` | Practice syllable, no invented word meaning. |
| ам | `f1-model-am` | `foundation-am` | Practice syllable; compare order with ма only after both are introduced. |
| О / о | `f2-o` | `foundation-o` | This stressed vowel example; not a rule for every written О. |
| К / к | `f2-k` | `foundation-ka` | Beginning of the whole syllable; Say the whole ка. |
| Т / т | `f2-t` | `foundation-ta` | Beginning of the whole syllable; Say the whole та. |
| кот | `f2-cat` | `foundation-kot` | Cat; use taught К/О/Т and word meaning. |
| Д / д | `f3-d` | `foundation-da` | Beginning of да; Say the whole word. |
| да | `f3-written-yes` | `foundation-da` | Yes; meaning previously introduced at `f1-hear-yes`. |
| там | `f3-there` | `foundation-tam` | There: the distant spot in the authored doorway scene. |
| Там кот. | `f3-message` | `foundation-tam-kot` | “There is a cat there.” Whole phrase audio exists. Space, capital and period are taught by this step. |
| он | `c4-he-script` | `foundation-on` | He, for the taught male-person referent. Requires Н model. |
| она | `c4-stress` | `foundation-ona` | She. Stress annotation **она́** is teaching support, ordinary ink target **она**. |
| окно | `c5-window-read` | `foundation-okno` | Window. Whole-word model; stressed final vowel, no invented vowel inside кн. |
| оно | `c5-neuter-script` | `foundation-ono` | It, referring to the previously introduced neuter noun окно; not generic English “it” substitution. |
| они | `c6-ni-context` | `foundation-oni` | They, for the taught people; requires И and Н models. |

Н/И currently lack stroke models in the inspected pad, so the continuation rows require authored models before activation. Exclude нет, я, ты from writing until their script is taught; their earlier audio exposure alone is insufficient. No synthetic word-by-word concatenation is needed for the first phrase. Later **Он там / Она там / Они там** is a useful construction proposal, but its phrase audio is not in this pack; don't label separate word clips as a whole-phrase model.

## Two complete sequences

**Hear → Write → Compare → Say.** Present the known meaning/scene if applicable, plus “Listen, then write what you hear.” Hide the target spelling in headings, instruction, formation hints and accessible names as well as the model panel. The first whole recording is task input; replay remains available. Commit and save the original ink before revealing the answer. Compare side by side, highlighting one useful thing to inspect; then invite saying the whole target, with reference replay available. A correction is a new linked take. For a single consonant, explicitly name the extraction task (beginning of the syllable); do not imply the recording itself says the letter.

**See → Copy → Hear → Say.** Show ordinary target spelling and meaning together. Copy at the learner's pace, save/compare, play the whole model, then invite saying it. Listening earlier and saying while writing are available, not mandatory concurrent demands. On another attempt, offer Hear-and-write; later offer meaning-to-writing for known words. These are distinct routes, not a single difficulty score.

The writing space must actually fit the message. Current `model()` distributes a fixed 300-unit horizontal allowance equally across characters: an eight-character phrase shrinks each glyph substantially. Do not merely whitelist phrases while retaining a tiny letter model and cramped writing area. Give the phrase a readable model and sufficient line length/height; retain a stable coordinate system or version its change. Keep meaningful spacing rather than spacing every character like independent tiles.

For **Там кот.**, the optional compact grammar explanation is: “Там says where; кот names what is there. In this present-time message Russian needs no separate word for ‘is’.” The lesson can ask the learner to choose the distant cat scene before producing the message. This checks the taught meaning; it is not evidence of a general zero-copula rule. Later explicit pronoun + там construction should include a changed referent and a fresh response.

## Minimal persisted contract

Separate an immutable authored activity from a learner run:

- **Activity:** ID/revision; course source step/revision; ordinary target; optional stress display; required graphemes, meanings, samples and conventions; whole audio ID; `whole_target` versus `initial_segment_of_syllable`; sequence; concise meaning, comparison focus and optional grammar.
- **Run:** activity identity and source presentation link; chosen sequence; current stage; exact original ink IDs and linked repairs; actual audio completions/model reveals; requested cue availability and use; final speaking disposition (`self_reported_said`, `skipped`, `not_reported`). Optional recorded speech is a separate artifact, not implied by a button.
- **Task roles:** sound is required input for dictation; spelling is required input for copying/reading; sound and spelling become answer-bearing help during meaning-to-form retrieval. Case instruction identifies the expected form rather than measuring a distinction that cannot be heard. Meaning remains available unless a deliberately different task is chosen.

Resume the actual stage after interruption. Failed ink persistence prevents answer reveal/advance; failed audio never counts as heard. The original ink, help history and repair remain inspectable. A completed sequence records practice, not correctness. These facts can later inform scheduling without inventing a handwriting score.

## What the requested Kagi comparison changed

Five actual `~/swarm/research.py kagi ... -n 6 --json` queries and raw responses are preserved in `.local/research-writing-2026/pedagogy/`. They cover adult L2 word writing, dictation/read-aloud, adaptive retrieval cues, interleaving/feedback, and direct writing-versus-retrieval comparison. Relevant results were followed to primary full texts. This is a focused comparison, not an exhaustive “state of the art” ranking.

| Primary comparison, inspected scope | Result and limit | Decision |
| --- | --- | --- |
| Candry dissertation (2019), chapter 4, pp. 91–111; counterpart [Candry, Decloedt & Eyckmans 2020](https://doi.org/10.1016/j.system.2020.102206). Full prepublication chapter read; published article itself restricted. | 179 Dutch-speaking B1 EFL students aged 15–18 learned 15 English words. Meaning-cued handwritten retrieval beat repeated copying on immediate and one-week form/meaning recall; delayed strict form recall 42.8% versus 27.1%, reported d=.60. Copying omitted the translation during practice, and the control always came first. This compares learning tasks, not pen versus keyboard or adults learning Cyrillic. | Use meaningful retrieval after initial teaching; copying alone is insufficient. Do not remove meaning from all copying activities merely to imitate the experimental condition. |
| [Fiechter & Benjamin 2019](https://doi.org/10.3758/s13423-019-01617-6), full article | Six experiments, 57–69 returning adults each; 12 five-letter English–Iñupiaq pairs, six practice rounds, final test 12–36 hours later. Adaptive requested/revealed letters helped difficult retrieval without feedback. With corrective feedback, retention was broadly comparable across adaptive, diminishing and ordinary retrieval; adaptive practice took longer. In experiment 3b, mean total practice time/item was about 96s adaptive versus 39s diminishing and 44s ordinary. | Offer help on demand, but don't require a complex adaptive algorithm or prolonged guessing. Compare retention per time, including help/repair. This tests letter cues on typed pairs, not optimal fading of sound, meaning and handwriting models. |
| [Yu, Boers & Tremblay, online 2022 / issue 2025](https://doi.org/10.1177/13621688221117242), full publisher text read | 142 Chinese university EFL learners with about ten years' English, assigned by class to dictation, dictogloss or comprehension questions. Immediate multiword-item benefits weakened at two weeks. Dictogloss versus dictation was not significantly different; dictation versus baseline no longer significant in the reported adjusted delayed pairwise comparison. Correct-before-transcript items predicted later recall better than repairs in some analyses, not a randomized test of correction's causal effect. | Use short comprehensible utterances and later returns; Compare alone is not proof of lasting learning. Do not impose long dictogloss or partner work on a zero beginner. |
| [Krishnan, Watkins & Bishop 2017](https://doi.org/10.1186/s40359-017-0198-8), full preregistered study | 36 English-speaking adults learned 27 picture-linked pseudowords through spoken recall, repetition or restudy. The predicted condition advantages in oral production were not supported at one week; recognition was near ceiling. Novel English-like phonology, no corrective feedback and immediate tests across conditions limit generalization. | Include saying because speech is a desired skill, but don't claim adding an imitation step guarantees retention or that writing transfers automatically to speaking. |
| [Nakata & Suzuki 2019](https://doi.org/10.1111/modl.12581), existing full text reread | 115 Japanese learners, five English grammar structures. Interleaving produced more training errors but better one-week grammaticality judgments than blocking. | Mix already introduced distinctions while keeping mechanics stable. This does not validate random mode changes during every stroke, a particular word order, or conversational transfer. |

The direct oral-retrieval comparison [Kang, Gollan & Pashler 2013](https://doi.org/10.3758/s13423-013-0450-z) also surfaced; its primary indexed abstract reports benefits over imitation at a two-day test. Its author PDF returned 404 during acquisition, so this note does not import uninspected sample details or use it to erase the later null result. A 2025 grammar-interleaving paper surfaced, but its institutional repository had no full text; no new numerical claim is based on that listing.

## Selection algorithm worth shipping, and the next experiment

Start with an understandable authored policy: **teach unknown mappings → permit model/copy → offer a fresh sound-cued attempt → Compare/repair → explicit Say → later return with fewer answer cues**. If the learner asks for help, supply the requested cue immediately and preserve that fact. If a comparison indicates uncertainty, offer one specific repair and another target before returning. Don't infer ink correctness from a completed drawing or a recall probability from latency; pen movement, access needs and interface use contaminate time.

For already taught material, alternate a small number of targets and routes, rather than force every mode on every word. Keep the route visible (“write what you hear” versus “write the word for this meaning”). A learner-controlled “more help / try without the model” policy is an honest first adaptive system. The model-based next step would predict success **for a specified task and cue configuration**, value improvement per minute, and test that policy prospectively against this baseline. These papers identify neither Ember's parameters nor an optimal cross-modal controller.

A lean comparison can assign comparable newly taught words to repeated-copy-first versus brief-copy-then-retrieval, equalize total practice time and reference exposure, and revisit before models on another day. Measure meaning→writing, sound→writing and meaning→speech separately; add an unpractised combination to test composition. Save pre-help attempts. Include correction time and allow stopping. A handful of personal observations can guide the interface; it cannot establish a general population effect. No friend is required to label anything.

## Acquisitions and limits

New files are in `/Users/ember/paperbin/rufren/writing-crossmodal/`:

- `candry2019-dissertation.pdf` / `.txt`: [Ghent author dissertation](https://backoffice.biblio.ugent.be/download/8621387/8621388), specifically chapter 4 for the comparison above.
- `fiechter-benjamin2019.pdf` / `.txt`: [author laboratory PDF](https://labs.psychology.illinois.edu/~asbenjam/pubs/FiechterBenjamin2019.pdf).
- `krishnan2017.html` / `.txt`: [full PMC article](https://pmc.ncbi.nlm.nih.gov/articles/PMC5545031/).
- `acquisition*.json`: exact source URLs, bytes, SHA256 hashes and failed requests. Publisher Yu HTML/PDF downloads returned 403; its full text was read through the browser tool, not falsely listed as a locally acquired paper. The Webb/Piasecki repository returned 403; no result is inferred from that download.

Existing `/Users/ember/paperbin/rufren/nakata-suzuki2019-interleaving.{pdf,txt}` supplies the interleaving comparison. No new audio/model inference, app edits or builds were performed for this review.
