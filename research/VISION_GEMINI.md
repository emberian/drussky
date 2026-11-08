# ЧЕРТЁЖ (CHERTYÓZH): A Synthetic Russian Acquisition Compiler for `ember`

## 1. Executive Thesis & The Core Metaphor

Standard language apps fail a systems engineer in two opposite ways. Gamified consumer apps (Duolingo, Babbel) treat the learner as an easily bored child who must be bribed with streaks to memorize tourist phrasebooks (*«Где вокзал?»*), hiding the formal structure of the language behind opaque trial-and-error. Conversely, "comprehensible input" purists and unstructured flashcard decks (Anki) dump thousands of surface forms onto the learner’s working memory, demanding that an adult infer a complex six-case, two-aspect, stress-shifting morphosyntactic system by sheer statistical osmosis.

For `ember`—a computer scientist fluent in formal methods, type systems, and compilers, two weeks into Russian, living with a native Russian speaker, aiming to read Kolmogorov, Lotman, and the Strugatskys within six months, and operating under variable working-memory capacity—both approaches are pessimal.

**Чертёж** (*Chertyózh*, "technical blueprint / draft") is built on a single foundational premise: **Russian is a strongly typed, morphologically compiled language, and an adult CS learner acquires it fastest by treating acquisition as a two-stage compiler pipeline: explicit declarative specification followed by latency-targeted procedural lowering.**

### Why Working Memory Is the Central Design Constraint
In English, syntactic roles are encoded mostly by word order; individual tokens barely mutate. In Russian, uttering or parsing a simple four-word clause requires resolving:
1. **Lexical root & derivational affixes** (what is the word family?)
2. **Governor type signature** (which case or preposition does this verb/noun demand?)
3. **Morphological inflection** (gender $\times$ animacy $\times$ number $\times$ declension class $\times$ case ending, plus consonant mutation)
4. **Verbal aspect & motion vector** (imperfective state/iteration vs. perfective transition; unidirectional vs. multidirectional + spatial prefix)
5. **Prosodic compilation** (Zaliznyak stress paradigm $a, b, c \dots$, which triggers vowel reduction—*аканье/иканье*—and changes the acoustic shape of the word)

When a novice tries to execute all five passes simultaneously in real time, working memory ($WM$) saturates, latency spikes past 3,000 ms, and speech/listening collapses. 

**Чертёж** solves this via **Degree-of-Freedom Isolation (DoFI)** and a **Working-Memory Governor**. The system never asks the learner to compute $N$ uncompiled linguistic operations at once. A new grammatical rule or lexeme is introduced synthetically with its explicit formal signature visible on screen (zero $WM$ lookup cost). Then, micro-drills compile each sub-operation down to automaticity ($<650\text{ ms}$ response latency) in isolation before composing them into full clauses.

```
[Declarative Spec]      Explicit Type Signature / Paradigm Table / Derivational Tree
        │               (Externalized on iPad/Mac canvas; zero working-memory tax)
        ▼
[Pass 1: Factoring]     Single-Degree-of-Freedom Micro-Drills (Case OR Aspect OR Stress)
        │               Target: Accuracy > 92%, Median Latency < 850ms
        ▼
[Pass 2: Lowering]      Acoustic & Motor Binding (iPhone Audio Loops + iPad Cursive / Mac JCUKEN)
        │               Target: Phonological decoding at 4.5 syll/sec; eyes-free production
        ▼
[Pass 3: Composition]   Stratified Reading Workbench & Bounded Partner Protocols
                        Target: Syntactic tree flattening & live domestic deployment
```

---

## 2. The Core Loop: Compile, Lower, Compose

Every piece of linguistic knowledge in **Чертёж** moves through a four-phase loop governed by telemetry rather than arbitrary lesson numbers:

1. **Structural Briefing (Synthetic Spec, 2–3 min):** No guessing games. If introducing the Instrumental Case (*творительный падеж*) or prefixal verbs of motion (*в-/вы-/пере-/про-* + *-ходить/-йти*), the system presents a clean algebraic spec: the semantic operators, the morphological transformation rules (including fleeting vowels and velar/hushing spelling rules *ГКХ / ЖШЩЧЦ*), and the stress shift class.
2. **Single-Axis Compilation (5–8 min):** Rapid, low-$WM$ exercises where *everything except the target operation is held constant*. To drill case endings, the lexeme stays fixed across 6 syntactic frames, or the syntactic frame (`без + Gen`) stays fixed across 8 nouns of known declension. The UI keeps reference tables visible until latency drops below $1,200\text{ ms}$, then fades them.
3. **Sensorimotor Lowering (5–10 min):** Moving the compiled rule from visual-analytic cortex into auditory/motor loops—Apple Pencil cursive (*прописи*) on iPad, blind JCUKEN typing on Mac, or eyes-free acoustic shadowing on iPhone.
4. **Contextual Composition (10–20 min):** Reading real (initially syntactically scaffolded) mathematical, philosophical, or SF prose on the Mac/iPad Workbench, or executing a 7-minute **Domestic Partner Protocol** with the Russian speaker at home.

---

## 3. What a Day and a Week Look Like

Because working memory fluctuates across the day and week, **Чертёж** has no fixed "daily streak" quota. Instead, opening the app exposes a three-state **Cognitive Load Selector**:
* **`[FULL STACK]` (High WM / Fresh):** Introduces new morphological paradigms, complex participial syntax, and dense reading.
* **`[STEADY]` (Normal WM):** Balanced review, single-axis speed compilation, and scaffolded reading.
* **`[LOW RAM]` (Stretched WM / Tired):** Zero new items, zero multi-step compositional production. Switches entirely to receptive acoustic training, Pencil tracing with visible guides, or heavily glossed reading where clicking any clause flattens its dependency tree automatically.

### A Representative Weekday (~55 Minutes Distributed Across 4 Touchpoints)

#### 08:30 — Morning Boot on iPad + Apple Pencil (15 min)
* **08:30–08:35 | Orthographic & Cursive Calibration:** The iPad displays 5 root families (e.g., `-пис-`: *писать, записать, переписка, описание, письменный*). Using the Apple Pencil, `ember` writes the cursive forms into a lined *пропись* grid while hearing the native audio with explicit stress (*записа́ть* vs. *за́пись*). Stroke-order telemetry flags if the hooks before *л, м, я* are omitted.
* **08:35–08:45 | Synthetic Paradigm Drill:** Today’s target: Prepositional vs. Accusative of Direction/Location (*в пространстве* vs. *в пространство*; *на поверхности* vs. *на поверхность*). The screen holds the vocabulary constant (5 high-utility mathematical/spatial nouns) and fires 24 rapid binary transformation prompts.

#### 13:15 — Midday Eyes-Free Walk on iPhone + AirPods (12 min)
* **Phonological Decoding & Shadowing Loop:** Phone stays in pocket.
  * *Part A (4 min):* Minimal-pair perception radar—distinguishing hard vs. palatalized consonants (*угол* [corner] vs. *уголь* [coal]; *быть* vs. *бить*) and unstressed vowel reduction (*молоко́* $\rightarrow$ `[məlɐˈko]`). `ember` taps AirPods stem (single vs. double tap) or speaks "один / два".
  * *Part B (8 min):* Latency-gated audio prompt loop. System speaks an English or symbolic cue (`"in this space"`, `"without a proof"`); `ember` has $1,100\text{ ms}$ of silence to speak `«в этом пространстве»`, `«без доказательства»` before the reference native audio plays for immediate self-correction.

#### 19:30 — Domestic Partner Protocol (7–10 min, Live in Kitchen/Living Room)
* Most learners with a native-speaking partner fail to use them effectively: free conversation is impossible at Week 2, and asking a partner "why is this genitive?" turns intimacy into unpaid, frustrating tutoring.
* **Чертёж** generates a **Bounded Partner Card** on `ember`’s iPhone (or AirDropped to the partner’s phone). It gives the partner a strict 5-minute game script using *only* structures `ember` has compiled above 80% accuracy:
  > **Partner Card #14 (Imperative + Accusative/Instrumental Spatial Manipulation):**
  > *Instructions for Partner:* Sit at the table with a cup, pen, notebook, and phone. Issue 12 commands at natural speed using *положи, возьми, открой, дай* + prepositions *на, в, под, рядом с, без*. Wait for `ember` to execute the action AND state what they did in past perfective (*«Я положил ручку рядом с тетрадью»*). Only correct stress or case ending by repeating the phrase naturally once; tap `[+]` on any word where `ember` hesitated.

#### 21:15 — Evening Workbench on Mac (20 min)
* **10 min | Stratified Reading Ladder:** Reading a 150-word passage (in Month 1: a synthetic proof about sets and functions; by Month 3: an excerpt from Kolmogorov or Zamyatin). Unknown morphological endings are highlighted by syntactic role; pressing `Space` while hovering over a word reveals its root decomposition (`пре- + образова- + -ние`) and governor link.
* **10 min | JCUKEN Touch-Typing & Cloze Synthesis:** Typing Russian without looking at the keyboard, completing morphological transformations of sentences extracted from the evening’s reading.

### What a Week Looks Like
* **Monday–Thursday:** Acquisition & Compilation cycles (roughly 15 new lexemes/day organized into 3–4 root families, plus 1 morphosyntactic operator every 2 days).
* **Friday:** **Integration & Diagnostic Probe.** Timed latency sweeps across the week's paradigms; no new grammar introduced.
* **Saturday:** **Deep Text Excavation (35–45 min on Mac/iPad).** Working through a longer target-domain text (a full theorem + proof, a philosophical aphorism cluster, or a 1-page SF scene) with the iPad Pencil used to diagram complex clauses, followed by a 10-minute Partner Protocol reading the text aloud for prosody and palatalization coaching.
* **Sunday:** **System GC (Garbage Collection).** 15 minutes of low-load review of leeches (items where latency plateaued or error rate exceeded 20%); offline Mac batch pipeline compiles the coming week’s personalized curriculum.

---

## 4. Concrete Screens & Interfaces

### Screen A: The iPad Morphological Scope & Pencil Canvas (`Морфоскоп`)
```
+-----------------------------------------------------------------------------------+
| ЧЕРТЁЖ // МОРФОСКОП                       [RAM: STEADY]  [Zaliznyak: 1a / 3b*]    |
+-----------------------------------+-----------------------------------------------+
| ROOT DAG:  -МЕР- (measure/bound)  | ACTIVE OPERATOR:  РОД. ПАДЕЖ (Genitive)       |
|                                   | Signature:        отсутствие / часть / предел |
|         [мер-а] (f, 1a)           | Prepositions:     без, от, до, из, для, у     |
|            │                      |                                               |
|   +--------+--------+             | GOVERNOR FRAME:                               |
|   ▼        ▼        ▼             |   «Система не существует без ___________»     |
| [раз-мер] [при-мер] [из-мере-ние] |   Target: [точная мера] (sg)                  |
|  (m, 1a)   (m, 1a)   (n, 7a)      |                                               |
|                                   | DECOMPOSITION SCAFFOLD (Auto-fades < 900ms):  |
| Phonetic Realization:             |   Adj (f.Gen.sg):  точн- + -ой  [ˈtotɕnəj]    |
|   измере́ние -> [ɪzmʲɪˈrʲenʲɪjə]   |   Noun (1st.Gen):  ме́р-  + -ы   [ˈmʲerɨ]      |
+-----------------------------------+-----------------------------------------------+
| APPLE PENCIL CURSIVE INPUT (Пропись):                                             |
|                                                                                   |
|   ... без   т о ч н о й    м е́ р ы  .                                             |
|   -----------------------------------------------------------------------------   |
|   [Stroke Telemetry: 'ч-н' ligature OK | Stress mark placed on 'е' OK | 1,140ms]  |
+-----------------------------------------------------------------------------------+
```

### Screen B: The Mac Stratified Reading Workbench (`Читалка-Анализатор`)
```
+-----------------------------------------------------------------------------------+
| ЧЕРТЁЖ // ЧИТАЛКА: А. Н. Колмогоров — «Основные понятия теории вероятностей»       |
| [Layer: 1-Plain] [2-Stress+Chunks] [*3-Dependency Flatten*] [4-Root X-Ray]        |
+---------------------------------------------------+-------------------------------+
| ORIGINAL TEXT (with prosodic stress & chunking):  | SYNTACTIC FLATTENER (Focus):  |
|                                                   |                               |
| [Пусть E — коне́чное мно́жество элемента́рных        | Clause Type: Participial Mod  |
|  собы́тий] , │ [а F — систе́ма подмно́жеств E] ,     | (Причастный оборот -> Который)|
|  │ [называ́емая по́лем собы́тий] , │ [удовлетворя́ющая|                               |
|  следующим аксио́мам] :                            | Surface Form:                 |
|                                                   |   «систе́ма F, называ́емая      |
|  1. F соде́ржит само́ мно́жество E как э́лемент.      |    по́лем собы́тий»             |
|  2. Е́сли A и B принадлежа́т F, то ...              |                               |
|                                                   | Desugared Kernel:             |
|---------------------------------------------------|   «систе́ма F, кото́рую         |
| HOVER INSPECTOR: «удовлетворя́ющая»                |    называ́ют по́лем собы́тий»    |
|  Lemma:   удовлетворя́ть (impf) -> + Dat (чему́?)   |                               |
|  Parse:   Participle, Pres.Act, Nom.Sg.Fem        | Case Assignment:              |
|  Agrees:  <── систе́ма (Nom.Sg.Fem)                |   называ́ть (X:Acc, Y:Inst)    |
|  Governs: ──> аксио́мам (Dat.Pl of аксио́ма)        |   по́лем = Inst.Sg of по́ле     |
|  Roots:   у- + до- + воля (will) + -твор- (make)  |                               |
+---------------------------------------------------+-------------------------------+
```

---

## 5. How Each Modality Is Taught and Assessed

### 5.1 Grammar & Morphosyntax: The Algebraic Type System
Russian grammar is taught not as chapters in a tourist textbook, but as a **formal type system with explicit reduction rules**:
* **Cases as Typed Channels:** Every preposition and verb is stored and displayed with its case signature:
  * $\text{принадлежать} : \text{Nom}_{\text{elem}} \rightarrow \text{Dat}_{\text{set}} \rightarrow \text{Prop}$ (*принадлежать множеству*)
  * $\text{зависеть} : \text{Nom} \rightarrow \text{от}(\text{Gen}) \rightarrow \text{Prop}$ (*зависеть от времени*)
  * $\text{являться} : \text{Nom} \rightarrow \text{Inst}_{\text{pred}} \rightarrow \text{Prop}$ (*являться функцией*)
* **Morphological Paradigms via Sound Laws, Not Brute-Force Tables:** Instead of memorizing 50 scheinbar-irregular tables, `ember` learns the **Three Orthographic Invariants** (7-letter rule, 5-letter rule, 8-letter rule) plus **Phonemic Stem Endings** (hard consonant vs. soft consonant vs. velar vs. sibilant). Once the stem ends in a phonemic representation (e.g., `/knʲig-/`), almost all "irregularities" compile deterministically.
* **Zaliznyak Stress Classes:** Stress is never omitted. Every word in **Чертёж** is rendered with its acute stress mark (`´`) and tagged with its simplified Zaliznyak accentual paradigm:
  * **Type `a`:** Fixed stem stress (*мно́жество, фу́нкция*).
  * **Type `b`:** Fixed ending stress (*число́ — числа́, очки́*).
  * **Type `c`:** Mobile stress shifting between singular and plural (*сло́во — слова́, ме́сто — места́, сторона́ — сто́роны*).
* **Verbal Aspect as a State-Transition Automaton:** Aspect pairs (*решать / решить*, *определять / определить*) are taught via state diagrams: Imperfective = `[In-Progress / Iterative / Unbounded Process / Statement of Fact]`; Perfective = `[Atomic Event / Telic Boundary Crossed / Next Sequential State]`.
* **Assessment:** Grammar is assessed along two orthogonal axes: **Rule Accuracy** (can you derive the form?) and **Compilation Latency** (can you produce the inflected token inside a syntactic frame in $<750\text{ ms}$?). A paradigm is marked `DECLARATIVE_ONLY` until latency drops below threshold.

### 5.2 Listening: Bottom-Up Phonological Decoding to Prosodic Parsing
Adults fail at Russian listening because they learn how words look on paper (*хорошо, говорил, с братом*) and then cannot recognize their acoustic realizations (`[xərɐˈʂo]`, `[ɡəvɐˈrʲil]`, `[ˈzbratəm]`).
* **Stage 1: The Phonological Compiler (Weeks 2–5):** Explicit training on the five rules that transform Russian orthography into sound:
  1. *Аканье / Иканье* (two-tier unstressed vowel reduction relative to the stressed syllable).
  2. *Palatalization ($C$ vs. $C^j$)*: Focusing on acoustic formant transitions in the preceding vowel rather than just the consonant release.
  3. *Regressive Voicing Assimilation & Final Devoicing* (*код* = *кот* `[kot]`; *вчера* = `[ftɕɪˈra]`; *сделать* = `[ˈzdʲelətʲ]`).
  4. *Consonant Cluster Simplification* (*здравствуйте*, *чувство*, *солнце*).
  5. *Bryzgunova Intonation Constructions (ИК-1 to ИК-5)*: Essential because Russian yes/no questions have identical word order to statements, distinguished *solely* by a sharp pitch spike (ИК-3) on the stressed syllable of the queried word (*«Это лине́йное пространство?»* vs. *«Это линейное простра́нство?»*).
* **Stage 2: Clause-Chunked Time-Compressed Audio:** Long sentences are first presented with $350\text{ ms}$ synthetic pauses inserted at syntactic phrase boundaries, then progressively stitched together to 1.0x and 1.2x native speed.
* **Assessment:** Dictation-by-constituent (hearing an utterance and typing or selecting the exact case/number/aspect encoded by the unstressed ending, e.g., distinguishing *он говори́т* from *они́ говоря́т*, or *кни́ги* [Gen.Sg] from *кни́ге* [Dat/Prep.Sg] in context).

### 5.3 Speaking: Low-WM Articulatory Framing & Domestic Protocols
* **Motor Compilation:** Before attempting open-ended thoughts, `ember` drills **Prosodic Words** (preposition + clitic + noun treated as a single phonetic word with one stress peak: *на́ воду*, *за́ руку*, *без пяти́*).
* **Syntactic Slot-Stepping:** On iPhone/Mac, the system gives a base clause (*«Я читаю статью о топологии»*) and issues rapid single-axis mutation operators:
  * `+ PAST` $\rightarrow$ *«Я читал статью о топологии»*
  * `+ NEGATION (Genitive shift)` $\rightarrow$ *«Я не нашёл статьи о топологии»*
  * `+ PLURAL` $\rightarrow$ *«Я не нашёл статей о топологии»*
  Because only *one* syntactic register changes per step, working memory never overflows.
* **Assessment:** Forced alignment via local Wav2Vec2-Russian acoustic model measures: (1) Reaction latency from prompt to voice onset ($VOT$), (2) Vowel duration ratio between stressed and unstressed syllables (detecting flat English-style rhythm), and (3) Palatalization posterior probability on target minimal pairs.

### 5.4 Reading: Syntactic Desugaring & Root-Derivational X-Ray
Russian academic, philosophical, and literary prose is infamous for **nested left-branching participial constructions** (*«множество всех точек, не принадлежащих замкнутому подпространству, порождённому данными векторами...»*) where English uses right-branching relative clauses.
* **The 4-Layer Reader Lens:** Every text in **Чертёж** is pre-parsed into a Universal Dependencies (UD) graph and can be toggled across four visual modes:
  1. **Raw + Stress:** Clean typography with acute stress marks on all polysyllabic words (optional toggle to hide stress on `Mastered` words).
  2. **Constituent Bracket Mode:** Subtle typographic spacing and color-coded underlines grouping `[Participial Clauses]`, `[Nominal Heads]`, and `[Governed Case Objects]` so the eye sees the phrase structure without hunting for agreement endings across 12 words.
  3. **Syntactic Desugaring (Tree Flattener):** Tapping any dense clause rewrites it into 2–3 flat kernel sentences in Russian (converting *причастия* and *деепричастия* into *который* and *когда/так как* clauses) before showing any English translation.
  4. **Derivational X-Ray:** Splits every compound word into `[Prefix] + [Root] + [Suffix] + [Inflection]`, linking it to already-known words sharing the same root.
* **Assessment:** Instead of dull comprehension quizzes, assessment happens inline via **Referent & Governor Resolution**: occasionally, a pronoun (*который, его, ней, этим*) or an inflected participle in the text pulses; `ember` must tap the exact head noun it agrees with or refers to, proving syntactic parse comprehension rather than keyword guessing.

### 5.5 Writing: Pencil *Прописи* to Blind JCUKEN Production
* **Why Handwriting Matters for Russian:** Russian cursive is not decorative; it is how natives write on whiteboards, margin notes, and slips of paper, and its visual topology (*л, м, и, ш, щ, п, т*) is completely distinct from block print. More importantly, Apple Pencil handwriting slows down production just enough to bind orthographic roots into motor memory without keyboard hunting.
* **Phase 1 (Weeks 2–6): iPad Pencil *Прописи*:** Tracing and freehand writing of roots, hooks (*соединительные элементы*), and inflected endings.
* **Phase 2 (Weeks 3+): Mac JCUKEN Blind Typing:** A built-in mechanical typing trainer mapped to Russian phonetic/morphological frequency (starting with central home-row morphemes `о а е и н т с р в л`), transitioning into **Morphological Reconstruction Drills** where `ember` sees a lemmatized skeleton (`[Пусть] [функция] [f] [быть] [определить] [на] [отрезок] [a,b]`) and types the fully inflected sentence (`Пусть функция f определена на отрезке [a,b]`).

---

## 6. Content Production & Neuro-Symbolic Quality Assurance

A fatal flaw of current "AI language apps" is calling an LLM at runtime and showing its raw output to the learner. LLMs frequently hallucinate stress placement in Russian (confusing *за́мок* and *замо́к*, or placing stress on the wrong syllable of inflected forms like *ру́ки* vs. *руки́*), invent unidiomatic aspectual usage, or violate pedagogical constraints by sneaking five unknown grammar rules into a "simple" example sentence.

**Чертёж never runs unverified generative output at runtime.** Instead, the Mac runs an **Offline Neuro-Symbolic Content Compiler** (`chertyozh-compile`) that uses Claude/Gemini/Codex as candidate generators inside a deterministic verification harness.

```
[Target Spec: Active Lexicon + Allowed Grammar Graph + Domain Corpus Seed]
                                  │
                                  ▼
         [LLM Generator: Claude 4 / Gemini / Codex Batch API]
         Generates candidate exercises, synthetic bridges, desugarings
                                  │
                                  ▼
+-----------------------------------------------------------------------+
| DETERMINISTIC VERIFICATION HARNESS (Mac Local Gatekeeper)             |
|                                                                       |
| 1. Morphological & Vocabulary Gate (pymorphy3 / OpenCorpora DAWG):    |
|    - Parses every token. Rejects if >1 unknown lemma or uncompiled    |
|      inflectional slot appears outside the designated target slot.    |
|                                                                       |
| 2. Stress & Homograph Resolver (Zaliznyak Accented Dictionary DB):    |
|    - Injects deterministic stress marks based on (Lemma + Tag).       |
|    - Flags heterophonic homographs (все́/всё, го́да/года́, призна́ть)     |
|      for mandatory UD dependency disambiguation.                      |
|                                                                       |
| 3. Syntactic Tree & Complexity Gate (Natasha / Slovnet / Stanza UD):  |
|    - Computes dependency tree depth & non-projective arc count.       |
|    - Rejects sentences exceeding current Working-Memory budget.       |
|                                                                       |
| 4. Idiomaticity & Collocation Gate (Russian National Corpus N-Grams   |
|    + Cross-Model Adversarial Critic):                                 |
|    - Checks verb+preposition+case frames against НКРЯ frequency.      |
|    - Second LLM model blind-backtranslates & critiques aspect choice. |
+-----------------------------------------------------------------------+
                                  │
                   [Pass: 100% Deterministic Match]
                                  │
                                  ▼
       [Silero / ElevenLabs Phoneme-Forced Audio Synthesis + Alignment]
                                  │
                                  ▼
          [Signed SQLite Content Bundle -> Synced to iPad/iPhone]
```

### Domestic Partner QA Micro-Queue (Optional 60-Second Check)
For conversational sentences or Partner Protocol cards, any item that passes the automated compiler with a border-line idiomaticity score is placed into a 5-item swipe queue on the partner's phone (or web link): the partner sees 5 sentences and taps `Естественно` (Natural) or `Криво` (Awkward) in 30 seconds while waiting for tea to boil.

---

## 7. The Learner Model & Epistemic Humility

### 7.1 Graph Architecture
Standard SRS (Anki, SuperMemo, vanilla FSRS) models knowledge as flat, independent flashcards (`Card #4912: "книга" -> "book"`). In Russian, this is mathematically wrong: knowing *книга* tells you something about *книжный*, *книжка*, and 1st-declension feminine nouns, while telling you nothing about whether you can hear *кни́гой* in fast speech.

**Чертёж** models knowledge as a **Factorized Bipartite Hypergraph** stored in a local SQLite database (`LearningGraph.db`) synced via CloudKit/Syncthing:

1. **Root Nodes ($R$):** ~450 high-yield Slavonic/international roots (`-вод-`, `-лож-/-лаг-`, `-множ-`, `-вид-`, `-зна-`, `-предел-`).
2. **Lexeme Nodes ($L$):** Lemmas linked to their root + derivational affixes (`о- + -предел- + -ение` $\rightarrow$ *определение*, "definition").
3. **Morphological Operator Nodes ($M$):** Specific `(DeclensionClass, Case, Number, StressShift)` or `(ConjugationClass, Aspect, Person)` transformations.
4. **Syntactic Construction Nodes ($S$):** Governor frames and clause structures (`пусть + Nom + Impv3Sg`, `чем + Comp, тем + Comp`, passive participial inversion).
5. **Phonological Contrast Nodes ($P$):** Acoustic decoding filters (`/t/ vs /tʲ/`, unstressed `/o/ -> [ɐ]/[ə]`, cluster voicing).

Each node maintains **four separate channel states** parameterized by an FSRS-v5 memory state $(S_{\text{stability}}, D_{\text{difficulty}})$ **augmented with an Exponential Moving Average of Response Latency ($\tau_{\text{EMA}}$)**:

```sql
CREATE TABLE skill_channel_state (
    node_id TEXT NOT NULL,            -- e.g., 'morph:decl1_fem:inst_sg' or 'lex:определение'
    channel TEXT NOT NULL,            -- 'READ_PARSE' | 'LISTEN_DECODE' | 'WRITE_PRODUCE' | 'SPEAK_PRODUCE'
    stability_days REAL NOT NULL,
    difficulty REAL NOT NULL,
    latency_ema_ms REAL NOT NULL,     -- Procedural compilation metric
    wm_cost_estimate REAL NOT NULL,   -- Derived from latency + error variance: 0.1 (automatic) to 1.0 (heavy)
    evidence_count INTEGER NOT NULL,
    last_verified_at INTEGER NOT NULL,
    PRIMARY KEY (node_id, channel)
);
```

### 7.2 The Working-Memory Budget Equations
Every exercise prompt $E$ is a hyperedge connecting a subset of nodes $\{n_1, n_2, \dots, n_k\}$. Before presenting $E$, the scheduler computes the predicted Working-Memory Load:

$$\text{Load}(E) = \sum_{n_i \in E} \text{wm\_cost}(n_i, \text{channel}) \quad \text{where} \quad \text{wm\_cost}(n_i) = \text{clamp}\left(\frac{\tau_{\text{EMA}}(n_i) - 400\text{ ms}}{1600\text{ ms}} + 0.5 \cdot P(\text{error} \mid n_i),\; 0.05,\; 1.0\right)$$

* When `ember` selects `[FULL STACK]`, the scheduler permits exercises with $\text{Load}(E) \le 1.8$ (e.g., one brand-new morphological rule $\approx 1.0$ plus two semi-compiled lexical items $\approx 0.4$ each).
* When `ember` selects `[LOW RAM]`, the scheduler caps $\text{Load}(E) \le 0.85$ (all scaffolded components must already be compiled below $650\text{ ms}$ if a single shaky element is tested).

### 7.3 Epistemic Humility: What the Model Is Allowed and Forbidden to Claim

| Evidence Observed | What the Model IS Allowed to Claim | What the Model Is STRICTLY FORBIDDEN to Claim |
| :--- | :--- | :--- |
| `ember` correctly selects or translates *«в этом пространстве»* while reading in $1,100\text{ ms}$. | **Receptive Visual Recognition** of lexeme *пространство* and rule `Prep.Sg.Neut (-е)`. | That `ember` can **produce** *пространстве* in speech, **hear** it in continuous audio, or inflect *пространство* in Genitive Plural (*пространств*). |
| `ember` produces *«без доказательства»* in a written Mac drill in $3,200\text{ ms}$. | **Declarative Rule Accessibility**: `ember` knows the rule `без + Gen` and can compute it consciously. | **Procedural Automaticity**: Because $\tau > 1,500\text{ ms}$, `wm_cost` remains high ($0.85$). The system is forbidden from stacking a new participial clause on top of this construction yet. |
| `ember` masters 5 nouns in 1st Declension Instrumental (*водой, книгой, мерой, системой, функцией*) with $\tau < 650\text{ ms}$. | **Operator Compilation (`morph:decl1:inst_sg`)**: Prior probability for unseen regular 1st-declension nouns in `-ой` is boosted; they skip isolated morphological drills. | That `ember` has mastered soft/sibilant variants (*землёй, задачей, матрицей*) where the 5-letter spelling rule mutates `-ой` to `-ей`. Those inhabit a separate operator node. |
| `ember` reads an adapted passage with 98% comprehension using Layer 3 (Dependency Flattener). | **Lexical & Kernel Comprehension** of the passage's vocabulary and local case frames. | **Unassisted Real Reading**: Until Layer 1 (Raw Text) is read at $>90\text{ wpm}$ with $>90\%$ referent resolution accuracy, full syntactic mastery is not claimed. |

---

## 8. The 6-Month Trajectory to Real Reading

A critical insight of **Чертёж** is the **Inverted Genre Ladder**. Conventional wisdom says literature and everyday conversation are "easy" and mathematics/philosophy are "advanced." In Russian, the exact opposite is true for a computer scientist:
* **Everyday Russian / Fiction Dialogue** has low cognate density, massive idiomatic ellipsis, slang, diminutive suffixes (*чаёк, минуточка, хреновина*), and complex physical verbs of motion (*притащиться, заскочить, вывернуться*).
* **Mathematical Russian** (Kolmogorov, Arnold, Gelfand, Pontryagin) has **~40% international or structural cognates** (*аксиома, группа, изоморфизм, топология, вектор, оператор, инвариант*), a tiny core lexicon of ~600 high-frequency words (*множество, отображение, пространство, условие, следствие, пусть, тогда и только тогда, существует единственный*), and highly stereotyped syntax. However, it uses **100% real, uncompromised Russian morphology and participial syntax**.

Therefore, **Чертёж** uses **Mathematics as the Structural Launchpad (Months 1–3)** to compile all Russian cases, pronouns, and participial clauses on familiar semantic ground, then bridges into **Philosophy & Semiotics (Months 3–5)**, and reaches **Science Fiction (Months 4–6)**.

| Month | Lexicon & Root Target | Morphosyntax & Phonology Compiled | Authentic Reading Milestones |
| :--- | :--- | :--- | :--- |
| **Month 1**<br>*(Weeks 3–6)* | **450 lemmas** (120 roots). Logical operators (*пусть, если... то, так как, следовательно*), core spatial prepositions, domestic nouns for Partner Protocols. | Phonological reduction & palatalization; Singular cases (Nom, Acc, Gen, Prep, Dat, Inst) for regular declensions; Present/Past tense; basic Imperative for Partner Protocols. | **Synthetic Micro-Proofs & Definitions:** Elementary set theory, equivalence relations, graph definitions. Reading 100-word definitions at Layer 2. |
| **Month 2**<br>*(Weeks 7–10)* | **1,050 lemmas** (220 roots). Derivational prefixes (*в-, вы-, пере-, под-, раз-, со-*) + core verbal roots (*-ход-, -вод-, -лож-, -став-, -дел-*). | Plural case paradigms (especially Genitive Plural); Aspect pairs in past/future/infinitive; *который* relative clauses in all cases; reflexive *-ся* verbs (*называется, определяется*). | **Gelfand (*Лекции по линейной алгебре*) & Kolmogorov (*Основные понятия теории вероятностей*):** Reading real definitions, theorems, and short proofs on the Mac Workbench (Layer 2 $\rightarrow$ Layer 1). |
| **Month 3**<br>*(Weeks 11–14)* | **1,800 lemmas** (310 roots). Abstract nominalizations (*-ость, -ение, -ание, -ство*), epistemic/philosophical verbs (*означать, полагать, воспринимать, сознание*). | **Active & Passive Participles** (*причастия*: *-ущий, -вший, -емый, -нный*); Short-form adjectives/participles (*определена, замкнуто, равно*); Zaliznyak mobile stress patterns. | **V. I. Arnold (*Обыкновенные дифференциальные уравнения*) & Yu. M. Lotman (*Внутри мыслящих миров* / semiotics essays):** Transitioning from math proofs into structured prose essays on systems, signs, and culture. |
| **Month 4**<br>*(Weeks 15–18)* | **2,700 lemmas** (380 roots). Verbs of motion (unidirectional/multidirectional + all 14 spatial prefixes); psychological, perceptual, and physical-environment vocabulary. | **Adverbial Participles** (*деепричастия*: *-я, -в, -вши*); Subjunctive/Conditional (*чтобы, если бы*); Impersonal dative constructions (*мне кажется, необходимо, приходится*); Numerals + Case agreement. | **M. Mamardashvili (*Лекции о Прусте* / *Картезианские размышления*) & E. Zamyatin (*Мы*):** Mamardashvili's spoken-philosophical syntax + Zamyatin's *Мы* (written by a mathematician/engineer; short, crystalline, geometric clauses). |
| **Month 5**<br>*(Weeks 19–22)* | **3,800 lemmas** (430 roots). Physical action verbs, sensory adjectives, conversational particles (*же, ведь, -то, разве, неужели, вроде*), SF domain lexemes. | Elliptical conversational syntax; expressive word order inversion (Theme-Rheme/Topic-Focus shifts); aspectual nuances in imperatives and negation (*не надо открывать* vs. *не открой*). | **Strugatsky Brothers — *Понедельник начинается в субботу* (Monday Begins on Saturday):** The ideal bridge text—a hilarious Soviet cybernetics/math research institute meets folklore/SF, blending scientific register with lively dialogue. |
| **Month 6**<br>*(Weeks 23–26)* | **5,000+ lemmas** (~480 roots = ~12,000 recognizable surface word-forms via derivational morphology). Colloquial idioms and colloquial register shifts. | Full stylistic register switching; rapid parsing of reduced spoken dialogue and complex literary syntax at $<600\text{ ms}$ per constituent. | **Strugatsky Brothers — *Пикник на обочине* (Roadside Picnic) & *Трудно быть богом*; A. N. Kolmogorov’s papers on algorithmic complexity (*Три подхода к определению понятия «количество информации»*).** Reading unadapted on iPad/Mac with $<3\%$ dictionary lookups. |

---

## 9. Hardware & Machine Learning Architecture

**Чертёж** treats `ember`’s Apple hardware not as interchangeable screens for the same web wrapper, but as specialized peripheral instruments wired to a shared local state graph.

### 9.1 Device Specialization
* **Mac ( Apple Silicon M-Series ) — The Compiler & Reading Workbench:**
  * Runs the nightly/weekly **Offline Content Compiler** (`Python 3.12 + Rust + MLX/CoreML`).
  * Hosts the wide-screen **Stratified Reading Workbench** (dual-pane text + syntactic/root inspector) and **JCUKEN Mechanical Typing Engine** (capturing keystroke inter-arrival latency down to $1\text{ ms}$ to detect morphological hesitation mid-word, e.g., typing `пространств...` fast and pausing $600\text{ ms}$ before `-ами`).
* **iPad + Apple Pencil — The Spatial & Motor Canvas:**
  * **PencilKit Cursive Analyzer:** Captures raw stroke coordinates $(x, y, t, \text{pressure}, \text{azimuth}, \text{altitude})$. Evaluates Cyrillic cursive ligatures (specifically the mandatory lower connection hooks on *л, м, я* that prevent *лишишь* from becoming an illegible sawtooth wave) and verifies manual placement of the acute stress mark *immediately* after completing the word.
  * **Syntactic Tree Sketching:** On Saturday deep-reading sessions, `ember` can draw dependency arrows directly over unadapted Russian PDFs/EPUBs with the Pencil; the system validates the drawn governor-dependent arcs against the underlying UD parse.
* **iPhone + AirPods — The Acoustic & Domestic Field Unit:**
  * **Eyes-Free Audio State Machine:** Designed for walks and low-WM periods. Uses spatial audio cues (left ear = Imperfective / Hard consonant; right ear = Perfective / Palatalized consonant; center = Stressed syllable pulse) to train phonological categories subconsciously.
  * **1-Tap Domestic Capture (`Захват`):** A Lock Screen / Action Button widget. When `ember`’s partner says an interesting phrase at home, `ember` presses the button or types/dictates a rough phonetic approximation. The Mac pipeline matches it, extracts the root/construction, and schedules a micro-drill for tomorrow.

### 9.2 The ML & Linguistic Stack
1. **Deterministic Linguistic Core (Always First):**
   * **Zaliznyak SQLite Database:** Full digital inflectional & accentual dictionary (~100,000 lemmas, ~2.5M accented word forms with exact Zaliznyak paradigm codes `1a`, `4b`, `6*c`).
   * **pymorphy3 + Dawg + Slovnet/Stanza (UD `ru_syntagrus`):** Sub-millisecond morphological tagging and dependency parsing.
2. **Local Acoustic Models (CoreML / MLX on Mac & iPhone):**
   * **ASR & Forced Alignment:** Fine-tuned `Wav2Vec2-XLS-R-300M-Russian` + `Whisper-large-v3-turbo` (on Mac) / `Distil-Whisper-Ru` (on iPhone). Why Wav2Vec2 alongside Whisper? Whisper is a generative seq2seq model that *hallucinates grammatically correct endings* even when the speaker mumbles the case ending! Wav2Vec2 provides raw **frame-level CTC phoneme posteriors**, allowing **Чертёж** to grade whether `ember` actually pronounced `[-oj]` vs `[-əj]` or palatalized `[tʲ]` without language-model autocorrect masking the error.
   * **Prosody & Stress Extractor:** `Parselmouth / Praat` pitch ($F_0$) and intensity contour extraction to verify vowel reduction and ИК-1/2/3 intonation curves.
   * **TTS:** Local `Silero-TTS v4` (instant, obeys explicit `+` stress markers deterministically) for fast drills, paired with cached high-expressivity neural audio for long reading passages.
3. **Frontier LLM APIs (Claude / Gemini / Codex):**
   * Invoked **asynchronously during Mac compilation** (and on-demand only when `ember` explicitly clicks `[Ask Compiler Why]` on a complex sentence). Used for generating domain-specific bridge texts, synthesizing Partner Protocol scripts, and constructing minimal-pair syntactic transformations.

---

## 10. Comparison with Drussky: Keep, Discard, Diverge

### What I Would Keep from Drussky
1. **The Clean `LearningCore` Architectural Separation:** Separating pure domain logic, scheduling math, and linguistic data structures into a deterministic, testable core library independent of SwiftUI presentation views is foundational.
2. **Respect for the Adult Learner & Native Apple Craft:** The refusal to build a bloated cross-platform Electron/React-Native toy; leaning into native macOS/iOS/iPadOS capabilities, hardware keyboard shortcuts, Pencil support, and zero-fluff typography.
3. **Multi-Skill & Multi-Modal Awareness:** Recognizing early that reading, listening, handwriting, and speaking cannot be collapsed into a single multiple-choice button press, and that local/offline tooling on the Mac is a first-class asset for content preparation.

### What I Would Throw Away
1. **Atomic Word/Phrase SRS Cards:** Any data model that schedules words or phrases as isolated strings rather than nodes in a `(Root × Lexeme × MorphologicalOperator × Channel)` hypergraph. In Russian, treating *дом, дома, дому, домом, домашний* as either one opaque card or five unrelated cards breaks either accuracy or scalability.
2. **Open-Ended Runtime LLM Chat / Unbounded Generation Loops:** Any feature where the learner chats freely with an LLM tutor in Russian during the first 12 weeks, or where exercises are generated on the fly without a deterministic Zaliznyak + UD verification gate. Open-ended chat at Week 2 causes immediate working-memory thrashing and teaches the learner to lean on translation crutches.
3. **Standard Thematic Textbook Progression:** Anything resembling CEFR A1 tourist curricula (ordering borscht, booking hotels, colors, clothing, small talk about the weather). None of that helps `ember` talk to their partner at home or read Kolmogorov and the Strugatskys.
4. **Pure Accuracy-Based Grading Without Latency Telemetry:** Grading an exercise `Good/Easy` solely because the user eventually produced the right answer after staring at the screen for 6 seconds. Without latency gating ($\tau_{\text{EMA}}$), the scheduler promotes uncompiled declarative rules into compositional exercises, causing cognitive overload.

### Where Чертёж Differs Most Fundamentally
1. **The Working-Memory Governor & Degree-of-Freedom Isolation:** **Чертёж** explicitly models the cognitive load of every exercise ($\sum \text{wm\_cost}$) and never composes two uncompiled operations ($\tau > 850\text{ ms}$) in the same prompt.
2. **The Math-First Inverted Reading Ladder:** Exploiting `ember`’s formal background to read real Russian mathematical proofs by Month 2—using high cognate density and rigid logical syntax as a Trojan horse to compile heavy Russian participial and case grammar before tackling Lotman and the Strugatskys.
3. **The Domestic Partner Protocol Engine:** Turning the native-speaking housemate from an underutilized or frustrated bystander into a high-leverage, zero-prep 7-minute daily acquisition peripheral via scripted, vocabulary-bounded Partner Cards.
4. **Wav2Vec2 Phoneme Alignment Over Whisper Autocorrect:** Grading spoken case endings and palatalization against raw acoustic phoneme posteriors rather than a language model that politely guesses what the learner meant to say.

---

## 11. Honest Uncertainty & Failure Modes

No design survives first contact with human neurology unchanged. Here are the four highest-risk assumptions in **Чертёж**, how we will detect failure early, and the fallback plan for each:

1. **Risk: The "Analytic Trap" (Over-Intellectualizing Production)**
   * *The Uncertainty:* Because `ember` loves formal structure, there is a real danger of becoming a "human parser" who can analyze any sentence on the Mac Workbench via type signatures, yet freezes in live speech because the brain insists on running the declarative type-checker before opening the mouth.
   * *Detection & Mitigation:* If `READ_PARSE` stability grows $>3\times$ faster than `SPEAK_PRODUCE` latency drops, the scheduler automatically locks the Mac analytic views for 48 hours and forces **Acoustic Reflex Mode** (sub-800ms audio-only slot-stepping where visual grammar tags are completely hidden).
2. **Risk: Partner Protocol Friction**
   * *The Uncertainty:* Even a 7-minute structured game with a domestic partner can feel like a chore if the timing is bad, the partner is tired, or the prompts feel robotic.
   * *Detection & Mitigation:* Partner Protocols are strictly optional in the dependency graph—every speaking target has an autonomous iPhone audio-loop equivalent. If Partner Protocol completion drops below 2x/week, the system shifts the domestic channel to **Passive Domestic Eavesdropping + 1-Tap Capture** (giving `ember` 3 target phrases per day to slip into normal English/Russian domestic life without requiring a sit-down session).
3. **Risk: The Month 4–5 Lexical Cliff (Math $\rightarrow$ SF Transition)**
   * *The Uncertainty:* Moving from Kolmogorov and Lotman (abstract, Latinate/structural vocabulary, explicit logical connectors) to the Strugatskys (*Пикник на обочине*) involves a brutal spike in concrete sensory verbs, Soviet everyday material culture, and colloquial particles (*мол, дескать, небось, авось*).
   * *Detection & Mitigation:* Starting in Week 8, 20% of daily root acquisition is reserved as a **"Concrete & Colloquial Pre-Fetch Buffer"** specifically mined from the exact lemmatized frequency list of *Понедельник начинается в субботу* and *Пикник на обочине*, ensuring those roots are already familiar months before the books are opened.
4. **Risk: Acoustic Forced-Alignment False Rejections**
   * *The Uncertainty:* Frame-level CTC phoneme scoring on Russian palatalization (`/r/` vs `/rʲ/`, `/l/` vs `/lʲ/`) can be overly brittle on noisy room microphones or non-native vocal tracts, causing frustrating false negatives.
   * *Detection & Mitigation:* Acoustic scoring operates in **Calibrated Differential Mode**: whenever the partner validates a pronunciation during a Saturday session, that audio clip is added to `ember`’s personal speaker-adaptation reference bank, calibrating the phoneme boundary thresholds to `ember`’s actual voice rather than an idealized studio voice actor.
