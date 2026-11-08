# Deputy Editor Report — Units 7–30, Morphology, Tutor Prompts & Russian UI

**Editor:** Gemini 4 Argon (Deputy Editor)  
**Branch:** `deputy-gemini-course`  
**Scope:** Units 7–30 (`apple/scripts/build_course.py`, `apple/scripts/course_reading.py`, `apple/scripts/course_families.py`, `apple/scripts/course_read.py`), Morphology & Cognates (`apple/scripts/build_morphology.py`), Tutor System Prompts (`Tutor.swift`, `PageWriter.swift`, `DiaryTutor.swift`, `TalkTutor.swift`), and Hardcoded Russian UI Strings (`apple/Drussky/**/*.swift`).

---

## 1. Line-by-Line Fixes (`unit/page id — before → after — why`)

### Unit 11 (`apple/scripts/build_course.py`)
- **`u11-p1` ("Reading: я чита́ю" — `explain.text`)**
  - **Before:** `"Drop -ть from чита'ть and add the ending for who is doing it: я -ю, ты -ешь, он and она' -ет."`
  - **After:** `"Drop -ть from чита'ть and add the ending for who is doing it: я -ю, ты -ешь, он and она' -ет. Notice that кни'га becomes кни'гу when it is the thing being read."`
  - **Why:** `u11-p1` introduces `чита'ю кни'гу`, `Я чита'ю кни'гу.`, and `Он чита'ет кни'гу.` before `u18-p3` ("I see: ви́жу + what") teaches the `-а → -у` accusative object rule. Adding one sentence to the `explain` block prevents an unexplained ending change on `кни'га`.

### Unit 12 (`apple/scripts/build_course.py`)
- **`u12-p2` ("In Russian: по-ру́сски" — `explain.text`)**
  - **Before:** `"Понима'ть, to understand, goes like чита'ть: я понима'ю, ты понима'ешь. По-ру'сски and по-англи'йски name the language."`
  - **After:** `"You already know я не понима'ю; the full verb понима'ть (to understand) goes like чита'ть: я понима'ю, ты понима'ешь. По-ру'сски and по-англи'йски name the language."`
  - **Why:** In `UNIT_ORDER`, `c1` ("Keeping up: я не понима́ю · ещё раз · как по-ру́сски?") is placed 8 units *before* `u12`. Acknowledging `я не понима'ю` connects the survival chunk to the full `-ать` paradigm instead of presenting it as if unseen.

### Unit 16 (`apple/scripts/build_course.py`)
- **`LEXICON` (Unit 16 entry)**
  - **Before:** `W("пла'нета", "planet", "🪐")`
  - **After:** `W("плане'та", "planet", "🪐")`
  - **Why:** Wrong stress. In Russian, «плане́та» is stressed on the second syllable (`-не́-`), never `*пла́нета`.
- **`u16-p1` ("Robots and rockets" — `explain.examples`, `listen_pick`, `translate`)**
  - **Before:** `("пла'нета", "planet")`, `lp("пла'нета", ["пла'нета", "раке'та", "звезда'"])`, `tl("planet", "пла'нета")`
  - **After:** `("плане'та", "planet")`, `lp("плане'та", ["плане'та", "раке'та", "звезда'"])`, `tl("planet", "плане'та")`
  - **Why:** Fixes the stress of «плане́та» across all exercises on the page.

### Unit 18 (`apple/scripts/course_reading.py`)
- **`u18-p2` ("In the city, in space" — intro, `explain`, `fill`) & `LEXICON`**
  - **Before:**
    - Intro: `"Feminine and masculine alike take -е: в го́роде, на пла́нете, в ко́смосе."`
    - `explain`: `"... на пла'нете ..."` and `("на пла'нете", "on the planet")`
    - `fill`: `fl("Ро'бот на ", "пла'нете", ".", "The robot is on the planet.")`
    - `LEXICON`: `W("на пла'нете", "on the planet")`
  - **After:**
    - Intro: `"Feminine and masculine alike take -е: в го́роде, на плане́те, в ко́смосе."`
    - `explain`: `"... на плане'те ..."` and `("на плане'те", "on the planet")`
    - `fill`: `fl("Ро'бот на ", "плане'те", ".", "The robot is on the planet.")`
    - `LEXICON`: `W("на плане'те", "on the planet")`
  - **Why:** Fixes the stress of «на плане́те» (`плане́та` has fixed stem stress on `-не́-`).
- **`u18-p3` ("I see: ви́жу + what" — `explain.text`, `read_aloud`)**
  - **Before:**
    - `explain.text`: `"For the object, -а becomes -у and -я becomes -ю; masculine and neuter things stay as they are: я ви'жу стол, я ви'жу окно', я ви'жу кни'гу. People and animals change too: я ви'жу бра'та."`
    - `read_aloud`: `ra("Я ви'жу звезду' и пла'нету.")`
  - **After:**
    - `explain.text`: `"For the object, -а becomes -у and -я becomes -ю; inanimate masculine and neuter nouns stay as they are: я ви'жу стол, я ви'жу окно', я ви'жу кни'гу. Living masculine nouns take -а instead: брат, я ви'жу бра'та."`
    - `read_aloud`: `ra("Я ви'жу звезду' и плане'ту.")`
  - **Why:**
    1. Explicitly states the animate masculine accusative rule (`living masculine nouns take -а instead: брат, я ви'жу бра'та`) rather than the vague `"People and animals change too"`.
    2. Fixes the stress of «плане́ту» in the `read_aloud` sentence.
- **`u18-p4` ("The station" — exercise order & `explain.text`)**
  - **Before:** `ps("Ста'нция в ко'смосе... Он ви'дит Зе'млю в окне'. Ещё он ви'дит звёзды и Луну'...")` appeared *before* `ex("As the object, Земля' becomes Зе'млю and Луна' becomes Луну'.", ...)`.
  - **After:** Moved `ex("As the object, Земля' becomes Зе'млю (note the stress shift) and Луна' becomes Луну'.", [("Земля'", "the Earth"), ("Луна'", "the Moon")])` to the top of `u18-p4` (before `ps`) and noted the stress shift `Земля́ → Зе́млю`.
  - **Why:** A learner should not encounter `Зе'млю` (with a retracted stress shift from `Земля'`) and `Луну'` inside a reading passage before the card that introduces `Земля'` and `Луна'` and explains their accusative forms.

### Unit 21 (`apple/scripts/course_reading.py`)
- **`u21-p4` ("A tiny proof" — `passage.text`, `translate`) & `LEXICON`**
  - **Before:** `Тео'рема: су'мма двух чётных чи'сел — чётное число'...`, `tl("theorem", "тео'рема")`, `W("тео'рема", "theorem")`
  - **After:** `Теоре'ма: су'мма двух чётных чи'сел — чётное число'...`, `tl("theorem", "теоре'ма")`, `W("теоре'ма", "theorem")`
  - **Why:** Wrong stress. In Russian, «теоре́ма» is stressed on `-ре́-` (unlike «тео́рия»), never `*тео́рема`. (Note: `COGNATES` in `build_morphology.py` already had `теоре'ма` on line 857 while `LEMMAS` line 652 and `course_reading.py` had `тео'рема`.)

### Unit 22 (`apple/scripts/course_reading.py`)
- **`u22-p3` ("A dialogue" — `passage.text`)**
  - **Before:** `— Тогда' сна'чала ну'жно узна'ть, что тако'е му'дрость.`
  - **After:** `— Тогда' снача'ла ну'жно узна'ть, что тако'е му'дрость.`
  - **Why:** Wrong stress. In Russian, «снача́ла» is stressed on the second syllable (`-ча́-`), never `*сна́чала`.

### Unit 24 (`apple/scripts/course_read.py`)
- **`u24-p5` ("Six cases, one story" — `explain.text`)**
  - **Before:** `"... вини'тельный (кого'? что?, the object), ..."`
  - **After:** `"... вини'тельный (кого'? что?, the object — like роди'тельный for living masculine nouns), ..."`
  - **Why:** Connects `кого'?` in the accusative row of the six-case summary directly to the animate masculine accusative = genitive syncretism (`бра'та`, `челове'ка`).

### Unit 29 (`apple/scripts/course_read.py`)
- **`u29-p2` ("Theorem and proof: пусть, тогда́" — `copy`)**
  - **Before:** `cp("тео'рема, ле'мма, сле'дствие, доказа'тельство", 1)`
  - **After:** `cp("теоре'ма, ле'мма, сле'дствие, доказа'тельство", 1)`
  - **Why:** Wrong stress (`*тео́рема` → `теоре́ма`).
- **`u29-p3` ("For all, there exists, equals" — `explain`, `read_meaning`, `copy`, `DIARY`) & `LEXICON`**
  - **Before:**
    - `explain`: `зна'чение фу'нкции в то'чке` and `("зна'чение фу'нкции в то'чке два", "the value of the function at the point two")`
    - `read_meaning`: `rm("Зна'чение фу'нкции в то'чке два равно' четырём.", ...)`
    - `copy`: `cp("зна'чение фу'нкции в то'чке", 1)`
    - `DIARY["u29-p3"]`: `"Three true statements with бо'льше, ме'ньше and ра'вно."`
    - `LEXICON`: `W("зна'чение", "value; meaning")`, `W("зна'чение фу'нкции", "the value of the function")`
  - **After:**
    - `explain`: `значе'ние фу'нкции в то'чке` and `("значе'ние фу'нкции в то'чке два", "the value of the function at the point two")`
    - `read_meaning`: `rm("Значе'ние фу'нкции в то'чке два равно' четырём.", ...)`
    - `copy`: `cp("значе'ние фу'нкции в то'чке", 1)`
    - `DIARY["u29-p3"]`: `"Three true statements with бо'льше, ме'ньше and равно'."`
    - `LEXICON`: `W("значе'ние", "value; meaning")`, `W("значе'ние фу'нкции", "the value of the function")`
  - **Why:**
    1. Wrong stress on «значе́ние»: in Russian it is always `значе́ние` (stressed on `-че́-`, as in `build_morphology.py` line 371), never `*зна́чение`.
    2. Wrong stress on «равно́» in the `u29-p3` diary prompt (`*ра́вно` → `равно́`).
- **`u29-p4` ("A real proof: Euclid" — `passage.text`)**
  - **Before:** `ps("Тео'рема: просты'х чи'сел бесконе'чно мно'го.\n...")`
  - **After:** `ps("Теоре'ма: просты'х чи'сел бесконе'чно мно'го.\n...")`
  - **Why:** Wrong stress (`*Тео́рема` → `Теоре́ма`).

### Unit 30 (`apple/scripts/course_read.py`)
- **`u30-p1` ("Abstract nouns" — `explain`, `read_aloud`, `read_meaning`) & `LEXICON`**
  - **Before:**
    - `explain`: `(де'йствительный, де'йствительность)` and `("де'йствительность", "reality, actuality")`
    - `read_aloud`: `ra("И'стина, свобо'да, созна'ние, бытие', мышле'ние, су'щность, де'йствительность, возмо'жность, необходи'мость, ра'зум.")`
    - `read_meaning`: `rm("Возмо'жность — ещё не де'йствительность.", ...)`
    - `LEXICON`: `W("де'йствительность", "reality, actuality")`
  - **After:**
    - `explain`: `(действи'тельный, действи'тельность)` and `("действи'тельность", "reality, actuality")`
    - `read_aloud`: `ra("И'стина, свобо'да, созна'ние, бытие', мышле'ние, су'щность, действи'тельность, возмо'жность, необходи'мость, ра'зум.")`
    - `read_meaning`: `rm("Возмо'жность — ещё не действи'тельность.", ...)`
    - `LEXICON`: `W("действи'тельность", "reality, actuality")`
  - **Why:** Wrong stress. While «де́йствие» and «де́йствовать» are stressed on `де́й-`, the derivatives «действи́тельный», «действи́тельность», and «действи́тельно» shift stress to `-ви́-`. `*де́йствительность` does not exist in Russian.
- **`u30-p3` ("Connecting an argument" — `explain`, `dictation`) & `LEXICON`**
  - **Before:**
    - `explain`: `... де'йствительно (indeed).` and `("Де'йствительно, э'то так.", "Indeed, that is so.")`
    - `dictation`: `dc("Де'йствительно, э'то так. Одна'ко э'то тру'дно.")`
    - `LEXICON`: `W("де'йствительно", "indeed, really")`
  - **After:**
    - `explain`: `... действи'тельно (indeed).` and `("Действи'тельно, э'то так.", "Indeed, that is so.")`
    - `dictation`: `dc("Действи'тельно, э'то так. Одна'ко э'то тру'дно.")`
    - `LEXICON`: `W("действи'тельно", "indeed, really")`
  - **Why:** Wrong stress (`*де́йствительно` → `действи́тельно`).

---

### Morphology, Word Families & Cognates (`apple/scripts/build_morphology.py`)
Because Unit 23 (`course_families.py`) and the in-app **Word Families / Cognates** views draw directly from `morphology.json` (`build_morphology.py`), every Russian word and example in `build_morphology.py` was audited:
1. **`ROOTS` (`дей`, line 223):** `де'йствительность ... де'йствительно` → `действи'тельность ... действи'тельно` (wrong stress).
2. **`PREFIXES` (`над`, line 254):** `"надпи'сь"` → `"на'дпись"` (in Russian, «на́дпись» is stressed on the prefix `на́д-`, never `*надпи́сь`).
3. **`SUFFIXES` (`ец`, line 290):** `"коне'ц учени'ец"` → `"коне'ц мудре'ц"` (`*учени́ец` is not a Russian word; «учени́к» uses `-ник`, whereas «мудре́ц» from `му'дрый` is a real `-ец` noun built from a root the learner knows).
4. **`LEMMAS` (`узна'ть`, line 368):** `Сна'чала ну'жно узна'ть...` → `Снача'ла ну'жно узна'ть...` (wrong stress `*сна́чала` → `снача́ла`).
5. **`LEMMAS` (`переписа'ть`, line 421):** `example=("Переписа'й э'то сло'во.", "Copy out this word.")` → `example=("Перепиши' э'то сло'во.", "Copy out this word.")` (`*Переписа́й` is a broken non-native imperative; `писа'ть / переписа'ть` alternates `с → ш` in the present/imperative: `перепиши́`).
6. **`LEMMAS` (`след`, line 439):** `example=("На снегу' след ко'та.", "On the snow, a cat's track.")` → `example=("На снегу' след кота'.", "On the snow, a cat's track.")` (`кот` has end-stressed oblique forms: `кота́`, `коту́`, `кото́м`, never `*ко́та`).
7. **`LEMMAS` (`окру'жность`, line 600):** `example=("Дли'на окру'жности.", ...)` → `example=("Длина' окру'жности.", ...)` (the noun «длина́» "length" is end-stressed, never `*дли́на`, which is the short feminine of `дли'нный`).
8. **`LEMMAS` (`равно'`, line 606):** `M("[ра'вн]|о", "adv", "equals; equally")` → `M("[равн]|о'", "adv", "equals; equally")` («равно́» is end-stressed, never `*ра́вно`).
9. **`LEMMAS` (`теоре'ма`, line 652):** `("тео'рема", "noun", "theorem", ...)` → `("теоре'ма", "noun", "theorem", ...)` (wrong stress).
10. **`FORMS` (lines 709, 713, 724, 726):** Fixed source stress on `учи'тель`, `ма'льчик`, `плане'та` (and added `планету`), `писа'ть`.
11. **`LEMMAS` (`противоре'чие`, line 748):** `M("проти'во|[реч]|и|е", ...)` → `M("противо|[ре'ч]|и|е", ...)` (primary stress in «противоре́чие» is on `-ре́ч-`, matching its example sentence on the same line).
12. **`LEMMAS` (`сле'дствие`, line 749):** `example=("Сле'дствие тео'ремы.", ...)` → `example=("Сле'дствие теоре'мы.", ...)` (wrong stress `*тео́ремы` → `теоре́мы`).
13. **`LEMMAS` (`откры'ть`, line 772):** `example=("Откры'й окно'.", "Open the window.")` → `example=("Откро'й окно'.", "Open the window.")` (`*Откры́й` is a broken non-native imperative; `крыть / откры'ть` forms its stem in `-кро'й-`: `откро́й`).
14. **`LEMMAS` (`действи'тельность`, `действи'тельно`, lines 798–799):** `M("[де'й]|ств|и|тельн|ость", ...)` → `M("[дей]|ств|и'|тельн|ость", ...)` and `M("[дей]|ств|и'|тельн|о", ...)`.
15. **`COGNATES` & `RULES` headword stress fixes (lines 887, 925, 955, 1004, 1011):**
    - `хао'с` → `ха'ос` (in modern scientific and philosophical Russian, «ха́ос» is stressed on the first syllable).
    - `ска'ляр` → `скаля'р` (in Russian mathematics, «скаля́р» is stressed on `-ля́р`, never `*ска́ляр`).
    - `ква'драт` → `квадра'т` (in Russian, «квадра́т» is stressed on `-ра́т`, never `*ква́драт`).
16. **`COGNATES` `build_word` tile stress fixes (lines 859, 864–867, 877, 886, 964, 986):**
    - Five cognates had stress placed on the **wrong tile** (contradicting their own headword stress!):
      - `фило'соф`: `[("фило-", ...), ("-со'ф", ...)]` → `[("фило'-", ...), ("-соф", ...)]`
      - `гипо'теза`: `[("гипо-", ...), ("-те'за", ...)]` → `[("гипо'-", ...), ("-теза", ...)]`
      - `пара'метр`: `[("пара-", ...), ("-ме'тр", ...)]` → `[("пара'-", ...), ("-метр", ...)]`
      - `мета'фора`: `[("мета-", ...), ("-фо'ра", ...)]` → `[("мета'-", ...), ("-фора", ...)]`
      - `пери'од`: `[("пери-", ...), ("-о'д", ...)]` → `[("пери'-", ...), ("-од", ...)]`
    - Four cognates had **missing stress** on their tiles:
      - `ана'лиз`: `[("ана'-", ...), ("-лиз", ...)]`
      - `си'нтез`: `[("си'н-", ...), ("-тез", ...)]`
      - `ме'тод`: `[("ме'т-", ...), ("-од", ...)]`
      - `а'том`: `[("а'-", ...), ("-том", ...)]`

---

## 2. Complete List of Changed Spoken Strings (For Audio Revoicing)

Every string below appears in an audio-bearing field (`say`, `text`, `examples.ru`, `answer`, `lexicon.ru`, or `morphology` headword/example) and had its stress or wording corrected:

| Location | Field | Before (with acute `́`) | After (with acute `́`) |
|---|---|---|---|
| `LEXICON` (`u16`), `u16-p1` (`ex`, `lp`, `tl`), `morphology` | word / `say` / `answer` | `пла́нета` | `плане́та` |
| `LEXICON` (`u18`), `u18-p2` (`ex`, `fl`) | word / `examples.ru` / `answer` | `на пла́нете` | `на плане́те` |
| `u18-p2` (`fl` full sentence) | `before + answer + after` | `Ро́бот на пла́нете.` | `Ро́бот на плане́те.` |
| `u18-p3` (`ra`) | `text` | `Я ви́жу звезду́ и пла́нету.` | `Я ви́жу звезду́ и плане́ту.` |
| `LEXICON` (`u21`), `u21-p4` (`tl`), `morphology` | word / `answer` | `тео́рема` | `теоре́ма` |
| `u21-p4` (`ps`) | `text` | `Тео́рема: су́мма двух чётных чи́сел — чётное число́. Доказа́тельство. Чётное число́ — э́то число́, кото́рое де́лится на два. Возьмём два чётных числа́. Ка́ждое из них — два, умно́женное на це́лое число́. Их су́мма — два, умно́женное на су́мму э́тих це́лых чи́сел. Зна́чит, су́мма то́же де́лится на два, то есть она́ чётная. Что и тре́бовалось доказа́ть.` | `Теоре́ма: су́мма двух чётных чи́сел — чётное число́. Доказа́тельство. Чётное число́ — э́то число́, кото́рое де́лится на два. Возьмём два чётных числа́. Ка́ждое из них — два, умно́женное на це́лое число́. Их су́мма — два, умно́женное на су́мму э́тих це́лых чи́сел. Зна́чит, су́мма то́же де́лится на два, то есть она́ чётная. Что и тре́бовалось доказа́ть.` |
| `u22-p3` (`ps`) | `text` | `… — Тогда́ сна́чала ну́жно узна́ть, что тако́е му́дрость.` | `… — Тогда́ снача́ла ну́жно узна́ть, что тако́е му́дрость.` |
| `u29-p2` (`cp`) | `text` | `тео́рема, ле́мма, сле́дствие, доказа́тельство` | `теоре́ма, ле́мма, сле́дствие, доказа́тельство` |
| `LEXICON` (`u29`) | word | `зна́чение` | `значе́ние` |
| `LEXICON` (`u29`) | word | `зна́чение фу́нкции` | `значе́ние фу́нкции` |
| `u29-p3` (`cp`) | `text` | `зна́чение фу́нкции в то́чке` | `значе́ние фу́нкции в то́чке` |
| `u29-p3` (`ex`) | `examples.ru` | `зна́чение фу́нкции в то́чке два` | `значе́ние фу́нкции в то́чке два` |
| `u29-p3` (`rm`) | `text` | `Зна́чение фу́нкции в то́чке два равно́ четырём.` | `Значе́ние фу́нкции в то́чке два равно́ четырём.` |
| `u29-p4` (`ps`) | `text` | `Тео́рема: просты́х чи́сел бесконе́чно мно́го. …` | `Теоре́ма: просты́х чи́сел бесконе́чно мно́го. …` |
| `LEXICON` (`u30`), `u30-p1` (`ex`) | word / `examples.ru` | `де́йствительность` | `действи́тельность` |
| `u30-p1` (`ra`) | `text` | `И́стина, свобо́да, созна́ние, бытие́, мышле́ние, су́щность, де́йствительность, возмо́жность, необходи́мость, ра́зум.` | `И́стина, свобо́да, созна́ние, бытие́, мышле́ние, су́щность, действи́тельность, возмо́жность, необходи́мость, ра́зум.` |
| `u30-p1` (`rm`) | `text` | `Возмо́жность — ещё не де́йствительность.` | `Возмо́жность — ещё не действи́тельность.` |
| `LEXICON` (`u30`) | word | `де́йствительно` | `действи́тельно` |
| `u30-p3` (`ex`) | `examples.ru` | `Де́йствительно, э́то так.` | `Действи́тельно, э́то так.` |
| `u30-p3` (`dc`) | `text` | `Де́йствительно, э́то так. Одна́ко э́то тру́дно.` | `Действи́тельно, э́то так. Одна́ко э́то тру́дно.` |
| `build_morphology.py` | lemma / cognate / example | `надпи́сь`, `учени́ец`, `Сна́чала ну́жно узна́ть, что тако́е му́дрость.`, `Переписа́й э́то сло́во.`, `На снегу́ след ко́та.`, `Дли́на окру́жности.`, `ра́вно`, `проти́воречие`, `Сле́дствие тео́ремы.`, `Откры́й окно́.`, `хао́с`, `ска́ляр`, `ква́драт` | `на́дпись`, `мудре́ц`, `Снача́ла ну́жно узна́ть, что тако́е му́дрость.`, `Перепиши́ э́то сло́во.`, `На снегу́ след кота́.`, `Длина́ окру́жности.`, `равно́`, `противоре́чие`, `Сле́дствие теоре́мы.`, `Откро́й окно́.`, `ха́ос`, `скаля́р`, `квадра́т` |

---

## 3. Tutor Prompts & Hardcoded Russian UI Strings

### Register Decision: Addressing Ember (`вы` vs. `ты`)
- **Choice:** **`вы`** whenever the app or tutor speaks in its own voice as Ember's teacher (`репети́тор`), while teaching and accepting **`ты`** for home/kitchen dialogues with a housemate or family member.
- **Why:**
  1. In real Russian pedagogy, a private tutor addressing an adult student uses respectful `вы` («Напиши́те…», «Что вы ска́жете?», «Вы хоте́ли сказа́ть…?»). Gamified apps use faux-chummy `ты` with adult strangers; a serious Russian workbook does not.
  2. The app's existing UI (`Напиши́те…`, `Что вы написа́ли?`, `Что вы ска́жете?`, `Вста́вьте текст…`) and `TalkTutor.swift` (`«Вы хоте́ли сказа́ть …?»`) already use `вы`.
  3. Making this rule explicit in `Tutor.swift`, `PageWriter.swift`, and `TalkTutor.swift` prevents the LLM from randomly drifting between `ты` and `вы` within the same session.

### Prompt & UI Fixes (`file:line — before → after — why`)
- **`apple/Drussky/Services/Tutor.swift:62-69`**
  - **Before:** Instructed stress mark `(´)` without specifying `U+0301` or forbidding stress on `ё`/monosyllables, and gave no guidance on `вы` vs. `ты` or English-calqued word order.
  - **After:** Specifies `U+0301` on words of 2+ syllables (never on `ё` or one-syllable words), requires idiomatic non-calqued examples, and standardizes on `вы` when addressing the learner as tutor while explaining `ты` for home/kitchen Russian.
- **`apple/Drussky/Services/PageWriter.swift:79-81`**
  - **Before:** `"Russian must be correct and natural."`
  - **After:** `"Russian must be correct, idiomatic and natural (never calqued from English word order, and never mixing ты and вы within a dialogue)."`
- **`apple/Drussky/Diary/DiaryTutor.swift:25, 35-37`**
  - **Before:** `"never on ё"` and `"keep their meaning and their words wherever they are right; fix what is wrong, never restyle."`
  - **After:** Added `"never on ё or one-syllable words"` and `"Accept natural Russian word order and omitted subject pronouns (e.g. «Чита́ю кни́гу.») when grammatical; do not force English SVO order."`
- **`apple/Drussky/Talk/TalkTutor.swift:47, 91-92, 323-327`**
  - **Before:** Missing `"or one-syllable words"` in stress instructions; `markSystem` did not remind the marker to accept natural spoken Russian word order and omitted subject pronouns.
  - **After:** Added explicit one-syllable stress prohibition, reinforced consistent `вы` without mixing `ты`/`вы`, and instructed `markSystem` to accept natural spoken word order and pronoun drop.
- **`apple/Drussky/Diary/DiaryExercise.swift:73`**
  - **Before:** `Text("Дневник")`
  - **After:** `Text("Дневни́к")`
  - **Why:** Missing stress mark on polysyllabic Russian UI header.
- **`apple/Drussky/Diary/DiaryBook.swift:64` & `apple/Drussky/Diary/QA+Diary.swift:36`**
  - **Before:** `"Дневник · Diary"`
  - **After:** `"Дневни́к · Diary"`
  - **Why:** Missing stress mark on the inserted Diary page title.
- **`apple/Drussky/Views/PageView.swift:87`**
  - **Before:** `Text(page.isReview ? "Повторение" : "Классная работа")`
  - **After:** `Text(page.isReview ? "Повторе́ние" : "Кла́ссная рабо́та")`
  - **Why:** Missing stress marks on top-right cursive workbook page headers.
- **`apple/Drussky/Views/PageView.swift:209-211`**
  - **Before:** `"Отлично!"`, `"Хорошо!"`, `"Молодец — дальше!"`
  - **After:** `"Отли́чно!"`, `"Хорошо́!"`, `"Молоде́ц — да́льше!"`
  - **Why:** Missing stress marks on teacher's red-pen grade stamps (note: `case nil: "Готово!"` on line 212 was left unstressed because `DiaryTests.swift:195` asserts exact equality `XCTAssertEqual(GradeStamp.praise(for: nothingAssessed), "Готово!")`).
- **`apple/Drussky/Views/ContentsView.swift:28`**
  - **Before:** `.navigationTitle("Алфавит")`
  - **After:** `.navigationTitle("Алфави́т")`
  - **Why:** Missing stress mark on navigation title.
- **`apple/Drussky/Views/TutorView.swift:204`**
  - **Before:** `Text("Вопросы")`
  - **After:** `Text("Вопро́сы")`
  - **Why:** Missing stress mark on margin notes section title.
- **`apple/Drussky/Views/WritePageView.swift:61` & `apple/Drussky/Goal/GoalViews.swift:244`**
  - **Before:** `Text("Вставьте текст…")`
  - **After:** `Text("Вста́вьте текст…")`
  - **Why:** Missing stress mark on Russian text editor placeholder.
- **`apple/Drussky/Day/Book+Day.swift:41-43`**
  - **Before:** `"Новый день · New day"`, `"С возвращением · Welcome back"`, `"Подготовка · Getting ready"`
  - **After:** `"Но́вый день · New day"`, `"С возвраще́нием · Welcome back"`, `"Подгото́вка · Getting ready"`
  - **Why:** Missing stress marks on daily page unit headers.

---

## 4. Pedagogical Ordering Notes

Because `build_course.py` decouples authoring numbers from presentation order via `UNIT_ORDER` and `PAGE_MOVES`:
```python
UNIT_ORDER = [
    "u1", "u2", "u3", "u4", "u5", "u6", "u7",
    "u13", "c1", "u14", "u8", "u15", "u9", "u10",
    "u11", "u18", "u12", "u16", "u17", "u19", "u20",
    "u21", "u22", "u23", "u24", "u25", "u26", "u27",
    "u28", "u29", "u30",
]
PAGE_MOVES = [
    ("u19-p1", "u17", "u17-p1"),  # today/yesterday/tomorrow before the past tense
    ("u22-p1", "u12", None),      # что-clauses and why/because as everyday tools
    ("u22-p2", "u12", None),
]
```
several pedagogical seams arise from that reordering:

1. **Accusative `-а → -у` appears in `u11-p1` before `u18-p3` (Fixed in place):**
   - `u11` precedes `u18` in `UNIT_ORDER`, so `u11-p1` introduces `чита'ю кни'гу`, `Я чита'ю кни'гу.`, and `Он чита'ет кни'гу.` before `u18-p3` ("I see: ви́жу + what") explains that `-а` becomes `-у` for direct objects.
   - **Fix applied:** Added a one-sentence explanation directly to `u11-p1`'s `explain` block (`"Notice that кни'га becomes кни'гу when it is the thing being read."`).
2. **Animate masculine accusative in `u18-p3` and `u24-p5` (Fixed in place):**
   - `u18-p3` previously said `"People and animals change too: я ви'жу бра'та."` without stating the actual rule (animate masculine objects take `-а`, matching the genitive).
   - **Fix applied:** Clarified `u18-p3` (`"Living masculine nouns take -а instead: брат, я ви'жу бра'та."`) and `u24-p5` (`"вини'тельный (кого'? что?, the object — like роди'тельный for living masculine nouns)"`).
3. **`Земля́ → Зе́млю` and `Луна́ → Луну́` on `u18-p4` appeared *after* the passage that uses them (Fixed in place):**
   - On `u18-p4`, the passage `ps(...)` was the first item on the page, while the `explain` card introducing `Земля'` and `Луна'` and their accusative forms `Зе'млю` / `Луну'` was third.
   - **Fix applied:** Moved `ex(...)` before `ps(...)` on `u18-p4` and noted the stress shift `Земля́ → Зе́млю`.
4. **`c1-p1` ("Keeping up: я не понима́ю") precedes `u12-p2` ("In Russian: по-ру́сски") by 8 units (Fixed in place):**
   - Because `c1` was moved early (right after `u13`), the learner already knows `Я не понима'ю.` and `Как э'то по-ру'сски?` long before `u12-p2`.
   - **Fix applied:** Updated `u12-p2`'s `explain` text to connect what they already know (`я не понима'ю`) to the full conjugation of `понима'ть`.
5. **Kitchen chunks in `u13` and `u14` precede gender/adjectives (`u8`), genitive (`u10`), and instrumental (`u24`):**
   - Moving `u13` ("At the table") and `u14` ("Breakfast and dinner") immediately after `u7` (Alphabet completion) is a smart move for someone living with a Russian speaker — it gives immediate kitchen phrases (`До'брое у'тро!`, `Прия'тного аппети'та!`, `Ко'фе с молоко'м`, `Мне чай, пожа'луйста`).
   - However, when the learner reaches `u8-p5` (`но'вый, но'вая, но'вое`), `u10-p4` (`у меня' нет молока'`), `u24-p2` (`мне`), and `u24-p3` (`с молоко'м`), a one-line callback to the kitchen chunks they already know from `u13`–`u14` makes the grammar feel like an explanation of familiar speech rather than new overhead.
6. **Idiom note on `u24-p4` (`Кот на столе'. Кот идёт на стол.`):**
   - `u24-p4` contrasts `на столе'` (prepositional of location) with `на стол` (accusative of motion) using `Кот идёт на стол.` (`LEXICON` entry `W("на стол", "onto the table")`). While kept intact so the lexicon item and its production invariant remain valid, note that in natural Russian a cat jumps (`пры'гает на стол`) or climbs (`ле'зет на стол`) rather than walks (`идёт на стол`); if `LEXICON` is expanded in a future pass, `Па'па на рабо'те. Па'па идёт на рабо'ту.` or `Кот пры'гает на стол.` would be more idiomatic.

---

## 5. Overall Editorial View of the Course's Russian

### What Level It Really Is
- **Units 1–15 (`u1`–`u7`, `u13`, `c1`, `u14`, `u8`, `u15`, `u9`, `u10`):** Genuine, well-paced **A1 Foundation**. It teaches the 33 letters, phonetics (hard/soft pairs, vowel reduction, final devoicing), gender, plurals, `у меня́ есть / нет`, numbers 1–20, and high-utility domestic Russian.
- **Units 16–23 (`u11`, `u18`, `u12`, `u16`, `u17`, `u19`, `u20`, `u21`, `u22`, `u23`):** **High A1 to Early A2**, with an early jump into domain-specific reading (a 4-part sci-fi story on station *Zvezda*, an even-number sum proof, a Socratic dialogue on justice, Tyutchev's «Умо́м Росси́ю не поня́ть», and morphological root families).
- **Units 24–30 (`u24`–`u30`):** A compressed **A2 → B1/B2 Passive Reading Bridge**. In just 7 units (32 pages), it covers the entire 6-case system, verbal aspect in past/future/imperative, numeral-noun case government (`1 / 2–4 / 5+`), comparatives, short-form predicates, `кото́рый` relative clauses, `-ся` verbs, all four participle types (`-ущий`, `-вший`, `-нный`, `-тый`) and gerunds (`чита́я`, `прочита́в`), formal mathematical proofs (Euclid's infinitude of primes), and original philosophical prose from Tolstoy's *Confession* and Dostoevsky's *The Idiot*.

### Where It Is Too Easy or Too Hard for Weeks 3–8
- **Too Easy / Under-leveraged (Units 8–12, Weeks 2–3):**
  - For a systems engineer and PL/formal-methods thinker who loves structure, `u11` (`-ать` verbs) and `u12` (`-ить` verbs) treat conjugation a bit too much like isolated vocabulary items (`W("мы чита'ем", "we read")`, `W("вы чита'ете", "you (plural) read")`) rather than explicit stem + ending morphology. Ember will grasp the two conjugation classes and consonant mutations (`писа́ть → пишу́`, `люби́ть → люблю́`) much faster if shown the underlying morphophonological rules upfront.
- **Just Right (Units 17–23, Weeks 4–5):**
  - The progression from `u17` (past/future + aspect preview) and `u19` (`идти́ / ходи́ть / е́хать`) into `u20` (Mira's prime-number signal from space), `u21` (quantifiers `для любо́го` / `существу́ет` and a real proof), and `u23` (Russian word families as a derivational graph) is the strongest part of the workbook. It respects the learner's intelligence and builds real reading momentum.
- **Too Steep / Compressed (Units 24–28, Weeks 6–8):**
  - **Unit 24** packs Genitive (`у, из, по́сле, мно́го, ма́ло`), Dative (`кому́, мне нра́вится, ско́лько тебе́ лет`), Instrumental (`с кем, чем, быть/стать кем`), and Prepositional/Accusative into **5 pages** (one page per case). In Russian, case endings are the load-bearing type system of every sentence; 1 page per case is enough to *see* the table, but not enough to make case decoding automatic before hitting `кото́рый` in `u27-p4` and participles in `u28-p2`.
  - **Unit 28 (`u28-p2`)** introduces all four participles (`чита́ющий`, `прочита́вший`, `напи́санный`, `откры́тый`) and both gerunds (`чита́я`, `прочита́в`) on a **single page**. Even for passive recognition, Russian literary and scientific prose relies heavily on participial clauses (`Стена́, отделя́ющая нас от ле́са...`), which need at least 2–3 pages to unpack comfortably.

### Three Things I Would Change About How It Teaches
1. **Unpack Case Endings and Adjective/Pronoun Agreement Before `кото́рый` (Split Unit 24 into 2–3 Units):**
   - Right now, the course teaches noun case endings in `u24`, skips adjective case endings almost entirely, and then drops oblique adjective/pronoun forms (`двух чётных чи́сел`, `э́тих це́лых чи́сел`, `в кото́ром`, `на кото́рой`, `мы́слящим существо́м`, `предстоя́щей мне сме́ртью`) inside reading glosses. Adding a dedicated 3-page unit on **Adjective & Pronoun Case Patterns** (`-ого/-его`, `-ому/-ему`, `-ым/-им`, `-ой/-ей`, `-ых/-их`) would turn dozens of opaque reading glosses into transparent, rule-governed forms.
2. **Treat Verb Stems & Consonant Alternations as a Formal System (Earlier Morphology):**
   - Unit 23 ("Word families") is brilliant, but it arrives after 22 units of treating inflected forms (`я люблю́`, `я пишу́`, `я ви́жу`, `я хожу́`, `перепиши́`) as separate lexical entries. Giving Ember a 1-page "One-Stem / Labial & Dental Alternation" map (`б→бл`, `с→ш`, `д→ж`, `т→ч`) around Unit 12–14 would immediately unify present tense, imperatives, comparatives (`моло́же`), and derived nouns (`мышле́ние`, `движе́ние`).
3. **Bridge Everyday Kitchen Dialogues with Aspect and Particles (`же, ведь, ну, дава́й, мо́жно`):**
   - Because Ember lives with a Russian speaker, spoken Russian at home is full of aspectual imperatives (`Посмотри́!`, `Дава́й поу́жинаем`, `Сде́лай ко́фе`) and conversational particles (`же`, `ведь`, `ну`, `вообще́`). Right now `же` and `ведь` wait until `u28-p3` ("The machinery of written Russian"), when in reality they are the lifeblood of spoken Moscow kitchen Russian. Introducing `дава́й + future/infinitive` and `же / ведь` alongside kitchen dialogues in Units 13–19 would make home conversation click weeks earlier.
