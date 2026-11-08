# Deputy Author's Report: «Ку́хня» (`kitchen.json`) & Early Domestic Russian

**Author:** Gemini 4 Argon (Deputy Author, branch `deputy-gemini-kitchen`)  
**Target Learner:** `ember` (adult CS/math/philosophy reader living with a Russian speaker)  
**Deliverables:**
1. `apple/Drussky/Resources/kitchen.json` — 30 overheard domestic dialogues (`kukhnya-01`..`kukhnya-30`, 25–80 words and 4–6 turns each) rising across all 30 units of the course from `u3-p3` to `u30-p4`.
2. `apple/scripts/validate_series.py` — minimally updated to accept any series file path as `sys.argv[1]`, validate the optional `notice` array (`1..3` stressed strings), and check `25..80` words per episode when `series.id == "kukhnya"`.
3. This report (`DEPUTY_REPORT.md`) — the 150 domestic words and phrases built around, per-episode coverage, concrete unit/page changes to pull home conversation forward, and design disagreements with the current course.

---

## 1. Summary of Changes to `apple/scripts/validate_series.py`

To ensure `python3 apple/scripts/validate_series.py apple/Drussky/Resources/kitchen.json` validates `kitchen.json` with zero duplication while preserving default validation of `series.json`, three minimal edits were made to `apple/scripts/validate_series.py`:

1. **CLI path argument (`validate(series_path)` & `__main__`, lines 139, 145–150, 351, 355–357):** `validate()` now accepts `series_path: pathlib.Path = SERIES_PATH`, and `__main__` passes `pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else SERIES_PATH`.
2. **Optional `notice` field validation (lines 242–251):** When an episode includes `"notice"`, `validate_series.py` verifies that it is a list of `1..3` non-empty strings and runs `check_stress()` on every Russian token inside each notice string.
3. **Series-appropriate word-count bounds (lines 256–258):** Uses `min_w, max_w = (25, 80) if sid == "kukhnya" else (100, 250)`. All other checks (stress on polysyllables without `ё`, no stress on `ё` or monosyllables, letter progression by `after_page`, single-word lexicon vs. `glosses` coverage, no Cyrillic in `en` / `glosses` values / question `options`, and non-decreasing `after_page`) apply identically to both `series.json` and `kitchen.json`.

---

## 2. The Cast & World of «Ку́хня»

- **Ка́тя (`Katya`)** — actually knows how to cook soup, pelmeni, and Sunday *syrniki*; warms the teapot before steeping tea; leaves handwritten notes on the fridge (`kukhnya-13`); spots things Max is staring right at (`Она́ же тут, на столе́!`).
- **Макс (`Max`)** — a computer scientist who makes the morning coffee (preferring the *turka* over the machine), mistakes the salt jar for sugar before he's fully awake (`kukhnya-05`), forgets to buy milk even when he goes to the shop (`kukhnya-10`, `kukhnya-12`, finally redeeming himself in `kukhnya-30`), and stays up late at the kitchen table with tea, Descartes (`kukhnya-17`), and kitchen proofs (`kukhnya-21`, `kukhnya-29`).
- **Ти́ма (`Tima`)** — their ginger cat, who stalks cheese on the windowsill (`kukhnya-01`), sleeps directly on top of Max's lost phone (`kukhnya-15`), tries to bolt out the front door when someone knocks (`kukhnya-16`), and meows at 9 a.m. pretending Max didn't feed him fish at 8 a.m. (`kukhnya-18`).
- **Ве́ра Па́вловна (`Vera Pavlovna`, `kukhnya-16`)** & **Курье́р (`The Courier`, `kukhnya-20`)** — the next-door neighbour and delivery courier, providing natural, real-time switching between domestic `ты` (`Подержи́ Ти́му!`, `Дай па́чку со́ли!`, `Встре́ть его́!`) and polite `вы` (`Извини́те`, `Проходи́те`, `Возьми́те`, `Откро́йте, пожа́луйста`, `Поднима́йтесь`).

---

## 3. The 150 Domestic Words & Phrases Built Around

The following 150 words and multi-word spoken patterns form the recurring spine of `kitchen.json`. They are grouped functionally so the course author can pull them into early units (`u3`–`u15`).

### A. Aspectual Kitchen Imperatives: Requests, Steps, Prohibitions & Invitations (1–30)

| # | Stressed Russian | English / Pragmatic Role | First Ep (`after_page`) |
|---|---|---|---|
| 1 | **смотри́! / посмотри́!** | look! (impf. attention-getter vs. pf. check a spot) | `kukhnya-01` (`u3-p3`) / `kukhnya-05` (`u7-p5`) |
| 2 | **иди́ сюда́! / иди́ туда́!** | come here! / go over there! | `kukhnya-02` (`u4-p4`), `kukhnya-03` (`u5-p3`) |
| 3 | **бери́! / возьми́!** | go ahead and take! (impf. invitation) vs. take/grab! (pf. request) | `kukhnya-03` (`u5-p3`), `kukhnya-05` (`u7-p5`) |
| 4 | **дай (мне)! / дава́й (+ acc.)!** | hand me / give me! (`Дай таре́лку!`, `Дава́й ча́шку!`, `Дава́й тря́пку!`) | `kukhnya-03` (`u5-p3`), `kukhnya-06` (`u13-p3`) |
| 5 | **попро́буй!** | taste it / try it! (pf. imperative) | `kukhnya-03` (`u5-p3`) |
| 6 | **подожди́!** | wait / hold on! (pf. imperative) | `kukhnya-04` (`u6-p3`) |
| 7 | **положи́! (на стол)** | put / lay flat! (pf. imperative for bread, cutlery, keys) | `kukhnya-04` (`u6-p3`) |
| 8 | **поста́вь! (на ме́сто / на стол)** | put / stand upright! (pf. imperative for cups, jars, plates, pots) | `kukhnya-05` (`u7-p5`) |
| 9 | **сде́лай (мне ко́фе)!** | make (me coffee)! (pf. imperative) | `kukhnya-05` (`u7-p5`) |
| 10 | **сади́сь (за стол)! / сади́тесь!** | sit down (at the table)! (impf. invitation, `ты` vs. `вы`) | `kukhnya-06` (`u13-p3`), `kukhnya-25` (`u25-p4`) |
| 11 | **переда́й, пожа́луйста!** | pass (the butter / salt), please! | `kukhnya-06` (`u13-p3`) |
| 12 | **на, держи́!** | here you go, take/hold it! (said while handing something over) | `kukhnya-06` (`u13-p3`) |
| 13 | **наре́жь! / пока́ не режь!** | slice it! (pf.) vs. don't cut it yet! (impf. prohibition) | `kukhnya-06` (`u13-p3`), `kukhnya-25` (`u25-p4`) |
| 14 | **скажи́ ещё раз, поме́дленнее!** | say it once more, slower! (essential kitchen survival line) | `kukhnya-07` (`c1-p1`) |
| 15 | **купи́!** | buy! (pf. imperative) | `kukhnya-10` (`u15-p4`) |
| 16 | **помеша́й (ло́жкой)!** | stir (with a spoon)! (pf. imperative) | `kukhnya-11` (`u9-p4`) |
| 17 | **посоли́ (чуть-чу́ть)!** | salt it (a tiny bit)! (pf. imperative) | `kukhnya-11` (`u9-p4`) |
| 18 | **вы́ключи (плиту́ / свет)!** | turn off / switch off (the stove / the light)! | `kukhnya-11` (`u9-p4`), `kukhnya-14` (`u11-p4`) |
| 19 | **прости́! / извини́те!** | sorry / forgive me (`ты`) vs. excuse me (`вы`) | `kukhnya-12` (`u10-p4`), `kukhnya-16` (`u22-p2`) |
| 20 | **разогре́й (сам)!** | heat it up / warm it up (yourself)! | `kukhnya-13` (`u31-p7`) |
| 21 | **покорми́ (кота́ / Ти́му)!** | feed (the cat / Tima)! | `kukhnya-13` (`u31-p7`) |
| 22 | **помо́й / вы́мой (ча́шку / ру́ки)!** | wash (your cup / hands)! | `kukhnya-14` (`u11-p4`), `kukhnya-20` (`u20-p4`) |
| 23 | **закро́й (окно́)! / откро́йте!** | close (the window)! / open (the door/intercom, polite `вы`)! | `kukhnya-14` (`u11-p4`), `kukhnya-20` (`u20-p4`) |
| 24 | **подержи́! — Держу́!** | hold (him/this) for a moment! — Got (him/it)! | `kukhnya-16` (`u22-p2`) |
| 25 | **проходи́те / возьми́те / бери́те!** | come on in / please take / help yourselves! (`вы` hospitality) | `kukhnya-16` (`u22-p2`), `kukhnya-25` (`u25-p4`) |
| 26 | **нале́й! (Нали́ть тебе́ ча́ю?)** | pour some! (answering dative + infinitive offer `Нали́ть тебе́…?`) | `kukhnya-17` (`u16-p4`) |
| 27 | **сходи́ (за хле́бом)! / не ходи́!** | run out (for bread, round trip)! vs. don't go! | `kukhnya-19` (`u19-p4`) |
| 28 | **не забу́дь (+ inf.)!** | don't forget (to take out…)! | `kukhnya-24` (`u24-p5`) |
| 29 | **осторо́жно! не тро́гай!** | careful / watch out! don't touch! (impf. warning) | `kukhnya-25` (`u25-p4`) |
| 30 | **помоги́ (мне)! / не расстра́ивайся!** | help (me)! / don't be upset! | `kukhnya-26` (`u26-p5`), `kukhnya-28` (`u28-p5`) |

### B. Conversational Particles, Discourse Connectors & Spoken Replies (31–55)

| # | Stressed Russian | English / Pragmatic Role | First Ep (`after_page`) |
|---|---|---|---|
| 31 | **же** (`Она́ же тут!`, `Я же ходи́л!`) | emphatic particle pointing to the obvious ('right here! / after all!') | `kukhnya-04` (`u6-p3`) |
| 32 | **ведь** (`Э́то ведь соль!`) | particle appealing to shared knowledge ('you know / after all') | `kukhnya-05` (`u7-p5`) |
| 33 | **ну** (`Ну как где?`, `Ну дава́й`, `Ну ла́дно`) | discourse softener, prompt, or gentle tease | `kukhnya-02` (`u4-p4`), `kukhnya-03` (`u5-p3`) |
| 34 | **вообще́ (вообще́ нет)** | at all / completely out of (`Молока́ вообще́ нет!`) | `kukhnya-10` (`u15-p4`) |
| 35 | **дава́й + 1pl pf.** (`Дава́й сде́лаем / испечём`) | let's (do a single completed action together)! — `Дава́й!` (Deal!) | `kukhnya-08` (`u14-p5`), `kukhnya-21` (`u21-p4`) |
| 36 | **дава́й я + 1sg pf.** (`Дава́й я помо́ю`) | let me / I'll volunteer to (wash the dishes)! | `kukhnya-09` (`u8-p5`) |
| 37 | **дава́й + impf. inf.** (`Дава́й пить чай / за́втракать`) | let's sit down and (drink tea / have breakfast)! | `kukhnya-23` (`u23-p6`), `kukhnya-30` (`u30-p4`) |
| 38 | **мо́жно (мо́жно ещё…?)** | may I have (more…) / is it possible? | `kukhnya-04` (`u6-p3`), `kukhnya-06` (`u13-p3`) |
| 39 | **так** (`Так э́то Ти́ма!`) | unstressed initial particle of realization ('Why, that's…!') | `kukhnya-01` (`u3-p3`) |
| 40 | **ла́дно** | alright, okay, fine (universal domestic agreement) | `kukhnya-02` (`u4-p4`) |
| 41 | **гото́во!** | done! / it's ready! | `kukhnya-02` (`u4-p4`), `kukhnya-24` (`u24-p5`) |
| 42 | **коне́чно** | of course! | `kukhnya-04` (`u6-p3`) |
| 43 | **сейча́с (+ 1sg pf.)** (`Сейча́с сде́лаю / налью́ / откро́ю`) | I'm on it right now! (immediate promise) | `kukhnya-04` (`u6-p3`), `kukhnya-05` (`u7-p5`) |
| 44 | **пока́** (`А я пока́ вы́тру стол`) | meanwhile / in the meantime (coordinating two people's chores) | `kukhnya-09` (`u8-p5`), `kukhnya-20` (`u20-p4`) |
| 45 | **вон там / вот э́то** | right over there / this thing right here (pointing deictics) | `kukhnya-07` (`c1-p1`), `kukhnya-09` (`u8-p5`) |
| 46 | **раз, два, три…** | one, two, three… (counting aloud with `раз`, not `оди́н`) | `kukhnya-10` (`u15-p4`) |
| 47 | **а то** (`…а то опя́ть забу́ду!`) | or else / otherwise (colloquial conjunction) | `kukhnya-10` (`u15-p4`) |
| 48 | **чуть-чу́ть** | a tiny bit, just a little | `kukhnya-11` (`u9-p4`) |
| 49 | **совсе́м** (`совсе́м забы́л`, `совсе́м дёшево`) | completely, totally, quite | `kukhnya-12` (`u10-p4`), `kukhnya-26` (`u26-p5`) |
| 50 | **почти́** (`почти́ всё`) | almost | `kukhnya-13` (`u31-p7`) |
| 51 | **пора́ (+ inf.)** (`Уже́ по́здно, пора́ спать!`) | it's time to (sleep / go)! | `kukhnya-14` (`u11-p4`) |
| 52 | **то́чно!** (`То́чно!`, `то́чно так же`) | spot on! / exact! / the exact same way | `kukhnya-15` (`u18-p4`), `kukhnya-18` (`u17-p5`) |
| 53 | **про́сто** (`Он про́сто хи́трый кот`) | just, simply | `kukhnya-18` (`u17-p5`) |
| 54 | **всё равно́** | anyway, all the same, regardless | `kukhnya-17` (`u16-p4`) |
| 55 | **то́лько что / не мо́жет быть! / де́ло в том, что** | just now / it can't be! / the thing is that… | `kukhnya-26`, `kukhnya-28`, `kukhnya-30` |

### C. Kitchen Appliances, Furniture & Home Fixtures (56–75)

| # | Stressed Russian | English / Key Case Frames | First Ep (`after_page`) |
|---|---|---|---|
| 56 | **ку́хня** (`на ку́хне`, `на ку́хню`) | kitchen (in the kitchen / into the kitchen) | `kukhnya-13` (`u31-p7`), `kukhnya-16` (`u22-p2`) |
| 57 | **стол** (`на столе́`, `на стол`, `за стол`) | table (on the table / onto the table / at the table) | `kukhnya-02` (`u4-p4`), `kukhnya-04` (`u6-p3`), `kukhnya-06` (`u13-p3`) |
| 58 | **окно́** (`на окне́`, `у окна́`, `к окну́`, `в окно́`) | window / windowsill | `kukhnya-01` (`u3-p3`), `kukhnya-02` (`u4-p4`) |
| 59 | **плита́** (`на плите́`, `вы́ключи плиту́`) | stove, cooker | `kukhnya-03` (`u5-p3`), `kukhnya-11` (`u9-p4`) |
| 60 | **холоди́льник** (`в холоди́льнике`, `из холоди́льника`) | fridge, refrigerator | `kukhnya-10` (`u15-p4`), `kukhnya-12` (`u10-p4`), `kukhnya-24` (`u24-p5`) |
| 61 | **ра́ковина** (`у ра́ковины`, `на ра́ковину`, `в ра́ковине`) | kitchen sink | `kukhnya-07` (`c1-p1`), `kukhnya-09` (`u8-p5`), `kukhnya-29` (`u29-p4`) |
| 62 | **шкаф** (`в шкафу́`) | cupboard, cabinet (with true locative `-у́`) | `kukhnya-05` (`u7-p5`) |
| 63 | **по́лка** (`на по́лке`) | shelf | `kukhnya-05` (`u7-p5`) |
| 64 | **духо́вка** (`из духо́вки`) | oven | `kukhnya-24` (`u24-p5`), `kukhnya-25` (`u25-p4`) |
| 65 | **стул** (`на сту́ле`) | chair | `kukhnya-15` (`u18-p4`) |
| 66 | **дверь** (`стучи́т в дверь`, `у две́ри`) | door | `kukhnya-16` (`u22-p2`), `kukhnya-20` (`u20-p4`) |
| 67 | **свет** (`вы́ключу свет`) | light | `kukhnya-14` (`u11-p4`) |
| 68 | **домофо́н** (`домофо́н внизу́ не рабо́тает`) | intercom, entry buzzer | `kukhnya-20` (`u20-p4`) |
| 69 | **эта́ж** (`на тре́тий эта́ж`) | floor, storey | `kukhnya-20` (`u20-p4`) |
| 70 | **хле́бница** (`в хле́бнице`) | breadbox | `kukhnya-13` (`u31-p7`) |
| 71 | **ту́рка** (`свари́ть ко́фе в ту́рке`) | cezve (Turkish coffee pot) | `kukhnya-27` (`u27-p5`) |
| 72 | **кофемаши́на** (`из кофемаши́ны`) | coffee machine | `kukhnya-27` (`u27-p5`) |
| 73 | **часы́** (`Я не ви́жу часы́`) | clock, watch | `kukhnya-26` (`u26-p5`) |
| 74 | **ми́ска** (`у ми́ски`, `ми́ска с сала́том`) | bowl (cat's bowl or salad bowl) | `kukhnya-01` (`u3-p3`), `kukhnya-13` (`u31-p7`), `kukhnya-24` (`u24-p5`) |
| 75 | **тря́пка** (`дава́й тря́пку`) | kitchen cloth, rag | `kukhnya-28` (`u28-p5`) |

### D. Cookware, Tableware, Containers & Everyday Objects (76–95)

| # | Stressed Russian | English / Key Case Frames | First Ep (`after_page`) |
|---|---|---|---|
| 76 | **ча́шка** (`моя́ ча́шка`, `ча́шку`, `ча́шка из-под ко́фе`) | cup | `kukhnya-04` (`u6-p3`), `kukhnya-29` (`u29-p4`) |
| 77 | **кру́жка** (`Чья э́то кру́жка?`) | mug | `kukhnya-09` (`u8-p5`) |
| 78 | **стака́н** (`два стака́на молока́`) | glass, tumbler | `kukhnya-02` (`u4-p4`), `kukhnya-21` (`u21-p4`) |
| 79 | **таре́лка** (`таре́лку`, `ско́лько таре́лок`, `ря́дом с таре́лками`) | plate | `kukhnya-02` (`u4-p4`), `kukhnya-03` (`u5-p3`), `kukhnya-24` (`u24-p5`) |
| 80 | **ло́жка** (`помеша́й ло́жкой`, `ло́жки`) | spoon | `kukhnya-04` (`u6-p3`), `kukhnya-11` (`u9-p4`) |
| 81 | **ви́лка** (`ви́лки`) | fork | `kukhnya-02` (`u4-p4`), `kukhnya-24` (`u24-p5`) |
| 82 | **нож** (`ножи́`) | knife | `kukhnya-06` (`u13-p3`), `kukhnya-24` (`u24-p5`) |
| 83 | **ча́йник / зава́рочный ча́йник** | kettle / teapot for steeping tea | `kukhnya-04` (`u6-p3`), `kukhnya-22` (`u22-p4`) |
| 84 | **кастрю́ля** (`в кастрю́ле`, `в кастрю́лю`, `от кастрю́ли`) | saucepan, pot | `kukhnya-07` (`c1-p1`), `kukhnya-08` (`u14-p5`), `kukhnya-11` (`u9-p4`) |
| 85 | **сковорода́** | frying pan, skillet | `kukhnya-07` (`c1-p1`) |
| 86 | **кры́шка** (`кры́шка от кастрю́ли`) | lid | `kukhnya-07` (`c1-p1`) |
| 87 | **дуршла́г** | colander, strainer | `kukhnya-07` (`c1-p1`) |
| 88 | **посу́да** (`помо́ю посу́ду`, `мыть посу́ду`) | the dishes (collective singular) | `kukhnya-09` (`u8-p5`), `kukhnya-17` (`u16-p4`) |
| 89 | **ба́нка** (`вот э́та ба́нка`) | jar | `kukhnya-05` (`u7-p5`) |
| 90 | **па́чка** (`две па́чки молока́`, `па́чку со́ли`) | pack, carton, box | `kukhnya-10` (`u15-p4`), `kukhnya-16` (`u22-p2`) |
| 91 | **буты́лка** (`буты́лку молока́`) | bottle | `kukhnya-26` (`u26-p5`) |
| 92 | **салфе́тки** | napkins | `kukhnya-24` (`u24-p5`) |
| 93 | **про́тивень / прихва́тка / доска́** | baking tray / oven mitt / cutting board | `kukhnya-25` (`u25-p4`) |
| 94 | **ключи́ / очки́ / телефо́н** | keys / glasses / phone | `kukhnya-15` (`u18-p4`), `kukhnya-20` (`u20-p4`) |
| 95 | **запи́ска / спи́сок / реце́пт / су́мка / посы́лка** | fridge note / shopping list / recipe / bag / parcel | `kukhnya-10`, `kukhnya-13`, `kukhnya-20`, `kukhnya-21`, `kukhnya-26` |

### E. Everyday Foods, Drinks & Ingredients (96–120)

| # | Stressed Russian | English / Key Collocations | First Ep (`after_page`) |
|---|---|---|---|
| 96 | **ко́фе** (`сде́лай мне ко́фе`, `свари́ть ко́фе`, `кре́пкий ко́фе`) | coffee (masculine agreement!) | `kukhnya-05` (`u7-p5`), `kukhnya-27` (`u27-p5`) |
| 97 | **чай** (`наш чай`, `ча́ю с мя́той`, `к ча́ю`) | tea | `kukhnya-04` (`u6-p3`), `kukhnya-17` (`u16-p4`), `kukhnya-19` (`u19-p4`) |
| 98 | **молоко́** (`молока́ нет`, `с молоко́м`, `молоко́ убежа́ло`) | milk | `kukhnya-02` (`u4-p4`), `kukhnya-08` (`u14-p5`), `kukhnya-10` (`u15-p4`) |
| 99 | **вода́** (`вода́ кипи́т`, `в воде́`) | water | `kukhnya-02` (`u4-p4`), `kukhnya-05` (`u7-p5`), `kukhnya-11` (`u9-p4`) |
| 100 | **сок** | juice | `kukhnya-01` (`u3-p3`), `kukhnya-24` (`u24-p5`) |
| 101 | **сыр** (`с сы́ром`) | cheese | `kukhnya-01` (`u3-p3`), `kukhnya-08` (`u14-p5`) |
| 102 | **ма́сло** (`сто гра́ммов ма́сла`) | butter | `kukhnya-02` (`u4-p4`), `kukhnya-06` (`u13-p3`) |
| 103 | **хлеб** (`за све́жим хле́бом`) | bread | `kukhnya-04` (`u6-p3`), `kukhnya-19` (`u19-p4`) |
| 104 | **са́хар** (`без са́хара`, `с са́харом`) | sugar | `kukhnya-04` (`u6-p3`), `kukhnya-05` (`u7-p5`), `kukhnya-08` (`u14-p5`) |
| 105 | **соль** (`немно́го со́ли`) | salt | `kukhnya-05` (`u7-p5`), `kukhnya-16` (`u22-p2`) |
| 106 | **суп** (`бу́дем суп?`, `свари́ть суп`) | soup | `kukhnya-03` (`u5-p3`), `kukhnya-13` (`u31-p7`) |
| 107 | **сала́т** (`ми́ска с сала́том`) | salad | `kukhnya-03` (`u5-p3`), `kukhnya-08` (`u14-p5`), `kukhnya-24` (`u24-p5`) |
| 108 | **ка́ша** (`свари́ла ка́шу`) | porridge | `kukhnya-06` (`u13-p3`), `kukhnya-23` (`u23-p6`) |
| 109 | **макаро́ны** (`макаро́ны с сы́ром`) | pasta, macaroni | `kukhnya-08` (`u14-p5`), `kukhnya-23` (`u23-p6`) |
| 110 | **пельме́ни** (`варю́ пельме́ни`) | pelmeni, dumplings | `kukhnya-11` (`u9-p4`) |
| 111 | **карто́шка** (`варю́ карто́шку`) | potatoes | `kukhnya-11` (`u9-p4`) |
| 112 | **лук** | onion | `kukhnya-10` (`u15-p4`) |
| 113 | **яйцо́** (`три яйца́`, `де́сять яи́ц`, `с яйцо́м`) | egg(s) | `kukhnya-10` (`u15-p4`), `kukhnya-21` (`u21-p4`) |
| 114 | **смета́на** (`смета́ны нет`, `смета́ну`, `дади́м смета́ны`) | sour cream | `kukhnya-10` (`u15-p4`), `kukhnya-11` (`u9-p4`), `kukhnya-12` (`u10-p4`) |
| 115 | **кефи́р** | kefir | `kukhnya-12` (`u10-p4`) |
| 116 | **мука́** (`ру́ки в муке́`, `четы́реста гра́ммов муки́`) | flour | `kukhnya-20` (`u20-p4`), `kukhnya-21` (`u21-p4`) |
| 117 | **мёд и имби́рь** (`чай с мёдом`) | honey and ginger | `kukhnya-22` (`u22-p4`) |
| 118 | **лимо́н / мя́та / кори́ца** (`с лимо́ном`, `с мя́той`, `с кори́цей`) | lemon / mint / cinnamon | `kukhnya-08` (`u14-p5`), `kukhnya-17` (`u16-p4`), `kukhnya-19` (`u19-p4`) |
| 119 | **варе́нье / я́годы / грибы́** (`с варе́ньем`, `с гриба́ми`) | jam / berries / mushrooms | `kukhnya-23` (`u23-p6`), `kukhnya-24` (`u24-p5`) |
| 120 | **пиро́г / бу́лочки / сы́рники** | pie / buns / cottage-cheese pancakes | `kukhnya-19`, `kukhnya-21`, `kukhnya-24`, `kukhnya-30` |

### F. Core Domestic Verbs & Spoken Predicates (121–138)

| # | Stressed Russian | English / Construction | First Ep (`after_page`) |
|---|---|---|---|
| 121 | **бу́дешь / бу́дем + food noun** (`Бу́дем суп?`, `Чай бу́дешь?`) | will you have / shall we have…? (omitting 'eat/drink') | `kukhnya-03` (`u5-p3`), `kukhnya-08` (`u14-p5`) |
| 122 | **кипе́ть** (`ча́йник / вода́ кипи́т`) | to boil (intransitive: the water/kettle is boiling) | `kukhnya-04` (`u6-p3`), `kukhnya-08` (`u14-p5`), `kukhnya-11` (`u9-p4`) |
| 123 | **па́хнуть + instr.** (`вку́сно па́хнет!`, `чем па́хнет? — пирого́м!`) | to smell (good / of X in instrumental) | `kukhnya-06` (`u13-p3`), `kukhnya-24` (`u24-p5`), `kukhnya-28` (`u28-p5`) |
| 124 | **я так хочу́ есть!** | I'm so hungry! (literally 'I want to eat so badly') | `kukhnya-08` (`u14-p5`) |
| 125 | **мыть / помы́ть / вы́мыть** (`посу́ду`, `ру́ки`, `ча́шку`) | to wash (dishes, hands, cup) | `kukhnya-09` (`u8-p5`), `kukhnya-14` (`u11-p4`), `kukhnya-17` (`u16-p4`) |
| 126 | **вы́тереть** (`вы́тру стол / плиту́`) | to wipe (the table / stove) | `kukhnya-09` (`u8-p5`), `kukhnya-28` (`u28-p5`) |
| 127 | **убра́ть** (`уберу́ хлеб`, `убира́ть со стола́`) | to put away, clear away | `kukhnya-09` (`u8-p5`) |
| 128 | **забы́ть / забы́ть про + acc.** (`забу́ду молоко́`, `забы́л про молоко́`) | to forget (about the milk) | `kukhnya-10` (`u15-p4`), `kukhnya-12` (`u10-p4`) |
| 129 | **ко́нчиться** (`молоко́ ко́нчилось ещё у́тром!`) | to run out, be all gone | `kukhnya-12` (`u10-p4`) |
| 130 | **допи́ть / съесть** (`когда́ допьёшь чай`, `всё съел за мину́ту`) | to finish drinking / eat all up | `kukhnya-14` (`u11-p4`), `kukhnya-18` (`u17-p5`) |
| 131 | **звони́ть / взять тру́бку** (`у тебя́ звони́т телефо́н`, `возьми́ тру́бку`) | to ring / pick up the phone | `kukhnya-20` (`u20-p4`) |
| 132 | **вари́ть / свари́ть / завари́ть / перевари́ть** | boil / cook / steep (tea) / overcook (root `вар-`) | `kukhnya-11` (`u9-p4`), `kukhnya-23` (`u23-p6`), `kukhnya-27` (`u27-p5`) |
| 133 | **испе́чь** (`дава́й испечём пиро́г`) | to bake | `kukhnya-21` (`u21-p4`) |
| 134 | **доста́ть (из холоди́льника / духо́вки)** | to take out, get out (of the fridge / oven) | `kukhnya-22` (`u22-p4`), `kukhnya-24` (`u24-p5`), `kukhnya-25` (`u25-p4`) |
| 135 | **накрыва́ть на стол** | to set the table | `kukhnya-24` (`u24-p5`) |
| 136 | **осты́ть** (`пусть он немно́го осты́нет`) | to cool down | `kukhnya-25` (`u25-p4`) |
| 137 | **разобра́ть су́мку** | to unpack a grocery bag | `kukhnya-26` (`u26-p5`) |
| 138 | **убежа́ть** (`Ой, молоко́ убежа́ло!`) | to boil over (literally 'run away') | `kukhnya-28` (`u28-p5`) |

### G. Domestic Adjectives, Modals, Time & Quantity Frames (139–150)

| # | Stressed Russian | English / Construction | First Ep (`after_page`) |
|---|---|---|---|
| 139 | **горя́чий / тёплый / холо́дный** | hot / warm / cold (for things; vs. `хо́лодно` for ambient) | `kukhnya-04` (`u6-p3`), `kukhnya-14` (`u11-p4`), `kukhnya-27` (`u27-p5`) |
| 140 | **вку́сный / невку́сный / вкусне́е** | tasty / bad-tasting / tastier | `kukhnya-03` (`u5-p3`), `kukhnya-19` (`u19-p4`), `kukhnya-27` (`u27-p5`) |
| 141 | **све́жий / ста́рый** | fresh / old (stale) | `kukhnya-10` (`u15-p4`), `kukhnya-19` (`u19-p4`), `kukhnya-23` (`u23-p6`) |
| 142 | **гря́зный / чи́стый** (`гря́зные таре́лки`, `гря́зная ча́шка`) | dirty / clean | `kukhnya-09` (`u8-p5`), `kukhnya-29` (`u29-p4`) |
| 143 | **пусто́й** (`пуста́я ча́шка`) | empty | `kukhnya-15` (`u18-p4`) |
| 144 | **голо́дный / голо́дная** | hungry (masculine / feminine) | `kukhnya-08` (`u14-p5`), `kukhnya-13` (`u31-p7`) |
| 145 | **кре́пкий / кре́пче** (`са́мый кре́пкий ко́фе`) | strong / stronger (of coffee or tea) | `kukhnya-27` (`u27-p5`) |
| 146 | **откры́т / закры́т** (`магази́н уже́ закры́т`) | open / closed (short predicates) | `kukhnya-16` (`u22-p2`) |
| 147 | **мне ну́жен / нужна́ / ну́жно** | I need (agreeing with the thing needed) | `kukhnya-22` (`u22-p4`), `kukhnya-27` (`u27-p5`) |
| 148 | **нас то́лько дво́е** | there are only two of us | `kukhnya-21` (`u21-p4`) |
| 149 | **че́рез де́сять мину́т / полчаса́ до у́жина / без двадцати́ семь** | in 10 minutes / half an hour until dinner / 6:40 | `kukhnya-08` (`u14-p5`), `kukhnya-26` (`u26-p5`) |
| 150 | **из-под + gen.** (`гря́зная ча́шка из-под ко́фе`) | container named by what it recently held | `kukhnya-29` (`u29-p4`) |

---

## 4. Per-Episode Coverage (`kukhnya-01` to `kukhnya-30`)

Every episode aligns with one of the 30 units from `u3` to `u30` in exact course page order (`UNIT_ORDER` in `apple/scripts/build_course.py`). Episodes 1–4 strictly obey the partial alphabet met at `u3-p3` (14 letters), `u4-p4` (18 letters), `u5-p3` (23 letters), and `u6-p3` (28 letters).

| Ep | ID | `after_page` (Pg#) | Title (`title_en`) | Words | Core Spoken Notices (`notice`) | Key Glossed Words (New & Reused) |
|---:|---|---|---|---|---|---|
| 1 | `kukhnya-01` | `u3-p3` (#10) | **Сыр и Ти́ма** (*Cheese and Tima*) | 44 | `Смотри́!` (`ты` imperative); `А где…?` (topic pivot); `Так э́то Ти́ма!` (realization `так`) | `смотри`, `миска`, `окне`, `снова`, `мясо`, `твоя`, `так` |
| 2 | `kukhnya-02` | `u4-p4` (#14) | **Где молоко́?** (*Where Is the Milk?*) | 47 | `Ну как где?` (gentle tease); `там` vs. `иди́ туда́!`; `Ла́дно, гото́во` | `утро`, `столе`, `только`, `ну`, `у окна`, `масло`, `стакан`, `вилка`, `тарелка`, `иди`, `туда`, `ладно`, `готово` |
| 3 | `kukhnya-03` | `u5-p3` (#17) | **Бу́дем суп?** (*Shall We Have Soup?*) | 49 | `Бу́дем суп?` (future `быть` + food noun); `бери́` (impf.) vs. `попро́буй! / дай!` (pf.) | `сюда`, `обед`, `плите`, `будем`, `салат`, `давай`, `бери`, `тарелку`, `попробуй`, `вкусный`, `рис`, `яблоко`, `дай` |
| 4 | `kukhnya-04` | `u6-p3` (#20) | **Ча́йник и ча́шка** (*The Kettle and the Cup*) | 49 | `Она́ же тут!` (particle `же`); `Подожди́! / Положи́!`; `А я сде́лаю чай` (pf. future) | `чашка`, `хочу`, `же`, `ложка`, `чайник`, `уже`, `горячий`, `подожди`, `сейчас`, `кипит`, `можно`, `сахар`, `конечно`, `положи`, `сделаю` |
| 5 | `kukhnya-05` | `u7-p5` (#25) | **Соль и́ли са́хар?** (*Salt or Sugar?*) | 49 | `Сде́лай — Сейча́с сде́лаю!`; `Э́то ведь соль!` (particle `ведь`); `Поста́вь на ме́сто` vs. `Возьми́` | `спишь`, `сделай`, `пожалуйста`, `посмотри`, `шкафу`, `берёшь`, `ведь`, `банка`, `поставь`, `место`, `возьми`, `полке` |
| 6 | `kukhnya-06` | `u13-p3` (#28) | **Переда́й ма́сло** (*Pass the Butter*) | 46 | `На, держи́!` (handing something over); `А тебе́ наре́зать хлеб?` (dative + inf. offer); `Дава́й ча́шку!` | `садись`, `пахнет`, `приятного аппетита`, `нож`, `на`, `держи`, `нарезать`, `нарежь`, `немного`, `чашку`, `налью` |
| 7 | `kukhnya-07` | `c1-p1` (#29) | **Как э́то по-ру́сски?** (*How Do You Say This in Russian?*) | 50 | `Подожди́, скажи́ поме́дленнее!`; pointing with `Э́то вот э́то?`; `кры́шка от кастрю́ли` | `дуршлаг`, `крышку/крышка`, `вон`, `раковины`, `скажи`, `раз`, `помедленнее`, `сковорода`, `кастрюля`, `от` |
| 8 | `kukhnya-08` | `u14-p5` (#34) | **Дава́й поу́жинаем** (*Let's Have Dinner*) | 49 | `Я так хочу́ есть!` vs. `голо́дная`; `Дава́й сде́лаем — Дава́й!`; `с лимо́ном, но без са́хара` | `голодная`, `сделаем`, `макароны`, `сыром`, `будешь`, `молоком`, `лимоном`, `без сахара`, `кастрюле`, `через десять минут` |
| 9 | `kukhnya-09` | `u8-p5` (#39) | **Чья э́то кру́жка?** (*Whose Mug Is This?*) | 50 | `Чья э́то кру́жка?`; `Дава́й я помо́ю посу́ду!` (volunteering); mid-sentence `пока́` ('meanwhile') | `раковину`, `грязные`, `оставили`, `после завтрака`, `кружка`, `чья`, `помою`, `посуду`, `пока`, `вытру`, `уберу` |
| 10 | `kukhnya-10` | `u15-p4` (#43) | **Купи́ яйцо́ и смета́ну** (*Buy Eggs and Sour Cream*) | 48 | Counting aloud `Раз, два, три…`; `вообще́ нет` ('none at all'); `а то` ('or else') | `иду`, `магазин`, `купить/купи`, `холодильнике`, `яиц/яйца`, `раз`, `молока`, `сметаны/сметану`, `вообще`, `две пачки`, `запишу`, `список`, `забуду` |
| 11 | `kukhnya-11` | `u9-p4` (#47) | **Когда́ бу́дет гото́во?** (*When Will It Be Ready?*) | 47 | Chained pf. steps `Помеша́й, посоли́ и вы́ключи плиту́`; impf. duration `Ско́лько мину́т ещё вари́ть?`; `чуть-чу́ть` | `скоро`, `варю/варить`, `пельмени`, `картошку`, `кастрюлю`, `сильно`, `помешай`, `ложкой`, `посоли`, `чуть-чуть`, `выключи`, `плиту` |
| 12 | `kukhnya-12` | `u10-p4` (#51) | **Опя́ть молока́ нет!** (*There's No Milk Again!*) | 52 | `Оно́ ко́нчилось!` ('ran out'); `забы́ть про + acc.`; partitive genitive `дади́м смета́ны` | `открыла`, `холодильник`, `ходил`, `кефир`, `кончилось`, `прости`, `купил`, `совсем`, `забыл про`, `пить`, `тиме`, `дадим` |
| 13 | `kukhnya-13` | `u31-p7` (#58) | **Запи́ска на холоди́льнике** (*A Note on the Fridge*) | 48 | `Алло́!`; fridge-note imperatives `Разогре́й сам`, `Покорми́ Ти́му`; `Что там напи́сано?` | `алло`, `кухне`, `записку`, `почерк`, `почти`, `разогрей`, `сам`, `хлебнице`, `написано`, `непонятно`, `покорми`, `голодный`, `сидит` |
| 14 | `kukhnya-14` | `u11-p4` (#62) | **Вы́ключи свет!** (*Turn Off the Light!*) | 48 | `Уже́ по́здно, пора́ спать!`; completion prefix `до-` (`допьёшь чай`); `закро́й окно́ и вы́ключи свет` | `поздно`, `пора спать`, `тёплый`, `приду`, `допьёшь`, `помой`, `закрой/закрою`, `холодно`, `выключу`, `свет`, `спокойной ночи` |
| 15 | `kukhnya-15` | `u18-p4` (#66) | **Где ключи́ и телефо́н?** (*Where Are the Keys and the Phone?*) | 49 | Negative past question `Ты не ви́дела мои́ ключи́?`; double negation `нигде́ не ви́жу`; `То́чно!` | `видела`, `ключи`, `телефон`, `нигде`, `вижу`, `очки`, `пустая`, `под книгой`, `стуле`, `спит`, `точно`, `прямо`, `котом` |
| 16 | `kukhnya-16` | `u22-p2` (#71) | **Сосе́дка у две́ри** (*The Neighbour at the Door*) | 55 | `ты` (to Max/Katya) vs. `вы` + patronymic (`Ве́ра Па́вловна`); `Подержи́ — Держу́!`; `закры́т` | `кто-то`, `стучит`, `дверь`, `подержи/держу`, `выбежит`, `извините`, `закрыт`, `проходите`, `кухню`, `пачку`, `возьмите` |
| 17 | `kukhnya-17` | `u16-p4` (#75) | **Ночь, чай и звёзды** (*Night, Tea, and Stars*) | 53 | `Нали́ть тебе́ ча́ю? — Нале́й!`; `всё равно́` ('anyway'); dative obligation + end focus `мыть тебе́!` | `спим`, `какие`, `большие`, `маленькая`, `налить/налей`, `мятой`, `космосе`, `декарте`, `всё равно`, `мыть`, `философия`, `суровый` |
| 18 | `kukhnya-18` | `u17-p5` (#80) | **Ты покорми́л кота́?** (*Did You Feed the Cat?*) | 58 | Past aspect pair `Ты корми́л его́? — Коне́чно, покорми́л!`; stress shift `спала́`; `про́сто` | `громко`, `мяукает`, `кормил/покормил`, `дал`, `рыбу`, `съел`, `за минуту`, `будто`, `ел`, `просто`, `хитрый`, `спала`, `артист` |
| 19 | `kukhnya-19` | `u19-p4` (#84) | **Я схожу́ за хле́бом** (*I'll Run Out for Bread*) | 53 | Motion prefixes `сходи́ть за хле́бом` (round trip) & `зайти́ в пека́рню` (drop by); `к ча́ю`; `Ско́ро бу́ду!` | `по дороге`, `зайду`, `пекарню`, `что-нибудь`, `сходи/схожу`, `за свежим хлебом`, `к чаю`, `булочки`, `корицей`, `подойдут`, `невкусный`, `обратно` |
| 20 | `kukhnya-20` | `u20-p4` (#88) | **Курье́р звони́т** (*The Courier Is Calling*) | 56 | `Возьми́ тру́бку, у меня́ ру́ки в муке́!`; phone/courier `вы` (`Откро́йте`, `Поднима́йтесь`) vs. `ты` (`Встре́ть`) | `звонит`, `трубку`, `руки в муке`, `здравствуйте`, `слушаю`, `курьер`, `привёз`, `посылку`, `продукты`, `домофон`, `откройте`, `поднимайтесь`, `этаж`, `встреть`, `вымою` |
| 21 | `kukhnya-21` | `u21-p4` (#92) | **Полови́на и реце́пт** (*Half a Recipe*) | 67 | `Нас то́лько дво́е` (collective numeral); `возьмём / испечём`; container/weight + genitive | `испечём`, `яблочный пирог`, `рецепт`, `четыреста граммов муки`, `двое`, `возьмём`, `двести`, `математик`, `отдадим`, `требовалось доказать` |
| 22 | `kukhnya-22` | `u22-p4` (#94) | **Заче́м и почему́** (*What For and Why*) | 54 | `Почему́?` (cause) vs. `Заче́м?` (purpose); `что́бы + inf.` vs. `что́бы чай был вку́сным`; `зава́рочный ча́йник` | `выключила`, `налила`, `согреть`, `заварочный чайник`, `вкусным`, `подождать`, `зачем/затем`, `достала`, `мёд`, `имбирь`, `заболеть` |
| 23 | `kukhnya-23` | `u23-p6` (#100) | **Оди́н ко́рень на ку́хне** (*One Root in the Kitchen*) | 56 | Kitchen root `вар-`: `вари́ть / свари́ть / завари́ть / перевари́ть / варе́нье`; `вот тебе́ и…`; `нельзя́ + pf. inf.` | `приставки`, `звучит`, `одинаково`, `сварить`, `заварить`, `переварить`, `корень`, `нельзя`, `варенье`, `ягоды` |
| 24 | `kukhnya-24` | `u24-p5` (#105) | **К нам иду́т го́сти** (*Guests Are Coming Over*) | 58 | `накрыва́ть на стол`; `поста́вь таре́лки` vs. `положи́ ви́лки и ножи́`; `Чем па́хнет? — Пирого́м с гриба́ми!` | `осталось`, `к нам`, `гости/гостей`, `накрываю`, `тарелок/тарелками`, `рядом с`, `салфетки`, `чем пахнет`, `духовки`, `пирогом с грибами`, `не забудь достать` |
| 25 | `kukhnya-25` | `u25-p4` (#109) | **Не тро́гай, горя́чо!** (*Don't Touch, It's Hot!*) | 51 | Aspect triangle in imperatives: pf. (`Поста́вь!`), impf. prohibition (`Не тро́гай! Пока́ не режь!`), impf. invitation (`Проходи́те, сади́тесь, бери́те!`); `пусть осты́нет` | `осторожно`, `не трогай`, `противень`, `руками`, `прихватку`, `достану`, `доску`, `режь`, `пусть остынет`, `садитесь`, `берите`, `попробуем` |
| 26 | `kukhnya-26` | `u26-p5` (#114) | **Полчаса́ до у́жина** (*Half an Hour Until Dinner*) | 52 | Clock time `без двадцати́ семь` (6:40); `то́лько что пришла́`; `помоги́ мне разобра́ть су́мку` | `который час`, `часы`, `без двадцати`, `пришла с рынка`, `килограмма яблок`, `бутылку`, `помоги`, `разобрать сумку` |
| 27 | `kukhnya-27` | `u27-p5` (#119) | **Како́й ко́фе лу́чше?** (*Which Coffee Is Better?*) | 57 | `свари́ть ко́фе в ту́рке`; `намно́го вкусне́е и кре́пче, чем…`; `мне нужна́ ло́жка` vs. `мне ну́жен ко́фе` (masculine!) | `турке`, `кофемашины`, `свари`, `намного`, `вкуснее`, `крепче/крепкий`, `лежит`, `впереди` |
| 28 | `kukhnya-28` | `u28-p5` (#124) | **Молоко́ убежа́ло!** (*The Milk Boiled Over!*) | 53 | `Молоко́ убежа́ло!`; `не расстра́ивайся!`; rheme-final word order `плиту́ вы́тру я` | `убежало`, `поставила`, `отвернулась/отворачиваешься`, `секунду`, `дело в том, что`, `смеёшься`, `серьёзно`, `закон природы`, `не расстраивайся`, `тряпку` |
| 29 | `kukhnya-29` | `u29-p4` (#128) | **Теоре́ма о чи́стой ра́ковине** (*The Theorem of the Clean Sink*) | 61 | `ча́шка из-под ко́фе` (`из-под + gen.`); `допу́стим` in math & speech; `посу́ду мо́ет а́втор теоре́мы` | `грязная`, `из-под`, `квартире`, `состоит из`, `допустим`, `зелёный`, `ни тебе, ни коту`, `памятью`, `моет` |
| 30 | `kukhnya-30` | `u30-p4` (#132) | **Бытие́ и у́тренний ко́фе** (*Being and Morning Coffee*) | 64 | `Не мо́жет быть!`; `Дава́й за́втракать!`; `сы́рники` & verb-drop dash (`а на́ше у́тро — горя́чий ко́фе и сы́рники!`) | `солнце`, `сварил`, `не может быть`, `наступила`, `гармония`, `выходной`, `философы`, `полностью согласна`, `своя`, `завтракать`, `сырники` |

---

## 5. What the Course Should Change to Support Home Conversation Sooner (with Unit & Page IDs)

Right now, `ember` lives with a Russian speaker in Week 2 (`u3`–`u7`), yet the main course waits until **Unit 25 (`u25-p4`, page #109 of 132!)** to teach imperatives and until **Unit 28 (`u28-p3`, page #122 of 132!)** to teach conversational particles (`же`, `ведь`, `да́же`). Meanwhile, `u7-p1` (`По́лдень`) and `u16-p1` (`Звезда́, плане́та, ко́смос`) teach `ко́смос`, `плане́та`, and `телеско́п` before `ember` has met `ча́шка`, `ло́жка`, `таре́лка`, `холоди́льник`, `плита́`, or `посмотри́`.

Here are the concrete, surgical changes the course author should make, naming exact unit and page IDs:

1. **Pull bare `ты` imperatives into `u5-p3` and `u7-p3` (instead of waiting until `u25-p4`):**
   - In **`u5-p3`** (*Mini-dialogues*, page #17) and **`u7-p3`** (*Simple questions*, page #23), add an `explain` card introducing the 6 highest-frequency domestic imperatives as lexical chunks long before full imperative morphology: **`смотри́ / посмотри́`**, **`подожди́`**, **`дай`**, **`возьми́`**, **`иди́ сюда́`**, and **`сде́лай`**.
   - Keep **`u25-p4`** (*Requests and commands*) for the full morphological synthesis (how `-и / -й / -ь` are formed from the 3pl stem and the aspectual triangle of single request vs. `не` prohibition vs. guest invitation).

2. **Expand `u13` (*Morning Kitchen*, `u13-p1`..`u13-p3`, pages #26–28) from 3 pages to 5 pages (`u13-p4`, `u13-p5`):**
   - Currently `u13-p1` (*Coffee, tea, bread, butter, cheese*) only teaches food nouns (`хлеб`, `ма́сло`, `ка́ша`, `ча́шка`, `ло́жка`) and `u13-p2` only teaches `Я хочу́… / Ты хо́чешь…? / Бу́дешь чай?`.
   - Add **`u13-p4` (*Kitchenware & surfaces*)**: pull `таре́лка`, `ви́лка`, `нож`, `стака́н`, `ча́йник`, `кастрю́ля`, `плита́`, `холоди́льник`, and `ра́ковина` into `u13` so every noun in the real kitchen is clickable in the course lexicon by page #30.
   - Add **`u13-p5` (*Table interaction & «Дава́й»*)**: teach `На, держи́!`, `Переда́й, пожа́луйста…`, `Дава́й + 1pl` (`Дава́й сде́лаем чай`), and `Дава́й я + 1sg` (`Дава́й я помо́ю посу́ду`).

3. **Add kitchen trouble-shooting & pointing phrases to `c1` (*Survival Conversation*, `c1-p1`, page #29):**
   - `c1-p1` is only a single page teaching `Как э́то по-ру́сски?`, `Повтори́те, пожа́луйста`, and `Ме́дленнее, пожа́луйста`.
   - Add a second page **`c1-p2` (*At Home: Pointing & Clarifying*)** that teaches the `ты` equivalents ember actually uses at home (`Скажи́ ещё раз, поме́дленнее!`, `Подожди́!`, `Что зна́чит «…»?`, `Э́то вот э́то?`, `А вон там — э́то что?`). A learner living with a Russian speaker says `Скажи́ поме́дленнее` (informal `ты`), not `Повтори́те` (formal `вы`).

4. **Move conversational particles (`же`, `ведь`, `ну`, `ла́дно`, `вообще́`, `то́чно`) from `u28-p3` forward into `u8-p4` and `u14-p4`:**
   - In **`u8-p4`** (*Here, there, at home*, page #38), add `же` (`Она́ же тут!`), `ну` (`Ну как где?`), and `ла́дно` (`Ла́дно, гото́во`). Pointing out where things are (`тут / там / до́ма`) is where `же` and `ну` naturally live.
   - In **`u14-p4`** (*At the cafe / At the table*, page #33) and **`u15-p2`** (*У нас нет молока́*, page #41), add `вообще́ нет` (`Молока́ вообще́ нет!`), `ко́нчилось` (`Оно́ ко́нчилось`), and `ведь` (`Э́то ведь соль!`).
   - Repurpose **`u28-p3`** (page #122) to teach written/argumentative discourse markers (`одна́ко`, `впро́чем`, `тем не ме́нее`, `де́ло в том, что`) rather than holding basic spoken `же` and `ведь` hostage until the third-to-last unit.

5. **Teach `с + instrumental` ('with milk / with lemon') alongside `без + genitive` in `u14-p2` / `u15-p2`:**
   - Currently `u15-p2` teaches `без са́хара` and `без молока́` (page #41), while `с молоко́м` and `с лимо́ном` wait until **`u18-p2`** (page #64, 23 pages later!). Nobody orders or makes tea for 6 weeks knowing how to say 'without milk' (`без молока́`) but unable to say 'with milk' (`с молоко́м`). Pair `с молоко́м / без молока́` and `с са́харом / без са́хара` as contrastive chunks right in **`u14-p2`** (`Чай и́ли ко́фе?`).

---

## 6. What We Disagree With in the Existing Course Design

1. **Multi-word entries in `course["lexicon"]` break single-word tap lookup and force redundant glossing:**
   - In `apple/scripts/build_course.py` (and `course_*.py`), dozens of core words are registered in `LEXICON` only inside multi-word strings — e.g. `"я ду́маю"`, `"я понима́ю"`, `"я хочу́"`, `"ты хо́чешь"`, `"я бу́ду"`, `"ты бу́дешь"`, `"на столе́"`, `"на ку́хне"`, `"до́брое у́тро"`, `"он был"`, `"она́ была́"`.
   - Because `validate_series.py` (and the app's single-word tap lookup) filters `lex_single = {k for k in lex_all if " " not in k}`, the individual tokens `думаю`, `понимаю`, `хочу`, `хочешь`, `буду`, `будешь`, `столе`, `кухне`, `доброе`, `был`, `была` are treated as **unknown** across the entire course! Every time a character says `Я хочу́ чай` or `на столе́` in `series.json` or `kitchen.json`, those words have to be re-glossed manually.
   - **Fix:** `build_course.py` should either split multi-word `LEXICON` entries into single-word lexicon entries (preserving the multi-word phrase as an example) or auto-populate `course["lexicon"]` with each constituent word at the same `first_page`.

2. **Treating imperatives (`u25`) and particles (`u28`) as "late grammar" instead of "Day 1 pragmatics":**
   - Traditional Russian textbooks put imperatives at the end because their stem alternations (`писать → пиши́`, `резать → режь`, `пить → пей`) look morphologically irregular. But in a home with a Russian speaker, imperatives are 40% of all utterances directed at the learner (`Иди́ сюда́`, `Посмотри́`, `Подожди́`, `Дай`, `Возьми́`, `Попро́буй`, `Не тро́гай`). Waiting until page #109 (`u25-p4`) to introduce them mistakes morphological completeness for communicative utility.

3. **Front-loading `вы` in survival units (`c1-p1`, `u14`) while under-teaching the `ты ↔ вы` switch:**
   - `c1-p1` teaches `Повтори́те, пожа́луйста` and `Говори́те ме́дленнее`. That is tourist Russian for speaking to customs officers or hotel clerks, not domestic Russian for speaking to the partner you live with (`Подожди́, скажи́ поме́дленнее!`). Conversely, when a neighbour (`kukhnya-16`) or courier (`kukhnya-20`) appears, the learner needs to hear how two people at home talk to each other in `ты` and pivot in the same breath to `вы` for the person at the door.

4. **Teaching aspect (`u17`) only through narrative past/present pairs (`чита́л / прочита́л`) while ignoring kitchen aspect:**
   - At home, `ember` experiences aspect first and foremost in three spoken environments:
     1. **Immediate offers & promises:** `Сде́лай мне ко́фе! — Сейча́с сде́лаю!` / `Нали́ть тебе́ ча́ю? — Нале́й!`
     2. **Did-it-happen vs. completed-result past questions:** `Ты корми́л его́ у́тром? — Коне́чно, покорми́л!` (`kukhnya-18`)
     3. **The imperative aspect triangle:** single step (`Поста́вь на до́ску!`) vs. prohibition (`Не тро́гай! Пока́ не режь!`) vs. warm invitation (`Проходи́те, сади́тесь, бери́те!`, `kukhnya-25`).
   - `u17` (`u17-p1`..`u17-p5`) should use these exact domestic exchanges rather than relying solely on third-person narrative sentences.
