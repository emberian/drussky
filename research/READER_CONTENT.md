# Starter Reader anthology

Prepared 2026-11-06. The anthology supplies four coherent texts, with their original text preserved separately from all teaching assistance. It is stored in [reader-v1.json](../crates/rufren-content/content/reader-v1.json). It does not alter the embedded lesson pack, existing phrase IDs, or practice history.

## The four readings

| Stable definition ID, version `1` | Reading | Russian words | Chunks | What the reader can explore |
| --- | --- | ---: | ---: | --- |
| `ru.reader.echo` | A voice on a recording | 58 | 10 | Anya recognizes her recorded voice but not its words. A sound stops when she mutes playback. Observation, a proposed inference and uncertainty remain distinct. |
| `ru.reader.no_largest` | Why there is no largest natural number | 46 | 8 | An arbitrary number, a construction of a larger one, and a universal conclusion. The argument works whether zero is included among the natural numbers. |
| `ru.reader.words_and_evidence` | Words, evidence and a reason to help | 54 | 8 | Two speakers distinguish conclusive proof from a reason to investigate or help. The dialogue does not decide whether a robot has experiences. |
| `ru.reader.chekhov_student_opening` | Chekhov: the first sounds of *The Student* | 21 | 5 | Quiet weather can contain bird calls; a sound comparison does not identify an actual bottle or a particular unseen creature. |

Word counts count Cyrillic words, retaining hyphenated forms such as что-то as one word and excluding dialogue dashes. The inventory contains 31 exact chunks, 99 anchored gloss/grammar notes and 8 optional comprehension prompts. These counts describe material, not difficulty, proficiency or learning gains.

The first three texts are original AI-authored works created for Rufren. The mathematical argument is exposition, not fiction; the philosophical dialogue presents attributed positions, not an answer the learner must endorse. The shared source-kind value `ai_authored` correctly covers all three, and source provenance specifies their genres. No named author’s work has been imitated, silently simplified or presented as an original quotation.

## The verified literary source

The Chekhov selection is the first **two complete sentences** of «Студент», on page 306 of:

> Чехов А. П. Полное собрание сочинений и писем: В 30 т. Сочинения: В 18 т. АН СССР; Ин-т мировой лит. им. А. М. Горького. Москва: Наука, 1977. Том 8: Рассказы. Повести, 1892–1894. С. 306–309.

The [FEB digital scholarly text](https://feb-web.ru/feb/chekhov/texts/sp0/sp8/sp8-306-.htm?cmd=p) was opened and inspected on 2026-11-06. Its bibliographic header supplies that edition and page range; the two sentences occur together in the opening paragraph. A separate AI editorial pass compared the selected text with that page. The [Wikisource transcription](https://ru.wikisource.org/wiki/Студент_(Чехов)) independently identifies the underlying work as published in 1894 and the author as 1860–1904; it is corroborating metadata, not the selected textual authority.

The underlying Russian literary work is public domain. The anthology reproduces only that text, not the modern edition’s commentary, apparatus, website design, or any published translation. Its rights field does not purport to license the whole 1977 volume or source website. All English translation and reading help are newly AI-authored and attributed independently.

The quoted source retains its ordinary spelling, punctuation and single paragraph. It ends at a sentence boundary. No internal omission, modernization or inserted ellipsis occurs. The source-selection note outside the original makes clear that this is an excerpt rather than the complete opening paragraph or story. The excerpt contains no е/ё modernization issue; added stress is still a separate support layer.

SHA-256 of the exact excerpt’s UTF-8 original bytes, without a trailing newline:

```text
789b66e16086a3d62d893c78818213ac6d279d897b75ce3de44b5bd503c3b83c
```

A checksum establishes which local bytes are preserved; it does not by itself establish their accuracy against an external edition. The source comparison supplies that distinct check.

## Reading support without replacing the text

The original should remain visible as coherent prose, including its paragraphs and dialogue turns. Chunk boundaries provide optional help within that prose; they are not a reason to turn every sentence into a disconnected phrase card. Some chunks deliberately continue a sentence, such as “for every natural number” followed by “there is a larger natural number.” Show enough surrounding text that the dependency remains intelligible.

Every chunk has an optional English translation and a stress scaffold. Selected exact spans offer lexical meanings and compact grammar explanations. A gloss chooses the sense that fits the passage: запись is a recording here, точно introduces a simile in Chekhov, and доказательство is interpreted as conclusive proof in the authored philosophical distinction while its broader evidence sense is acknowledged. All help is explicitly AI-authored, even when the original is an authentic literary excerpt.

For an absolute beginner, offer a supported preview: inspect the full meaning, hear or read one short chunk, identify one useful construction, and return to the intact passage. These are interesting destinations, not a new claim that all four texts are independent beginner reading. A learner can understand the scene with assistance before being able to produce its language. There is no compulsory vocabulary quiz or prerequisite lock.

The 24 lessons supply useful encounter points:

- **Echo:** station scene, past time, hearing, reporting a thought, uncertainty and the second signal scene. Additional help is supplied for a negative instruction, possession and the apparent source of a sound.
- **No largest:** mathematical let, comparison, quantifiers and scope. Follow the logical steps before requesting a complete account of every inflection.
- **Words and evidence:** reasons, reported propositions, dative experiencers, need and negation. Keep the hypothetical robot’s quote distinct from the two human-language speakers’ questions and judgments.
- **Chekhov:** location, past descriptions and listening vocabulary. The unfamiliar bird and marsh words are glossed rather than assumed. Interpretation can remain in English while the learner attends to the Russian clues.

The original station story was revised during editorial review to report a sound from the direction of the door, rather than asserting that someone knocked. The latter would partly settle the very inference the reader is invited to consider. The final wording also specifies muting sound, avoiding an ambiguity between stopping playback and stopping a recording session. The elliptical question about memory remains intentionally unresolved.

## Optional comprehension, not hidden grading

Each text has two optional prompts with teaching notes. English answers are explicitly welcome. The prompts ask about meaning or textual evidence; they do not require agreement with a philosophical position or an invented solution to a mystery.

- Echo separates the observed stopping of a sound from the robot’s suggestion that nobody is there.
- Mathematics separates a construction for each starting number from a single number larger than all numbers. Naming a familiar theorem from memory does not establish comprehension of its Russian argument.
- Philosophy distinguishes “not conclusive proof” from “no evidence at all,” and understanding a speaker from endorsing the speaker.
- Chekhov distinguishes calm weather from total silence and a comparison from an asserted object in the scene. The excerpt does not name the unidentified living thing’s species.

`teaching_note` is teacher-facing support, not a model answer to reveal before the learner chooses to respond. The current ReaderPrompt has no grading contract. Reading, opening help, playing a model, or keeping a phrase must not silently create a retrieval success. A fixed prompt revisited later is not automatically a novel transfer probe.

## Shared schema and immutable identity

The JSON container is:

```text
{schema_version: 1, id: "ru.reader.anthology", version: "1", documents: [ReaderDefinition, ...]}
```

The document shapes follow the shared `rufren-core::reader` DTOs, rather than a second content-only model. Each definition has `id`, `version`, `title`, `original`, `original_sha256`, `source`, `help_provenance` and `chunks`. Source metadata identifies authorship, edition, URL where applicable, rights and provenance. It is separate from `help_provenance`.

Every chunk’s `span` is a half-open interval of **global UTF-8 byte offsets in the original**. Chunks partition the entire original contiguously, including spaces and newline delimiters; their `text` values concatenate exactly to `original`. Every gloss and grammar note also has a global byte span, lies inside its owning chunk, and quotes the exact source slice in `text`. Neither grapheme indices nor offsets in the stressed string can be substituted for these coordinates.

A stress scaffold only adds U+0301 acute marks. Removing those marks gives the chunk’s exact original bytes, including punctuation, ё, spaces and newlines. It does not normalize spelling. Translation is independently authored assistance; it is not a replacement source layer. This anthology contains no separately simplified adaptation, so none should be inferred from an English gloss.

Version `1` identifies these authored definitions. After integration, any source or help change needs a new version and preservation of already imported immutable definitions. Lookups and document construction should use exact ID plus version; no silent latest-version substitution is intended. Reader audio derivatives and deliberately kept phrases retain their document/chunk provenance through the shared Reader contract, without replacing historical lesson items.

## Validation and limits

Read-only checks passed for four original SHA-256 hashes, unique document/chunk/prompt identities, exact contiguous UTF-8 partitions, all 99 note spans, stress-stripped equality, stress placement after Russian vowels, complete multisyllabic stress scaffolds, eight prompts and the transcribed Chekhov excerpt. Original texts are 666, 612, 603 and 242 UTF-8 bytes, within the shared initial 4096-byte original-text bound. Source metadata is below the shared per-source bounds.

A second AI editorial review found no clear Russian stress, spelling, grammar, translation or argument error in the final draft. It prompted corrections to inference wording and avoided adding an unspecified “room” to questions whose source only says “there.” This is AI review, not native-speaker certification.

This lane ran no Cargo, Dioxus, speech generation or inference. The content owner integrates static loading and the canonical Rust validation; the root serializes build and device verification. Existing lesson pack `.4`, historical IDs and practice semantics remain unchanged by this anthology. The existence of precise source spans does not establish actual audio quality, alignment, effective teaching or reading competence.
