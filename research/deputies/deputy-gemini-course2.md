# Deputy Author Report — `deputy-gemini-course2`

**Author:** Gemini 4 Argon (Deputy Author)  
**Branch:** `deputy-gemini-course2`  
**Files Added / Edited:**
- `apple/scripts/course_home.py` (new module exposing `pages(h)`, `DIARY_PROMPTS`, and `LEXICON`)
- `apple/scripts/build_course.py` (imports `course_home`, appends its 10 pages to `U`, merges `course_home.LEXICON` and `course_home.DIARY_PROMPTS`)
- `apple/scripts/build_morphology.py` (registers the new domestic lemmas and inflected forms from `course_home.LEXICON` so `validate_morphology.py` passes alongside `validate_course.py` and `validate_paradigms.py`)

---

## 1. New Pages Added (`h1`–`h6`, `s1`, `a1`–`a3`)

No units were reordered and no existing pages were renumbered. Each page is authored as an explicit 4-tuple `(pid, title, intro, exercises)` appended to its host unit in `build_course.py` following the exact pattern of `course_grammar.py`. Every page has 7–8 body exercises (plus 1 English `diary` prompt appended by `add_diary_lines`, giving 8–9 total exercises per page), a strict majority of Pencil writing tasks (`copy`, `fill`, `translate`, `dictation`), at least one hearing task (`listen_pick` or `dictation`), and at least one speaking task (`read_aloud`).

| Unit | Page ID | Title | What It Teaches | Which Recommendation It Fulfils |
|---|---|---|---|---|
| `u13` | `h1` | **Let's make tea: дава́й and the kitchen** | `дава́й` + 1pl pf. future (`Дава́й сде́лаем ко́фе!`) vs. `дава́й` + impf. infinitive (`Дава́й пить чай!`); replies (`Дава́й!`, `Ла́дно!`, `Коне́чно!`, `Сейча́с!`, `Гото́во!`, `На, держи́!`); polite `мо́жно`; kitchen appliances & tableware (`ча́йник`, `плита́`, `холоди́льник`, `ра́ковина`, `шкаф`, `по́лка`, `ба́нка`, `кру́жка`, `кастрю́ля`, `сковорода́`, `кры́шка`, `ми́ска`, `стака́н`, `ви́лка`, `нож`). | `deputy-gemini-course.md` §5.3 (`дава́й` + future/infinitive alongside kitchen dialogues) & `deputy-gemini-kitchen.md` §5.2 (`u13` kitchenware + `дава́й` table interactions). |
| `u14` | `h2` | **Salt or sugar? же, ведь, ну and the table** | Particles `же` (`Она́ же тут!`), `ведь` (`Э́то ведь соль!`), and `ну` (`Ну ла́дно!`, `Ну как где?`) in two-line exchanges; `бу́дешь` + food noun (`Бу́дешь суп?`); contrastive `без молока́ / с молоко́м` and `без са́хара / с лимо́ном / с мя́той`; `Я так хочу́ есть!`, `Вку́сно па́хнет!`; fridge & table foods (`ма́сло`, `яйцо́`, `смета́на`, `кефи́р`, `макаро́ны`, `карто́шка`, `пельме́ни`, `сала́т`, `рис`, `я́блоко`, `лимо́н`, `мёд`, `имби́рь`, `мя́та`, `кори́ца`, `мука́`, `варе́нье`, `я́годы`, `грибы́`, `пиро́г`, `бу́лочки`, `сы́рники`). | `deputy-gemini-course.md` §5.3 (`же / ведь / ну` in `u13`–`u19`) & `deputy-gemini-kitchen.md` §5.4–5.5 (moving particles forward from `u28-p3` into `u14` and pairing `без + gen.` with `с + instr.`). |
| `u15` | `h3` | **Out of milk: ко́нчилось, вообще́ нет, and kitchen states** | Counting aloud with `раз, два, три…`; running out & quantities (`ко́нчилось`, `вообще́ нет`, `оста́лось`, `чуть-чу́ть`, `совсе́м`, `почти́`, `то́чно`, `про́сто`, `а то`, `па́чка`, `буты́лка`, `спи́сок`, `запи́ска`); domestic predicates & states (`кипи́т`, `горя́чий`, `тёплый`, `холо́дный`, `кре́пкий`, `све́жий`, `вку́сный`, `невку́сный`, `чи́стый`, `гря́зный`, `пусто́й`, `откры́т`, `закры́т`, `голо́дный`, `голо́дная`). | `deputy-gemini-kitchen.md` §3 B/G & §5.4 (`ко́нчилось`, `вообще́ нет`, domestic adjectives and quantity frames in `u15`). |
| `u12` | `s1` | **One stem, consonant shifts: люблю́, пишу́, хожу́, хочу́** | One-page map of the four predictable 1sg stem consonant alternations before `-ю / -у`: `б → бл` (`люби́ть → я люблю́, ты лю́бишь`), `с → ш` (`писа́ть → я пишу́, ты пи́шешь`), `д → ж` (`ходи́ть → я хожу́, ты хо́дишь`; `ви́деть → я ви́жу, ты ви́дишь`), and `т → ч` (`хоте́ть → я хочу́, ты хо́чешь`), contrasting 2nd-conjugation 1sg-only shifts (`люблю́ / лю́бишь`, `хожу́ / хо́дишь`, `ви́жу / ви́дишь`) with 1st-conjugation full-present shifts (`пишу́ / пи́шешь`). | `deputy-gemini-course.md` §5.2 (one-page "one stem, consonant alternations" map around units 12–14). |
| `u17` | `h4` | **Pass, put, take: kitchen imperatives in pairs** | One-line rule of thumb for aspect in imperatives (perfective for one concrete step right now; imperfective for an open invitation, general habit, or `не` prohibition) + core pairs (`положи́ / клади́`, `поста́вь / ставь`, `возьми́ / бери́`, `посмотри́ / смотри́`, `наре́жь / не режь`) and high-frequency requests (`иди́ сюда́`, `дай`, `помеша́й`, `посоли́`, `попро́буй`, `сади́сь`, `вы́ключи`, `нале́й`, `помо́й`, `закро́й`, `разогре́й`, `сде́лай`, `осторо́жно`, `не тро́гай`). | `deputy-gemini-course.md` §5.3 & `deputy-gemini-kitchen.md` §3 A, §5.1, §6.2, §6.4 (pulling aspectual kitchen imperatives forward instead of waiting until `u25-p4`). |
| `u17` | `h5` | **Cooking and cleaning up: Дава́й я помо́ю посу́ду!** | Volunteering with `Дава́й я + 1sg pf.` (`Дава́й я помо́ю посу́ду!`); coordinating chores with mid-sentence `пока́` (`А я пока́ вы́тру стол и уберу́ хлеб`); past aspect in domestic questions (`Ты корми́л кота́? — Коне́чно, покорми́л! Он всё съел!`); cooking verbs (`свари́ть`, `завари́ть`, `перевари́ть`, `испе́чь`, `накрыва́ть на стол`, `доста́ть`, `осты́нет`, `убежа́ло`, `допьёшь`) and cookware (`посу́да`, `хле́бница`, `салфе́тки`, `тря́пка`, `дуршла́г`, `доска́`, `про́тивень`, `прихва́тка`, `духо́вка`, `ту́рка`, `кофемаши́на`). | `deputy-gemini-kitchen.md` §3 D/F, §5.2, §6.4 (domestic aspect in offers, promises, and past result checks in `u17`). |
| `u19` | `h6` | **Keys, the courier, and the neighbour: switching between ты and вы** | Real-time code-switching between domestic `ты` (`Подержи́ кота́!`, `Возьми́ тру́бку!`, `Сходи́ за хле́бом!`, `Помоги́ мне разобра́ть су́мку!`) and polite `вы` (`Здра́вствуйте!`, `Извини́те!`, `Проходи́те!`, `Сади́тесь!`, `Возьми́те!`, `Бери́те!`, `Откро́йте, пожа́луйста!`, `Поднима́йтесь!`); household logistics (`ключи́`, `очки́`, `часы́`, `телефо́н`, `тру́бку`, `дверь`, `свет`, `сосе́дка`, `курье́р`, `посы́лка`, `проду́кты`, `домофо́н`, `эта́ж`, `пека́рня`, `реце́пт`, `нас то́лько дво́е`). | `deputy-gemini-kitchen.md` §3 A/C/D & §6.3 (teaching the real-time `ты ↔ вы` switch when a neighbour or courier appears). |
| `u24` | `a1` | **Adjective cases I: of and to (-ого/-его, -ому/-ему, -ой/-ей)** | Singular genitive (`-ого / -его` m/n, `-ой / -ей` f) and dative (`-ому / -ему` m/n, `-ой / -ей` f) of adjectives with `э́тот / э́то / э́та` (`э́того`, `э́тому`, `э́той`) and `мой / наш` (`моего́`, `моему́`, `мое́й`, `на́шего`, `на́шему`, `на́шей`). | `deputy-gemini-course.md` §5.1 (dedicated pages on adjective and pronoun case patterns after `u24`'s noun cases and before `u27`'s `кото́рый` and `u28`'s participles). |
| `u24` | `a2` | **Adjective cases II: with, in, and the feminine object (-ым/-им, -ом/-ем, -ую, -ой/-ей)** | Singular instrumental (`-ым / -им` m/n, `-ой / -ей` f), prepositional (`-ом / -ем` m/n, `-ой / -ей` f), and feminine accusative (`-ую`: `э́ту мою́ но́вую кни́гу` vs. `с э́той мое́й но́вой кни́гой`, `в э́том моём но́вом до́ме`). | `deputy-gemini-course.md` §5.1 (singular instrumental, prepositional, and feminine `-ую` vs. `-ой` syncretism). |
| `u24` | `a3` | **Adjective cases III: the plural (-ые/-ие, -ых/-их, -ым/-им, -ыми/-ими)** | Gender-neutral plural adjective and determiner declension across all cases: nom/acc `-ые / -ие` (`э́ти мои́ но́вые кни́ги`), gen/prep `-ых / -их` (`нет э́тих мои́х но́вых книг`, `в э́тих на́ших больши́х зада́чах`), dat `-ым / -им` (`к э́тим мои́м но́вым кни́гам`), and instr `-ыми / -ими` (`с э́тими на́шими но́выми кни́гами`). | `deputy-gemini-course.md` §5.1 (plural adjective and pronoun case patterns, especially `-ых / -их` genitive/prepositional). |

---

## 2. New Lexicon Entries (`course_home.LEXICON` — 178 entries)

None of these `ru` keys or `en` glosses duplicate existing entries in `build_course.py`, `course_reading.py`, `course_families.py`, `course_read.py`, or `course_grammar.py`. Every entry is presented with its meaning in an `explain` example and used in at least one reading, writing, speaking, or listening task on its page.

- **`h1` (`u13`, 27 entries):** `дава́й` ("let's; go on, give"), `дава́й сде́лаем` ("let's make, let's do"), `пить` ("to drink"), `сде́лаю` ("I will make, I will get done"), `ла́дно` ("alright, deal"), `коне́чно` ("of course"), `гото́во` ("done, it is ready"), `на, держи́` ("here, take it; hold it"), `ча́йник` ("kettle, teapot"), `плита́` ("stove, cooker"), `на плите́` ("on the stove"), `холоди́льник` ("fridge, refrigerator"), `ра́ковина` ("kitchen sink"), `шкаф` ("cupboard, cabinet"), `в шкафу́` ("in the cupboard"), `по́лка` ("shelf"), `на по́лке` ("on the shelf"), `ба́нка` ("jar"), `кру́жка` ("mug"), `кастрю́ля` ("saucepan, pot"), `сковорода́` ("frying pan, skillet"), `кры́шка` ("lid"), `ми́ска` ("bowl"), `стака́н` ("glass, tumbler"), `воды́` ("of water (genitive)"), `ви́лка` ("fork"), `нож` ("knife").
- **`h2` (`u14`, 32 entries):** `ну` ("well, come on (particle)"), `ой` ("oh, oops"), `без` ("without"), `без молока́` ("without milk"), `без са́хара` ("without sugar"), `с лимо́ном` ("with lemon"), `с мя́той` ("with mint"), `я так хочу́ есть` ("I am so hungry"), `па́хнет` ("smells (it smells)"), `пельме́ни` ("pelmeni, dumplings"), `макаро́ны` ("pasta, macaroni"), `с сы́ром` ("with cheese"), `карто́шка` ("potatoes"), `рис` ("rice"), `сала́т` ("salad"), `ма́сло` ("butter"), `яйцо́` ("egg"), `смета́на` ("sour cream"), `кефи́р` ("kefir"), `я́блоко` ("apple"), `лимо́н` ("lemon"), `мёд` ("honey"), `имби́рь` ("ginger"), `мя́та` ("mint"), `кори́ца` ("cinnamon"), `мука́` ("flour"), `варе́нье` ("jam, preserves"), `я́годы` ("berries"), `грибы́` ("mushrooms"), `пиро́г` ("pie, tart"), `бу́лочки` ("buns, rolls"), `сы́рники` ("syrniki (quark pancakes)").
- **`h3` (`u15`, 32 entries):** `раз` ("one (counting aloud); a time"), `ко́нчилось` ("ran out, is all gone (neuter)"), `вообще́` ("at all, in general"), `оста́лось` ("is left, remains (neuter)"), `чуть-чу́ть` ("a tiny bit"), `то́чно` ("spot on, exactly"), `купи́ть` ("to buy"), `па́чка` ("pack, carton"), `буты́лка` ("bottle"), `спи́сок` ("list"), `запи́ска` ("written note"), `а то` ("or else, otherwise"), `совсе́м` ("completely, quite"), `забу́ду` ("I will forget"), `магази́н` ("shop, store"), `откры́т` ("open (short masculine)"), `закры́т` ("closed (short masculine)"), `кипи́т` ("is boiling"), `горя́чий` ("hot (of things)"), `кре́пкий` ("strong (of tea or coffee)"), `тёплый` ("warm"), `холо́дный` ("cold"), `све́жий` ("fresh"), `вку́сный` ("tasty, delicious (masculine)"), `невку́сный` ("bad-tasting, not tasty"), `чи́стый` ("clean (masculine)"), `гря́зный` ("dirty (masculine)"), `пусто́й` ("empty (masculine)"), `почти́` ("almost"), `про́сто` ("just, simply"), `голо́дный` ("hungry (masculine)"), `голо́дная` ("hungry (feminine)").
- **`s1` (`u12`, 5 entries):** `люби́ть` ("to love"), `ты лю́бишь` ("you (one person) love"), `ты пи́шешь` ("you (one person) write"), `ходи́ть` ("to go (regularly, on foot)"), `ты хо́дишь` ("you (one person) go (regularly)").
- **`h4` (`u17`, 26 entries):** `положи́` ("put, lay down (perfective imperative)"), `клади́` ("put, lay down (imperfective imperative)"), `поста́вь` ("put, stand upright (perfective imperative)"), `ставь` ("put, stand upright (imperfective imperative)"), `возьми́` ("take (perfective imperative)"), `бери́` ("take, help yourself (imperfective imperative)"), `посмотри́` ("look, check (perfective imperative)"), `наре́жь` ("slice, cut (perfective imperative)"), `не режь` ("don't cut (imperfective imperative)"), `осторо́жно` ("careful, watch out"), `не тро́гай` ("don't touch"), `ло́жку` ("spoon (as the object)"), `помеша́й` ("stir (imperative)"), `посоли́` ("salt it (imperative)"), `попро́буй` ("taste it, try it (imperative)"), `сади́сь` ("sit down (informal imperative)"), `вы́ключи` ("turn off, switch off (imperative)"), `плиту́` ("stove (as the object)"), `нале́й` ("pour (imperative)"), `помо́й` ("wash (imperative)"), `ча́шку` ("cup (as the object)"), `закро́й` ("close (imperative)"), `иди́ сюда́` ("come here"), `дай` ("give, hand me (imperative)"), `разогре́й` ("warm up, reheat (imperative)"), `сде́лай` ("make, do (perfective imperative)").
- **`h5` (`u17`, 34 entries):** `посу́да` ("the dishes"), `посу́ду` ("the dishes (as the object)"), `помо́ю` ("I will wash"), `вы́тру` ("I will wipe"), `уберу́` ("I will put away, I will clear"), `хле́бница` ("breadbox"), `салфе́тки` ("napkins"), `корми́л` ("fed, was feeding (he)"), `покорми́л` ("fed (completed; he)"), `покорми́` ("feed (imperative)"), `кота́` ("cat (as the object)"), `съел` ("ate up (he)"), `убежа́ло` ("boiled over, ran away (neuter)"), `не расстра́ивайся` ("don't be upset"), `всё равно́` ("anyway, all the same"), `мыть` ("to wash"), `тря́пка` ("kitchen cloth, rag"), `свари́ть` ("to boil, to cook (perfective)"), `завари́ть` ("to steep, to brew (tea)"), `перевари́ть` ("to overcook, to overboil"), `испе́чь` ("to bake (perfective)"), `накрыва́ть на стол` ("to set the table"), `дуршла́г` ("colander, strainer"), `доска́` ("cutting board; board"), `про́тивень` ("baking tray, sheet pan"), `прихва́тка` ("oven mitt, potholder"), `доста́ть` ("to take out, to get out"), `духо́вка` ("oven"), `осты́нет` ("will cool down"), `ту́рка` ("cezve (Turkish coffee pot)"), `кофемаши́на` ("coffee machine"), `допьёшь` ("you finish drinking"), `по́здно` ("late (it is late)"), `пора́ спать` ("it's time to sleep").
- **`h6` (`u19`, 35 entries):** `ключи́` ("keys"), `очки́` ("glasses, spectacles"), `часы́` ("clock, watch"), `телефо́н` ("phone, telephone"), `звони́т` ("is ringing, calls"), `тру́бку` ("receiver, phone (in возьми́ тру́бку)"), `дверь` ("door"), `стучи́т` ("knocks, is knocking"), `свет` ("light"), `сосе́дка` ("female neighbour"), `подержи́` ("hold (for a moment; imperative)"), `держу́` ("I'm holding, got it"), `здра́вствуйте` ("hello (polite or plural)"), `извини́те` ("excuse me, sorry (polite)"), `прости́` ("forgive me, sorry (informal)"), `проходи́те` ("come on in (polite imperative)"), `сади́тесь` ("sit down (polite imperative)"), `возьми́те` ("take (polite perfective imperative)"), `бери́те` ("take, help yourself (polite imperfective imperative)"), `курье́р` ("courier, delivery person"), `посы́лка` ("parcel, package"), `проду́кты` ("groceries"), `домофо́н` ("intercom, entry buzzer"), `откро́йте` ("open (polite imperative)"), `поднима́йтесь` ("come up, go up (polite imperative)"), `эта́ж` ("floor, storey"), `пека́рня` ("bakery"), `реце́пт` ("recipe"), `сходи́ за хле́бом` ("run out for bread"), `не ходи́` ("don't go"), `не забу́дь` ("don't forget"), `нас то́лько дво́е` ("there are only two of us"), `помоги́` ("help (informal imperative)"), `разобра́ть` ("to unpack, to sort out"), `су́мку` ("bag (as the object)").
- **`a1`–`a3` (`u24`, 47 entries):** `друг` ("friend"), `дру́га` ("friend (genitive or animate accusative)"), `дру́гу` ("to a friend (dative)"), `дру́гом` ("with a friend (instrumental)"), `э́того` ("of this (masculine or neuter genitive)"), `э́тому` ("to this (masculine or neuter dative)"), `э́той` ("of/to/with/in this (feminine oblique)"), `моего́` ("of my (masculine or neuter genitive)"), `моему́` ("to my (masculine or neuter dative)"), `мое́й` ("of/to/with/in my (feminine oblique)"), `на́шего` ("of our (masculine or neuter genitive)"), `на́шему` ("to our (masculine or neuter dative)"), `на́шей` ("of/to/with/in our (feminine oblique)"), `но́вого` ("of new (masculine or neuter genitive)"), `но́вому` ("to new (masculine or neuter dative)"), `но́вой` ("of/to/with/in new (feminine oblique)"), `сло́ва` ("of a word (genitive)"), `сло́ву` ("to a word (dative)"), `сло́вом` ("with a word (instrumental)"), `к` ("toward, to (with dative)"), `э́ту` ("this (feminine accusative)"), `мою́` ("my (feminine accusative)"), `на́шу` ("our (feminine accusative)"), `но́вую` ("new (feminine accusative)"), `большу́ю` ("big (feminine accusative)"), `зада́чу` ("problem, task (as the object)"), `э́тим` ("with this (masculine/neuter instrumental); to these (dative plural)"), `мои́м` ("with my (masculine/neuter instrumental); to my (dative plural)"), `но́вым` ("with new (masculine/neuter instrumental); to new (dative plural)"), `ру́сским` ("with Russian (instrumental)"), `кни́гой` ("with a book (instrumental)"), `э́том` ("in this (masculine or neuter prepositional)"), `моём` ("in my (masculine or neuter prepositional)"), `на́шем` ("in our (masculine or neuter prepositional)"), `но́вом` ("in new (masculine or neuter prepositional)"), `большо́м` ("in big (masculine or neuter prepositional)"), `мои́` ("my (plural)"), `на́ши` ("our (plural)"), `больши́е` ("big (plural)"), `э́тих` ("of/in these (plural genitive or prepositional)"), `мои́х` ("of/in my (plural genitive or prepositional)"), `на́ших` ("of/in our (plural genitive or prepositional)"), `но́вых` ("of/in new (plural genitive or prepositional)"), `больши́х` ("of/in big (plural genitive or prepositional)"), `кни́гах` ("in books (prepositional plural)"), `зада́чах` ("in problems, in tasks (prepositional plural)"), `кни́гам` ("to books (dative plural)"), `э́тими` ("with these (instrumental plural)"), `на́шими` ("with our (instrumental plural)"), `но́выми` ("with new (instrumental plural)"), `кни́гами` ("with books (instrumental plural)").

---

## 3. Every New Spoken String (One per Line, Stressed)

```text
дава́й
дава́й сде́лаем
пить
сде́лаю
ла́дно
коне́чно
гото́во
на, держи́
ча́йник
плита́
на плите́
холоди́льник
ра́ковина
шкаф
в шкафу́
по́лка
на по́лке
ба́нка
кру́жка
кастрю́ля
сковорода́
кры́шка
ми́ска
стака́н
воды́
ви́лка
нож
ну
ой
без
без молока́
без са́хара
с лимо́ном
с мя́той
я так хочу́ есть
па́хнет
пельме́ни
макаро́ны
с сы́ром
карто́шка
рис
сала́т
ма́сло
яйцо́
смета́на
кефи́р
я́блоко
лимо́н
мёд
имби́рь
мя́та
кори́ца
мука́
варе́нье
я́годы
грибы́
пиро́г
бу́лочки
сы́рники
раз
ко́нчилось
вообще́
оста́лось
чуть-чу́ть
то́чно
купи́ть
па́чка
буты́лка
спи́сок
запи́ска
а то
совсе́м
забу́ду
магази́н
откры́т
закры́т
кипи́т
горя́чий
кре́пкий
тёплый
холо́дный
све́жий
вку́сный
невку́сный
чи́стый
гря́зный
пусто́й
почти́
про́сто
голо́дный
голо́дная
люби́ть
ты лю́бишь
ты пи́шешь
ходи́ть
ты хо́дишь
положи́
клади́
поста́вь
ставь
возьми́
бери́
посмотри́
наре́жь
не режь
осторо́жно
не тро́гай
ло́жку
помеша́й
посоли́
попро́буй
сади́сь
вы́ключи
плиту́
нале́й
помо́й
ча́шку
закро́й
иди́ сюда́
дай
разогре́й
сде́лай
посу́да
посу́ду
помо́ю
вы́тру
уберу́
хле́бница
салфе́тки
корми́л
покорми́л
покорми́
кота́
съел
убежа́ло
не расстра́ивайся
всё равно́
мыть
тря́пка
свари́ть
завари́ть
перевари́ть
испе́чь
накрыва́ть на стол
дуршла́г
доска́
про́тивень
прихва́тка
доста́ть
духо́вка
осты́нет
ту́рка
кофемаши́на
допьёшь
по́здно
пора́ спать
ключи́
очки́
часы́
телефо́н
звони́т
тру́бку
дверь
стучи́т
свет
сосе́дка
подержи́
держу́
здра́вствуйте
извини́те
прости́
проходи́те
сади́тесь
возьми́те
бери́те
курье́р
посы́лка
проду́кты
домофо́н
откро́йте
поднима́йтесь
эта́ж
пека́рня
реце́пт
сходи́ за хле́бом
не ходи́
не забу́дь
нас то́лько дво́е
помоги́
разобра́ть
су́мку
друг
дру́га
дру́гу
дру́гом
э́того
э́тому
э́той
моего́
моему́
мое́й
на́шего
на́шему
на́шей
но́вого
но́вому
но́вой
сло́ва
сло́ву
сло́вом
к
э́ту
мою́
на́шу
но́вую
большу́ю
зада́чу
э́тим
мои́м
но́вым
ру́сским
кни́гой
э́том
моём
на́шем
но́вом
большо́м
мои́
на́ши
больши́е
э́тих
мои́х
на́ших
но́вых
больши́х
кни́гах
зада́чах
кни́гам
э́тими
на́шими
но́выми
кни́гами
— Дава́й сде́лаем ко́фе! — Дава́й! Сейча́с сде́лаю.
— Дава́й пить чай! — Ла́дно, ча́йник уже́ на плите́, гото́во!
— Мо́жно стака́н воды́? — Коне́чно! На, держи́!
На ку́хне есть плита́, холоди́льник, ра́ковина и шкаф, а в шкафу́ на по́лке — ба́нка и кру́жка.
Кастрю́ля, сковорода́ и кры́шка на плите́, а ми́ска, ви́лка и нож тут.
Дава́й сде́лаем ко́фе!
Дава́й пить чай!
Мо́жно стака́н воды́?
— Дава́й сде́лаем ко́фе! Где кру́жка и ба́нка? — Ла́дно! Кру́жка и ба́нка в шкафу́ на по́лке, а ча́йник, кастрю́ля, сковорода́ и кры́шка на плите́. Коне́чно, сейча́с сде́лаю — гото́во!
холоди́льник, ра́ковина, плита́, шкаф, по́лка, ми́ска, стака́н воды́, ви́лка, нож
Дава́й
— Дава́й пить чай! — На, держи́!
Дава́й сде́лаем
— Дава́й сде́лаем ко́фе! — Сейча́с сде́лаю.
— Где соль? — Ну как где? Она́ же тут, на столе́!
— Э́то ведь соль, а не са́хар! — Ой! Ну ла́дно, дава́й ко́фе без молока́ и чай без са́хара, с лимо́ном и с мя́той.
— Я так хочу́ есть! Вку́сно па́хнет! Бу́дешь суп и́ли пельме́ни? — Бу́ду! И макаро́ны с сы́ром, карто́шка, рис и сала́т то́же тут.
В холоди́льнике есть ма́сло, яйцо́, смета́на, кефи́р и я́блоко, а на по́лке — мука́, мёд, имби́рь, кори́ца и лимо́н.
У нас есть варе́нье, я́годы, пиро́г, бу́лочки и сы́рники, а на у́жин — грибы́.
Э́то ведь соль!
Она́ же тут!
Ну ла́дно!
— Я так хочу́ есть! Вку́сно па́хнет! Бу́дешь суп, пельме́ни и́ли макаро́ны с сы́ром? — Ну ла́дно, дава́й! А где смета́на и ма́сло? — Ну как где? Они́ же тут: яйцо́, смета́на, кефи́р, карто́шка, рис, сала́т и я́блоко на столе́!
лимо́н, мёд, имби́рь, мя́та, кори́ца, мука́, варе́нье, я́годы, грибы́, пиро́г, бу́лочки, сы́рники
— Э́то ведь соль! — Ой, дава́й ко́фе без молока́ и без са́хара.
— Где ча́шка? — Она́ же тут, а там чай с лимо́ном и с мя́той!
— Раз, два, три… Молоко́ ко́нчилось! — То́чно, молока́ вообще́ нет, оста́лось то́лько чуть-чу́ть!
Ну́жно купи́ть молоко́ и во́ду: вот па́чка молока́, буты́лка воды́, спи́сок и запи́ска, а то я совсе́м забу́ду!
Ча́йник уже́ кипи́т: чай горя́чий и кре́пкий, суп ещё тёплый, а сок холо́дный.
Хлеб све́жий и вку́сный, а не ста́рый и невку́сный; стака́н чи́стый, а не гря́зный и пусто́й.
Магази́н почти́ закры́т и́ли ещё откры́т? Про́сто я голо́дный, и она́ то́же голо́дная!
Молоко́ ко́нчилось!
Ча́йник уже́ кипи́т!
Хлеб све́жий и вку́сный.
— Раз, два, три… Молоко́ ко́нчилось, молока́ вообще́ нет, оста́лось чуть-чу́ть! — То́чно! Вот па́чка молока́, буты́лка воды́, спи́сок и запи́ска, а то я совсе́м забу́ду купи́ть молоко́, пока́ магази́н откры́т, а не закры́т.
ча́йник кипи́т, горя́чий, тёплый, холо́дный, све́жий, вку́сный, невку́сный, кре́пкий, чи́стый, гря́зный, пусто́й, почти́, про́сто, голо́дный, голо́дная
Молоко́ ко́нчилось, молока́ вообще́ нет!
Ча́йник уже́ кипи́т, чай горя́чий и кре́пкий.
Люби́ть: я люблю́ чай, а ты лю́бишь ко́фе.
Писа́ть: я пишу́ по-ру́сски, и ты пи́шешь по-ру́сски.
Ходи́ть и ви́деть: я хожу́ до́ма, и ты хо́дишь до́ма; я ви́жу дом, и ты ви́дишь дом.
Хоте́ть: я хочу́ знать, и ты хо́чешь знать.
— Что ты лю́бишь и что ты пи́шешь? — Я люблю́ чай и пишу́ по-ру́сски. А ты хо́дишь до́ма, ви́дишь то, что я ви́жу, и хо́чешь знать то, что я хочу́ знать!
Люби́ть: ты лю́бишь ко́фе, и я то́же люблю́ ко́фе.
пишу́
Писа́ть: ты пи́шешь по-ру́сски, и я то́же пишу́ по-ру́сски.
хожу́
Ходи́ть: ты хо́дишь до́ма, и я то́же хожу́ до́ма.
хочу́
Хоте́ть: ты хо́чешь знать, и я то́же хочу́ знать.
люби́ть — люблю́ — ты лю́бишь, писа́ть — пишу́ — ты пи́шешь, ходи́ть — хожу́ — ты хо́дишь, хоте́ть — хочу́ — ты хо́чешь
— Положи́ нож на стол и поста́вь ча́шку на по́лке! — Ла́дно, а куда́ класть ви́лку? — Клади́ тут и ставь стака́н там!
— Возьми́ ло́жку, помеша́й суп, посоли́ и попро́буй! — Гото́во! — Сади́сь, бери́ хлеб!
— Посмотри́, ча́йник кипи́т? — Смотри́, уже́ кипи́т! Вы́ключи плиту́ и нале́й мне чай, пожа́луйста.
— Наре́жь пиро́г! — Подожди́, осторо́жно, не тро́гай и пока́ не режь: он о́чень горя́чий!
— Иди́ сюда́, дай мне таре́лку, разогре́й суп и сде́лай чай, а пото́м помо́й ча́шку и закро́й окно́!
Положи́ нож на стол!
Клади́ тут!
Возьми́ ло́жку!
— Иди́ сюда́, посмотри́ в шкафу́, дай мне таре́лку, разогре́й суп и сде́лай чай! — Смотри́, всё тут! — Положи́ нож на стол, поста́вь стака́н, наре́жь хлеб, сади́сь и бери́ суп!
положи́ — клади́, поста́вь — ставь, возьми́ — бери́, посмотри́ — смотри́, наре́жь — не режь, осторо́жно, не тро́гай
Возьми́
— Возьми́ ло́жку, помеша́й суп, посоли́ и попро́буй!
— Сади́сь и бери́ хлеб, а горя́чий пиро́г пока́ не режь и не тро́гай!
Вы́ключи
— Вы́ключи плиту́, нале́й чай, а пото́м помо́й ча́шку и закро́й окно́!
— Дава́й я помо́ю посу́ду! — Ла́дно, а я пока́ вы́тру стол, уберу́ хлеб в хле́бницу, где чи́стая посу́да и салфе́тки.
— Ты корми́л кота́? Покорми́ кота́! — Коне́чно, покорми́л! Он всё съел.
— Ой, молоко́ убежа́ло! — Не расстра́ивайся, всё равно́ мыть плиту́: вот тря́пка!
На ку́хне ну́жно свари́ть суп, завари́ть чай и не перевари́ть макаро́ны; вот дуршла́г, доска́, про́тивень и прихва́тка, что́бы доста́ть пиро́г из духо́вки — пусть осты́нет!
У́тром мо́жно свари́ть ко́фе в ту́рке и́ли из кофемаши́ны, накрыва́ть на стол и испе́чь пиро́г; а ве́чером, когда́ допьёшь чай, уже́ по́здно, пора́ спать!
Дава́й я помо́ю посу́ду!
Молоко́ убежа́ло!
Ты корми́л кота́?
— Дава́й я помо́ю посу́ду! А ты корми́л кота́? Покорми́ кота́! — Коне́чно, покорми́л, он всё съел! А я пока́ вы́тру стол, вот тря́пка, хле́бница и духо́вка: ну́жно доста́ть пиро́г, пусть осты́нет.
свари́ть, завари́ть, перевари́ть, испе́чь, накрыва́ть на стол, дуршла́г, доска́, про́тивень, прихва́тка, салфе́тки, ту́рка, кофемаши́на
— Дава́й я помо́ю посу́ду! — А я пока́ вы́тру стол и уберу́ хлеб.
— Ой, молоко́ убежа́ло! — Не расстра́ивайся, всё равно́ мыть плиту́!
Когда́ допьёшь чай, уже́ по́здно, пора́ спать!
— Кто́-то стучи́т в дверь: э́то сосе́дка! Подержи́ кота́! — Держу́! — Здра́вствуйте, проходи́те, сади́тесь, извини́те за шум, возьми́те и бери́те пиро́г!
— У тебя́ звони́т телефо́н, возьми́ тру́бку! — Здра́вствуйте! — Э́то курье́р, тут посы́лка и проду́кты: откро́йте домофо́н, пожа́луйста! — Поднима́йтесь, наш эта́ж там!
— Где мои́ ключи́, очки́ и часы́? — Прости́, не зна́ю! Сходи́ за хле́бом в пека́рню, то́лько не ходи́ до́лго и не забу́дь купи́ть молоко́: вот реце́пт, нас то́лько дво́е.
— Помоги́ мне разобра́ть су́мку и вы́ключи свет на ку́хне!
Возьми́ тру́бку!
Откро́йте, пожа́луйста!
Подержи́ кота́!
— Кто́-то стучи́т в дверь, и звони́т телефо́н: возьми́ тру́бку и подержи́ кота́! — Держу́! Здра́вствуйте! Да, э́то наш домофо́н, поднима́йтесь, вот наш эта́ж: курье́р тут, вот посы́лка и проду́кты!
ключи́, очки́, часы́, телефо́н, дверь, свет, сосе́дка, пека́рня, реце́пт, сходи́ за хле́бом, не ходи́, не забу́дь, прости́, извини́те, проходи́те, сади́тесь, возьми́те, бери́те, откро́йте, нас то́лько дво́е
— У тебя́ звони́т телефо́н, возьми́ тру́бку!
Здра́вствуйте
— Здра́вствуйте, проходи́те и сади́тесь, пожа́луйста!
Помоги́ мне разобра́ть су́мку и вы́ключи свет на ку́хне.
Имени́тельный: э́тот мой но́вый друг · э́то на́ше но́вое сло́во · э́та моя́ но́вая кни́га
Роди́тельный: у э́того моего́ но́вого дру́га · нет э́того на́шего но́вого сло́ва · нет э́той мое́й но́вой кни́ги
Да́тельный: э́тому моему́ но́вому дру́гу · к э́тому на́шему но́вому сло́ву · к э́той на́шей но́вой кни́ге
Я зна́ю э́того но́вого дру́га. — Я пишу́ э́тому но́вому дру́гу.
Я зна́ю э́того но́вого дру́га. Я пишу́ э́тому но́вому дру́гу.
— У э́того моего́ но́вого дру́га есть э́та моя́ но́вая кни́га? — Нет э́той на́шей но́вой кни́ги и нет э́того на́шего но́вого сло́ва, но я пишу́ э́тому моему́ но́вому дру́гу к э́тому на́шему но́вому сло́ву и к э́той на́шей но́вой кни́ге.
У э́того на́шего но́вого дру́га нет э́той но́вой кни́ги.
У э́того моего́ но́вого дру́га нет э́той мое́й но́вой кни́ги.
Я пишу́ письмо́ э́тому моему́ но́вому дру́гу.
э́тот мой но́вый друг, у э́того моего́ но́вого дру́га, э́тому моему́ но́вому дру́гу, нет э́той мое́й но́вой кни́ги, к э́той на́шей но́вой кни́ге
Вини́тельный: э́тот мой но́вый дом · э́ту мою́ но́вую кни́гу · э́ту на́шу большу́ю зада́чу
Твори́тельный: с э́тим мои́м но́вым дру́гом · с ру́сским сло́вом · с э́той мое́й но́вой кни́гой
Предло́жный: в э́том моём но́вом до́ме · в на́шем большо́м го́роде · в э́той на́шей но́вой кни́ге
Я чита́ю э́ту мою́ но́вую кни́гу. — Я иду́ с э́той мое́й но́вой кни́гой.
Я чита́ю э́ту мою́ но́вую кни́гу. Я иду́ с э́той мое́й но́вой кни́гой.
— Ты чита́ешь э́ту на́шу большу́ю зада́чу в э́той на́шей но́вой кни́ге? — Да, мы живём в э́том моём но́вом до́ме в на́шем большо́м го́роде и говори́м с э́тим мои́м но́вым дру́гом с ру́сским сло́вом.
Мы живём в э́том на́шем но́вом до́ме.
Я иду́ в го́род с э́тим мои́м но́вым дру́гом.
Э́то сло́во есть в э́той на́шей но́вой кни́ге, а я чита́ю э́ту мою́ но́вую кни́гу.
э́ту мою́ но́вую кни́гу, с э́той мое́й но́вой кни́гой, в э́той на́шей но́вой кни́ге, с э́тим мои́м но́вым дру́гом, в э́том моём но́вом до́ме
Имени́тельный и вини́тельный: э́ти мои́ но́вые кни́ги · э́ти на́ши больши́е зада́чи
Роди́тельный и предло́жный: нет э́тих мои́х но́вых книг · в э́тих на́ших больши́х зада́чах
Да́тельный и твори́тельный: к э́тим мои́м но́вым кни́гам · с э́тими на́шими но́выми кни́гами
Я чита́ю э́ти мои́ но́вые кни́ги. — В э́тих мои́х но́вых кни́гах есть э́ти на́ши больши́е зада́чи.
Я чита́ю э́ти мои́ но́вые кни́ги. В э́тих мои́х но́вых кни́гах есть э́ти на́ши больши́е зада́чи.
— У нас нет э́тих мои́х но́вых книг и э́тих больши́х книг? — В э́тих на́ших больши́х зада́чах и в э́тих мои́х но́вых кни́гах мно́го книг: я иду́ с э́тими на́шими но́выми кни́гами к э́тим мои́м но́вым кни́гам.
У нас нет э́тих мои́х но́вых книг.
Я иду́ домо́й с э́тими на́шими но́выми кни́гами.
э́ти мои́ но́вые кни́ги, нет э́тих мои́х но́вых книг, в э́тих на́ших больши́х зада́чах, к э́тим мои́м но́вым кни́гам, с э́тими на́шими но́выми кни́гами
```

---

## 4. What Could Not Be Done Without Reordering Units (and What I Would Do Next)

1. **Alphabet-stage domestic imperatives (`u5` / `u7` vs. `u13` / `u17`):**
   - Because we never insert pages before the earliest unit a learner may not have reached yet (`u13`) or reorder units, the bare `ты` imperatives remain introduced in `h1` (`u13`: `Дава́й!`, `На, держи́!`) and `h4` (`u17`: full aspectual pairs `положи́ / клади́`, `поста́вь / ставь`, `возьми́ / бери́`, `посмотри́ / смотри́`, `наре́жь / не режь`) rather than inside `u5`–`u7`. Since `u13` is the very first post-alphabet unit in `UNIT_ORDER`, `ember` now meets `дава́й`, `ла́дно`, `сейча́с`, `гото́во`, `коне́чно`, and `на, держи́` on page #29 of the course (`h1`).
2. **Prepositional case frames before `u18`:**
   - In `UNIT_ORDER`, `u13`, `u14`, and `u15` precede `u18` (*In, on, and what you see*). Consequently, `на плите́`, `в шкафу́`, `на по́лке`, and `на столе́` are taught on `h1` and `h2` as high-utility lexical chunks (just as `до́ма` is taught in `u4`), before `u18` and `g6` generalize the prepositional case.
3. **What I would do next:**
   - **Auto-index single words from multi-word `LEXICON` entries in `build_course.py`:** Right now, multi-word entries (`я хочу́`, `мы чита́ем`, `на столе́`, `дава́й сде́лаем`) require either separate single-word entries or episode-level glosses in `series.json` / `kitchen.json`. Populating single-word lookup keys automatically at build time would make every constituent word tappable everywhere in the app.
   - **Add `paradigm_fill` and `cue_sort` exercises to `a1`–`a3`:** `build_paradigms.py` already defines the `adj`, `poss-мой`, and `poss-наш` tables; adding a `paradigm_fill` or `cue_sort` card to `a3` (once `этот` is added to `paradigms.json`) would connect the new adjective case pages directly to the interactive grammar reference.
