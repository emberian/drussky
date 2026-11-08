# Deputy Report — Native Mathematical Russian Audit (`math.json`)

**Editor**: Gemini 4 Argon (Deputy Editor, `deputy-gemini-math`)  
**Target**: `apple/Drussky/Resources/math.json` (24 graded mathematical readings originally drafted by Codex)

---

## 1. Line-by-Line Fixes in `math.json` (Grouped by Rung)

Rungs `math-07-identity`, `math-11-sqrt-two`, `math-15-open-sets`, and `math-19-sum-theorem` passed the native register, stress, notation, and question audit without requiring changes. Fixes applied across the remaining 20 rungs:

### Rung 1 — `math-01-sets`
- `math-01-sets` — `В мно́жестве нет поря́дка: 1, 2, 3 и 3, 2, 1 — здесь одно́ и то же мно́жество.` → `В мно́жестве нет поря́дка: здесь 1, 2, 3 и 3, 2, 1 — одно́ и то же мно́жество.` — Placed sentence-level scene-setting adverb `здесь` before the compound subject rather than wedging it immediately after the copular dash.
- `math-01-sets` — `В матема́тике ва́жно, что есть в мно́жестве, а не где э́то на страни́це.` → `В матема́тике ва́жно, что есть в мно́жестве, а не где элеме́нты на страни́це.` (`en`: `not where it is on the page` → `not where the elements are on the page`) — Using bare neuter `э́то` as an anaphoric pronoun referring back to a `что`-clause (`where it is on the page`) is an English calque; naming `элеме́нты` makes the contrast idiomatic and concrete at `u7-p4`.

### Rung 2 — `math-02-elements`
- `math-02-elements` — `Здесь ва́жен не разме́р числа́, а соста́в мно́жества.` → `Здесь важна́ не величина́ числа́, а соста́в мно́жества.` (`en`: `size of the number` → `magnitude of the number`; `glosses`: replaced `важен`, `размер` with `важна`, `величина`) — Russian mathematicians never say `разме́р числа́` for the numerical size/magnitude of a number (`разме́р` is physical dimension, array length, or font size); the standard term is `величина́ числа́`, which also lets the learner see feminine short-adjective agreement (`важна́ … величина́`) alongside masculine (`соста́в`).
- `math-02-elements` (`notation`) — Added `{ "symbol": "2 ∈ A", "ru": "два принадлежи́т A", "en": "two belongs to A; symbolic set names stay undeclined in print" }` (and `"два"` to `glosses`) — Rung 2 introduces the full expression `За́пись 2 ∈ A` in the text, so the learner needs the full spoken clause (`два принадлежи́т A`), not only the isolated verb `принадлежи́т`.

### Rung 3 — `math-03-subsets`
- `math-03-subsets` — `Само́ мно́жество то́же есть среди́ его́ подмно́жеств.` → `Само́ мно́жество то́же есть среди́ свои́х подмно́жеств.` (`en`: `among its subsets` → `among its own subsets`; `glosses`: replaced `его` with `своих`) — Because `Само́ мно́жество` is the grammatical subject of the clause, Russian binding rules require the reflexive possessive `свои́х`; non-reflexive `его́` sounds as though it refers to a different set mentioned in a previous sentence.
- `math-03-subsets` (`notation`) — Added `{ "symbol": "A ≠ B", "ru": "A не равно́ B", "en": "A is not equal to B" }` (and `"равно"` to `glosses`) — `A ≠ B` is used directly in the definition of strict inclusion (`A ⊂ B — стро́гое включе́ние: A — подмно́жество B, но A ≠ B`) and needs its spoken form (`не равно́`, neuter agreeing with implicit `мно́жество`).

### Rung 4 — `math-04-functions`
- `math-04-functions` — `Два ра́зных аргуме́нта и одно́ о́бщее значе́ние — здесь нет пробле́мы.` → `Два ра́зных аргуме́нта и одно́ о́бщее значе́ние — здесь нет наруше́ния пра́вила.` (`en`: `are no problem here` → `do not violate the rule here`; `glosses`: replaced `проблемы` with `нарушения`, `правила`) — `здесь нет пробле́мы` is a conversational English calque ("no problem here") out of place in textbook prose; `здесь нет наруше́ния пра́вила` preserves the genitive-of-negation pedagogical target while staying in authentic textbook register and echoing `Фу́нкция … — э́то пра́вило` from the opening sentence.

### Rung 5 — `math-05-injections`
- `math-05-injections` — `Инъе́кция — э́то фу́нкция без совпаде́ний значе́ний у ра́зных аргуме́нтов.` → `Инъе́кция — э́то фу́нкция без совпаде́ния значе́ний у ра́зных аргуме́нтов.` (`en`: `coincidences of values` → `coincidence of values`; `glosses`: `совпадений` → `совпадения`) — Russian mathematical prose strongly prefers the singular verbal noun after `без` (`без совпаде́ния значе́ний`), avoiding the awkward cacophony of two consecutive `-ий` genitive plurals (`совпаде́ний значе́ний`).
- `math-05-injections` — `Для инъе́кции есть и обра́тное утвержде́ние.` → `Для инъе́кции ве́рно и обра́тное утвержде́ние.` (`en`: `also has the converse statement` → `the converse statement holds as well`; added `верно` to `glosses`) — Every conditional statement *has* a converse (`обра́тное утвержде́ние существу́ет всегда́`); what characterizes an injection is that the converse *holds / is true* (`ве́рно и обра́тное утвержде́ние`).
- `math-05-injections` (`questions[2]`) — `What changes between the general implication and the injection condition?` → `In «из x = y сле́дует f(x) = f(y)», how does Russian mark which equation is the premise?` — The original `inference` question tested only the mathematical definition of converse; the replacement tests how Russian inverted proof syntax (`из X сле́дует Y`) marks the premise with `из` and makes the conclusion `f(x) = f(y)` the post-verbal grammatical subject of `сле́дует`.

### Rung 6 — `math-06-equivalence`
- `math-06-equivalence` — `Рефлекси́вность: для ка́ждого x есть x R x.` → `Рефлекси́вность: для ка́ждого x ве́рно x R x.` (`en`: `there is x R x` → `x R x holds`; added `верно` to `glosses`) — `есть x R x` ("there is `x R x`") is ungrammatical in Russian for stating that a relation holds; `ве́рно x R x` (or `выполня́ется x R x`) is standard and keeps the nominal/short-adjective style appropriate for `u22-p2`.

### Rung 8 — `math-08-even-sum`
- `math-08-even-sum` — `Мы записа́ли a + b = 2(k + m).` → `Запи́шем a + b = 2(k + m).` (`how_read[2]`: `мы записа́ли … мы доказа́ли …` → `запи́шем … мы доказа́ли …`; `glosses`: `записали` → `запишем`) — In Russian proof style, introducing a fresh algebraic rewrite on the spot uses the first-person plural perfective future (`Запи́шем a + b = 2(k + m)`), whereas `Мы записа́ли` means "We wrote [earlier]" and only makes sense when recalling a prior equation. Pairing `Запи́шем …` (live step) with `Мы доказа́ли …` (retrospective summary at Q.E.D.) teaches the authentic aspectual contrast of Russian proofs.
- `math-08-even-sum` (`questions[2]`) — `Why does the first paragraph mention halves of real numbers?` → `In «оно́ равно́ удво́енному це́лому числу́», why is «равно́» nominative neuter while «це́лому числу́» is dative?` — Replaced a question solvable from English/math common sense with one testing Russian short-adjective agreement (`оно́ равно́`) and case government (`равно́ + Dative`).

### Rung 9 — `math-09-pigeonhole`
- `math-09-pigeonhole` — `Е́сли n + 1 предме́тов размести́ть в n я́щиках` → `Е́сли n + 1 предме́т размести́ть в n я́щиках` and `Но мы размести́ли n + 1 предме́тов.` → `Но мы размести́ли n + 1 предме́т.` (added `предмет` to `glosses`) — **Grammatical error fix**: In Russian, any compound numeral expression ending in `оди́н` — including symbolic `n + 1`, read aloud as `n плюс оди́н` — governs the **singular** (`n + 1 предме́т`, `21 предме́т`), never the genitive plural `предме́тов` (`предме́тов` remains in `не ме́нее двух предме́тов` and `не бо́лее n предме́тов`).

### Rung 10 — `math-10-primes`
- `math-10-primes` — `Предполо́жим, что просты́х чи́сел коне́чно мно́го.` → `Предполо́жим, что просты́х чи́сел коне́чное число́.` (`en`: `finitely many primes` → `a finite number of primes`; `glosses`: `конечно` → `конечное`) — While `бесконе́чно мно́го` is completely idiomatic in Russian for "infinitely many", its literal opposite `*коне́чно мно́го` does not exist in standard Russian mathematical prose (and `коне́чно` is homographs with the parenthetical "of course"); Russian mathematicians always write `просты́х чи́сел коне́чное число́` (or `мно́жество просты́х чи́сел коне́чно`).
- `math-10-primes` — `Сле́довательно, q нет в спи́ске.` → `Сле́довательно, числа́ q нет в спи́ске.` (`en`: `q is not in the list` → `the number q is not in the list`; added `числа` to `glosses`) — Adding the classifier noun `числа́` before the undeclined symbol `q` both sounds more natural in Russian textbook style and makes the genitive of negation after `нет` morphologically visible to the learner.
- `math-10-primes` (`notation[0]`) — `"ru": "произведе́ние всех чи́сел из спи́ска"` → `"ru": "произведе́ние всех чи́сел от p пе́рвого до p э́нного"` (`"en": "the product of all the numbers from p sub one to p sub n"`; added `до`, `первого`, `энного` to `glosses`) — `произведе́ние всех чи́сел из спи́ска` was a paraphrase of the meaning rather than how a Russian mathematician reads subscripted `p₁⋯pₙ` aloud (`от p пе́рвого до p э́нного`).
- `math-10-primes` (`questions[2]`) — `What changes if «все» is removed from «все просты́е чи́сла»?` → `Why does «у кото́рого» have masculine/neuter genitive agreement in «натура́льное число́, бо́льшее едини́цы, у кото́рого …»?` — Replaced a language-independent logic question with a Russian relative-clause agreement check (`кото́рого` jumping over feminine `едини́цы` to agree with neuter `число́`).

### Rung 12 — `math-12-degrees`
- `math-12-degrees` — `О́ба выраже́ния счита́ют одно́ и то же, поэ́тому они́ равны́.` → `О́ба выраже́ния даю́т одно́ и то же число́, поэ́тому они́ равны́.` (`en`: `Both expressions count the same thing` → `Both expressions give the same number`; `glosses`: `считают` → `дают`) — In Russian, the verb `счита́ть` ("count") requires an animate agent (`мы счита́ем`, `посчита́ем`); mathematical expressions do not "count", they `даю́т` ("give/yield") or `выража́ют` ("express") the same number.

### Rung 13 — `math-13-paths`
- `math-13-paths` — `соедине́ны маршру́том` → `соединены́ маршру́том` and `повторе́ния разреше́ны` → `повторе́ния разрешены́` — **Critical stress fixes**: Short passive participles in `-ены́` from `-и́ть` verbs (`соедини́ть → соединены́`, `разреши́ть → разрешены́`) have fixed ending stress `-ы́`, never `-е́ны`.
- `math-13-paths` — `Е́сли в нём два́жды встреча́ется одна́ верши́на` → `Е́сли в нём два́жды встреча́ется одна́ и та же верши́на` (added `та` to `glosses`) and `Э́то противоре́чит вы́бору.` → `Э́то противоре́чит вы́бору маршру́та.` (`en` updated accordingly) — Bare `одна́ верши́на` in Russian reads as "one [single] vertex occurs twice"; Russian textbook prose writes `одна́ и та же верши́на` ("one and the same vertex"). Likewise, bare `противоре́чит вы́бору` without its genitive complement (`вы́бору маршру́та`, compare `вы́бору дро́би` in Rung 11) feels clipped.

### Rung 14 — `math-14-inverse`
- `math-14-inverse` — `О́ба усло́вия ну́жны для э́того определе́ния.` → `О́ба усло́вия нужны́ для э́того определе́ния.` — **Critical mobile-stress fix**: The short plural of `ну́жный` (`ну́жен, нужна́, ну́жно, нужны́`) has final stress `нужны́`, not stem stress `*ну́жны`.

### Rung 16 — `math-16-union-theorem`
- `math-16-union-theorem` — `При пусто́м I объедине́ние пусто́.` → `При пусто́м I объедине́ние пу́сто.` — **Critical mobile-stress fix**: The short forms of `пусто́й` shift stress to the stem in the neuter and plural (`пуст, пуста́, пу́сто, пу́сты`); `*пусто́` is a classic non-native stress error.
- `math-16-union-theorem` (`questions[2]`) — `Why does the empty family not give a counterexample?` → `In «одно́ мно́жество семе́йства, содержа́щее э́ту то́чку», which noun does «содержа́щее» modify and why?` — Replaced a pure topology question with a Russian participle-agreement question (why neuter accusative/nominative `содержа́щее` modifies `мно́жество` across the intervening genitive singular `семе́йства`).

### Rung 17 — `math-17-union-proof`
- `math-17-union-proof` — `Выбира́я Uᵢ, мы мо́жем учиты́вать x.` → `Выбира́я Uᵢ, мы мо́жем учи́тывать x.` — **Critical stress fix**: Imperfective `-ывать` derived from `уче́сть` is stressed on the root vowel `и`: `учи́тывать`, never `*учиты́вать`.

### Rung 18 — `math-18-continuity`
- `math-18-continuity` (`notation`) — `"ru": "мо́дуль ра́зности x и x нулево́го ме́ньше числа́ де́льта"` → `"ru": "мо́дуль ра́зности x и x нулево́го ме́ньше де́льта"` and `"ru": "для любо́го положи́тельного числа́ э́псилон существу́ет положи́тельное число́ де́льта"` → `"ru": "для любо́го э́псилон бо́льше нуля́ существу́ет де́льта бо́льше нуля́"` (`en` updated; `glosses`: replaced `положительного`, `положительное` with `больше`, `нуля`) — When reading `|x − x₀| < δ` and `∀ ε > 0 ∃ δ > 0` aloud at the blackboard, Russian mathematicians never insert the classifier word `число́` before Greek letters (`*числа́ де́льта`, `*положи́тельного числа́ э́псилон`); they read the symbols directly as `ме́ньше де́льта` and `для любо́го э́псилон бо́льше нуля́ существу́ет де́льта бо́льше нуля́`.

### Rung 20 — `math-20-sum-proof`
- `math-20-sum-proof` (`notation[0]`) — `"ru": "число́ де́льта равно́ ми́нимуму из чи́сел де́льта оди́н и де́льта два"` → `"ru": "де́льта равно́ ми́нимуму из де́льта оди́н и де́льта два"` — Removed unnatural spoken classifier `число́ / из чи́сел` before `де́льта`.
- `math-20-sum-proof` (`questions[2]`) — `Why is the maximum not justified by this proof?` → `How does «Вы́брав ма́ксимум вме́сто ми́нимума, мы не смогли́ бы обоснова́ть э́тот шаг» express a counterfactual condition?` — Replaced a math-only question with one testing how Russian mathematical prose uses a perfective adverbial participle (`Вы́брав …`) + subjunctive (`не смогли́ бы`) to express a counterfactual conditional ("Had we chosen …, we could not have justified …") without an explicit `е́сли бы` clause.

### Rung 21 — `math-21-probability`
- `math-21-probability` — `мы учиты́ваем` → `мы учи́тываем`, `учиты́вались бы` → `учи́тывались бы`, and `не обяза́тельно пусто́:` → `не обяза́тельно пу́сто:` — **Critical stress fixes** (`учи́тывать` family and short neuter `пу́сто`).
- `math-21-probability` — `Без усло́вия о непересече́нии` → `Без усло́вия непересече́ния` (`glosses`: `непересечении` → `непересечения`) — In Russian mathematical prose, `усло́вие` takes a direct adnominal genitive when naming a mathematical property (`усло́вие непересече́ния`, `усло́вие непреры́вности`, `усло́вие инъекти́вности`), not `усло́вие о + Prepositional`.

### Rung 22 — `math-22-independence`
- `math-22-independence` (`questions[2]`) — `What does «при P(B) > 0» limit?` → `In «При P(B) = 0 незави́симость по-пре́жнему определена́, а приведённая фо́рмула … не определя́ет число́», what is the grammatical difference between «определена́» and «не определя́ет число́»?` — Replaced a question answerable purely from the formula `P(A ∩ B)/P(B)` with one testing the morphosyntactic contrast between the feminine short passive participle `определена́` ("is defined", agreeing with subject `незави́симость`) and the active transitive verb `не определя́ет число́` ("does not define a number", with subject `фо́рмула` and object `число́`).

### Rung 23 — `math-23-quantifiers`
- `math-23-quantifiers` — `Сло́ва «не все» не означа́ют «все не».` → `Слова́ «не все» не означа́ют «все не».` — **Critical mobile-stress fix**: Nominative plural of `сло́во` is `слова́` (end-stressed); `сло́ва` is genitive singular!
- `math-23-quantifiers` — `При пусто́м A о́бе сто́роны запи́санной эквивале́нтности ло́жны.` → `При пусто́м A о́бе стороны́ запи́санной эквивале́нтности ло́жны.` (`glosses["стороны"]` updated to `"sides of the equivalence (genitive singular after both)"`) — **Critical mobile-stress & numeral-government fix**: Like `две`, the feminine numeral `о́бе` governs the **genitive singular** (`стороны́`), which has final stress, not the nominative plural `сто́роны`.

### Rung 24 — `math-24-information`
- `math-24-information` — `учиты́вает` → `учи́тывает` — **Critical stress fix** (`учи́тывать` family).
- `math-24-information` — `конкре́тной стро́ки` → `конкре́тной строки́`, `э́ту стро́ку` → `э́ту строку́`, `но не от стро́ки.` → `но не от строки́.`, and `едини́чной стро́ки` → `едини́чной строки́` — **Critical mobile-stress fixes**: `строка́` has end stress throughout the singular (`строка́, строки́, строке́, строку́, строко́й, о строке́`) and stem stress only in the plural (`стро́ки, строк, стро́кам`). Writing `конкре́тной стро́ки` and `э́ту стро́ку` misplaces plural stress onto singular forms four times in one paragraph.
- `math-24-information` — `На́ше кра́ткое сопоставле́ние развива́ет разли́чие трёх подхо́дов Колмого́рова.` → `На́ше кра́ткое сопоставле́ние раскрыва́ет разли́чие трёх подхо́дов Колмого́рова.` (`en`: `develops Kolmogorov's distinction` → `brings out Kolmogorov's distinction`; `glosses`: `развивает` → `раскрывает`) — In Russian, one does not "develop a distinction" (`*развива́ть разли́чие`); a comparison `раскрыва́ет разли́чие` ("reveals / brings out the distinction") or `проясня́ет разли́чие`.
- `math-24-information` (`questions[2]`) — `What depends on the two languages but not the string?` → `In «сло́жность меня́ется не бо́лее чем на конста́нту, зави́сящую от двух языко́в, но не от строки́», which noun does «зави́сящую» modify and what does that say about what depends on the two languages?` — Anchored the prompt explicitly in the Russian participle agreement (`конста́нту, зави́сящую …`).

---

## 2. Overall Assessment of the Shelf's Russian

### Register & Architectural Strengths
Overall, Codex's architectural conception of the 24-rung shelf is **remarkably strong**. The progression from zero-copula definitions (`Мно́жество без элеме́нтов — пусто́е мно́жество`) to explicit case-governed predicates (`принадлежи́т мно́жеству`, `явля́ется фу́нкцией`, `равно́ удво́енному це́лому числу́`), first-person plural proof imperatives (`Пусть … Поло́жим … Возьмём … Зафикси́руем …`), full active/passive/reflexive participles (`содержа́щее` vs. `содержа́щаяся`, `определённая` vs. `определена́`), and adverbial participles (`Вы́брав …, найдём …`, `Скла́дывая …, полу́чим …`) mirrors the real syntactic spine of Russian university textbooks (Зорич, Колмогоров–Фомин, Виленкин, Верещагин–Шень).

### What a Russian Mathematician Would Smile At
1. **Admiring smiles**:
   - The crisp contrast between `содержа́щее` (active) and `содержа́щаяся` (reflexive passive) in Rungs 15–17 (`math-15-open-sets` to `math-17-union-proof`) — this is the single most important participial trap for English-speaking mathematicians reading Russian topology, and the shelf teaches it head-on.
   - The deliberate separation of `не все …` (`¬∀`) and `все не …` (`∀¬`) in Rung 23 (`math-23-quantifiers`), and the clean capsule of Kolmogorov's 1965 «Три подхо́да к определе́нию поня́тия „коли́чество информа́ции“» in Rung 24 (`math-24-information`).
2. **Wry smiles (before our fixes)**:
   - **Mobile-stress traps**: Codex knew the rule for `число́ / чи́сла / чи́сел` (because it was in the prompt), but fell into almost every *other* classic Russian mobile-stress trap: `*пусто́` for `пу́сто`, `*ну́жны` for `нужны́`, `*соедине́ны / *разреше́ны` for `соединены́ / разрешены́`, `*сло́ва` (genitive singular!) for nominative plural `слова́`, `*о́бе сто́роны` for `о́бе стороны́`, `*стро́ки / *стро́ку` for singular `строки́ / строку́`, and `*учиты́вать` for `учи́тывать`.
   - **Over-literal spoken notation**: No Russian lecturer at МГУ or НМУ has ever stood at a blackboard and read `∀ ε > 0 ∃ δ > 0` as *«для любо́го положи́тельного числа́ э́псилон существу́ет положи́тельное число́ де́льта»* or `|x − x₀| < δ` as *«… ме́ньше числа́ де́льта»*. Everyone says *«для любо́го э́псилон бо́льше нуля́ существу́ет де́льта бо́льше нуля́»* and *«… ме́ньше де́льта»*.
   - **Compound numeral agreement with symbols**: Writing `n + 1 предме́тов` instead of `n + 1 предме́т` in Rung 9 (`math-09-pigeonhole`). Because `n + 1` ends in the spoken word `оди́н` (`эн плюс оди́н`), Russian syntax obligatorily requires the singular noun `предме́т` — a lovely quirk of Russian mathematical prose that a native mathematician notices immediately.

---

## 3. Which Rungs Are Too Hard for Their `after_page` and Why

By cross-checking each rung's `after_page` against the main Drussky course progression (`build_course.py` / `course_reading.py`), three structural pacing bottlenecks stand out:

1. **Rungs 1–3 (`math-01-sets` at `u7-p4`, `math-02-elements` at `u8-p3`, `math-03-subsets` at `u10-p4`) — Heavy Case Morphology Before the Course Teaches Those Cases**:
   - At `u7-p4` (Unit 7), the learner has only just met basic nominative nouns, gender, and early `в/на` prepositional phrases. Yet `math-01-sets` already relies on **genitive plural** (`мно́жество чи́сел`, `без элеме́нтов`), **paucal genitive singular after numerals** (`два элеме́нта`, `три элеме́нта`), **genitive of negation** (`нет поря́дка`, `нет пе́рвого элеме́нта`), and **declined prepositional pronouns** (`в нём`).
   - By `math-03-subsets` (`u10-p4`), the learner encounters full **genitive plural adjective + noun chains** (`свои́х подмно́жеств`, `ка́ждого мно́жества`, `стро́гого включе́ния`) long before Genitive Plural is systematically taught in the main course. While per-word `glosses` and `how_read` frames cushion the blow, a true week-2 beginner at Unit 7–10 is decoding these endings as lexicalized chunks rather than productive morphology.
2. **The 8-Unit Gap Followed by a Double Rung at `u18-p2` (`math-04-functions` and `math-05-injections`)**:
   - There are no math rungs at all between `u10-p4` and `u18-p2`, and then two rungs unlock simultaneously at `u18-p2`. `math-05-injections` (`Инъе́кция`) is noticeably denser than `math-04-functions`: it stacks verbal nouns with nested genitives (`без совпаде́ния значе́ний у ра́зных аргуме́нтов`, `о нали́чии всех элеме́нтов B среди́ значе́ний фу́нкции`) and inverted `из … сле́дует …` syntax at a point where the learner has not yet reached the main course's coverage of verbal nouns or advanced case government (`u20` lacks a rung; moving `math-05-injections` to `u20-p3` would smooth the ramp).
3. **Triple-Rung Pileups at `u25-p4` (Rungs 9, 10, 11) and `u27-p5` (Rungs 12, 13, 14)**:
   - Unlocking three full proof readings on the exact same page (`math-09-pigeonhole`, `math-10-primes`, `math-11-sqrt-two` all at `u25-p4`; `math-12-degrees`, `math-13-paths`, `math-14-inverse` all at `u27-p5`) creates a steep wall of ~45 new glossed word forms per page.
   - In particular, **Rung 10 (`math-10-primes`) at `u25-p4`** introduces `у кото́рого` relative clauses plus synthetic comparative `бо́льшее едини́цы` before the main course's relative-clause unit (`u27`), and **Rung 13 (`math-13-paths`) at `u27-p5`** combines superlative adjectives (`кратча́йший`, `наиме́ньшей`), irregular comparative (`коро́че исхо́дного`), instrumental plural agreement (`с те́ми же нача́лом и концо́м`, `ме́жду э́тими двумя́ появле́ниями`), and short passive participles (`соединены́`, `разрешены́`) all in 115 words.

---

## 4. Three Readings to Add to the Shelf Next

1. **«При́нцип математи́ческой инду́кции» (Mathematical Induction — Base Case and Inductive Step)**  
   *Rationale*: Induction is the single most common proof template in discrete math and computer science, and its Russian scaffolding (`ба́за инду́кции при n = 1`, `индукти́вное предположе́ние`, `индукти́вный перехо́д от n к n + 1`, `по предположе́нию инду́кции`) is indispensable for reading any Russian algebra, combinatorics, or theory-of-computation text.

2. **«Счё́тные и несчё́тные мно́жества: диагона́льный ме́тод Ка́нтора» (Countable and Uncountable Sets: Cantor's Diagonal Argument)**  
   *Rationale*: Bridges Rung 14 (`Обра́тная фу́нкция / бие́кция`) and Rung 24 (`Три подхо́да к информа́ции`) by teaching the Russian vocabulary of cardinality (`мо́щность мно́жества`, `взаи́мно однозна́чное соотве́тствие`, `счё́тное мно́жество`) and the exact diagonal construction (`постро́им после́довательность, отлича́ющуюся от n-й строки́ на n-м ме́сте`) that directly foreshadows uncomputability in Rung 24.

3. **«Сходи́мость после́довательности и еди́нственность преде́ла» (Convergence of a Sequence and Uniqueness of the Limit)**  
   *Rationale*: Gives the learner the `∀ ε > 0 ∃ N` discrete counterpart to Rung 18's `ε–δ` continuity (`после́довательность сходи́тся к преде́лу a`, `начина́я с неко́торого но́мера N`, `по нера́венству треуго́льника`) while reusing the `ε/2` triangle-inequality technique from Rungs 19–20 in a simpler one-quantifier-alternation setting.
