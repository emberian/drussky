# Rufren curriculum: conversation that grows into reading and ideas

Editorial audit and implemented extension, 2026-11-05. Authored and reviewed by AI contributors. This is a concrete curriculum proposal for Ember, not a validated fastest-learning sequence or a proficiency certification. [DESIGN.md](../DESIGN.md) supplies the product goal; [EVIDENCE.md](EVIDENCE.md), [OPTIMAL_LEARNING.md](OPTIMAL_LEARNING.md), and [AI_TEACHER_PEDAGOGY.md](AI_TEACHER_PEDAGOGY.md) distinguish empirical findings from our design hypotheses.

## The teaching destination

Ember begins with zero Russian and wants listening, speaking, reading and writing. Intellectual sophistication does not imply that an unfamiliar sound sequence is easy to retain. The course should offer interesting ideas immediately while making the *linguistic* step small enough to work with. Rich explanations and short production are compatible.

The AI teacher provides instruction, examples, synthetic reference speech, correction, practice and assessment. A friend is an optional breakfast or dinner conversation partner. No lesson depends on that friend preparing topics, recording examples, grading answers, approving content, or becoming a tutor. The optional conversation invitations describe opportunities for ordinary conversation; the complete practice path remains available with the AI.

The destination is a repertoire: understand a new spoken message, express an intended meaning, read an unfamiliar passage, and compose something of one's own. A lesson opened, a translation remembered, a chart understood, and a correct acoustic realization are different accomplishments.

## What the existing 16 lessons do well

The base pack starts with social contact and conversational repair. It ties Cyrillic to usable words, separates supported and ordinary spelling, keeps ё, introduces endings through contrasting meanings, and permits mathematically or philosophically interesting material before every inflection is productive. The original station scene provides a recurring world without misrepresenting generated prose as a quotation from an author. Listening inputs and hidden production references are separate in the content model.

The pack is a useful **first encounter path**, not a complete beginner course. Its 103 phrases and selected constructions cannot establish broad A1 competence, much less spontaneous reading of literature. Neither the count of covered cases nor completion of the quantifier lesson means that the learner controls all their uses. Preserve the supported bridges; build sufficient everyday language and varied practice around them.

## The extension implemented here

`crates/rufren-content/content/extensions/conversation-to-ideas-v1.json` contributes 8 lessons, 66 phrases, 56 exercises, 16 targets and 4 source records in two units. Merged with the audited base revision `2026-11-05.3`, the result contains 24 lessons, 169 phrases, 172 exercises and 50 targets. Stable IDs preserve distinct editorial identities. All new Russian material is `generated_draft`; every audio reference is `pending_generated_speech`.

| Lesson ID suffix | Everyday job | Productive contrast | Reading payoff |
| --- | --- | --- | --- |
| `preferences` | Say what appeals to you or what you need | мне нравится / мне нужна; нравится / нравятся | Distinguish experiencer from grammatical subject |
| `quantities` | Request cups or a little water | одна чашка / две чашки / пять чашек; немного воды | Interpret small quantities without treating every noun form as a plural |
| `together` | Name company, an instrument, an accompaniment | пишу с Аней / пишу ручкой; с молоком / без молока | Select construction-specific case, rather than translating English prepositions word for word |
| `topics` | Discuss what a book or thought is about | думаю о роботе / думаю, что робот здесь | Separate topic, proposition and the speaker's commitment |
| `relative` | Identify which person or book | которая видит Аню / которую видит Аня | Track local roles in a clause rather than copying the antecedent's case |
| `journeys` | Describe a trip or usual route | иду / еду; хожу / езжу; по комнате / в комнату | Track path and viewpoint while resisting a false once-versus-always rule |
| `return_signal` | Retell two events in an original scene | писала / написала; услышала as coming to hear | Read activity, bounded result, event order and future result |
| `scope` | Compare claims in a small fictional world | each door has a fitting key / one shared key; not every / none | Follow quantifier dependency and negation without guessing from mathematical facts |

Each lesson contains explicit explanations, an exchange or miniature scene, a reading question, a form–meaning contrast, written retrieval, spoken retrieval, composition, listening comprehension and dictation. This is a reusable content inventory, **not a requirement to do seven tasks in one sitting**. The AI chooses an appropriate next action and can split a lesson across sessions.

The extension is an editorial fragment, not an independently loadable `ContentPack`. The integration contract and read-only validator are in [the extension README](../crates/rufren-content/content/extensions/README.md). Runtime integration and the canonical Rust validator remain the content owner's responsibility. No Cargo or Dioxus build was run in this lane, to respect the shared build owner.

## Progression should branch, then reconverge

A single hard linear chain confuses encounter order with mastery. The following are suggested encounter points, not locks:

1. Start with greeting, repair, naming/decoding and a short self-introduction. Reuse those turns in every later lesson.
2. After location, requesting, possession and describing, offer the everyday extensions. Preferences and quantities need not wait until mathematical quantifiers have been encountered.
3. Continue actor/patient roles, time, destinations and plans while revisiting everyday frames with new words. Introduce topics once a small clause is comfortable; the topic-versus-proposition contrast is self-contained.
4. Let the first station story, reasons, mathematical statements and narrator/character lesson remain optional supported previews. Interest is a reason to visit, not a demand to produce every structure immediately.
5. Return to relative clauses, journeys and the second station scene after their named encounter prerequisites. Revisit the earlier previews with more of their language now available.
6. Treat quantifier dependency as a small supported interpretation problem, then expand toward real definitions and arguments. Do not allow English logic knowledge alone to answer a Russian comprehension test.

The suggested prerequisite graph is deliberately modest. It says which encounters may help; it does not infer that Ember has acquired a feature because an earlier lesson was opened. A host test that insists there is exactly one available lesson at every point must be replaced by a reachability/order invariant when branching content is integrated.

## The instructional unit is a meaning distinction

For a target such as relative-clause role, use this loop:

- Establish a situation with two visible participants. Explain the meaning of one complete short message and play its model audio.
- Contrast two messages while keeping irrelevant cues constant: девушка, которая видит Аню versus девушка, которую видит Аня. Ask who sees whom. Do not make common sense or the first-noun heuristic always select the right answer.
- Offer the compact rule and, if wanted, a two-clause derivation. Understanding the explanation is useful but is not a substitute for interpreting the next message.
- Elicit a short intended message. Keep the model hidden until support is requested; preserve the response before correction.
- Give one consequential correction, invite a retry, and use the revised message in a meaningful follow-up.
- Return later, with a different participant or context, and with a separately recorded cue condition and modality.

This explicit explanation plus comprehension and production design follows the evidence summarized in the repository reviews. The cited studies do not establish that this exact sequence is optimal for Russian or for Ember. Interleave confusable constructions once the task is understandable, while keeping instructions and supports stable. Do not use working-memory variability as a reason to eliminate meaning contrasts or presume a fixed capacity.

For a low-energy session, keep the context visible, use one clause, replay a short natural span, offer a partial cue and permit an untimed response. For a more expansive session, retain the same core task but add a richer explanation, a story continuation or a short argument. These are adjustable supports, not a prescribed therapeutic protocol or a scientifically optimal dose.

## Avoid the tidy rules that teach the wrong language

The extension explicitly prevents several tempting errors:

- **“The first noun does the action.”** Case and clause structure matter; information structure also motivates different orders.
- **“An object always gets -у.”** That describes selected singular -а nouns, not all objects.
- **“A motion verb forces accusative.”** A place may be the area of movement; по комнате and в комнату express different relations.
- **“Идти is once, ходить is always.”** The motion distinction is richer: walking around can happen now, and directed verbs can occur in repeated scenarios. The SPbU overview explicitly gives repeated directed uses. [Institutional grammar discussion](https://iphil.ru/glagol-narechie-bezlichno-predikativnye-slova-9/).
- **“Imperfective means definitely unfinished; perfective means finish everything.”** Писала does not settle completion, and услышала presents a perceptual event. The same overview discusses perfective boundaries other than completion. [Aspect discussion](https://iphil.ru/glagol-narechie-bezlichno-predikativnye-slova-9/).
- **“Который copies the antecedent's case.”** Gender and number connect to the antecedent; the role within the relative clause determines the case. Syncretism means some role contrasts do not have distinct visible endings. [Lyutikova's author-hosted study](https://www.philol.msu.ru/~otipl/new/main/courses/syntax/Zagadki_Lyutikova_2008.pdf).
- **“Not every means some do.”** In the explicitly nonempty three-robot world, not every allows none as well as a proper subset; no robot is the stronger claim. Existence never silently means uniqueness. These are authored logical distinctions, not empirical learning claims.

Primary grammar checks support bounded claims, not certification of every original sentence. The extension source records retain access limitations. The direct Gramota dictionary request returned 403; it is not represented as a successful full-entry inspection. The institutional number-pattern source was available as an indexed passage only. All generated wording, stress and reference audio remain open to correction.

## Keep two reading paths alive

**An accessible recurring story path** supplies enough familiar language to reread, listen, retell, and eventually continue a scene. Recycle people, locations and ordinary goals; vary the message rather than merely permuting identical flashcards. Original speculative scenes can be interesting without using a dense specialist vocabulary at every turn.

**A supported desired-text path** lets Ember inspect an actual chosen literary, mathematical or philosophical passage. Preserve author, work, edition, rights, original text and boundaries between quotation, gloss, translation and generated adaptation. Identify the few lexical senses and constructions that obstruct this particular passage, teach them, and return to the original. No author preference has been inferred, and this extension contains no attributed literary quotations.

The two paths solve different problems. A dense interesting page is not a substitute for ample accessible reading; a simplified story is not the endpoint of an ambition to read serious work. Do not prescribe a universal lexical-coverage percentage as a comprehension guarantee. Scope, reference, negation and assumed subject knowledge can obstruct a passage even when its individual words look familiar.

For mathematics, progress from a small finite world to quantified sentences, definitions, examples and counterexamples, then connected proof steps. For philosophy, add reporting another person's claim, reasons, evidence, uncertainty, objections and the difference between understanding a claim and endorsing it. Familiar concepts may help motivation; they must not make the Russian unnecessary to the task.

## What remains to become a full course

These are specific content and representation gaps, not work assigned to Ember or the friend:

1. **A more complete beginner lexical and functional spine.** Add eating/drinking, daily routines, time/date, real questions and short answers, getting help, people/relationships, directions and preferences using recurring stories. Select words by useful senses and constructions; a count of phrase strings is not a vocabulary measure.
2. **Systematic person/number reuse.** The pack contains many forms but not a sustained productive progression through я/ты/он/она/мы/вы/они, common verb classes and irregular high-utility verbs. Add a narrow anchor family, vary one feature, then revisit it in speech and writing.
3. **A case system that becomes reusable.** Add recipient uses of the dative; animate/inanimate accusatives with suitable context; plural patterns; adjective and pronoun agreement across cases; preposition–case meanings. Provide compact references after enough useful examples, with exceptions and syncretism visible. Avoid six disconnected lists of endings.
4. **A fuller sound-and-script progression.** Build a documented inventory for consonant softness, vowel distinctions, stress and reduction, voicing in context, syllable/word boundaries, clusters and conversational intonation. Each needs suitable words, actual playable reference audio and separate perceptual/productive tasks. Printed acute marks and an alphabet table cannot establish these skills.
5. **Connected writing and conversation.** Add short messages, descriptions, questions after a reply, retellings and later arguments. Preserve meaning and communicative success alongside spelling and form accuracy. Typing, dictation and original composition must not collapse into a single writing score.
6. **Deeper reading grammar as it becomes useful.** Relative clauses here precede more demanding participial constructions, passive patterns, conditionals, comparisons, reported speech and argument connectors. Revisit rather than “cover once.” Their order should respond partly to the selected passages.
7. **A lexeme/sense/construction layer.** Store lemma, sense, inflected form, stress, useful collocations, grammatical role and encounter/retrieval history. Do not award retrieval credit to every word merely because it appeared in a sentence. This layer can support a passage planner without pretending that simple word coverage is comprehension.

## Assessment and integration boundaries

Exercise target IDs now name only relevant distinctions. For example, a spoken preference question does not also credit needing something; a milk request does not credit bare-instrumental writing; an existence claim does not credit quantifier dependency or negative scope. Phrase target IDs are likewise specific. This is necessary metadata hygiene, but a target tag still does not prove that a particular response demonstrated mastery.

The fixed composition exercises are **changed-message practice**. The current content schema does not store held-out-probe identity, exposure eligibility, difficulty matching or a transfer claim. A genuinely novel delayed probe needs an item version that has not already been displayed, played, paraphrased, or revealed in feedback to this learner. Repeating a probe makes later attempts repeated-item evidence. Keep the probe generator/specification, concrete generated item, validity review and actual exposure history separate.

For interpretation, separate the Russian message from prior subject knowledge. In scope exercises, a fictional world can be changed so the learner cannot answer from an arithmetic fact. Distinguish “compatible with” from “required by.” For production, exact matching recognizes authored variants; a nonmatch does not prove the response is ungrammatical. Contextual AI review should keep the original answer and allow correction of its own judgment.

Listening choices are parallel English meanings, with balanced answer positions and no Russian script in the options. Hearing tasks require actual audio. Revealing the phrase, translation or model answer is recorded support, not an invisible part of independent success. Speech and composition use contextual review; text matching cannot establish pronunciation. TTS should retain exact ordinary and supported text plus provider/voice provenance, especially for stress and specialist phrasing.

Read-only editorial validation checks IDs, dependencies, membership, stress/spelling identity, accent placement, multisyllabic scaffold coverage, source links, choice contracts, hidden production inputs, audio requirements, all four skills and listening-position balance. A second AI editorial review found no clear Russian stress, ё or core grammar errors and identified assessment/distractor problems that were corrected. These checks do **not** verify Russian audio, real learning, native naturalness, or runtime device behavior.
