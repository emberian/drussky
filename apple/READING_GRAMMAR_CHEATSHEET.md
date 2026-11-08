# Russian Reading & Decoding Reference for Computer Scientists

This reference is designed as a structural decoding manual for reading Russian mathematics, science fiction, and philosophy. Instead of memorizing isolated textbook paradigms, use this guide as a **parser table**: given a Russian sentence, identify its syntactic skeleton, resolve case markers to recover argument roles regardless of word order, and decode verbal prefixes and participial clauses.

---

## 1. The Case System as a Type & Role Matrix

Russian uses **morphological tagging** rather than fixed positional slots (`SVO`) to mark the thematic role of each noun phrase. Because Russian uses discourse-driven word order (**Theme / Known Information → Rheme / New Information**), `О́рбиту ви́дит Ви́ктор` (`Object – Verb – Subject`) means *"It is Viktor who sees the orbit."*

Always parse a clause by scanning the **endings** first, not the position.

### 1.1 The Six Cases: Core Semantics & Prepositional Triggers

| Case | Question | Role in Type Signature | High-Frequency Prepositions | Math / Sci-Fi / Philosophy Usage |
| :--- | :--- | :--- | :--- | :--- |
| **Nominative** (`И.п.`) | `кто? что?` | **Subject (`S`)** / Predicate identity | *(none)* | `То́чка $x$ лежи́т в $M$.` (Point $x$ lies in $M$.) |
| **Genitive** (`Р.п.`) | `кого́? чего́?` | **Possession / Type parameter (`of T`)**, negation (`нет X`), quantity (`2..4`, `5+`, `мно́го`) | `без` (without), `для` (for), `до` (up to), `из` (from/out of), `от` (from), `по́сле` (after), `у` (by/at) | `мно́жество то́чек` (set of points), `реше́ния нет` (there is no solution) |
| **Dative** (`Д.п.`) | `кому́? чему́?` | **Recipient / Experiencer (`to/for X`)**, impersonal state (`мне ну́жно`) | `к` (toward), `по` (along / according to / by) | `по определе́нию` (by definition), `сле́довать по орби́те` (follow the orbit) |
| **Accusative** (`В.п.`) | `кого́? что?` | **Direct Object (`O`)**, motion target (`в/на + Acc`) | `в` (into), `на` (onto), `че́рез` (through / across / in $\Delta t$) | `доказа́ть теоре́му` (prove a theorem), `че́рез час` (in an hour) |
| **Instrumental** (`Т.п.`) | `кем? чем?` | **Instrument / Agent (`by/with X`)**, predicate state (`быть / явля́ться + Inst`) | `с` (with), `ме́жду` (between), `над` (above), `под` (under), `пе́ред` (before) | `явля́ться просты́м число́м` (to be a prime number), `ме́жду двумя́ звезда́ми` (between two stars) |
| **Prepositional** (`П.п.`) | `о ком? о чём? где?` | **Static Location (`in/at X`)** or **Topic (`about X`)** | `в` (in), `на` (on/at), `о / об` (about), `при` (given / in the presence of) | `при $n > 0$` (for/given $n > 0$), `в простра́нстве` (in space) |

### 1.2 Noun Ending Lookup Table (Hard / Soft Stems)

| Case | Masc. (`-Ø` / `-й,-ь`) | Neuter (`-о` / `-е` / `-ие`) | Fem. I (`-а` / `-я` / `-ия`) | Fem. III (`-ь`, e.g. `мысль`) | Plural (All Genders) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Nom** | `-Ø` / `-й`, `-ь` | `-о` / `-е` / `-ие` | `-а` / `-я` / `-ия` | `-ь` | `-ы` / `-и` (neut. `-а`/`-я`) |
| **Gen** | **`-а` / `-я`** | **`-а` / `-я`** | **`-ы` / `-и`** | **`-и`** | **`-ов` / `-ев` / `-ей` / `-Ø`** |
| **Dat** | **`-у` / `-ю`** | **`-у` / `-ю`** | **`-е`** (`-ии`) | **`-и`** | **`-ам` / `-ям`** |
| **Acc** | *Inanim:* `= Nom`<br>*Anim:* `= Gen` | `= Nom` | **`-у` / `-ю`** | `= Nom` (`-ь`) | *Inanim:* `= Nom`<br>*Anim:* `= Gen` |
| **Inst** | **`-ом` / `-ем` (`-ём`)** | **`-ом` / `-ем` (`-ём`)** | **`-ой` / `-ей` (`-ёй`)** | **`-ью`** | **`-ами` / `-ями`** |
| **Prep** | **`-е`** (`-ии`, loc. `-у́`) | **`-е`** (`-ии`) | **`-е`** (`-ии`) | **`-и`** | **`-ах` / `-ях`** |

### 1.3 Adjective & Participle Ending Lookup Table

Adjectives and participles are **reliable disambiguators** when a noun ending is ambiguous (e.g., `-и` or `-е`):

| Case | Masc. | Neuter | Feminine | Plural |
| :--- | :--- | :--- | :--- | :--- |
| **Nom** | `-ый` / `-о́й` / `-ий` | `-ое` / `-ее` | `-ая` / `-яя` | `-ые` / `-ие` |
| **Gen** | **`-ого` / `-его`** *(pronounced `-ова/-ева`)* | **`-ого` / `-его`** | **`-ой` / `-ей`** | **`-ых` / `-их`** |
| **Dat** | **`-ому` / `-ему`** | **`-ому` / `-ему`** | **`-ой` / `-ей`** | **`-ым` / `-им`** |
| **Acc** | `= Nom` (inanim) / `= Gen` (anim) | `= Nom` | **`-ую` / `-юю`** | `= Nom` (inanim) / `= Gen` (anim) |
| **Inst** | **`-ым` / `-им`** | **`-ым` / `-им`** | **`-ой` / `-ей`** | **`-ыми` / `-ими`** |
| **Prep** | **`-ом` / `-ем`** | **`-ом` / `-ем`** | **`-ой` / `-ей`** | **`-ых` / `-их`** |

> **Fast Parser Rule for Ambiguity (Syncretism)**:
> 1. **Inanimate Masculine/Neuter/Plural Accusative = Nominative**. If a clause has two Nominative-looking nouns (`Сигна́л даёт ко́смос`), look at (a) verb agreement (`даёт` is 3sg) or (b) semantic animacy/context, or (c) feminine/adjective endings where Accusative (`-у` / `-ую`) is distinct from Nominative (`-а` / `-ая`).
> 2. **Adjective `-ой` / `-ей` in Feminine** covers **Genitive, Dative, Instrumental, and Prepositional**. Let the **preposition** (`из но́вой`, `к но́вой`, `с но́вой`, `в но́вой`) or governing verb disambiguate immediately.

---

## 2. Verbal Aspect & Prefix Combinatorics

Think of Russian verbal aspect as a **state machine distinction**:
- **Imperfective (`НСВ`)** = **Process / Loop / State / Habit** (`while`, `is running`, general fact).
  - Present tense (`чита́ет`), compound future (`бу́дет чита́ть`), ongoing/repeated past (`чита́л`).
- **Perfective (`СВ`)** = **State Transition / Commit / Event** (`commit()`, completed result, bounded step in a sequence).
  - No present tense! Present-style conjugation = **Simple Future** (`прочита́ет` = *will finish reading*); past (`прочита́л` = *read to completion*).

### 2.1 The 10 High-Leverage Verbal Prefixes

When you know a base root verb (e.g. `йти/ходи́ть` *go*, `писа́ть` *write*, `де́лать` *do*, `ста́вить` *put*, `ве́сти` *lead*), prefixes act like **operators** modifying the verb's spatial or aspectual meaning:

| Prefix | Core Spatial / Logical Operator | Examples |
| :--- | :--- | :--- |
| **`в-` (`во-`)** | **In / Input / Insert** (`into`) | `входи́ть/войти́` (enter), `ввести́` (introduce/input, e.g. `введём пере́менную` *let us introduce a variable*) |
| **`вы-`** | **Out / Extract / Exhaustive derivation** *(Note: always stressed in perfective!)* | `выходи́ть/вы́йти` (exit), `вы́вести` (derive/deduce a formula), `вы́числить` (compute) |
| **`при-`** | **Arrival / Attachment / Approximation** | `приходи́ть/прийти́` (arrive), `приба́вить` (add to), `приближе́ние` (approximation) |
| **`у-`** | **Departure / Removal / Stabilization** | `уходи́ть/уйти́` (leave), `убра́ть` (remove), `установи́ть` (establish/install) |
| **`пере-`** | **Across / Re-do / Transform / Map** | `переходи́ть/перейти́` (cross/transition), `переписа́ть` (rewrite), `переме́нная` (variable) |
| **`про-`** | **Through / Past / Complete traversal** | `проходи́ть/пройти́` (pass through), `прочита́ть` (read through), `прове́рить` (verify) |
| **`под-`** | **Under / Approach / Substitute** | `подходи́ть/подойти́` (approach / fit), `подста́вить` (substitute a value into an expression) |
| **`от-`** | **Away from / Detach / Open** | `отходи́ть/отойти́` (step away), `откры́ть` (open / discover), `отве́тить` (answer) |
| **`с-` (`со-`)** | **Together (convergence) OR Down from** | `сходи́ться/сойти́сь` (converge, e.g. `ряд схо́дится` *the series converges*), `совпада́ть` (coincide) |
| **`раз-` (`рас-`)** | **Apart (divergence) / Decompose / Expand** | `расходи́ться` (diverge), `разложи́ть` (decompose/expand into a series), `рассмотре́ть` (examine/consider) |

---

## 3. Bookish Compression: Participles & Verbal Adverbs (Дееприча́стия)

Russian scientific and literary prose compresses relative clauses (`кото́рый...`) and temporal clauses (`когда́...`) into single words. Recognizing four suffixes lets you unpack dense academic sentences effortlessly.

### 3.1 Participles (Verbal Adjectives — decline like normal adjectives)

| Type | Suffixes | Meaning | Unpacked Equivalent (`кото́рый`) |
| :--- | :--- | :--- | :--- |
| **Present Active** | **`-ущ-` / `-ющ-`**, **`-ащ-` / `-ящ-`** | *"...which is currently doing X"* | `летя́щий зонд` = `зонд, кото́рый лети́т` (*the flying probe*)<br>`сле́дующий шаг` = *the following/next step* |
| **Past Active** | **`-вш-`**, **`-ш-`** | *"...which did X"* | `пролете́вший зонд` = `зонд, кото́рый пролете́л` (*the probe that flew past*) |
| **Present Passive** | **`-ем-`**, **`-им-`** | *"...which is being X-ed / X-able"* | `ви́димый свет` = *visible light* (`неви́димый` = *invisible*) |
| **Past Passive** | **`-нн-`**, **`-енн-`**, **`-т-`** *(short: `-н`, `-т`)* | *"...which has been X-ed"* | `дока́занная теоре́ма` = *proven theorem*<br>`закры́тый люк` = *closed hatch* (`люк закры́т` = *the hatch is closed*) |

### 3.2 Verbal Adverbs / Gerunds (Дееприча́стия — invariant, never decline!)

| Type | Suffixes | Meaning | Example in Math / Sci-Fi |
| :--- | :--- | :--- | :--- |
| **Imperfective** (simultaneous) | **`-а` / `-я`** | *"While doing X..."* | `Зная координа́ты, мы найдём орби́ту.` (*Knowing the coordinates, we will find the orbit.*) |
| **Perfective** (prior step) | **`-в` / `-вши` / `-ши`** | *"Having done X..."* | `Подста́вив $x = 0$, полу́чим $y = 1$.` (*Having substituted $x = 0$, we obtain $y = 1$.*) |

---

## 4. The 35 Structural Skeletons of Russian Math, Logic & Philosophy

These multi-word connectors form the control-flow graph of Russian proofs, philosophical arguments, and analytical sci-fi prose:

### 4.1 Definitions, Assumptions & Quantifiers
1. **`Пусть $X$ — ...`** — *Let $X$ be ...* (e.g. `Пусть $p$ — просто́е число́.`)
2. **`Предполо́жим, что ...` / `Допу́стим, что ...`** — *Suppose that ... / Assume that ...*
3. **`Рассмо́трим ...`** — *Let us consider ...*
4. **`По определе́нию`** — *By definition*
5. **`Для любо́го $x$`** / **`Для ка́ждого $x$`** — *For any $x$ / For every $x$ ($\forall x$)*
6. **`Существу́ет тако́й $x$, что ...`** — *There exists an $x$ such that ... ($\exists x$)*
7. **`Существу́ет и еди́нственно`** — *Exists and is unique ($\exists!$)*
8. **`Без ограниче́ния о́бщности`** — *Without loss of generality (WLOG)*
9. **`При усло́вии, что ...`** — *Provided that / Under the condition that ...*
10. **`В зави́симости от (+ Gen)`** — *Depending on ...*

### 4.2 Implication, Equivalence & Proof Steps
11. **`Е́сли ..., то ...`** — *If ..., then ... ($P \Rightarrow Q$)*
12. **`Тогда́ и то́лько тогда́, когда́ ...`** / **`Е́сли и то́лько е́сли ...`** — *If and only if ($\iff$)*
13. **`Необходи́мое и доста́точное усло́вие`** — *Necessary and sufficient condition*
14. **`Отку́да сле́дует, что ...`** / **`Из э́того сле́дует, что ...`** — *From which it follows that ...*
15. **`Сле́довательно, ...`** / **`Зна́чит, ...`** — *Consequently / Therefore, ...*
16. **`С друго́й стороны́, ...`** — *On the other hand, ...*
17. **`В ча́стности, ...`** — *In particular, ...*
18. **`Таким о́бразом, ...`** — *Thus / In this way, ...*
19. **`Доказа́тельство от проти́вного`** — *Proof by contradiction*
20. **`Получа́ем противоре́чие.`** — *We obtain a contradiction.*
21. **`Что и тре́бовалось доказа́ть (Ч.Т.Д.)`** — *Quod erat demonstrandum (Q.E.D.)*

### 4.3 Reference, Identity & Philosophical Framing
22. **`То, что ...`** — *The fact that ...* (nominalizer: turns an entire clause into a neuter noun phrase, e.g. `То, что ряд схо́дится, очеви́дно.`)
23. **`Тот, кто ...` / `То, что ...`** — *He who ... / That which ...*
24. **`Де́ло в том, что ...`** — *The thing/point is that ...*
25. **`То есть (т. е.)`** — *That is (i.e.)*
26. **`Ина́че говоря́, ...`** — *In other words, ...*
27. **`Как пра́вило, ...`** — *As a rule, ...*
28. **`По кра́йней ме́ре`** — *At least*
29. **`Несмотря́ на то, что ...`** — *Despite the fact that ...*
30. **`В то вре́мя как ...`** — *Whereas / While ...*
31. **`Чем [comp], тем [comp]`** — *The more X, the more Y* (e.g. `Чем да́льше зонд, тем ти́ше сигна́л.`)
32. **`Не то́лько ..., но и ...`** — *Not only ..., but also ...*
33. **`Ли́бо ..., ли́бо ...`** — *Either ..., or ... (exclusive OR)*
34. **`Само́ по себе́`** — *In itself (per se / Ding an sich)*
35. **`Име́ть смысл`** — *To make sense / have meaning* (`Э́то не име́ет смы́сла` = *This makes no sense.*)
