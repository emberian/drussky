# Deputy author report — Матема́тика

Built on branch `deputy-codex-math`, 2026-11-08. Author: Codex.

## Delivered

- `apple/Drussky/Resources/math.json`: **24 original CC0 readings**, 1752 Russian word tokens, 61–107 per passage; 24 full English translations; 997 contextual surface-form gloss entries; **72 questions** (24 each gist/detail/inference), 84 reading frames and 40 notation readings. All passage Russian, titles, frames and Russian quotations are stress-marked. Gloss keys intentionally omit stress.
- `apple/scripts/validate_math.py`: Python 3, standard library only; reuses the base Library checker, checks nested metadata, stress syntax plus selected audited mobile stresses, page order, answer indices, translation paragraph count, lengths and exact-form gloss coverage. It counts actual meaning-presentation events, not the whole future course lexicon. Hyphenated words match the reader's whole-word gloss lookup.
- `apple/scripts/test_validate_math.py`: **9 passing tests**, including corruption cases for missing early glosses, unknown/malformed nested fields, answer indices including booleans, stress placement, notation versus markup, hyphenated lookup and premature vocabulary credit from distractors.
- `apple/scripts/validate_library.py`: accepts an explicit collection path, recognizes the Optional teaching fields, and allows mathematical inequality/absolute-value notation while still rejecting HTML. Its original no-argument validation remains clean.
- `apple/COURSE_SCHEMA.md`: extension types, coverage policy, notation conventions, ownership and current integration limits.
- `research/MATH_FIRST.md`: a one-page proposal for an early guided mathematical strand and the exact units/function words to move forward.
- `research/artifacts/math-spoken.json`: complete machine-readable audio inventory; the literal new strings are also listed below. **No audio files or manifests were modified.**

The first six rungs stay with present-tense/nominal definitions and nominative, genitive and prepositional Russian. Symbolic variables do not receive visible case endings; e.g. the mapping reading keeps A and B as notation. The next eight introduce case government, proof aspect and relative clauses. Rungs 15–17 form a topology definition–theorem–proof triplet; 18–20 do the same for continuity and sums. Rungs 21–24 cover finite probability, independence, classical quantifier negation and the three information approaches. `after_page` recommends available support, not a locked gate. The A1–B2 labels describe supported mathematical readings, not unassisted CEFR proficiency.

## What works and what the integration owner must do

The **actual, unchanged** `Model/Library.swift` decodes this file successfully via `JSONDecoder`. I compiled it with a small standalone harness and checked all 24 IDs and topics. Unknown Optional metadata is ignored by synthesized Codable; it is not rejected.

However, the premise that a new file appears with no application change is false on this branch. `Library.load()` discovers only `library.json` and `PrivatePack/library.json`; `Library.Text` has no fields for authored translations/glosses/questions/frames. `PassageExercise` can already show translation and contextual glosses once supplied. Root integration should discover/merge the shelf, retain the new Optional fields and build the authored reading directly, then expose frames/questions. Simply appending records would expose the prose but still discard this teaching support. No Swift application code or notebook format changed here.

I ran `cd apple && xcodegen generate` as required after adding the resource. The generated project now includes `math.json`. **The project was already modified on arrival** (worktree-specific group paths); its generated diff includes that pre-existing churn. It has not been hand-edited or staged as part of this authoring handoff. Regenerate the project at integration.

## Checks

`python3 apple/scripts/validate_math.py`:

```text
Drussky/Resources/math.json: 24 texts {'A1': 4, 'A2': 7, 'B1': 8, 'B2': 5}, 0 narrated, 0 sentences, 0 with audio, 0 speakers
Rung after_page words types course_tokens course% +prior% new_types glosses missing
01 u7-p4          64    37            18   28.1    28.1        27      27       0
02 u8-p3          65    38            16   24.6    53.8        20      32       0
03 u10-p4         64    38            18   28.1    57.8        19      30       0
04 u18-p2         64    39            19   29.7    60.9        18      32       0
05 u18-p2         71    48            19   26.8    60.6        21      39       0
06 u22-p2         67    47            19   28.4    58.2        24      36       0
07 u24-p3         64    55            23   35.9    53.1        28      41       0
08 u25-p2         72    49            36   50.0    69.4        16      26       0
09 u25-p4         81    61            29   35.8    45.7        32      42       0
10 u25-p4         82    61            38   46.3    63.4        24      40       0
11 u25-p4         64    53            26   40.6    59.4        22      38       0
12 u27-p5         93    66            23   24.7    51.6        33      50       0
13 u27-p5         87    77            19   21.8    41.4        43      61       0
14 u27-p5         61    47            20   32.8    63.9        19      35       0
15 u28-p2         68    52            20   29.4    47.1        27      43       0
16 u28-p2         79    68            24   30.4    57.0        29      50       0
17 u28-p4         69    57            27   39.1    75.4        17      40       0
18 u29-p1         69    55            29   42.0    65.2        21      41       0
19 u29-p2         74    66            30   40.5    62.2        28      48       0
20 u29-p2         65    55            25   38.5    63.1        23      38       0
21 u29-p3         78    67            26   33.3    61.5        24      48       0
22 u29-p3         62    50            12   19.4    48.4        28      41       0
23 u29-p4         82    54            30   36.6    67.1        20      41       0
24 u29-p4        107    91            25   23.4    36.4        61      78       0
course% = exact forms presented with meaning by after_page; +prior% adds earlier shelf encounters, not mastery.
new_types = passage forms absent from both; glosses cover all learner-facing Russian, including frames and notation.
clean
```

`python3 apple/scripts/validate_library.py apple/Drussky/Resources/math.json`:

```text
Drussky/Resources/math.json: 24 texts {'A1': 4, 'A2': 7, 'B1': 8, 'B2': 5}, 0 narrated, 0 sentences, 0 with audio, 0 speakers
clean
```

Existing Library regression (`python3 apple/scripts/validate_library.py`):

```text
Drussky/Resources/library.json: 48 texts {'A1': 7, 'A2': 10, 'B1': 14, 'B2': 9, 'C1': 8}, 7 narrated, 242 sentences, 242 with audio, 5 speakers
clean
```

`python3 -m unittest discover -s apple/scripts -p 'test_validate_math.py'`:

```text
.........
----------------------------------------------------------------------
Ran 9 tests in 2.744s

OK
```

Standalone Swift decoder:

```text
PASS: unmodified Library.swift decodes all 24 texts and ignores authored-reading metadata
```

Harness command: `swiftc -module-cache-path /tmp/math-swift-module-cache apple/Drussky/Model/Library.swift /tmp/math_decode.swift -o /tmp/math_decode`, then `/tmp/math_decode apple/Drussky/Resources/math.json`. No Swift diagnostics. `git diff --check` passed.

**Simulator suite and screenshots: blocked, not passed.** `xcrun simctl create "Drussky Codex Math" …` failed because this sandbox cannot connect to CoreSimulatorService (log access denied; service connection refused). An `xcodebuild test` attempt scoped to DrusskyTests and LearningCoreTests with separate `.build/codex-math` derived data failed at destination resolution; no tests or app compilation ran. No simulator was created, no physical iPad was touched, and there are no screenshots. The correct integration rerun skips `DrusskyTests/DrusskyTests/testSpeechRecognizerHearsRecordedRussian` and `DrusskyTests/OnDeviceTutorTests`.

The validator establishes structure and coverage, not idiomaticity or theorem truth by itself. I manually reviewed agreement, case, aspect, stresses, translation and the mathematical hypotheses. In particular: proper/non-strict inclusion are separate; Euclid does not assert its constructed number is prime; the square-root proof requires a reduced fraction; paths need not be unique; open unions include the empty family; delta cannot depend on the tested x; zero-probability events need not be empty; conditional probability requires a positive denominator; the negation argument is explicitly classical; complexity invariance uses an optimal universal description language.

**Git handoff:** staging the eight named deliverables was attempted and denied: `Unable to create /Users/ember/dev/rufren/.git/worktrees/deputy-codex-math/index.lock: Operation not permitted`. The shared Git metadata is outside this session's writable roots. No commit or push was possible; all deliverables remain in this worktree. No permission escalation is available in this session.

## Evidence and editorial judgments

The requested vision documents were absent from this worktree; I read their counterparts in `/Users/ember/dev/rufren/research/`, specifically Gemini §5.4/§8 and Codex “Reading begins now”. No other worktree was edited.

For the information text I consulted [Kolmogorov's primary paper record](https://www.mathnet.ru/eng/ppi68), its [English translation](https://www.karlin.mff.cuni.cz/~krajicek/kolmogorov65.pdf), and [Vereshchagin's introduction](https://arxiv.org/abs/0809.2754). The Russian text and translation are original explanations, not copied passages. Reference metadata separates conceptual sources from authorship. Lexical spot checks included [полу́чим](https://gramota.ru/meta/poluchit), [Дирихле́ in Академос](https://orfo.ruslang.ru/abc/part/de?end=37016&start=36716), and [появля́ется](https://gramota.ru/poisk?mode=slovari&query=%D0%BF%D0%BE%D1%8F%D0%B2%D1%8F%D1%82%D1%81%D1%8F&simple=0).

I agree with using familiar mathematics to make case and logical scope legible early. I disagree with treating lexical overlap as comprehension, grouping all short predicates as participles (`равно́` is a short adjective), or promising a fixed six-month outcome from an unsupported cognate percentage. This shelf remains intensive guided reading: exact course coverage is modest even where the idea is familiar. A learner can understand a proof and still misparse its Russian. Conversely, a lower coverage percentage need not forbid reading with cheap lookup.

Next: integrate the authored fields into the reader; evaluate three readings with Ember, including an unseen altered hypothesis; use the actual analysis textbook to choose the next 20 high-value words; then commission native mathematical-Russian read-through and narration. Add short clause-expansion exercises only where the participle questions reveal a real parsing problem. Preserve normal Russian interaction and easy reading alongside the math strand.

## Audio handoff — every new spoken string

Snapshot: 1004 distinct candidate strings; **840 lack a current key or need stress disambiguation**; 164 have an existing key as a possible reuse candidate. Existing-key presence is not a claim that the clip exists or has the right pronunciation. The inventory retains the stress-marked form, source locations, existing clip, key and notation-expansion flag. No synthesis was performed.

Audio index SHA-256 at inventory time: `0726e17df4257ced42050ad0e3dc77fa1cfbd3805903cc9c4814d9dfe5c422a3`. Re-read the live audio index and sources before any write. Full passages and notation frames retain symbols/Latin variables by design: expand their readings before sending them to Silero. The current reader also splits on whitespace, so `display-token` entries include formula fragments: group and expand these rather than synthesizing them verbatim. The current audio key strips stress; these forms collide and need distinct contextual or stress-aware clips:

- `числа`: чи́сла / числа́
- `стороны`: сто́роны / стороны́

The following is the exhaustive literal list of new candidates (including individual tappable words). Existing candidates and every source location are in `research/artifacts/math-spoken.json`. Duplicate exact strings appear only once.

### 001 · display-token · math-01-sets

```text
A
```

### 002 · display-token · math-01-sets

```text
1
```

### 003 · display-token · math-01-sets

```text
2
```

### 004 · display-token · math-01-sets

```text
3
```

### 005 · display-token · math-01-sets

```text
B
```

### 006 · display-token · math-01-sets

```text
∅
```

### 007 · passage · math-01-sets

```text
Что тако́е мно́жество? Вот мно́жество A. В нём есть элеме́нты: 1, 2 и 3. Э́то чи́сла. В мно́жестве B есть то́лько 1 и 2. Э́то то́же мно́жество чи́сел. В A три элеме́нта, а в B два элеме́нта. В мно́жестве нет поря́дка: 1, 2, 3 и 3, 2, 1 — здесь одно́ и то же мно́жество. В нём нет «пе́рвого» элеме́нта. В матема́тике ва́жно, что есть в мно́жестве, а не где э́то на страни́це. Мно́жество без элеме́нтов — пусто́е мно́жество. Его́ знак — ∅.
```

### 008 · how_read · math-01-sets/how_read/1

```text
в мно́жестве
```

### 009 · how_read · math-01-sets/how_read/2

```text
мно́жество чи́сел
```

### 010 · how_read · math-01-sets/how_read/3

```text
без элеме́нтов
```

### 011 · word · math-01-sets

```text
нём
```

### 012 · word · math-01-sets

```text
элеме́нты
```

### 013 · word · math-01-sets

```text
чи́сла
```

### 014 · word · math-01-sets

```text
мно́жестве
```

### 015 · word · math-01-sets

```text
чи́сел
```

### 016 · word · math-01-sets

```text
элеме́нта
```

### 017 · word · math-01-sets

```text
поря́дка
```

### 018 · word · math-01-sets

```text
пе́рвого
```

### 019 · word · math-01-sets

```text
матема́тике
```

### 020 · word · math-01-sets

```text
страни́це
```

### 021 · word · math-01-sets

```text
элеме́нтов
```

### 022 · word · math-01-sets

```text
пусто́е
```

### 023 · word · math-01-sets

```text
его́
```

### 024 · display-token · math-02-elements

```text
4
```

### 025 · display-token · math-02-elements

```text
6
```

### 026 · display-token · math-02-elements

```text
∈
```

### 027 · title · math-02-elements

```text
Элеме́нт и мно́жество
```

### 028 · passage · math-02-elements

```text
Элеме́нт мно́жества — э́то объе́кт в э́том мно́жестве. Здесь A — мно́жество чи́сел 2, 4 и 6. Число́ 2 — элеме́нт A. За́пись 2 ∈ A — э́то кра́ткая фо́рма той же мы́сли. Число́ 3 — не элеме́нт A: в A нет числа́ 3. Здесь ва́жен не разме́р числа́, а соста́в мно́жества. Число́ 2 и мно́жество из одного́ элеме́нта 2 — не одно́ и то же. В пе́рвом слу́чае э́то число́, во второ́м — мно́жество. У мно́жества есть элеме́нт, а у числа́ в э́том описа́нии нет элеме́нтов.
```

### 029 · how_read · math-02-elements/how_read/1

```text
элеме́нт мно́жества
```

### 030 · how_read · math-02-elements/how_read/2

```text
нет числа́
```

### 031 · how_read · math-02-elements/how_read/3

```text
в пе́рвом слу́чае
```

### 032 · word · math-02-elements

```text
мно́жества
```

### 033 · word · math-02-elements

```text
э́том
```

### 034 · word · math-02-elements

```text
кра́ткая
```

### 035 · word · math-02-elements

```text
той
```

### 036 · word · math-02-elements

```text
мы́сли
```

### 037 · word · math-02-elements

```text
числа́
```

### 038 · word · math-02-elements

```text
ва́жен
```

### 039 · word · math-02-elements

```text
соста́в
```

### 040 · word · math-02-elements

```text
одного́
```

### 041 · word · math-02-elements

```text
пе́рвом
```

### 042 · word · math-02-elements

```text
слу́чае
```

### 043 · word · math-02-elements

```text
во
```

### 044 · word · math-02-elements

```text
второ́м
```

### 045 · word · math-02-elements

```text
описа́нии
```

### 046 · display-token · math-03-subsets

```text
⊂
```

### 047 · display-token · math-03-subsets

```text
≠
```

### 048 · display-token · math-03-subsets

```text
⊆
```

### 049 · title · math-03-subsets

```text
Подмно́жество
```

### 050 · passage · math-03-subsets

```text
A — подмно́жество B, е́сли ка́ждый элеме́нт A — э́то и элеме́нт B. Здесь A — мно́жество чи́сел 1 и 2, а B — мно́жество чи́сел 1, 2 и 3. Поэ́тому A — подмно́жество B. Но B — не подмно́жество A: в B есть 3, а в A нет числа́ 3. Для пусто́го мно́жества нет тако́го приме́ра: в нём вообще́ нет элеме́нтов. Поэ́тому пусто́е мно́жество — подмно́жество ка́ждого мно́жества. Здесь A ⊂ B — стро́гое включе́ние: A — подмно́жество B, но A ≠ B. Для нестро́гого включе́ния есть знак ⊆. Само́ мно́жество то́же есть среди́ его́ подмно́жеств. Здесь нет стро́гого включе́ния.
```

### 051 · how_read · math-03-subsets/how_read/1

```text
ка́ждый элеме́нт
```

### 052 · how_read · math-03-subsets/how_read/2

```text
для ка́ждого мно́жества
```

### 053 · notation · math-03-subsets/notation/1

```text
A — стро́гое подмно́жество B
```

### 054 · notation · math-03-subsets/notation/2

```text
A — подмно́жество B
```

### 055 · word · math-03-subsets

```text
подмно́жество
```

### 056 · word · math-03-subsets

```text
пусто́го
```

### 057 · word · math-03-subsets

```text
тако́го
```

### 058 · word · math-03-subsets

```text
приме́ра
```

### 059 · word · math-03-subsets

```text
вообще́
```

### 060 · word · math-03-subsets

```text
ка́ждого
```

### 061 · word · math-03-subsets

```text
стро́гое
```

### 062 · word · math-03-subsets

```text
включе́ние
```

### 063 · word · math-03-subsets

```text
нестро́гого
```

### 064 · word · math-03-subsets

```text
включе́ния
```

### 065 · word · math-03-subsets

```text
само́
```

### 066 · word · math-03-subsets

```text
среди́
```

### 067 · word · math-03-subsets

```text
подмно́жеств
```

### 068 · word · math-03-subsets

```text
стро́гого
```

### 069 · display-token · math-04-functions

```text
f
```

### 070 · display-token · math-04-functions

```text
→
```

### 071 · display-token · math-04-functions

```text
x
```

### 072 · display-token · math-04-functions

```text
f(x
```

### 073 · display-token · math-04-functions

```text
f(1
```

### 074 · display-token · math-04-functions

```text
=
```

### 075 · display-token · math-04-functions

```text
f(2
```

### 076 · passage · math-04-functions

```text
Фу́нкция f : A → B — э́то пра́вило: для ка́ждого элеме́нта x из A есть ро́вно одно́ значе́ние f(x) в B. Здесь A — о́бласть определе́ния, а B — мно́жество возмо́жных значе́ний. Все ли элеме́нты B есть среди́ значе́ний f? Не обяза́тельно. Наприме́р, A — мно́жество чи́сел 1 и 2, B — мно́жество чи́сел 1, 2 и 3. Е́сли f(1) = 2 и f(2) = 2, э́то фу́нкция. Для ка́ждого аргуме́нта есть одно́ значе́ние. Два ра́зных аргуме́нта и одно́ о́бщее значе́ние — здесь нет пробле́мы. Но два значе́ния для одного́ аргуме́нта — э́то уже́ не фу́нкция.
```

### 077 · how_read · math-04-functions/how_read/1

```text
для ка́ждого элеме́нта
```

### 078 · how_read · math-04-functions/how_read/2

```text
ро́вно одно́ значе́ние
```

### 079 · how_read · math-04-functions/how_read/3

```text
о́бласть определе́ния
```

### 080 · notation · math-04-functions/notation/1

```text
f отобража́ет A в B
```

### 081 · notation · math-04-functions/notation/2

```text
f от x
```

### 082 · word · math-04-functions

```text
пра́вило
```

### 083 · word · math-04-functions

```text
ро́вно
```

### 084 · word · math-04-functions

```text
о́бласть
```

### 085 · word · math-04-functions

```text
определе́ния
```

### 086 · word · math-04-functions

```text
возмо́жных
```

### 087 · word · math-04-functions

```text
значе́ний
```

### 088 · word · math-04-functions

```text
все
```

### 089 · word · math-04-functions

```text
обяза́тельно
```

### 090 · word · math-04-functions

```text
аргуме́нта
```

### 091 · word · math-04-functions

```text
ра́зных
```

### 092 · word · math-04-functions

```text
о́бщее
```

### 093 · word · math-04-functions

```text
пробле́мы
```

### 094 · word · math-04-functions

```text
значе́ния
```

### 095 · word · math-04-functions

```text
отобража́ет
```

### 096 · display-token · math-05-injections

```text
f(y
```

### 097 · display-token · math-05-injections

```text
y
```

### 098 · display-token · math-05-injections

```text
+
```

### 099 · display-token · math-05-injections

```text
g(x
```

### 100 · title · math-05-injections

```text
Инъе́кция
```

### 101 · passage · math-05-injections

```text
Инъе́кция — э́то фу́нкция без совпаде́ний значе́ний у ра́зных аргуме́нтов. Ина́че: е́сли f(x) = f(y), то x = y. Здесь ва́жен поря́док усло́вия и вы́вода. Для любо́й фу́нкции из x = y сле́дует f(x) = f(y). Для инъе́кции есть и обра́тное утвержде́ние. Наприме́р, у фу́нкции f(x) = x + 1 на мно́жестве натура́льных чи́сел нет одина́ковых значе́ний для ра́зных аргуме́нтов. Э́то инъе́кция. А у постоя́нной фу́нкции g(x) = 1 есть одно́ и то же значе́ние для 1 и 2. Э́то не инъе́кция. В определе́нии инъе́кции нет тре́бования о нали́чии всех элеме́нтов B среди́ значе́ний фу́нкции.
```

### 102 · how_read · math-05-injections/how_read/1

```text
е́сли … то …
```

### 103 · how_read · math-05-injections/how_read/2

```text
из … сле́дует …
```

### 104 · how_read · math-05-injections/how_read/3

```text
у ра́зных аргуме́нтов
```

### 105 · notation · math-05-injections/notation/1

```text
x ра́вен y
```

### 106 · word · math-05-injections

```text
инъе́кция
```

### 107 · word · math-05-injections

```text
совпаде́ний
```

### 108 · word · math-05-injections

```text
аргуме́нтов
```

### 109 · word · math-05-injections

```text
усло́вия
```

### 110 · word · math-05-injections

```text
вы́вода
```

### 111 · word · math-05-injections

```text
инъе́кции
```

### 112 · word · math-05-injections

```text
обра́тное
```

### 113 · word · math-05-injections

```text
утвержде́ние
```

### 114 · word · math-05-injections

```text
натура́льных
```

### 115 · word · math-05-injections

```text
одина́ковых
```

### 116 · word · math-05-injections

```text
постоя́нной
```

### 117 · word · math-05-injections

```text
определе́нии
```

### 118 · word · math-05-injections

```text
тре́бования
```

### 119 · word · math-05-injections

```text
нали́чии
```

### 120 · word · math-05-injections

```text
ра́вен
```

### 121 · display-token · math-06-equivalence

```text
R
```

### 122 · display-token · math-06-equivalence

```text
z
```

### 123 · title · math-06-equivalence

```text
Отноше́ние эквивале́нтности
```

### 124 · passage · math-06-equivalence

```text
Отноше́ние R на мно́жестве A — э́то пра́вило для пар элеме́нтов. Для ка́ждой па́ры есть отве́т: да и́ли нет. За́пись x R y — э́то «да» для па́ры x, y. Отноше́ние эквивале́нтности — рефлекси́вное, симметри́чное и транзити́вное отноше́ние. Рефлекси́вность: для ка́ждого x есть x R x. Симметри́чность: е́сли x R y, то y R x. Транзити́вность: е́сли x R y и y R z, то x R z. Приме́р на мно́жестве це́лых чи́сел: x R y тогда́ и то́лько тогда́, когда́ у x и y одина́ковая чётность. Здесь два кла́сса: чётные чи́сла и нечётные чи́сла. В ка́ждом кла́ссе все чи́сла эквивале́нтны.
```

### 125 · how_read · math-06-equivalence/how_read/1

```text
для ка́ждой па́ры
```

### 126 · how_read · math-06-equivalence/how_read/3

```text
в ка́ждом кла́ссе
```

### 127 · notation · math-06-equivalence/notation/1

```text
x и y эквивале́нтны
```

### 128 · word · math-06-equivalence

```text
отноше́ние
```

### 129 · word · math-06-equivalence

```text
эквивале́нтности
```

### 130 · word · math-06-equivalence

```text
ка́ждой
```

### 131 · word · math-06-equivalence

```text
па́ры
```

### 132 · word · math-06-equivalence

```text
рефлекси́вное
```

### 133 · word · math-06-equivalence

```text
симметри́чное
```

### 134 · word · math-06-equivalence

```text
транзити́вное
```

### 135 · word · math-06-equivalence

```text
рефлекси́вность
```

### 136 · word · math-06-equivalence

```text
симметри́чность
```

### 137 · word · math-06-equivalence

```text
транзити́вность
```

### 138 · word · math-06-equivalence

```text
це́лых
```

### 139 · word · math-06-equivalence

```text
одина́ковая
```

### 140 · word · math-06-equivalence

```text
чётность
```

### 141 · word · math-06-equivalence

```text
кла́сса
```

### 142 · word · math-06-equivalence

```text
чётные
```

### 143 · word · math-06-equivalence

```text
нечётные
```

### 144 · word · math-06-equivalence

```text
ка́ждом
```

### 145 · word · math-06-equivalence

```text
кла́ссе
```

### 146 · word · math-06-equivalence

```text
эквивале́нтны
```

### 147 · title · math-07-identity

```text
Тожде́ственная фу́нкция
```

### 148 · passage · math-07-identity

```text
Пусть A — мно́жество. Для ка́ждого x ∈ A поло́жим f(x) = x. Э́то пра́вило явля́ется фу́нкцией из A в A: ка́ждому элеме́нту A соотве́тствует ро́вно одно́ значе́ние, и оно́ принадлежи́т мно́жеству A. Така́я фу́нкция называ́ется тожде́ственной. Утвержде́ние: она́ явля́ется инъе́кцией.
Доказа́тельство. Пусть f(x) = f(y). По определе́нию f име́ем f(x) = x и f(y) = y. Сле́довательно, x = y. Э́то и есть усло́вие инъекти́вности. Здесь мы не счита́ем элеме́нты A. Мно́жество мо́жет быть коне́чным, бесконе́чным и да́же пусты́м: во всех слу́чаях одно́ и то же рассужде́ние остаётся ве́рным.
```

### 149 · how_read · math-07-identity/how_read/1

```text
принадлежи́т мно́жеству
```

### 150 · how_read · math-07-identity/how_read/2

```text
явля́ется фу́нкцией
```

### 151 · how_read · math-07-identity/how_read/3

```text
пусть … поло́жим …
```

### 152 · how_read · math-07-identity/how_read/4

```text
по определе́нию
```

### 153 · notation · math-07-identity/notation/1

```text
x принадлежи́т мно́жеству A
```

### 154 · word · math-07-identity

```text
тожде́ственная
```

### 155 · word · math-07-identity

```text
поло́жим
```

### 156 · word · math-07-identity

```text
фу́нкцией
```

### 157 · word · math-07-identity

```text
ка́ждому
```

### 158 · word · math-07-identity

```text
элеме́нту
```

### 159 · word · math-07-identity

```text
соотве́тствует
```

### 160 · word · math-07-identity

```text
така́я
```

### 161 · word · math-07-identity

```text
тожде́ственной
```

### 162 · word · math-07-identity

```text
инъе́кцией
```

### 163 · word · math-07-identity

```text
определе́нию
```

### 164 · word · math-07-identity

```text
име́ем
```

### 165 · word · math-07-identity

```text
усло́вие
```

### 166 · word · math-07-identity

```text
инъекти́вности
```

### 167 · word · math-07-identity

```text
счита́ем
```

### 168 · word · math-07-identity

```text
коне́чным
```

### 169 · word · math-07-identity

```text
бесконе́чным
```

### 170 · word · math-07-identity

```text
пусты́м
```

### 171 · word · math-07-identity

```text
слу́чаях
```

### 172 · word · math-07-identity

```text
рассужде́ние
```

### 173 · word · math-07-identity

```text
остаётся
```

### 174 · word · math-07-identity

```text
ве́рным
```

### 175 · word · math-07-identity

```text
принадлежа́ть
```

### 176 · word · math-07-identity

```text
явля́ться
```

### 177 · display-token · math-08-even-sum

```text
a
```

### 178 · display-token · math-08-even-sum

```text
b
```

### 179 · display-token · math-08-even-sum

```text
k
```

### 180 · display-token · math-08-even-sum

```text
m
```

### 181 · display-token · math-08-even-sum

```text
2k
```

### 182 · display-token · math-08-even-sum

```text
2m
```

### 183 · display-token · math-08-even-sum

```text
2(k
```

### 184 · title · math-08-even-sum

```text
Су́мма чётных чи́сел
```

### 185 · passage · math-08-even-sum

```text
Утвержде́ние. Су́мма двух чётных це́лых чи́сел явля́ется чётным число́м. Число́ называ́ется чётным, е́сли оно́ равно́ удво́енному це́лому числу́. Здесь ва́жно сло́во «це́лому»: полови́на любо́го веще́ственного числа́ существу́ет, но не всегда́ явля́ется це́лым число́м.
Доказа́тельство. Пусть a и b — чётные це́лые чи́сла. Тогда́ существу́ют це́лые чи́сла k и m, для кото́рых a = 2k и b = 2m. Мы записа́ли a + b = 2(k + m). Су́мма k + m то́же явля́ется це́лым число́м. Поэ́тому a + b — чётное число́. Мы доказа́ли утвержде́ние для любы́х двух чётных це́лых чи́сел, а не то́лько для одного́ приме́ра.
```

### 186 · how_read · math-08-even-sum/how_read/1

```text
равно́ це́лому числу́
```

### 187 · how_read · math-08-even-sum/how_read/2

```text
для кото́рых
```

### 188 · how_read · math-08-even-sum/how_read/3

```text
мы записа́ли … мы доказа́ли …
```

### 189 · notation · math-08-even-sum/notation/1

```text
a плюс b
```

### 190 · notation · math-08-even-sum/notation/2

```text
два k
```

### 191 · word · math-08-even-sum

```text
чётных
```

### 192 · word · math-08-even-sum

```text
двух
```

### 193 · word · math-08-even-sum

```text
число́м
```

### 194 · word · math-08-even-sum

```text
удво́енному
```

### 195 · word · math-08-even-sum

```text
це́лому
```

### 196 · word · math-08-even-sum

```text
числу́
```

### 197 · word · math-08-even-sum

```text
полови́на
```

### 198 · word · math-08-even-sum

```text
любо́го
```

### 199 · word · math-08-even-sum

```text
веще́ственного
```

### 200 · word · math-08-even-sum

```text
це́лым
```

### 201 · word · math-08-even-sum

```text
це́лые
```

### 202 · word · math-08-even-sum

```text
существу́ют
```

### 203 · word · math-08-even-sum

```text
кото́рых
```

### 204 · word · math-08-even-sum

```text
записа́ли
```

### 205 · word · math-08-even-sum

```text
доказа́ли
```

### 206 · word · math-08-even-sum

```text
любы́х
```

### 207 · display-token · math-09-pigeonhole

```text
n
```

### 208 · title · math-09-pigeonhole

```text
При́нцип Дирихле́
```

### 209 · passage · math-09-pigeonhole

```text
Утвержде́ние. Е́сли n + 1 предме́тов размести́ть в n я́щиках, где n — натура́льное число́, то хотя́ бы в одно́м я́щике бу́дет не ме́нее двух предме́тов. Не тре́буется, что́бы ка́ждый я́щик был непусты́м. Предме́ты мо́гут лежа́ть как уго́дно.
Доказа́тельство. Предполо́жим проти́вное: в ка́ждом я́щике не бо́лее одного́ предме́та. Тогда́ во всех n я́щиках вме́сте не бо́лее n предме́тов. Но мы размести́ли n + 1 предме́тов. Получи́ли противоре́чие. Зна́чит, предположе́ние неве́рно. Хотя́ бы оди́н я́щик соде́ржит два предме́та и́ли бо́льше. Здесь «хотя́ бы оди́н» не означа́ет «ро́вно оди́н»: таки́х я́щиков мо́жет быть не́сколько.
```

### 210 · how_read · math-09-pigeonhole/how_read/1

```text
хотя́ бы оди́н
```

### 211 · how_read · math-09-pigeonhole/how_read/2

```text
не ме́нее двух
```

### 212 · how_read · math-09-pigeonhole/how_read/3

```text
предполо́жим проти́вное
```

### 213 · how_read · math-09-pigeonhole/how_read/4

```text
получи́ли противоре́чие
```

### 214 · notation · math-09-pigeonhole/notation/1

```text
n плюс оди́н
```

### 215 · word · math-09-pigeonhole

```text
дирихле́
```

### 216 · word · math-09-pigeonhole

```text
предме́тов
```

### 217 · word · math-09-pigeonhole

```text
размести́ть
```

### 218 · word · math-09-pigeonhole

```text
я́щиках
```

### 219 · word · math-09-pigeonhole

```text
натура́льное
```

### 220 · word · math-09-pigeonhole

```text
одно́м
```

### 221 · word · math-09-pigeonhole

```text
я́щике
```

### 222 · word · math-09-pigeonhole

```text
ме́нее
```

### 223 · word · math-09-pigeonhole

```text
тре́буется
```

### 224 · word · math-09-pigeonhole

```text
я́щик
```

### 225 · word · math-09-pigeonhole

```text
непусты́м
```

### 226 · word · math-09-pigeonhole

```text
предме́ты
```

### 227 · word · math-09-pigeonhole

```text
мо́гут
```

### 228 · word · math-09-pigeonhole

```text
лежа́ть
```

### 229 · word · math-09-pigeonhole

```text
уго́дно
```

### 230 · word · math-09-pigeonhole

```text
проти́вное
```

### 231 · word · math-09-pigeonhole

```text
предме́та
```

### 232 · word · math-09-pigeonhole

```text
размести́ли
```

### 233 · word · math-09-pigeonhole

```text
получи́ли
```

### 234 · word · math-09-pigeonhole

```text
предположе́ние
```

### 235 · word · math-09-pigeonhole

```text
неве́рно
```

### 236 · word · math-09-pigeonhole

```text
соде́ржит
```

### 237 · word · math-09-pigeonhole

```text
означа́ет
```

### 238 · word · math-09-pigeonhole

```text
таки́х
```

### 239 · word · math-09-pigeonhole

```text
я́щиков
```

### 240 · display-token · math-10-primes

```text
p₁
```

### 241 · display-token · math-10-primes

```text
pₙ
```

### 242 · display-token · math-10-primes

```text
N
```

### 243 · display-token · math-10-primes

```text
p₁⋯pₙ
```

### 244 · display-token · math-10-primes

```text
q
```

### 245 · passage · math-10-primes

```text
Просто́е число́ — натура́льное число́, бо́льшее едини́цы, у кото́рого то́лько два натура́льных дели́теля: едини́ца и само́ число́. Утвержде́ние: просты́х чи́сел бесконе́чно мно́го.
Доказа́тельство. Предполо́жим, что просты́х чи́сел коне́чно мно́го. Пусть p₁, …, pₙ — все просты́е чи́сла. Возьмём N = p₁⋯pₙ + 1. Число́ N бо́льше едини́цы, поэ́тому у него́ есть просто́й дели́тель q. Но ни одно́ число́ из спи́ска не де́лит N: оста́ток от деле́ния N на ка́ждое из них ра́вен едини́це. Сле́довательно, q нет в спи́ске. Получи́ли противоре́чие. Зна́чит, просты́х чи́сел бесконе́чно мно́го. Мы не утвержда́ем, что само́ N обяза́тельно просто́е: ну́жен то́лько его́ просто́й дели́тель.
```

### 246 · how_read · math-10-primes/how_read/1

```text
у кото́рого
```

### 247 · how_read · math-10-primes/how_read/2

```text
ра́вен едини́це
```

### 248 · how_read · math-10-primes/how_read/3

```text
ни одно́ … не …
```

### 249 · how_read · math-10-primes/how_read/4

```text
возьмём …
```

### 250 · notation · math-10-primes/notation/1

```text
произведе́ние всех чи́сел из спи́ска
```

### 251 · word · math-10-primes

```text
просты́х
```

### 252 · word · math-10-primes

```text
просто́е
```

### 253 · word · math-10-primes

```text
едини́цы
```

### 254 · word · math-10-primes

```text
кото́рого
```

### 255 · word · math-10-primes

```text
дели́теля
```

### 256 · word · math-10-primes

```text
коне́чно
```

### 257 · word · math-10-primes

```text
просты́е
```

### 258 · word · math-10-primes

```text
возьмём
```

### 259 · word · math-10-primes

```text
него́
```

### 260 · word · math-10-primes

```text
дели́тель
```

### 261 · word · math-10-primes

```text
спи́ска
```

### 262 · word · math-10-primes

```text
де́лит
```

### 263 · word · math-10-primes

```text
оста́ток
```

### 264 · word · math-10-primes

```text
деле́ния
```

### 265 · word · math-10-primes

```text
них
```

### 266 · word · math-10-primes

```text
едини́це
```

### 267 · word · math-10-primes

```text
спи́ске
```

### 268 · word · math-10-primes

```text
утвержда́ем
```

### 269 · display-token · math-11-sqrt-two

```text
√2
```

### 270 · display-token · math-11-sqrt-two

```text
a/b
```

### 271 · display-token · math-11-sqrt-two

```text
a²
```

### 272 · display-token · math-11-sqrt-two

```text
2b²
```

### 273 · display-token · math-11-sqrt-two

```text
4k²
```

### 274 · display-token · math-11-sqrt-two

```text
b²
```

### 275 · display-token · math-11-sqrt-two

```text
2k²
```

### 276 · title · math-11-sqrt-two

```text
Ко́рень из двух
```

### 277 · passage · math-11-sqrt-two

```text
Утвержде́ние. Число́ √2 иррациона́льно: оно́ не явля́ется отноше́нием двух це́лых чи́сел.
Доказа́тельство. Предполо́жим, что √2 = a/b, где a и b — положи́тельные це́лые чи́сла без о́бщего дели́теля, бо́льшего едини́цы. Тогда́ a² = 2b², поэ́тому a² чётно. Квадра́т нечётного числа́ нечётен, зна́чит, a чётно. Запи́шем a = 2k. Полу́чим 4k² = 2b² и b² = 2k². Сле́довательно, b то́же чётно. Но тогда́ у a и b есть о́бщий дели́тель 2. Э́то противоре́чит вы́бору дро́би. Ита́к, √2 иррациона́льно. Усло́вие об отсу́тствии о́бщего дели́теля необходи́мо для противоре́чия, а не для преобразова́ния ра́венств.
```

### 278 · how_read · math-11-sqrt-two/how_read/1

```text
противоре́чит вы́бору
```

### 279 · how_read · math-11-sqrt-two/how_read/2

```text
без о́бщего дели́теля
```

### 280 · how_read · math-11-sqrt-two/how_read/3

```text
запи́шем … полу́чим …
```

### 281 · notation · math-11-sqrt-two/notation/1

```text
квадра́тный ко́рень из двух
```

### 282 · notation · math-11-sqrt-two/notation/2

```text
a в квадра́те
```

### 283 · notation · math-11-sqrt-two/notation/3

```text
a, делённое на b
```

### 284 · word · math-11-sqrt-two

```text
ко́рень
```

### 285 · word · math-11-sqrt-two

```text
иррациона́льно
```

### 286 · word · math-11-sqrt-two

```text
отноше́нием
```

### 287 · word · math-11-sqrt-two

```text
положи́тельные
```

### 288 · word · math-11-sqrt-two

```text
о́бщего
```

### 289 · word · math-11-sqrt-two

```text
бо́льшего
```

### 290 · word · math-11-sqrt-two

```text
чётно
```

### 291 · word · math-11-sqrt-two

```text
нечётного
```

### 292 · word · math-11-sqrt-two

```text
нечётен
```

### 293 · word · math-11-sqrt-two

```text
запи́шем
```

### 294 · word · math-11-sqrt-two

```text
полу́чим
```

### 295 · word · math-11-sqrt-two

```text
о́бщий
```

### 296 · word · math-11-sqrt-two

```text
противоре́чит
```

### 297 · word · math-11-sqrt-two

```text
вы́бору
```

### 298 · word · math-11-sqrt-two

```text
дро́би
```

### 299 · word · math-11-sqrt-two

```text
отсу́тствии
```

### 300 · word · math-11-sqrt-two

```text
необходи́мо
```

### 301 · word · math-11-sqrt-two

```text
противоре́чия
```

### 302 · word · math-11-sqrt-two

```text
преобразова́ния
```

### 303 · word · math-11-sqrt-two

```text
ра́венств
```

### 304 · word · math-11-sqrt-two

```text
противоре́чить
```

### 305 · word · math-11-sqrt-two

```text
квадра́тный
```

### 306 · word · math-11-sqrt-two

```text
квадра́те
```

### 307 · word · math-11-sqrt-two

```text
делённое
```

### 308 · title · math-12-degrees

```text
Сте́пени верши́н
```

### 309 · passage · math-12-degrees

```text
Рассмо́трим коне́чный неориенти́рованный граф без пе́тель и кра́тных рёбер. Сте́пень верши́ны — число́ рёбер, кото́рые выхо́дят из неё. Утвержде́ние: су́мма степене́й всех верши́н равна́ удво́енному числу́ рёбер.
Доказа́тельство. Посчита́ем концы́ рёбер двумя́ спо́собами. В ка́ждой верши́не число́ концо́в равно́ её сте́пени. Поэ́тому о́бщее число́ концо́в равно́ су́мме степене́й. С друго́й стороны́, у ка́ждого ребра́ есть два конца́. Зна́чит, о́бщее число́ концо́в равно́ удво́енному числу́ рёбер. О́ба выраже́ния счита́ют одно́ и то же, поэ́тому они́ равны́. Сле́дствие: число́ верши́н нечётной сте́пени чётно. Ина́че су́мма всех степене́й была́ бы нечётной, хотя́ она́ равна́ удво́енному це́лому числу́.
```

### 310 · how_read · math-12-degrees/how_read/1

```text
рёбра, кото́рые …
```

### 311 · how_read · math-12-degrees/how_read/2

```text
су́мма равна́
```

### 312 · how_read · math-12-degrees/how_read/3

```text
двумя́ спо́собами
```

### 313 · notation · math-12-degrees/notation/1

```text
су́мма степене́й верши́н равна́ удво́енному числу́ рёбер
```

### 314 · word · math-12-degrees

```text
сте́пени
```

### 315 · word · math-12-degrees

```text
верши́н
```

### 316 · word · math-12-degrees

```text
рассмо́трим
```

### 317 · word · math-12-degrees

```text
неориенти́рованный
```

### 318 · word · math-12-degrees

```text
граф
```

### 319 · word · math-12-degrees

```text
пе́тель
```

### 320 · word · math-12-degrees

```text
кра́тных
```

### 321 · word · math-12-degrees

```text
рёбер
```

### 322 · word · math-12-degrees

```text
сте́пень
```

### 323 · word · math-12-degrees

```text
верши́ны
```

### 324 · word · math-12-degrees

```text
выхо́дят
```

### 325 · word · math-12-degrees

```text
неё
```

### 326 · word · math-12-degrees

```text
степене́й
```

### 327 · word · math-12-degrees

```text
равна́
```

### 328 · word · math-12-degrees

```text
посчита́ем
```

### 329 · word · math-12-degrees

```text
концы́
```

### 330 · word · math-12-degrees

```text
двумя́
```

### 331 · word · math-12-degrees

```text
спо́собами
```

### 332 · word · math-12-degrees

```text
верши́не
```

### 333 · word · math-12-degrees

```text
концо́в
```

### 334 · word · math-12-degrees

```text
её
```

### 335 · word · math-12-degrees

```text
су́мме
```

### 336 · word · math-12-degrees

```text
стороны́
```

### 337 · word · math-12-degrees

```text
ребра́
```

### 338 · word · math-12-degrees

```text
конца́
```

### 339 · word · math-12-degrees

```text
о́ба
```

### 340 · word · math-12-degrees

```text
выраже́ния
```

### 341 · word · math-12-degrees

```text
счита́ют
```

### 342 · word · math-12-degrees

```text
равны́
```

### 343 · word · math-12-degrees

```text
нечётной
```

### 344 · word · math-12-degrees

```text
рёбра
```

### 345 · title · math-13-paths

```text
Кратча́йший путь
```

### 346 · passage · math-13-paths

```text
Пусть две ра́зные верши́ны коне́чного неориенти́рованного гра́фа соедине́ны маршру́том. Здесь маршру́т — после́довательность сме́жных верши́н; повторе́ния разреше́ны. Путь — маршру́т без повторе́ний верши́н. Утвержде́ние: ме́жду да́нными верши́нами существу́ет путь.
Доказа́тельство. Вы́берем маршру́т наиме́ньшей длины́. Е́сли в нём два́жды встреча́ется одна́ верши́на, уда́лим часть маршру́та ме́жду э́тими двумя́ появле́ниями. Оста́нется маршру́т с те́ми же нача́лом и концо́м, кото́рый коро́че исхо́дного. Э́то противоре́чит вы́бору. Поэ́тому повторе́ний нет: вы́бранный маршру́т явля́ется путём. Мы доказа́ли существова́ние пути́, но не его́ еди́нственность. В гра́фе мо́гут быть два ра́зных пути́, кото́рые име́ют одина́ковую наиме́ньшую длину́.
```

### 347 · how_read · math-13-paths/how_read/1

```text
ме́жду верши́нами
```

### 348 · how_read · math-13-paths/how_read/2

```text
кото́рый коро́че исхо́дного
```

### 349 · how_read · math-13-paths/how_read/3

```text
вы́берем … уда́лим …
```

### 350 · word · math-13-paths

```text
кратча́йший
```

### 351 · word · math-13-paths

```text
путь
```

### 352 · word · math-13-paths

```text
ра́зные
```

### 353 · word · math-13-paths

```text
коне́чного
```

### 354 · word · math-13-paths

```text
неориенти́рованного
```

### 355 · word · math-13-paths

```text
гра́фа
```

### 356 · word · math-13-paths

```text
соедине́ны
```

### 357 · word · math-13-paths

```text
маршру́том
```

### 358 · word · math-13-paths

```text
маршру́т
```

### 359 · word · math-13-paths

```text
после́довательность
```

### 360 · word · math-13-paths

```text
сме́жных
```

### 361 · word · math-13-paths

```text
повторе́ния
```

### 362 · word · math-13-paths

```text
разреше́ны
```

### 363 · word · math-13-paths

```text
повторе́ний
```

### 364 · word · math-13-paths

```text
да́нными
```

### 365 · word · math-13-paths

```text
верши́нами
```

### 366 · word · math-13-paths

```text
вы́берем
```

### 367 · word · math-13-paths

```text
наиме́ньшей
```

### 368 · word · math-13-paths

```text
длины́
```

### 369 · word · math-13-paths

```text
встреча́ется
```

### 370 · word · math-13-paths

```text
уда́лим
```

### 371 · word · math-13-paths

```text
маршру́та
```

### 372 · word · math-13-paths

```text
э́тими
```

### 373 · word · math-13-paths

```text
появле́ниями
```

### 374 · word · math-13-paths

```text
оста́нется
```

### 375 · word · math-13-paths

```text
те́ми
```

### 376 · word · math-13-paths

```text
нача́лом
```

### 377 · word · math-13-paths

```text
концо́м
```

### 378 · word · math-13-paths

```text
коро́че
```

### 379 · word · math-13-paths

```text
исхо́дного
```

### 380 · word · math-13-paths

```text
вы́бранный
```

### 381 · word · math-13-paths

```text
путём
```

### 382 · word · math-13-paths

```text
существова́ние
```

### 383 · word · math-13-paths

```text
пути́
```

### 384 · word · math-13-paths

```text
еди́нственность
```

### 385 · word · math-13-paths

```text
гра́фе
```

### 386 · word · math-13-paths

```text
име́ют
```

### 387 · word · math-13-paths

```text
одина́ковую
```

### 388 · word · math-13-paths

```text
наиме́ньшую
```

### 389 · word · math-13-paths

```text
длину́
```

### 390 · word · math-13-paths

```text
длина́
```

### 391 · display-token · math-14-inverse

```text
g
```

### 392 · display-token · math-14-inverse

```text
g(y
```

### 393 · display-token · math-14-inverse

```text
f(g(y
```

### 394 · display-token · math-14-inverse

```text
g(f(x
```

### 395 · title · math-14-inverse

```text
Обра́тная фу́нкция
```

### 396 · passage · math-14-inverse

```text
Фу́нкция f : A → B называ́ется бие́кцией, е́сли для ка́ждого y ∈ B существу́ет еди́нственный x ∈ A, для кото́рого f(x) = y. Утвержде́ние: у бие́кции существу́ет обра́тная фу́нкция g : B → A.
Доказа́тельство. Для ка́ждого y возьмём э́тот еди́нственный x и поло́жим g(y) = x. Значе́ние существу́ет и определено́ однозна́чно, поэ́тому g явля́ется фу́нкцией. По построе́нию f(g(y)) = y. Кро́ме того́, g(f(x)) = x, так как x — еди́нственный аргуме́нт со значе́нием f(x). Ита́к, g — обра́тная фу́нкция. Без существова́ния проо́браза значе́ние g иногда́ отсу́тствует; без еди́нственности появля́ется вы́бор. О́ба усло́вия ну́жны для э́того определе́ния.
```

### 397 · how_read · math-14-inverse/how_read/1

```text
существу́ет еди́нственный
```

### 398 · how_read · math-14-inverse/how_read/2

```text
для кото́рого
```

### 399 · how_read · math-14-inverse/how_read/3

```text
по построе́нию
```

### 400 · how_read · math-14-inverse/how_read/4

```text
определено́ однозна́чно
```

### 401 · notation · math-14-inverse/notation/1

```text
для любо́го y из B существу́ет еди́нственный x из A
```

### 402 · word · math-14-inverse

```text
обра́тная
```

### 403 · word · math-14-inverse

```text
бие́кцией
```

### 404 · word · math-14-inverse

```text
еди́нственный
```

### 405 · word · math-14-inverse

```text
бие́кции
```

### 406 · word · math-14-inverse

```text
определено́
```

### 407 · word · math-14-inverse

```text
однозна́чно
```

### 408 · word · math-14-inverse

```text
построе́нию
```

### 409 · word · math-14-inverse

```text
кро́ме
```

### 410 · word · math-14-inverse

```text
со
```

### 411 · word · math-14-inverse

```text
значе́нием
```

### 412 · word · math-14-inverse

```text
существова́ния
```

### 413 · word · math-14-inverse

```text
проо́браза
```

### 414 · word · math-14-inverse

```text
отсу́тствует
```

### 415 · word · math-14-inverse

```text
еди́нственности
```

### 416 · word · math-14-inverse

```text
появля́ется
```

### 417 · word · math-14-inverse

```text
вы́бор
```

### 418 · word · math-14-inverse

```text
ну́жны
```

### 419 · word · math-14-inverse

```text
э́того
```

### 420 · display-token · math-15-open-sets

```text
−
```

### 421 · display-token · math-15-open-sets

```text
ε
```

### 422 · display-token · math-15-open-sets

```text
>
```

### 423 · display-token · math-15-open-sets

```text
0
```

### 424 · display-token · math-15-open-sets

```text
U
```

### 425 · display-token · math-15-open-sets

```text
[0
```

### 426 · display-token · math-15-open-sets

```text
1]
```

### 427 · display-token · math-15-open-sets

```text
−∞
```

### 428 · display-token · math-15-open-sets

```text
+∞
```

### 429 · title · math-15-open-sets

```text
Откры́тые мно́жества
```

### 430 · passage · math-15-open-sets

```text
Рассма́триваем веще́ственную пряму́ю. Окре́стность то́чки x — интерва́л (x − ε, x + ε), где ε > 0. Мно́жество U называ́ется откры́тым, е́сли для ка́ждой то́чки x из U существу́ет окре́стность, цели́ком содержа́щаяся в U. Сло́во «содержа́щаяся» отно́сится к окре́стности: все её то́чки должны́ принадлежа́ть U.
Наприме́р, интерва́л (0, 1) откры́т. Для то́чки внутри́ него́ мо́жно вы́брать доста́точно ма́лую окре́стность. Мно́жество всех то́чек, не принадлежа́щих отре́зку [0, 1], то́же откры́то: э́то объедине́ние интерва́лов (−∞, 0) и (1, +∞). Здесь «не принадлежа́щих» характеризу́ет то́чки, а не мно́жество. Само́ мно́жество [0, 1] за́мкнуто: его́ дополне́ние откры́то.
```

### 431 · how_read · math-15-open-sets/how_read/1

```text
окре́стность, содержа́щаяся в U
```

### 432 · how_read · math-15-open-sets/how_read/2

```text
всех то́чек, не принадлежа́щих отре́зку
```

### 433 · how_read · math-15-open-sets/how_read/3

```text
его́ дополне́ние откры́то
```

### 434 · notation · math-15-open-sets/notation/1

```text
интерва́л от нуля́ до едини́цы
```

### 435 · notation · math-15-open-sets/notation/2

```text
отре́зок от нуля́ до едини́цы
```

### 436 · notation · math-15-open-sets/notation/3

```text
э́псилон бо́льше нуля́
```

### 437 · word · math-15-open-sets

```text
откры́тые
```

### 438 · word · math-15-open-sets

```text
рассма́триваем
```

### 439 · word · math-15-open-sets

```text
веще́ственную
```

### 440 · word · math-15-open-sets

```text
пряму́ю
```

### 441 · word · math-15-open-sets

```text
окре́стность
```

### 442 · word · math-15-open-sets

```text
то́чки
```

### 443 · word · math-15-open-sets

```text
интерва́л
```

### 444 · word · math-15-open-sets

```text
откры́тым
```

### 445 · word · math-15-open-sets

```text
цели́ком
```

### 446 · word · math-15-open-sets

```text
содержа́щаяся
```

### 447 · word · math-15-open-sets

```text
отно́сится
```

### 448 · word · math-15-open-sets

```text
окре́стности
```

### 449 · word · math-15-open-sets

```text
должны́
```

### 450 · word · math-15-open-sets

```text
откры́т
```

### 451 · word · math-15-open-sets

```text
внутри́
```

### 452 · word · math-15-open-sets

```text
вы́брать
```

### 453 · word · math-15-open-sets

```text
доста́точно
```

### 454 · word · math-15-open-sets

```text
ма́лую
```

### 455 · word · math-15-open-sets

```text
то́чек
```

### 456 · word · math-15-open-sets

```text
принадлежа́щих
```

### 457 · word · math-15-open-sets

```text
отре́зку
```

### 458 · word · math-15-open-sets

```text
объедине́ние
```

### 459 · word · math-15-open-sets

```text
интерва́лов
```

### 460 · word · math-15-open-sets

```text
характеризу́ет
```

### 461 · word · math-15-open-sets

```text
за́мкнуто
```

### 462 · word · math-15-open-sets

```text
дополне́ние
```

### 463 · word · math-15-open-sets

```text
нуля́
```

### 464 · word · math-15-open-sets

```text
отре́зок
```

### 465 · word · math-15-open-sets

```text
э́псилон
```

### 466 · display-token · math-16-union-theorem

```text
Uᵢ
```

### 467 · display-token · math-16-union-theorem

```text
i
```

### 468 · display-token · math-16-union-theorem

```text
I
```

### 469 · title · math-16-union-theorem

```text
Объедине́ние: теоре́ма
```

### 470 · passage · math-16-union-theorem

```text
Теоре́ма. Объедине́ние любо́го семе́йства откры́тых мно́жеств веще́ственной прямо́й откры́то. Речь идёт о мно́жествах, откры́тых на одно́й и той же прямо́й. Семе́йство мо́жет быть коне́чным и́ли бесконе́чным.
Обозна́чим э́ти мно́жества Uᵢ, где i ∈ I. Их объедине́ние U состои́т из то́чек, принадлежа́щих хотя́ бы одному́ Uᵢ. Не тре́буется, что́бы то́чка принадлежа́ла всем мно́жествам семе́йства. При пусто́м I объедине́ние пусто́. Пусто́е мно́жество откры́то: в нём нет то́чки, для кото́рой усло́вие откры́тости наруша́ется. В сле́дующем доказа́тельстве для непусто́го объедине́ния мы вы́берем то́чку, зате́м одно́ мно́жество семе́йства, содержа́щее э́ту то́чку.
```

### 471 · how_read · math-16-union-theorem/how_read/1

```text
объедине́ние семе́йства мно́жеств
```

### 472 · how_read · math-16-union-theorem/how_read/2

```text
то́чек, принадлежа́щих хотя́ бы одному́
```

### 473 · how_read · math-16-union-theorem/how_read/3

```text
мно́жество, содержа́щее э́ту то́чку
```

### 474 · notation · math-16-union-theorem/notation/1

```text
U — объедине́ние всех U с и́ндексом i
```

### 475 · word · math-16-union-theorem

```text
семе́йства
```

### 476 · word · math-16-union-theorem

```text
откры́тых
```

### 477 · word · math-16-union-theorem

```text
мно́жеств
```

### 478 · word · math-16-union-theorem

```text
веще́ственной
```

### 479 · word · math-16-union-theorem

```text
прямо́й
```

### 480 · word · math-16-union-theorem

```text
речь
```

### 481 · word · math-16-union-theorem

```text
мно́жествах
```

### 482 · word · math-16-union-theorem

```text
одно́й
```

### 483 · word · math-16-union-theorem

```text
семе́йство
```

### 484 · word · math-16-union-theorem

```text
обозна́чим
```

### 485 · word · math-16-union-theorem

```text
состои́т
```

### 486 · word · math-16-union-theorem

```text
одному́
```

### 487 · word · math-16-union-theorem

```text
принадлежа́ла
```

### 488 · word · math-16-union-theorem

```text
всем
```

### 489 · word · math-16-union-theorem

```text
мно́жествам
```

### 490 · word · math-16-union-theorem

```text
пусто́м
```

### 491 · word · math-16-union-theorem

```text
пусто́
```

### 492 · word · math-16-union-theorem

```text
кото́рой
```

### 493 · word · math-16-union-theorem

```text
откры́тости
```

### 494 · word · math-16-union-theorem

```text
наруша́ется
```

### 495 · word · math-16-union-theorem

```text
сле́дующем
```

### 496 · word · math-16-union-theorem

```text
доказа́тельстве
```

### 497 · word · math-16-union-theorem

```text
непусто́го
```

### 498 · word · math-16-union-theorem

```text
объедине́ния
```

### 499 · word · math-16-union-theorem

```text
то́чку
```

### 500 · word · math-16-union-theorem

```text
зате́м
```

### 501 · word · math-16-union-theorem

```text
содержа́щее
```

### 502 · word · math-16-union-theorem

```text
э́ту
```

### 503 · word · math-16-union-theorem

```text
содержа́щееся
```

### 504 · word · math-16-union-theorem

```text
и́ндексом
```

### 505 · display-token · math-17-union-proof

```text
V
```

### 506 · title · math-17-union-proof

```text
Объедине́ние: доказа́тельство
```

### 507 · passage · math-17-union-proof

```text
Доказа́тельство. Пусть U — объедине́ние откры́тых мно́жеств Uᵢ. Слу́чай пусто́го U уже́ рассмо́трен. Вы́брав произво́льную то́чку x из U, найдём мно́жество Uᵢ, содержа́щее x. По определе́нию объедине́ния тако́е мно́жество существу́ет. Так как Uᵢ откры́то, существу́ет окре́стность V то́чки x, содержа́щаяся в Uᵢ. Но Uᵢ ⊆ U. Сле́довательно, V цели́ком соде́ржится и в U. Мы нашли́ тре́буемую окре́стность для произво́льной то́чки U. Зна́чит, U откры́то.
Выбира́я Uᵢ, мы мо́жем учиты́вать x. Одно́ и то же Uᵢ не обяза́но подходи́ть для всех то́чек U. Доказа́тельство испо́льзует лока́льное усло́вие: у ка́ждой то́чки есть своя́ окре́стность.
```

### 508 · how_read · math-17-union-proof/how_read/1

```text
вы́брав то́чку, найдём …
```

### 509 · how_read · math-17-union-proof/how_read/2

```text
выбира́я …, мы …
```

### 510 · how_read · math-17-union-proof/how_read/3

```text
окре́стность, содержа́щаяся …
```

### 511 · how_read · math-17-union-proof/how_read/4

```text
слу́чай рассмо́трен
```

### 512 · notation · math-17-union-proof/notation/1

```text
U с и́ндексом i явля́ется подмно́жеством U
```

### 513 · word · math-17-union-proof

```text
рассмо́трен
```

### 514 · word · math-17-union-proof

```text
вы́брав
```

### 515 · word · math-17-union-proof

```text
произво́льную
```

### 516 · word · math-17-union-proof

```text
найдём
```

### 517 · word · math-17-union-proof

```text
соде́ржится
```

### 518 · word · math-17-union-proof

```text
нашли́
```

### 519 · word · math-17-union-proof

```text
тре́буемую
```

### 520 · word · math-17-union-proof

```text
произво́льной
```

### 521 · word · math-17-union-proof

```text
выбира́я
```

### 522 · word · math-17-union-proof

```text
мо́жем
```

### 523 · word · math-17-union-proof

```text
учиты́вать
```

### 524 · word · math-17-union-proof

```text
обяза́но
```

### 525 · word · math-17-union-proof

```text
подходи́ть
```

### 526 · word · math-17-union-proof

```text
испо́льзует
```

### 527 · word · math-17-union-proof

```text
лока́льное
```

### 528 · word · math-17-union-proof

```text
своя́
```

### 529 · word · math-17-union-proof

```text
подмно́жеством
```

### 530 · display-token · math-18-continuity

```text
[a
```

### 531 · display-token · math-18-continuity

```text
b]
```

### 532 · display-token · math-18-continuity

```text
<
```

### 533 · display-token · math-18-continuity

```text
x₀
```

### 534 · display-token · math-18-continuity

```text
δ
```

### 535 · display-token · math-18-continuity

```text
|x
```

### 536 · display-token · math-18-continuity

```text
x₀|
```

### 537 · display-token · math-18-continuity

```text
|f(x
```

### 538 · display-token · math-18-continuity

```text
f(x₀)|
```

### 539 · title · math-18-continuity

```text
Непреры́вность в то́чке
```

### 540 · passage · math-18-continuity

```text
Пусть f — веще́ственная фу́нкция, определённая на отре́зке [a, b], где a < b. То́чка x₀ принадлежи́т э́тому отре́зку. Фу́нкция f непреры́вна в то́чке x₀, е́сли для любо́го ε > 0 существу́ет δ > 0 тако́е, что для всех x из [a, b] при |x − x₀| < δ выполня́ется |f(x) − f(x₀)| < ε.
Здесь δ мо́жет зави́сеть от ε и от то́чки x₀, но не от проверя́емого x. Фу́нкция определена́ на всём отре́зке; непреры́вность пока́ тре́буется лишь в одно́й то́чке. На конце́ отре́зка мы по-пре́жнему проверя́ем то́лько те x, кото́рые принадлежа́т о́бласти определе́ния. То́чки вне отре́зка в э́том определе́нии не уча́ствуют.
```

### 541 · how_read · math-18-continuity/how_read/1

```text
фу́нкция, определённая на отре́зке
```

### 542 · how_read · math-18-continuity/how_read/2

```text
фу́нкция определена́
```

### 543 · how_read · math-18-continuity/how_read/3

```text
зави́сеть от то́чки
```

### 544 · how_read · math-18-continuity/how_read/4

```text
проверя́емого x
```

### 545 · notation · math-18-continuity/notation/1

```text
мо́дуль ра́зности x и x нулево́го ме́ньше числа́ де́льта
```

### 546 · notation · math-18-continuity/notation/2

```text
для любо́го положи́тельного числа́ э́псилон существу́ет положи́тельное число́ де́льта
```

### 547 · word · math-18-continuity

```text
непреры́вность
```

### 548 · word · math-18-continuity

```text
то́чке
```

### 549 · word · math-18-continuity

```text
веще́ственная
```

### 550 · word · math-18-continuity

```text
определённая
```

### 551 · word · math-18-continuity

```text
отре́зке
```

### 552 · word · math-18-continuity

```text
э́тому
```

### 553 · word · math-18-continuity

```text
непреры́вна
```

### 554 · word · math-18-continuity

```text
выполня́ется
```

### 555 · word · math-18-continuity

```text
зави́сеть
```

### 556 · word · math-18-continuity

```text
проверя́емого
```

### 557 · word · math-18-continuity

```text
определена́
```

### 558 · word · math-18-continuity

```text
всём
```

### 559 · word · math-18-continuity

```text
конце́
```

### 560 · word · math-18-continuity

```text
отре́зка
```

### 561 · word · math-18-continuity

```text
по-пре́жнему
```

### 562 · word · math-18-continuity

```text
проверя́ем
```

### 563 · word · math-18-continuity

```text
принадлежа́т
```

### 564 · word · math-18-continuity

```text
о́бласти
```

### 565 · word · math-18-continuity

```text
вне
```

### 566 · word · math-18-continuity

```text
уча́ствуют
```

### 567 · word · math-18-continuity

```text
ра́зности
```

### 568 · word · math-18-continuity

```text
нулево́го
```

### 569 · word · math-18-continuity

```text
де́льта
```

### 570 · word · math-18-continuity

```text
положи́тельного
```

### 571 · word · math-18-continuity

```text
положи́тельное
```

### 572 · display-token · math-19-sum-theorem

```text
h
```

### 573 · display-token · math-19-sum-theorem

```text
h(x
```

### 574 · display-token · math-19-sum-theorem

```text
ε/2
```

### 575 · title · math-19-sum-theorem

```text
Су́мма: теоре́ма о непреры́вности
```

### 576 · passage · math-19-sum-theorem

```text
Теоре́ма. Пусть фу́нкции f и g, определённые на одно́м отре́зке [a, b], непреры́вны в то́чке x₀ э́того отре́зка. Тогда́ фу́нкция h, определённая ра́венством h(x) = f(x) + g(x), то́же непреры́вна в x₀.
Сло́во «то́же» перено́сит сво́йство непреры́вности на су́мму. Оно́ не утвержда́ет, что f, g и h равны́ друг дру́гу. То́чка x₀ одна́ и та же для обе́их исхо́дных фу́нкций. Их непреры́вность в ра́зных то́чках была́ бы други́м усло́вием. Для доказа́тельства доста́точно ограни́чить ка́ждое из двух измене́ний величино́й ε/2. Скла́дывая оце́нки, мы полу́чим ну́жную оце́нку для h. Соотве́тствующие ра́диусы окре́стностей мо́гут различа́ться.
```

### 577 · how_read · math-19-sum-theorem/how_read/1

```text
фу́нкции, определённые …
```

### 578 · how_read · math-19-sum-theorem/how_read/2

```text
определённая ра́венством
```

### 579 · how_read · math-19-sum-theorem/how_read/3

```text
скла́дывая оце́нки, мы полу́чим …
```

### 580 · how_read · math-19-sum-theorem/how_read/4

```text
для обе́их фу́нкций
```

### 581 · notation · math-19-sum-theorem/notation/1

```text
h от x равно́ f от x плюс g от x
```

### 582 · notation · math-19-sum-theorem/notation/2

```text
э́псилон попола́м
```

### 583 · word · math-19-sum-theorem

```text
непреры́вности
```

### 584 · word · math-19-sum-theorem

```text
определённые
```

### 585 · word · math-19-sum-theorem

```text
непреры́вны
```

### 586 · word · math-19-sum-theorem

```text
ра́венством
```

### 587 · word · math-19-sum-theorem

```text
перено́сит
```

### 588 · word · math-19-sum-theorem

```text
сво́йство
```

### 589 · word · math-19-sum-theorem

```text
су́мму
```

### 590 · word · math-19-sum-theorem

```text
утвержда́ет
```

### 591 · word · math-19-sum-theorem

```text
дру́гу
```

### 592 · word · math-19-sum-theorem

```text
обе́их
```

### 593 · word · math-19-sum-theorem

```text
исхо́дных
```

### 594 · word · math-19-sum-theorem

```text
фу́нкций
```

### 595 · word · math-19-sum-theorem

```text
то́чках
```

### 596 · word · math-19-sum-theorem

```text
други́м
```

### 597 · word · math-19-sum-theorem

```text
усло́вием
```

### 598 · word · math-19-sum-theorem

```text
доказа́тельства
```

### 599 · word · math-19-sum-theorem

```text
ограни́чить
```

### 600 · word · math-19-sum-theorem

```text
измене́ний
```

### 601 · word · math-19-sum-theorem

```text
величино́й
```

### 602 · word · math-19-sum-theorem

```text
скла́дывая
```

### 603 · word · math-19-sum-theorem

```text
оце́нки
```

### 604 · word · math-19-sum-theorem

```text
ну́жную
```

### 605 · word · math-19-sum-theorem

```text
оце́нку
```

### 606 · word · math-19-sum-theorem

```text
соотве́тствующие
```

### 607 · word · math-19-sum-theorem

```text
ра́диусы
```

### 608 · word · math-19-sum-theorem

```text
окре́стностей
```

### 609 · word · math-19-sum-theorem

```text
различа́ться
```

### 610 · word · math-19-sum-theorem

```text
попола́м
```

### 611 · display-token · math-20-sum-proof

```text
δ₁
```

### 612 · display-token · math-20-sum-proof

```text
δ₂
```

### 613 · display-token · math-20-sum-proof

```text
min(δ₁
```

### 614 · display-token · math-20-sum-proof

```text
|h(x
```

### 615 · display-token · math-20-sum-proof

```text
h(x₀)|
```

### 616 · display-token · math-20-sum-proof

```text
≤
```

### 617 · display-token · math-20-sum-proof

```text
|g(x
```

### 618 · display-token · math-20-sum-proof

```text
g(x₀)|
```

### 619 · title · math-20-sum-proof

```text
Су́мма: доказа́тельство
```

### 620 · passage · math-20-sum-proof

```text
Доказа́тельство. Зафикси́руем ε > 0. Из непреры́вности f и g в x₀ полу́чим положи́тельные чи́сла δ₁ и δ₂. При |x − x₀| < δ₁ измене́ние f по мо́дулю ме́ньше ε/2; при |x − x₀| < δ₂ то же ве́рно для g. Поло́жим δ = min(δ₁, δ₂). Вы́брав x из отре́зка так, что |x − x₀| < δ, полу́чим сра́зу о́бе оце́нки. Применя́я нера́венство треуго́льника, нахо́дим: |h(x) − h(x₀)| ≤ |f(x) − f(x₀)| + |g(x) − g(x₀)| < ε. Сле́довательно, h непреры́вна в x₀.
Вы́бранное δ не зави́сит от x. Ми́нимум ну́жен, что́бы о́бе оце́нки выполня́лись одновре́менно. Вы́брав ма́ксимум вме́сто ми́нимума, мы не смогли́ бы обоснова́ть э́тот шаг. Э́то ва́жно для поря́дка вы́бора пара́метров.
```

### 621 · how_read · math-20-sum-proof/how_read/1

```text
зафикси́руем …
```

### 622 · how_read · math-20-sum-proof/how_read/2

```text
применя́я нера́венство, нахо́дим …
```

### 623 · how_read · math-20-sum-proof/how_read/3

```text
вы́бранное δ
```

### 624 · how_read · math-20-sum-proof/how_read/4

```text
не зави́сит от x
```

### 625 · notation · math-20-sum-proof/notation/1

```text
число́ де́льта равно́ ми́нимуму из чи́сел де́льта оди́н и де́льта два
```

### 626 · notation · math-20-sum-proof/notation/2

```text
ме́ньше и́ли равно́
```

### 627 · word · math-20-sum-proof

```text
зафикси́руем
```

### 628 · word · math-20-sum-proof

```text
измене́ние
```

### 629 · word · math-20-sum-proof

```text
мо́дулю
```

### 630 · word · math-20-sum-proof

```text
ве́рно
```

### 631 · word · math-20-sum-proof

```text
сра́зу
```

### 632 · word · math-20-sum-proof

```text
о́бе
```

### 633 · word · math-20-sum-proof

```text
применя́я
```

### 634 · word · math-20-sum-proof

```text
нера́венство
```

### 635 · word · math-20-sum-proof

```text
треуго́льника
```

### 636 · word · math-20-sum-proof

```text
нахо́дим
```

### 637 · word · math-20-sum-proof

```text
вы́бранное
```

### 638 · word · math-20-sum-proof

```text
зави́сит
```

### 639 · word · math-20-sum-proof

```text
ми́нимум
```

### 640 · word · math-20-sum-proof

```text
выполня́лись
```

### 641 · word · math-20-sum-proof

```text
одновре́менно
```

### 642 · word · math-20-sum-proof

```text
ма́ксимум
```

### 643 · word · math-20-sum-proof

```text
вме́сто
```

### 644 · word · math-20-sum-proof

```text
ми́нимума
```

### 645 · word · math-20-sum-proof

```text
смогли́
```

### 646 · word · math-20-sum-proof

```text
обоснова́ть
```

### 647 · word · math-20-sum-proof

```text
шаг
```

### 648 · word · math-20-sum-proof

```text
вы́бора
```

### 649 · word · math-20-sum-proof

```text
пара́метров
```

### 650 · word · math-20-sum-proof

```text
ми́нимуму
```

### 651 · display-token · math-21-probability

```text
Ω
```

### 652 · display-token · math-21-probability

```text
ω
```

### 653 · display-token · math-21-probability

```text
p(ω
```

### 654 · display-token · math-21-probability

```text
≥
```

### 655 · display-token · math-21-probability

```text
P(A
```

### 656 · display-token · math-21-probability

```text
∪
```

### 657 · display-token · math-21-probability

```text
P(B
```

### 658 · title · math-21-probability

```text
Коне́чная вероя́тность
```

### 659 · passage · math-21-probability

```text
Пусть Ω — непусто́е коне́чное мно́жество исхо́дов. Ка́ждому исхо́ду ω сопоста́влено число́ p(ω) ≥ 0, причём су́мма всех э́тих чи́сел равна́ едини́це. Собы́тие — подмно́жество Ω. Вероя́тность собы́тия A определена́ как су́мма p(ω) по всем исхо́дам, принадлежа́щим A.
Утвержде́ние. Е́сли собы́тия A и B не пересека́ются, то P(A ∪ B) = P(A) + P(B). Доказа́тельство. Скла́дывая вероя́тности исхо́дов из A и из B, мы учиты́ваем ка́ждый исхо́д объедине́ния ро́вно оди́н раз. Поэ́тому су́ммы равны́. Без усло́вия о непересече́нии о́бщие исхо́ды учиты́вались бы два́жды. Собы́тие, име́ющее нулеву́ю вероя́тность, здесь не обяза́тельно пусто́: в нём мо́гут быть исхо́ды, вероя́тности кото́рых равны́ нулю́.
```

### 660 · how_read · math-21-probability/how_read/1

```text
ка́ждому исхо́ду сопоста́влено число́
```

### 661 · how_read · math-21-probability/how_read/2

```text
по исхо́дам, принадлежа́щим A
```

### 662 · how_read · math-21-probability/how_read/3

```text
собы́тие, име́ющее нулеву́ю вероя́тность
```

### 663 · notation · math-21-probability/notation/1

```text
вероя́тность объедине́ния A и B
```

### 664 · notation · math-21-probability/notation/2

```text
p от оме́га бо́льше и́ли равно́ нулю́
```

### 665 · word · math-21-probability

```text
коне́чная
```

### 666 · word · math-21-probability

```text
вероя́тность
```

### 667 · word · math-21-probability

```text
непусто́е
```

### 668 · word · math-21-probability

```text
исхо́дов
```

### 669 · word · math-21-probability

```text
исхо́ду
```

### 670 · word · math-21-probability

```text
сопоста́влено
```

### 671 · word · math-21-probability

```text
причём
```

### 672 · word · math-21-probability

```text
э́тих
```

### 673 · word · math-21-probability

```text
собы́тие
```

### 674 · word · math-21-probability

```text
собы́тия
```

### 675 · word · math-21-probability

```text
исхо́дам
```

### 676 · word · math-21-probability

```text
принадлежа́щим
```

### 677 · word · math-21-probability

```text
пересека́ются
```

### 678 · word · math-21-probability

```text
вероя́тности
```

### 679 · word · math-21-probability

```text
учиты́ваем
```

### 680 · word · math-21-probability

```text
исхо́д
```

### 681 · word · math-21-probability

```text
су́ммы
```

### 682 · word · math-21-probability

```text
непересече́нии
```

### 683 · word · math-21-probability

```text
о́бщие
```

### 684 · word · math-21-probability

```text
исхо́ды
```

### 685 · word · math-21-probability

```text
учиты́вались
```

### 686 · word · math-21-probability

```text
име́ющее
```

### 687 · word · math-21-probability

```text
нулеву́ю
```

### 688 · word · math-21-probability

```text
нулю́
```

### 689 · word · math-21-probability

```text
оме́га
```

### 690 · display-token · math-22-independence

```text
∩
```

### 691 · display-token · math-22-independence

```text
P(A)P(B
```

### 692 · display-token · math-22-independence

```text
|
```

### 693 · display-token · math-22-independence

```text
B)/P(B
```

### 694 · title · math-22-independence

```text
Незави́симость собы́тий
```

### 695 · passage · math-22-independence

```text
Собы́тия A и B называ́ются незави́симыми, е́сли P(A ∩ B) = P(A)P(B). При P(B) > 0 усло́вная вероя́тность P(A | B) определена́ ра́венством P(A | B) = P(A ∩ B)/P(B). Э́то вероя́тность A при усло́вии, что B произошло́.
Утвержде́ние. При P(B) > 0 незави́симость равноси́льна ра́венству P(A | B) = P(A). Доказа́тельство. Раздели́в ра́венство из определе́ния незави́симости на P(B), полу́чим тре́буемое ра́венство. Обра́тно, умно́жив после́днее ра́венство на P(B), восстано́вим определе́ние незави́симости. О́ба ша́га допусти́мы, поско́льку знамена́тель положи́телен. При P(B) = 0 незави́симость по-пре́жнему определена́, а приведённая фо́рмула усло́вной вероя́тности не определя́ет число́. Непересека́ющиеся собы́тия положи́тельной вероя́тности зави́симы.
```

### 696 · how_read · math-22-independence/how_read/1

```text
раздели́в … полу́чим …
```

### 697 · how_read · math-22-independence/how_read/2

```text
умно́жив … восстано́вим …
```

### 698 · how_read · math-22-independence/how_read/3

```text
равноси́льна ра́венству
```

### 699 · how_read · math-22-independence/how_read/4

```text
приведённая фо́рмула
```

### 700 · notation · math-22-independence/notation/1

```text
вероя́тность A при усло́вии B
```

### 701 · notation · math-22-independence/notation/2

```text
пересече́ние A и B
```

### 702 · word · math-22-independence

```text
незави́симость
```

### 703 · word · math-22-independence

```text
собы́тий
```

### 704 · word · math-22-independence

```text
называ́ются
```

### 705 · word · math-22-independence

```text
незави́симыми
```

### 706 · word · math-22-independence

```text
усло́вная
```

### 707 · word · math-22-independence

```text
усло́вии
```

### 708 · word · math-22-independence

```text
произошло́
```

### 709 · word · math-22-independence

```text
равноси́льна
```

### 710 · word · math-22-independence

```text
ра́венству
```

### 711 · word · math-22-independence

```text
раздели́в
```

### 712 · word · math-22-independence

```text
ра́венство
```

### 713 · word · math-22-independence

```text
незави́симости
```

### 714 · word · math-22-independence

```text
тре́буемое
```

### 715 · word · math-22-independence

```text
обра́тно
```

### 716 · word · math-22-independence

```text
умно́жив
```

### 717 · word · math-22-independence

```text
после́днее
```

### 718 · word · math-22-independence

```text
восстано́вим
```

### 719 · word · math-22-independence

```text
ша́га
```

### 720 · word · math-22-independence

```text
допусти́мы
```

### 721 · word · math-22-independence

```text
поско́льку
```

### 722 · word · math-22-independence

```text
знамена́тель
```

### 723 · word · math-22-independence

```text
положи́телен
```

### 724 · word · math-22-independence

```text
приведённая
```

### 725 · word · math-22-independence

```text
усло́вной
```

### 726 · word · math-22-independence

```text
непересека́ющиеся
```

### 727 · word · math-22-independence

```text
положи́тельной
```

### 728 · word · math-22-independence

```text
зави́симы
```

### 729 · word · math-22-independence

```text
пересече́ние
```

### 730 · display-token · math-23-quantifiers

```text
P(x
```

### 731 · display-token · math-23-quantifiers

```text
P
```

### 732 · display-token · math-23-quantifiers

```text
¬∀x
```

### 733 · display-token · math-23-quantifiers

```text
⇔
```

### 734 · display-token · math-23-quantifiers

```text
∃x
```

### 735 · display-token · math-23-quantifiers

```text
¬P(x
```

### 736 · title · math-23-quantifiers

```text
Отрица́ние и ква́нторы
```

### 737 · passage · math-23-quantifiers

```text
Рабо́таем в класси́ческой ло́гике. Пусть P(x) — сво́йство элеме́нтов мно́жества A. Утвержде́ние «не все элеме́нты A облада́ют сво́йством P» равноси́льно утвержде́нию «существу́ет элеме́нт A, не облада́ющий сво́йством P». Символи́чески: ¬∀x ∈ A P(x) ⇔ ∃x ∈ A ¬P(x).
Доказа́тельство. Е́сли на́йден элеме́нт, не облада́ющий сво́йством, то всео́бщее утвержде́ние ло́жно. Обра́тно, е́сли тако́го элеме́нта нет, то ка́ждый элеме́нт облада́ет сво́йством, и отрица́ние всео́бщего утвержде́ния ло́жно. В класси́ческой ло́гике э́того доста́точно для эквивале́нтности.
Сло́ва «не все» не означа́ют «все не». В мно́жестве чи́сел 1 и 2 не все чи́сла чётные, но и не все нечётные. При пусто́м A о́бе сто́роны запи́санной эквивале́нтности ло́жны.
```

### 738 · how_read · math-23-quantifiers/how_read/1

```text
элеме́нт, не облада́ющий сво́йством
```

### 739 · how_read · math-23-quantifiers/how_read/2

```text
е́сли на́йден элеме́нт
```

### 740 · how_read · math-23-quantifiers/how_read/3

```text
отрица́ние всео́бщего утвержде́ния
```

### 741 · how_read · math-23-quantifiers/how_read/4

```text
не все … / все не …
```

### 742 · notation · math-23-quantifiers/notation/1

```text
для любо́го
```

### 743 · notation · math-23-quantifiers/notation/3

```text
тогда́ и то́лько тогда́
```

### 744 · word · math-23-quantifiers

```text
отрица́ние
```

### 745 · word · math-23-quantifiers

```text
ква́нторы
```

### 746 · word · math-23-quantifiers

```text
рабо́таем
```

### 747 · word · math-23-quantifiers

```text
класси́ческой
```

### 748 · word · math-23-quantifiers

```text
ло́гике
```

### 749 · word · math-23-quantifiers

```text
облада́ют
```

### 750 · word · math-23-quantifiers

```text
сво́йством
```

### 751 · word · math-23-quantifiers

```text
равноси́льно
```

### 752 · word · math-23-quantifiers

```text
утвержде́нию
```

### 753 · word · math-23-quantifiers

```text
облада́ющий
```

### 754 · word · math-23-quantifiers

```text
символи́чески
```

### 755 · word · math-23-quantifiers

```text
на́йден
```

### 756 · word · math-23-quantifiers

```text
всео́бщее
```

### 757 · word · math-23-quantifiers

```text
ло́жно
```

### 758 · word · math-23-quantifiers

```text
облада́ет
```

### 759 · word · math-23-quantifiers

```text
всео́бщего
```

### 760 · word · math-23-quantifiers

```text
утвержде́ния
```

### 761 · word · math-23-quantifiers

```text
сло́ва
```

### 762 · word · math-23-quantifiers

```text
означа́ют
```

### 763 · word · math-23-quantifiers

```text
сто́роны
```

### 764 · word · math-23-quantifiers

```text
запи́санной
```

### 765 · word · math-23-quantifiers

```text
ло́жны
```

### 766 · word · math-23-quantifiers

```text
облада́ть
```

### 767 · display-token · math-24-information

```text
log₂
```

### 768 · display-token · math-24-information

```text
p
```

### 769 · display-token · math-24-information

```text
−log₂
```

### 770 · title · math-24-information

```text
Три подхо́да к информа́ции
```

### 771 · passage · math-24-information

```text
Коли́чество информа́ции мо́жно определя́ть по-ра́зному. В комбинато́рном подхо́де мы счита́ем допусти́мые вариа́нты. Для вы́бора одного́ из N вариа́нтов есте́ственна ме́ра log₂ N. Вероя́тностный подхо́д учиты́вает распределе́ние вероя́тностей: при p > 0 ре́дкое собы́тие несёт бо́льше информа́ции, измеря́емой величино́й −log₂ p. Усредня́я э́ту величину́, получа́ем энтропи́ю.
Алгоритми́ческий подхо́д свя́зывает сло́жность конкре́тной стро́ки с длино́й кратча́йшей програ́ммы, печа́тающей э́ту стро́ку и заверша́ющей рабо́ту. Оптима́льный универса́льный язы́к описа́ния зафикси́рован зара́нее. При сме́не тако́го языка́ сло́жность меня́ется не бо́лее чем на конста́нту, зави́сящую от двух языко́в, но не от стро́ки. Э́та сло́жность в о́бщем слу́чае невычисли́ма. На́ше кра́ткое сопоставле́ние развива́ет разли́чие трёх подхо́дов Колмого́рова. Оно́ не отождествля́ет сло́жность едини́чной стро́ки с энтропи́ей распределе́ния и не измеря́ет смы́словую це́нность сообще́ния.
```

### 772 · how_read · math-24-information/how_read/1

```text
информа́ции, измеря́емой величино́й
```

### 773 · how_read · math-24-information/how_read/2

```text
програ́ммы, печа́тающей … и заверша́ющей …
```

### 774 · how_read · math-24-information/how_read/3

```text
конста́нту, зави́сящую от …
```

### 775 · how_read · math-24-information/how_read/4

```text
усредня́я …, получа́ем …
```

### 776 · notation · math-24-information/notation/1

```text
логари́фм N по основа́нию два
```

### 777 · notation · math-24-information/notation/2

```text
ми́нус логари́фм p по основа́нию два
```

### 778 · word · math-24-information

```text
подхо́да
```

### 779 · word · math-24-information

```text
информа́ции
```

### 780 · word · math-24-information

```text
коли́чество
```

### 781 · word · math-24-information

```text
определя́ть
```

### 782 · word · math-24-information

```text
по-ра́зному
```

### 783 · word · math-24-information

```text
комбинато́рном
```

### 784 · word · math-24-information

```text
подхо́де
```

### 785 · word · math-24-information

```text
допусти́мые
```

### 786 · word · math-24-information

```text
вариа́нты
```

### 787 · word · math-24-information

```text
вариа́нтов
```

### 788 · word · math-24-information

```text
есте́ственна
```

### 789 · word · math-24-information

```text
вероя́тностный
```

### 790 · word · math-24-information

```text
подхо́д
```

### 791 · word · math-24-information

```text
учиты́вает
```

### 792 · word · math-24-information

```text
распределе́ние
```

### 793 · word · math-24-information

```text
вероя́тностей
```

### 794 · word · math-24-information

```text
ре́дкое
```

### 795 · word · math-24-information

```text
несёт
```

### 796 · word · math-24-information

```text
измеря́емой
```

### 797 · word · math-24-information

```text
усредня́я
```

### 798 · word · math-24-information

```text
величину́
```

### 799 · word · math-24-information

```text
энтропи́ю
```

### 800 · word · math-24-information

```text
алгоритми́ческий
```

### 801 · word · math-24-information

```text
свя́зывает
```

### 802 · word · math-24-information

```text
сло́жность
```

### 803 · word · math-24-information

```text
конкре́тной
```

### 804 · word · math-24-information

```text
стро́ки
```

### 805 · word · math-24-information

```text
длино́й
```

### 806 · word · math-24-information

```text
кратча́йшей
```

### 807 · word · math-24-information

```text
програ́ммы
```

### 808 · word · math-24-information

```text
печа́тающей
```

### 809 · word · math-24-information

```text
стро́ку
```

### 810 · word · math-24-information

```text
заверша́ющей
```

### 811 · word · math-24-information

```text
оптима́льный
```

### 812 · word · math-24-information

```text
универса́льный
```

### 813 · word · math-24-information

```text
язы́к
```

### 814 · word · math-24-information

```text
описа́ния
```

### 815 · word · math-24-information

```text
зафикси́рован
```

### 816 · word · math-24-information

```text
зара́нее
```

### 817 · word · math-24-information

```text
сме́не
```

### 818 · word · math-24-information

```text
языка́
```

### 819 · word · math-24-information

```text
меня́ется
```

### 820 · word · math-24-information

```text
конста́нту
```

### 821 · word · math-24-information

```text
зави́сящую
```

### 822 · word · math-24-information

```text
языко́в
```

### 823 · word · math-24-information

```text
о́бщем
```

### 824 · word · math-24-information

```text
невычисли́ма
```

### 825 · word · math-24-information

```text
кра́ткое
```

### 826 · word · math-24-information

```text
сопоставле́ние
```

### 827 · word · math-24-information

```text
развива́ет
```

### 828 · word · math-24-information

```text
разли́чие
```

### 829 · word · math-24-information

```text
подхо́дов
```

### 830 · word · math-24-information

```text
колмого́рова
```

### 831 · word · math-24-information

```text
отождествля́ет
```

### 832 · word · math-24-information

```text
едини́чной
```

### 833 · word · math-24-information

```text
энтропи́ей
```

### 834 · word · math-24-information

```text
распределе́ния
```

### 835 · word · math-24-information

```text
измеря́ет
```

### 836 · word · math-24-information

```text
смы́словую
```

### 837 · word · math-24-information

```text
це́нность
```

### 838 · word · math-24-information

```text
сообще́ния
```

### 839 · word · math-24-information

```text
логари́фм
```

### 840 · word · math-24-information

```text
основа́нию
```

