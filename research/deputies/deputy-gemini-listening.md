# Deputy Editor Report — Native-Ear Audit of `listening.json`

- **Deputy**: Gemini 4 Argon (`deputy-gemini-listening`)
- **Target**: `apple/Drussky/Resources/listening.json` (12 practice sets / 94 items + 6 `weekly_samples` sets / 30 items = 124 items)
- **Schema / Structural Invariants**: **Zero changes** were made to item `id`s, `kind`s, `after_page` gates, option counts, or `answer` indices. All fixes were made in place to `ru`, `lines[*].ru`, `forms[*].ru`, `en`, `question`, `crucial_span`, `why`, and `glosses`.

---

## 1. Item-by-Item Fixes (Grouped by Set)

### `listen-01` (`after_page`: `u3-p4`)
- `listen-01-02` — `question: "Someone calls toward the next room. Is the speaker asking or telling?"` → `question: "Talking at home: is the speaker asking or telling?"` — In `question_or_statement`, English scene-setting must not leak the answer; `"calls toward the next room"` strongly biased `"Question"` (`answer: 1`) without requiring the learner to hear the IK-3 pitch rise on `Ма́ма до́ма?`.
- `listen-01-05` — `question: "Someone points toward the kitchen. Is the speaker asking or telling?"` → `question: "Checking on the others at home: is the speaker asking or telling?"` — Neutralized the prompt so `"points"` does not cue `"Statement"` vs `"Question"`.

### `listen-02` (`after_page`: `u4-p4`)
- `listen-02-01` — `"Дай сыр."` → `"Дай, пожа́луйста, сыр."` (`en`: `"Give me cheese, please."`, `glosses`: `+дай, +пожалуйста`) — A bare two-word imperative `"Дай сыр."` at the table sounds brusque/barked; inserting parenthetical `пожа́луйста` before the rheme (`сыр`) is natural domestic Russian and keeps `сыр` in sentence-final focus.
- `listen-02-02` — `"Дай мне сок."` (`crucial_span`: `"сок"`) → `"А мне дай сок."` (`en`: `"And give me juice."`, `crucial_span`: `"мне дай сок"`) — Starting a turn with contrastive `А мне...` reflects how people actually ask for their drink at home and avoids three identical `Дай...` openings in a row.
- `listen-02-03` — `"Торт, пожа́луйста."` (`crucial_span`: `"Торт"`) → `"Мне торт, пожа́луйста."` (`en`: `"Cake for me, please."`, `glosses`: `+мне`, `crucial_span`: `"Мне торт"`) — A bare nominative noun + `пожа́луйста` is café-counter shorthand; at home at the table, Russians use the dative beneficiary (`Мне торт, пожа́луйста`).
- `listen-02-04` — `"Дай мне сыр, пожа́луйста."` (`crucial_span`: `"сыр"`) → `"Торт тут, а дай мне сыр, пожа́луйста."` (`en`: `"The cake is here, and give me the cheese, please."`, `crucial_span`: `"дай мне сыр"`) — Mentioning a distractor (`торт`) inside the spoken line forces the learner to separate what is already on the table (`Торт тут`) from what is requested (`а дай мне сыр`), rather than just clicking the only food word heard.
- `listen-02-06` — `"Дай сок, а не сыр."` (`crucial_span`: `"а не сыр"`) → `"Нет, не сыр — дай сок."` (`en`: `"No, not cheese — give me juice."`, `crucial_span`: `"не сыр — дай сок"`) — Two fixes: (1) in real life, when someone reaches for the wrong item, you reject the wrong item first (`Нет, не сыр`) and put the wanted item in final focus (`— дай сок`); (2) the old `crucial_span` (`"а не сыр"`) only excluded cheese without deciding between juice and cake.
- `listen-02-08` — `"Положи́ торт на стол."` (`glosses`: `положи`) → `"Поста́вь торт на стол."` (`glosses`: `поставь`) — **Lexical collocation error**: in Russian, a cake (`торт`, or a cake box/dish) has a base and stands upright, so you **never** say `*положи́ торт`; a native speaker always says `поста́вь торт на стол`.

### `listen-03` (`after_page`: `u7-p4`)
- `listen-03-01` — `"Хлеб, пожа́луйста."` (`crucial_span`: `"Хлеб"`) → `"Дай хлеб, пожа́луйста."` (`en`: `"Give me the bread, please."`, `glosses`: `+дай`, `crucial_span`: `"Дай хлеб"`) — `listen-03-01` and `listen-03-02` had their natural Russian frames swapped: for bread on the table at home, you ask someone to hand/pass it (`Дай хлеб, пожа́луйста`), not order it like a dish from a waiter (`*Хлеб, пожа́луйста`).
- `listen-03-02` — `"Дай чай, пожа́луйста."` (`crucial_span`: `"чай"`) → `"Мне чай, пожа́луйста."` (`en`: `"Tea for me, please."`, `glosses`: `-дай, +мне`, `crucial_span`: `"Мне чай"`) — Conversely, when tea is being poured/offered, `"Мне чай, пожа́луйста"` is far more idiomatic than `"Дай чай"`.
- `listen-03-03` — `"Соль, а не хлеб, пожа́луйста."` (`crucial_span`: `"Соль, а не"`) → `"Дай соль, пожа́луйста, а не хлеб."` (`en`: `"Give me the salt, please, not the bread."`, `glosses`: `+дай`, `crucial_span`: `"соль, пожа́луйста, а не хлеб"`) — Added the verb `Дай` and fixed the truncated `crucial_span` (`"Соль, а не"` cut off before `хлеб`).
- `listen-03-06` — `A: "Где хлеб? Дай мне хлеб, пожа́луйста." / B: "Вот хлеб."` → `A: "А где хлеб? Дай, пожа́луйста." / B: "Вот он."` (`en`: `"A: And where is the bread? Give it to me, please.\nB: Here it is."`, `glosses`: `-мне`, `crucial_span`: `"А где хлеб? Дай"`) — Repeating the noun `хлеб` three times in 8 words is textbook-robotic. Once `А где хлеб?` establishes the topic, Russian drops the object in `Дай, пожа́луйста` and uses `Вот он` when handing it over.
- `listen-03-08` — `B: "Мне то́же сыр, пожа́луйста."` → `B: "И мне то́же сыр, пожа́луйста."` (`en`: `"And cheese for me too, please."`) — In spoken Russian, chiming in with `"me too"` on a request naturally begins with additive `И мне то́же...`.

### `listen-04` (`after_page`: `u14-p5`)
- `listen-04-01` — `B: "Нет, ко́фе с молоко́м, пожа́луйста."` → `B: "Нет, мне ко́фе с молоко́м, пожа́луйста."` (`en`: `"No, coffee with milk for me, please."`) — Adding `мне` makes the counter-request sound like a real kitchen response rather than a menu order.
- `listen-04-02` — `A: "Переда́й мне соль, пожа́луйста." / B: "Вот соль."` → `A: "Переда́й, пожа́луйста, соль." / B: "Вот, пожа́луйста."` (`en`: `"A: Pass the salt, please.\nB: Here you go."`, `crucial_span`: `"Переда́й, пожа́луйста, соль"`) — Nobody hands over the salt shaker saying `"Вот соль."` ("Behold the salt"); Russians say `"Вот, пожа́луйста"` or `"На, держи́"`. Also placed `пожа́луйста` after the imperative so `соль` lands in sentence-final rheme position.
- `listen-04-03` — `B: "Я то́же хочу́ суп."` (`crucial_span`: `"то́же хочу́ суп"`) → `B: "Да, я то́же хочу́."` (`en`: `"Yes, I want some too."`, `crucial_span`: `"я то́же хочу́"`) — In response to `"Я хочу́ суп. А ты?"`, repeating `"суп"` at the end of `"Я то́же хочу́ суп"` is unnatural; Russian elides the repeated object (`"Да, я то́же хочу́"`).
- `listen-04-04` — `"Переда́й са́хар, пожа́луйста, а не соль."` (`crucial_span`: `"Переда́й са́хар"`) → `"Не соль, а са́хар переда́й, пожа́луйста."` (`en`: `"Not the salt — pass the sugar, please."`, `crucial_span`: `"Не соль, а са́хар"`) — Fronting the contrastive corrective focus (`Не соль, а са́хар переда́й`) is much more idiomatic in spoken Russian and forces the learner to track the `Не X, а Y` construction rather than just latching onto the first noun after the verb.
- `listen-04-06` — `"Я хочу́ чай. Нет, ко́фе, пожа́луйста."` (`crucial_span`: `"Нет, ко́фе"`) → `"Хочу́ чай. Ой, нет — ко́фе, пожа́луйста."` (`en`: `"I want tea. Oh, no — coffee, please."`, `glosses`: `+ой`, `crucial_span`: `"Ой, нет — ко́фе"`) — Pronoun drop (`Хочу́ чай`) + spontaneous self-repair marker `Ой, нет — ...` instead of flat `Нет, ...`.
- `listen-04-07` — `A: "Я хочу́ ещё суп. А ты?"` (`crucial_span`: `"хочу́ ещё суп"`) → `A: "Я хочу́ ещё су́па. А ты?"` (`glosses`: `+супа`, `crucial_span`: `"хочу́ ещё су́па"`) — **Grammar/case error**: when asking for *more* of a mass food (`ещё` = "some more"), Russian requires the partitive genitive `ещё су́па` (just like `ещё ча́ю/ча́я`, `ещё воды́`). `*Я хочу́ ещё суп` means "I want another [bowl/kind of] soup".
- `listen-04-08` — `B: "Хорошо́. А мне переда́й соль."` → `B: "Вот, пожа́луйста. А мне переда́й соль."` (`en`: `"Here you go. And pass me the salt."`) — When physically passing the bread at the table, `"Вот, пожа́луйста"` is what a speaker says as they hand it over (`"Хорошо́"` sounds like agreeing to a future task).

### `listen-05` (`after_page`: `u10-p4`)
- `listen-05-01` — `B: "У меня́ есть. Вот хлеб."` → `B: "А у меня́ есть. Вот, пожа́луйста."` (`en`: `"And I do have some. Here you go."`) — Contrastive `А` (`А у меня́ есть`) is mandatory in Russian when contrasting one's own possession with the interlocutor's lack (`У меня́ нет хле́ба`), and `"Вот, пожа́луйста"` replaces the robotic `"Вот хлеб"`.
- `listen-05-02` — `B: "Нет. Дай мне чай, пожа́луйста."` → `B: "Нет, ча́я нет. Дай мне чай, пожа́луйста."` (`en`: `"No, there is no tea. Give me tea, please."`, `glosses`: `+чая`) — Echoing the genitive of absence (`Нет, ча́я нет`) is both more natural in kitchen speech and reinforces the `u10` genitive target.
- `listen-05-03` — `B: "У меня́ то́же нет."` → `B: "И у меня́ то́же нет."` (`en`: `"And I have none either."`) — Added initial `И` before `у меня́ то́же нет` for natural colloquial rhythm.
- `listen-05-06` — `"Дай мне ло́жку, пожа́луйста."` (`crucial_span`: `"ло́жку"`) → `"Ча́шка и таре́лка есть, а дай мне ло́жку, пожа́луйста."` (`en`: `"The cup and plate are here, and give me the spoon, please."`, `crucial_span`: `"дай мне ло́жку"`) — Per the prompt's `pick_object` rule ("the deciding span must really decide... and the distractors must be mentioned or plausible"), mentioning `ча́шка` and `таре́лка` as already present (`есть`) makes all three options heard in the audio so the learner must parse `а дай мне ло́жку`.
- `listen-05-07` — `forms: ["У тебя́ есть ко́фе.", "У тебя́ есть ко́фе?"]`, `question: "A visitor is checking what drinks are available..."` → `forms: ["Ко́фе ещё есть.", "Ко́фе ещё есть?"]`, `en: "There is still coffee."`, `question: "Two people are in the kitchen before breakfast. Is the speaker asking or telling?"`, `crucial_span: "Ко́фе ещё есть."` — **Pragmatic mismatch**: `played` is `0` (the statement), yet the prompt said *"A visitor is checking what drinks are available"* and the statement was `"У тебя́ есть ко́фе."` ("You have coffee" — an bizarre thing to declare to your host when checking what drinks are available!). `"Ко́фе ещё есть."` ("There's still coffee left.") / `"Ко́фе ещё есть?"` ("Is there still coffee left?") is completely natural both as a kitchen statement and as a yes/no question.
- `listen-05-08` — `"Дай две ча́шки. Нет, три ча́шки, пожа́луйста."` (`crucial_span`: `"Нет, три ча́шки"`) → `"Дай две ча́шки. Ой, нет — три, пожа́луйста."` (`en`: `"Give me two cups. Oh, no — three, please."`, `glosses`: `+ой`, `crucial_span`: `"Ой, нет — три"`) — In a self-correction of a number, Russians elide the repeated noun (`Ой, нет — три, пожа́луйста`).

### `listen-06` (`after_page`: `u12-p3`)
- `listen-06-02` — `B: "Она́ на столе́."` → `B: "А вот она́, на столе́."` (`en`: `"Here it is, on the table."`) — When someone says `"Я не зна́ю, где кни́га"` and the other person spots it right there, a Russian says `"А вот она́, на столе́"`, not a flat `"Она́ на столе́"`.
- `listen-06-03` — `B: "Я то́же. Ещё раз, пожа́луйста."` (`crucial_span`: `"Я то́же"`) → `B: "И я то́же. Ещё раз, пожа́луйста."` (`crucial_span`: `"И я то́же"`) — Added initial `И` (`И я то́же`) for natural agreement rhythm.
- `listen-06-05` — `"Приве́т. Я до́ма, на ку́хне. Чай на столе́."` (`q0.crucial_span`: `"Я до́ма"`) → `"Приве́т. Я уже́ до́ма, на ку́хне. Чай на столе́."` (`en`: `"Hi. I am already at home, in the kitchen. The tea is on the table."`, `glosses`: `+уже`, `q0.crucial_span`: `"Я уже́ до́ма"`) — Real check-in voice messages when arriving home use `Я уже́ до́ма` ("I'm already home"), not bare `Я до́ма`.
- `listen-06-06` — `"Я чита́ю кни́гу. Ма́ма на ку́хне. Она́ хо́чет чай."` (`q0.crucial_span`: `"Я чита́ю кни́гу"`) → `"Слу́шай, я тут чита́ю кни́гу, а ма́ма на ку́хне. Она́ хо́чет чай."` (`en`: `"Listen, I am here reading a book, and Mom is in the kitchen. She wants tea."`, `glosses`: `+слушай`, `q0.crucial_span`: `"я тут чита́ю кни́гу"`) — Turned three disconnected textbook declaratives into an actual voice message with opener `Слу́шай`, situational `тут`, and contrastive `а`.
- `listen-06-07` — `"Кни́га на столе́. Нет, на сту́ле."` (`crucial_span`: `"Нет, на сту́ле"`) → `"Кни́га на столе́. Ой, нет — на сту́ле."` (`en`: `"The book is on the table. Oh, no — on the chair."`, `glosses`: `+ой`, `crucial_span`: `"Ой, нет — на сту́ле"`) — Added natural visual self-correction `Ой, нет — ...`.
- `listen-06-08` — `A: "Я не зна́ю, где соль. А ты?"` → `A: "Я не зна́ю, где соль. А ты зна́ешь?"` — After `"Я не зна́ю, где X"`, Russian normally repeats the verb in the question (`"А ты зна́ешь?"` or `"А ты не зна́ешь?"`) rather than bare `"А ты?"`.

### `listen-07` (`after_page`: `u17-p5`)
- `listen-07-01` — `"Приве́т. Сего́дня я рабо́таю до́ма. За́втра бу́дем гуля́ть."` → `"Приве́т. Сего́дня я рабо́таю до́ма. А за́втра бу́дем гуля́ть."` (`en`: `"Hi. Today I am working at home. And tomorrow we will go for a walk."`, updated `crucial_span`s) — Contrastive time frames (`Сего́дня ... А за́втра ...`) require `А` in natural Russian discourse.
- `listen-07-02` — `"Вчера́ я чита́ла кни́гу. Не прочита́ла. Сего́дня бу́ду чита́ть ещё."` (`q0.crucial_span`: `"Не прочита́ла"`) → `"Вчера́ я чита́ла кни́гу, но ещё не прочита́ла. Сего́дня бу́ду чита́ть ещё."` (`en`: `"Yesterday I was reading the book, but I have not finished it yet. Today I will read some more."`, `q0.crucial_span`: `"но ещё не прочита́ла"`) — A standalone one-word-plus-negation sentence `"Не прочита́ла."` sounds telegraphic and clipped; linking the aspectual pair with `, но ещё не прочита́ла` is how Russians naturally express "was reading, but haven't finished it yet".
- `listen-07-03` — `"За́втра у́тром бу́ду до́ма. Нет, ве́чером."` (`crucial_span`: `"Нет, ве́чером"`) → `"За́втра у́тром бу́ду до́ма. Ой, нет, не у́тром — ве́чером."` (`en`: `"Tomorrow morning I will be at home. Oh, no, not in the morning — in the evening."`, `glosses`: `+ой`, `crucial_span`: `"Ой, нет, не у́тром — ве́чером"`) — Explicit negation of the wrong time (`Ой, нет, не у́тром — ве́чером`) matches real voice-message self-repair.
- `listen-07-04` — `B: "Я понима́ю. Он хо́чет чай."` → `B: "А я понима́ю. Он хо́чет чай."` (`en`: `"And I do understand. He wants tea."`) — Added contrastive `А` when B counters A's `"Я не понима́ю"`.
- `listen-07-05` — `"Суп гото́в. Я уже́ у́жинала. Хлеб на столе́."` (`q0.crucial_span`: `"Суп гото́в"`) → `"Слу́шай, суп гото́в. Я уже́ у́жинала, а хлеб на столе́."` (`en`: `"Listen, the soup is ready. I have already had dinner, and the bread is on the table."`, `glosses`: `+слушай`, `q0.crucial_span`: `"суп гото́в"`) — Added voice-message opener `Слу́шай,` and linked the last two clauses with `а` so it doesn't sound like three bullet points read aloud.
- `listen-07-06` — `forms: ["Ты прочита́л кни́гу.", "Ты прочита́л кни́гу?"]` → `forms: ["Ты уже́ прочита́л кни́гу.", "Ты уже́ прочита́л кни́гу?"]` (`en`: `"Have you already finished the book?"`, `glosses`: `+уже`, `crucial_span`: `"Ты уже́ прочита́л кни́гу?"`) — Without `уже́`, the declarative `"Ты прочита́л кни́гу."` ("You finished the book.") has almost no plausible context in conversation; `"Ты уже́ прочита́л кни́гу."` ("You've already finished the book [so we can talk / you don't need it]") and `"Ты уже́ прочита́л кни́гу?"` work naturally as an intonation pair with the exact same words.
- `listen-07-07` — `"Вчера́ мы гуля́ли. Нет, вчера́ мы рабо́тали, гуля́ли сего́дня."` → `"Вчера́ мы гуля́ли. Ой, нет, вчера́ мы рабо́тали, а гуля́ли сего́дня."` (`en`: `"Yesterday we went walking. Oh, no, yesterday we worked, and we went walking today."`, `glosses`: `+ой`) — Asyndetic `, гуля́ли сего́дня` without contrastive `а` is unidiomatic; `, а гуля́ли сего́дня` with `Ой, нет` is completely natural.
- `listen-07-08` — `"За́втра я бу́ду чита́ть до́ма. Ве́чером бу́дем говори́ть по-ру́сски. Ты то́же бу́дешь до́ма?"` (`q0.crucial_span`: `"Ве́чером бу́дем говори́ть по-ру́сски"`) → `"За́втра я бу́ду чита́ть до́ма. А ве́чером бу́дем говори́ть по-ру́сски. Ты то́же бу́дешь до́ма?"` (`en`: `"Tomorrow I will read at home. And in the evening we will speak Russian. Will you be home too?"`, `q0.crucial_span`: `"А ве́чером бу́дем говори́ть по-ру́сски"`) — Added `А` to transition from daytime solo plan to evening joint plan.

### `listen-08` (`after_page`: `u19-p4`)
- `listen-08-01` — `"Я иду́ в магази́н. До́ма нет хле́ба. По́сле магази́на бу́ду до́ма."` → `"Слу́шай, я сейча́с иду́ в магази́н. До́ма нет хле́ба. По́сле магази́на бу́ду до́ма."` (`en`: `"Listen, I am walking to the shop right now. There is no bread at home. After the shop I will be home."`, `glosses`: `+сейчас, +слушай`) — Real voice messages sent on the way out the door start with `"Слу́шай, я сейча́с иду́ в магази́н..."`.
- `listen-08-02` — `"Мы е́дем в го́род. Нет, я е́ду, а ты бу́дешь до́ма."` → `"Мы е́дем в го́род. То есть нет: я е́ду, а ты бу́дешь до́ма."` (`en`: `"We are going to the city. That is, no: I am going, and you will be home."`, `glosses`: `+то`) — When correcting `"Мы"` to `"я..., а ты..."`, Russians use `"То есть нет:"` ("I mean, no:"), not a bare `"Нет,"`.
- `listen-08-05` — `"Я бу́ду до́ма в семь. Нет, в во́семь."` (`crucial_span`: `"Нет, в во́семь"`) → `"Слу́шай, я бу́ду до́ма в семь. Ой, нет, не в семь — в во́семь."` (`en`: `"Listen, I will be home at seven. Oh, no, not at seven — at eight."`, `glosses`: `+ой, +слушай`, `crucial_span`: `"Ой, нет, не в семь — в во́семь"`) — Brought in the exact natural voice-message repair pattern highlighted in the editorial brief (`«Слу́шай, ... Ой, нет, не в семь — в во́семь»`).
- `listen-08-06` — `"Я иду́ домо́й. Хлеб уже́ есть. Молока́ нет, купи́ молоко́, пожа́луйста."` → `"Слу́шай, я иду́ домо́й. Хлеб уже́ есть, а молока́ нет — купи́ молоко́, пожа́луйста."` (`en`: `"Listen, I am walking home. We already have bread, and there is no milk — buy milk, please."`, `glosses`: `+слушай`) — Linked `Хлеб уже́ есть, а молока́ нет` with contrastive `а` and added `Слу́шай,`.
- `listen-08-07` — `B: "Я зна́ю. Идём!"` → `B: "А я зна́ю. Идём!"` (`en`: `"And I do know. Let’s go!"`) — Contrastive `А я зна́ю` in response to `A: "Я не зна́ю, где магази́н."`.

### `listen-09` (`after_page`: `u24-p5`)
- `listen-09-01` — `A: "Мне нужна́ кни́га, а бра́ту ну́жен хлеб."` → `A: "Мне нужна́ кни́га, а бра́ту ну́жен стул."` (`en`: `"A: I need the book, and my brother needs the chair.\nB: All right, I will give you the book."`) — Pairing "I need a book" with "and my brother needs bread" in a single coordinate clause is comical textbook zeugma; pairing two study/room objects (`кни́га` and `стул`) makes a coherent domestic scene.
- `listen-09-04` — `"Дай кни́гу бра́ту. Нет, сестре́, пожа́луйста."` (`crucial_span`: `"Нет, сестре́"`) → `"Дай кни́гу бра́ту. Ой, нет — сестре́, пожа́луйста."` (`en`: `"Give the book to my brother. Oh, no — to my sister, please."`, `glosses`: `+ой`, `crucial_span`: `"Ой, нет — сестре́"`) — Natural self-repair with `Ой, нет — ...`.
- `listen-09-05` — `"Положи́ кни́гу на стол. Хлеб положи́ на стул."` (`crucial_span`: `"Хлеб"`) → `"Положи́ кни́гу на стол, а хлеб положи́ на стул."` (`en`: `"Put the book on the table, and put the bread on the chair."`, `crucial_span`: `"а хлеб положи́ на стул"`) — Two fixes: (1) linked the two contrasting instructions with `а`; (2) fixed `crucial_span` from bare `"Хлеб"` (which did not distinguish `"Bread on the table"` from `"Bread on the chair"`) to `"а хлеб положи́ на стул"`.
- `listen-09-06` — `"Я говори́ла с сестро́й о рабо́те. За́втра она́ бу́дет до́ма. Мо́жно говори́ть с ней у́тром."` (`q0.crucial_span`: `"Мо́жно говори́ть с ней у́тром"`) → `"... Мо́жно поговори́ть с ней у́тром."` (`glosses`: `+поговорить`, `q0.crucial_span`: `"Мо́жно поговори́ть с ней у́тром"`) — **Aspect error**: suggesting a single bounded conversation tomorrow morning requires perfective `поговори́ть` (`Мо́жно поговори́ть с ней у́тром`), not imperfective `*Мо́жно говори́ть с ней у́тром`.
- `listen-09-07` — `"Я говори́л с бра́том о кни́ге. Нет, с сестро́й."` (`crucial_span`: `"Нет, с сестро́й"`) → `"Я говори́л с бра́том о кни́ге. Ой, нет — с сестро́й."` (`en`: `"I spoke with my brother about the book. Oh, no — with my sister."`, `glosses`: `+ой`, `crucial_span`: `"Ой, нет — с сестро́й"`) — Natural slip-of-the-tongue repair `Ой, нет — с сестро́й`.
- `listen-09-08` — `B: "Я то́же не понима́ю."` (`crucial_span`: `"то́же не понима́ю"`) → `B: "И я то́же не понима́ю."` (`crucial_span`: `"И я то́же не понима́ю"`) — Added initial `И` for natural spoken agreement.

### `listen-10` (`after_page`: `u25-p4`)
- `listen-10-02` — `"Приве́т, я бу́ду до́ма в семь. Купи́ хлеб, пожа́луйста. Нет, хлеб уже́ есть, купи́ то́лько молоко́."` → `"Приве́т, я бу́ду до́ма в семь. Купи́ хлеб, пожа́луйста. Ой, нет, хлеб же уже́ есть — купи́ то́лько молоко́."` (`en`: `"Hi, I will be home at seven. Buy bread, please. Oh, no, we already have bread — buy only milk."`, `glosses`: `+же, +ой`) — When suddenly remembering at home that you already have bread, Russian uses `Ой, нет` and the particle `же` (`хлеб же уже́ есть` — "wait, we already have bread after all!").
- `listen-10-04` — `"Напиши́ бра́ту. Нет, скажи́ ему́, когда́ бу́дешь до́ма."` → `"Напиши́ бра́ту. Нет, лу́чше скажи́ ему́, когда́ бу́дешь до́ма."` (`en`: `"Write to my brother. No, better tell him when you will be home."`, `glosses`: `+лучше`) — When revising an instruction from one action to a preferred alternative (`Напиши́...` → `скажи́...`), Russian uses `Нет, лу́чше скажи́...` ("No, better tell him...").
- `listen-10-07` — `"Бу́дем чита́ть у́тром. Нет, сна́чала напи́шем письмо́, а пото́м бу́дем чита́ть."` (`crucial_span`: `"сна́чала напи́шем письмо́"`, `why`: `"Сна́чала..."`) → `"... Нет, снача́ла напи́шем письмо́, а пото́м бу́дем чита́ть."` (`crucial_span`: `"снача́ла напи́шем письмо́"`, `why`: `"Снача́ла..."`) — **Stress error**: `снача́ла` has stress on the second syllable (`-ча́-`), never `*сна́чала`.
- `listen-10-08` — `"Не покупа́й кни́гу. У меня́ есть э́та кни́га. За́втра дам её тебе́."` (`glosses`: `эта`, `q0.crucial_span`: `"У меня́ есть э́та кни́га. За́втра дам её тебе́"`) → `"Слу́шай, не покупа́й э́ту кни́гу. Она́ у меня́ есть. За́втра дам её тебе́."` (`en`: `"Listen, do not buy this book. I have it. Tomorrow I will give it to you."`, `glosses`: `-эта, +слушай, +эту`, `q0.crucial_span`: `"Она́ у меня́ есть. За́втра дам её тебе́"`) — Repeating `кни́гу ... э́та кни́га` in adjacent short sentences is unnatural; in Russian, the demonstrative belongs on the first mention (`Слу́шай, не покупа́й э́ту кни́гу`) and the second sentence pronominalizes (`Она́ у меня́ есть`).

### `listen-11` (`after_page`: `u27-p5`)
- `listen-11-01` — `B: "Нет, я хочу́ быть до́ма."` (`crucial_span`: `"я хочу́ быть до́ма"`) → `B: "Нет, я хочу́ побы́ть до́ма."` (`en`: `"No, I want to stay at home."`, `glosses`: `+побыть`, `crucial_span`: `"я хочу́ побы́ть до́ма"`) — **English calque / aspect error**: in Russian, refusing an evening outing with "I want to stay at home" is `"Я хочу́ побы́ть до́ма"` (or `"оста́ться до́ма"`), never imperfective `*Я хочу́ быть до́ма`.
- `listen-11-06` — `A: "Мне ну́жно прочита́ть э́то письмо́, но я не понима́ю." / B: "Я то́же не понима́ю, э́то тру́дно."` → `A: "Мне ну́жно прочита́ть э́то письмо́, но я ничего́ не понима́ю." / B: "Я то́же не понима́ю, о́чень тру́дно."` (`en`: `"A: I need to read this letter, but I do not understand anything.\nB: I do not understand either; it is very difficult."`, `glosses`: `+ничего, +очень`) — `"я ничего́ не понима́ю"` and `"о́чень тру́дно"` replace the calqued English `"э́то тру́дно"` ("this is difficult").
- `listen-11-07` — `B: "Не согла́сна. Ста́рый стол ещё хоро́ший."` → `B: "А я не согла́сна. Ста́рый стол ещё хоро́ший."` (`en`: `"And I disagree. The old table is still good."`) — Bare `"Не согла́сна."` at the start of a domestic turn sounds clipped/bureaucratic; `"А я не согла́сна."` is natural conversational disagreement.
- `listen-11-08` — `"Мне нра́вится кни́га, кото́рую ты чита́ешь. Но сего́дня мне ну́жно рабо́тать. Поговори́м о ней за́втра."` (`q1/item crucial_span`: `"Поговори́м о ней за́втра"`) → `"Мне о́чень нра́вится кни́га, кото́рую ты чита́ешь. Но сего́дня мне ну́жно рабо́тать. Дава́й поговори́м о ней за́втра."` (`en`: `"I really like the book you are reading. But today I need to work. Let’s talk about it tomorrow."`, `glosses`: `+давай, +очень`, `q1/item crucial_span`: `"Дава́й поговори́м о ней за́втра"`) — Added `о́чень` (`Мне о́чень нра́вится...`) and proposal particle `Дава́й` (`Дава́й поговори́м о ней за́втра`).

### `listen-12` (`after_page`: `u30-p4`)
- `listen-12-01` — `B: "Понима́ю, но я ду́маю ина́че: сна́чала ну́жно прочита́ть всё письмо́."` (`crucial_span`: `"сна́чала ну́жно прочита́ть всё письмо́"`) → `"... снача́ла ну́жно прочита́ть всё письмо́."` (`crucial_span`: `"снача́ла ну́жно прочита́ть всё письмо́"`) — **Stress error**: `*сна́чала` → `снача́ла`.
- `listen-12-02` — `"Я сказа́л, что встре́ча в семь. Э́то непра́вда: она́ начина́ется в во́семь. Я бу́ду там в семь, что́бы всё прочита́ть."` → `"Я сказа́л, что встре́ча в семь. Э́то оши́бка: она́ начина́ется в во́семь. А я бу́ду там в семь, что́бы всё прочита́ть."` (`en`: `"I said the meeting was at seven. That is a mistake: it starts at eight. And I will be there at seven to read everything."`, `glosses`: `+ошибка`) — **Lexical/pragmatic error**: no Russian speaker corrects their own accidental time mix-up by saying `"Э́то непра́вда"` ("That is a lie / untruth"!). You say `"Э́то оши́бка"` (which is already used in `listen-week-06-02`). Also added contrastive `А` before `я бу́ду там в семь` since the speaker is contrasting the 8:00 start time with their own 7:00 arrival.
- `listen-12-08` — `A[0]: "Ита́к, за́втра идём в го́род." / B[1]: "Нет, я ещё не согла́сна. Мне ну́жно поду́мать." / A[2]: "Хорошо́, поговори́м за́втра."` → `A[0]: "Зна́чит, за́втра идём в го́род." / B[1]: "Подожди́, я ещё не согла́сна. Мне ну́жно поду́мать." / A[2]: "Ла́дно, поговори́м за́втра."` (`en` updated for `B[1]`: `"Wait, I have not agreed yet. I need to think."`, `glosses`: `+значит, +ладно`) — `"Ита́к,"` is lecturer/toastmaster register, never how a partner at home opens a plan check; at home you say `"Зна́чит, за́втра идём в го́род."` B's pushback naturally starts with `"Подожди́,"`, and A's concession naturally uses `"Ла́дно, поговори́м за́втра."`.

### `weekly_samples` (`listen-week-01` .. `listen-week-06`)
- `listen-week-01-01` (`u14-p5`) — `"Приве́т. Суп гото́в. Я хочу́ хлеб."` (`q0.crucial_span`: `"Я хочу́ хлеб"`) → `"Приве́т. Суп гото́в. Я хочу́ ещё хлеб, переда́й, пожа́луйста."` (`en`: `"Hi. The soup is ready. I also want bread; pass it, please."`, `q0.crucial_span`: `"Я хочу́ ещё хлеб"`) — Nobody sends a voice message saying `"Hi. The soup is ready. I want bread."` Turning the last sentence into `"Я хочу́ ещё хлеб, переда́й, пожа́луйста"` makes it a coherent domestic request.
- `listen-week-01-03` (`u14-p5`) — `"Ма́ма до́ма. Я то́же до́ма. Переда́й мне соль, пожа́луйста."` → `"Ма́ма до́ма, и я то́же до́ма. У́жин гото́в. Переда́й мне соль, пожа́луйста."` (`en`: `"Mom is at home, and I am at home too. Dinner is ready. Pass me the salt, please."`) — Linked `"Ма́ма до́ма, и я то́же до́ма"` and bridged the jump from "we are at home" to "pass me the salt" with `"У́жин гото́в."`.
- `listen-week-01-05` (`u14-p5`) — `"Я хочу́ чай с молоко́м. Хлеб не хочу́. Сыр, пожа́луйста."` (`q0.crucial_span`: `"Хлеб не хочу́. Сыр, пожа́луйста"`) → `"Я хочу́ чай с молоко́м. Хлеб не хочу́. А мне сыр, пожа́луйста."` (`en`: `"I want tea with milk. I do not want bread. And cheese for me, please."`, `q0.crucial_span`: `"Хлеб не хочу́. А мне сыр, пожа́луйста"`) — Replaced bare `"Сыр, пожа́луйста"` with `"А мне сыр, пожа́луйста"`.
- `listen-week-02-01` (`u12-p3`) — `"Кни́га на столе́. Я на ку́хне. Я чита́ю письмо́."` → `"Кни́га на столе́. А я на ку́хне, чита́ю письмо́."` (`en`: `"The book is on the table. And I am in the kitchen, reading a letter."`) — Combined the two `Я ...` sentences with contrastive `А` and pronoun drop in the second predicate (`, чита́ю письмо́`).
- `listen-week-02-04` (`u12-p3`) — `"Ма́ма чита́ет на ку́хне. Я не чита́ю, я ду́маю. Кот на сту́ле."` (`q1/item crucial_span`: `"Кот на сту́ле"`) → `"Ма́ма чита́ет на ку́хне. А я не чита́ю, я ду́маю. И кот на сту́ле."` (`en`: `"Mom is reading in the kitchen. And I am not reading; I am thinking. And the cat is on the chair."`, `q1/item crucial_span`: `"кот на сту́ле"`) — Added discourse connectors `А` and `И` so the three clauses form a single domestic vignette rather than three flashcards.
- `listen-week-03-02` (`u17-p5`) — `"За́втра у́тром я бу́ду до́ма. Днём рабо́таю. Ве́чером бу́дем говори́ть."` (`q0.crucial_span`: `"Ве́чером бу́дем говори́ть"`) → `"За́втра у́тром я бу́ду до́ма. Днём рабо́таю. А ве́чером поговори́м."` (`en`: `"Tomorrow morning I will be home. During the day I am working. And in the evening we will talk."`, `glosses`: `+поговорим`, `q0.crucial_span`: `"А ве́чером поговори́м"`) — **Aspect error**: scheduling a chat for the evening is perfective `А ве́чером поговори́м`, not imperfective `*Ве́чером бу́дем говори́ть`.
- `listen-week-03-03` (`u17-p5`) — `"Я чита́л, но не прочита́л кни́гу. Она́ о́чень интере́сная. За́втра бу́ду чита́ть ещё."` (`q0.crucial_span`: `"не прочита́л кни́гу"`) → `"Я чита́л кни́гу, но ещё не прочита́л. Она́ о́чень интере́сная. За́втра бу́ду чита́ть ещё."` (`en`: `"I was reading the book, but have not finished it yet. It is very interesting. Tomorrow I will read more."`, `q0.crucial_span`: `"но ещё не прочита́л"`) — **Syntax calque from English**: English allows right-node raising ("I was reading, but didn't finish, the book"), whereas Russian places the object on the first verb and elides it on the second (`Я чита́л кни́гу, но ещё не прочита́л`).
- `listen-week-03-05` (`u17-p5`) — `"У́тром я говори́ла с бра́том. Он бу́дет рабо́тать за́втра. Сего́дня он до́ма."` (`q1/item crucial_span`: `"Сего́дня он до́ма"`) → `"У́тром я говори́ла с бра́том. Он бу́дет рабо́тать за́втра. А сего́дня он до́ма."` (`en`: `"In the morning I spoke with my brother. He will work tomorrow. And today he is at home."`, `q1/item crucial_span`: `"сего́дня он до́ма"`) — Added contrastive `А` between `за́втра` and `сего́дня`.
- `listen-week-04-01` (`u24-p5`) — `"Мы е́дем в го́род за́втра. Сестра́ е́дет с на́ми. Брат бу́дет до́ма."` (`q1/item crucial_span`: `"Брат бу́дет до́ма"`) → `"За́втра мы е́дем в го́род. Сестра́ е́дет с на́ми, а брат бу́дет до́ма."` (`en`: `"Tomorrow we are going to the city. My sister is going with us, and my brother will be home."`, `q1/item crucial_span`: `"брат бу́дет до́ма"`) — Fronted the time topic `За́втра` and joined the contrasting sibling plans with `, а брат бу́дет до́ма`.
- `listen-week-04-04` (`u24-p5`) — `"Мы бу́дем говори́ть о рабо́те в семь. Нет, в во́семь. В семь я ещё бу́ду в го́роде."` (`q0.crucial_span`: `"Нет, в во́семь"`) → `"Мы бу́дем говори́ть о рабо́те в семь. Ой, нет — в во́семь. В семь я ещё бу́ду в го́роде."` (`en`: `"We will talk about work at seven. Oh, no — at eight. At seven I will still be in the city."`, `glosses`: `+ой`, `q0.crucial_span`: `"Ой, нет — в во́семь"`) — Natural self-repair `Ой, нет — в во́семь`.
- `listen-week-05-01` (`u27-p5`) — `"Купи́ три кни́ги. Нет, две: одна́ у меня́ уже́ есть."` (`q0.crucial_span`: `"Нет, две"`) → `"Купи́ три кни́ги. Ой, нет, две: одна́ у меня́ уже́ есть."` (`en`: `"Buy three books. Oh, no, two: I already have one."`, `glosses`: `+ой`, `q0.crucial_span`: `"Ой, нет, две"`) — Natural self-repair `Ой, нет, две`.
- `listen-week-05-02` (`u27-p5`) — `"У́тром прочита́й письмо́. Пото́м напиши́ отве́т. То́лько не пиши́ сейча́с, сна́чала ну́жно всё поня́ть."` (`q0.crucial_span`: `"сна́чала ну́жно всё поня́ть"`) → `"... снача́ла ну́жно всё поня́ть."` (`q0.crucial_span`: `"снача́ла ну́жно всё поня́ть"`) — **Stress error**: `*сна́чала` → `снача́ла`.
- `listen-week-05-05` (`u27-p5`) — `"Переда́й кни́гу сестре́. Она́ хо́чет её прочита́ть. Мне кни́га нужна́ то́лько в понеде́льник."` → `"Переда́й кни́гу сестре́. Она́ хо́чет её прочита́ть. А мне она́ нужна́ то́лько в понеде́льник."` (`en`: `"Pass the book to my sister. She wants to read it. And I only need it on Monday."`) — Replaced the repeated noun `кни́га` in the third sentence with `она́` and added contrastive `А мне...`.
- `listen-week-06-01` (`u30-p4`) — `"Я согла́сен, что нам ну́жно говори́ть. Но не сейча́с: я ещё не прочита́л письмо́. Поговори́м по́сле рабо́ты."` → `"Я согла́сен, что нам ну́жно поговори́ть. ..."` (`glosses`: `+поговорить`) — **Aspect error**: `"we need to have a talk"` is `нам ну́жно поговори́ть`, not imperfective `*нам ну́жно говори́ть`.
- `listen-week-06-03` (`u30-p4`) — `"... Сна́чала ну́жно реши́ть, кто нам помо́жет."` → `"... Снача́ла ну́жно реши́ть, кто нам помо́жет."` — **Stress error**: `*Сна́чала` → `Снача́ла`.
- `listen-week-06-04` (`u30-p4`) — `"... а я хочу́ сна́чала прочита́ть кни́гу, кото́рая у нас уже́ есть. ..."` → `"... а я хочу́ снача́ла прочита́ть кни́гу, кото́рая у нас уже́ есть. ..."` — **Stress error**: `*сна́чала` → `снача́ла`.
- `listen-week-06-05` (`u30-p4`) — `"Я сказа́ла, что не хочу́ е́хать с ва́ми. Не так: я хочу́, но не могу́ в суббо́ту. Е́сли вы пое́дете в воскресе́нье, я пое́ду то́же."` → `"Я сказа́ла, что не хочу́ е́хать с ва́ми. Э́то не так: я хочу́, но не могу́ в суббо́ту. Е́сли вы пое́дете в воскресе́нье, я то́же пое́ду."` — Two fixes: (1) sentence-initial `"Не так:"` is clipped; `"Э́то не так:"` is natural; (2) sentence-final `"*я пое́ду то́же"` is an English word-order calque ("I will go too"); in Russian, additive `то́же` precedes the verb when the subject is included (`я то́же пое́ду`).

---

## 2. Gender-Bound Items (Required Voice Assignment for TTS / Recording)

When the root agent assigns TTS or recorded voices (especially for `"voice_hint": "unfamiliar"` or `"any"`), **17 items** have first-person singular past-tense verbs (`-л` vs `-ла`) or short-form predicate adjectives (`согла́сен` vs `согла́сна`, `до́лжен` vs `должна́`) that grammatically lock the speaker's gender:

| Item ID | Kind | `voice_hint` | Gender-Bound Turn / Speaker | Required Voice Gender | Russian Form in Text |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `listen-07-02` | `message` | `any` | Single speaker | **Female** | `Вчера́ я чита́ла кни́гу, но ещё не прочита́ла.` |
| `listen-07-05` | `message` | `any` | Single speaker | **Female** | `Я уже́ у́жинала` |
| `listen-09-02` | `who_needs` | `any` | **Speaker A** (`B` is gender-free) | **Male** (Speaker A) | `Я говори́л с ма́мой.` |
| `listen-09-06` | `message` | `any` | Single speaker | **Female** | `Я говори́ла с сестро́й о рабо́те.` |
| `listen-09-07` | `correction` | `any` | Single speaker | **Male** | `Я говори́л с бра́том о кни́ге.` |
| `listen-10-03` | `who_needs` | `any` | **Speaker A** (`B` is gender-free) | **Male** (Speaker A) | `Я не по́нял.` |
| `listen-10-05` | `message` | `any` | Single speaker | **Male** | `Я прочита́л письмо́.` |
| `listen-11-02` | `disagreement` | `any` | **Speaker B** (`A` is gender-free) | **Male** (Speaker B) | `Не согла́сен.` |
| `listen-11-07` | `disagreement` | `any` | **Speaker B** (`A` is gender-free) | **Female** (Speaker B) | `А я не согла́сна.` |
| `listen-12-02` | `message` | `any` | Single speaker | **Male** | `Я сказа́л, что встре́ча в семь.` |
| `listen-12-04` | `correction` | `unfamiliar` | Single speaker | **Male** | `Я не согла́сен с тобо́й. ... я согла́сен` |
| `listen-12-07` | `message` | `any` | Single speaker | **Male** | `... но сего́дня до́лжен рабо́тать.` |
| `listen-12-08` | `disagreement` | `unfamiliar` | **Speaker B** (`A` is gender-free) | **Female** (Speaker B) | `Подожди́, я ещё не согла́сна.` |
| `listen-week-03-01` | `message` | `unfamiliar` | Single speaker | **Female** | `Вчера́ я рабо́тала до́ма.` |
| `listen-week-03-03` | `message` | `unfamiliar` | Single speaker | **Male** | `Я чита́л кни́гу, но ещё не прочита́л.` |
| `listen-week-03-05` | `message` | `unfamiliar` | Single speaker | **Female** | `У́тром я говори́ла с бра́том.` |
| `listen-week-04-03` | `message` | `unfamiliar` | Single speaker | **Male** | `Я купи́л хлеб и сыр.` |
| `listen-week-04-05` | `message` | `unfamiliar` | Single speaker | **Female** | `Я говори́ла с бра́том о сестре́.` |
| `listen-week-06-01` | `message` | `unfamiliar` | Single speaker | **Male** | `Я согла́сен ... я ещё не прочита́л письмо́.` |
| `listen-week-06-02` | `message` | `unfamiliar` | Single speaker | **Female** | `Я написа́ла в во́семь` |
| `listen-week-06-05` | `message` | `unfamiliar` | Single speaker | **Female** | `Я сказа́ла, что не хочу́ е́хать с ва́ми.` |

*(Count: 13 practice items + 8 weekly sample items = **21 gender-bound items** in total.)*

### Note on Addressee-Gender-Bound Items (Speaker Voice Remains Free)
Three additional items mark the grammatical gender of the **listener (`ты`)**, not the speaker. The speaker's voice can be either male or female, because `"Ты ..."` refers to the addressee:
- `listen-07-06` (`question_or_statement`): `"Ты уже́ прочита́л кни́гу?"` — Addressee is male; speaker voice is `any`.
- `listen-12-07` (`message`): `"Ты спра́шивал, почему́ я не хочу́ е́хать. ... но сего́дня до́лжен рабо́тать."` — Both addressee (`спра́шивал`) and speaker (`до́лжен`) are male.
- `listen-week-06-03` (`message`): `"Ты права́, э́та рабо́та ва́жная."` — Addressee is female (`права́`); speaker voice is free (`unfamiliar`).

---

## 3. Native-Ear Assessment of the Curriculum Overall

### A. What a Russian Speaker Would Say Was Unnatural (and What We Fixed)
Codex's structural design — testing **intent, repairs, intonation, object/recipient tracking, and unresolved disagreement** under a strict vocabulary gate — is pedagogically sound. However, before this edit pass, the Russian text suffered from six systematic "textbook-ese" habits:
1. **The "Inventory-List" Voice Message**: Early `message` and `weekly_samples` items were written as three disconnected SVO sentences with repeated subjects (`"Кни́га на столе́. Я на ку́хне. Я чита́ю письмо́."`). No Russian speaks into Telegram or WhatsApp like a first-grade reader. Real voice messages open with an attention getter (`Слу́шай, ...`, `Приве́т, ...`), drop redundant pronouns, and stitch clauses together with contrastive `а`, additive `и`, and particles (`уже́`, `же`, `тут`, `сейча́с`).
2. **Robotic Self-Corrections with Bare `Нет,`**: Almost every `correction` item used a flat `"X. Нет, Y."` (`"Кни́га на столе́. Нет, на сту́ле."`). When a Russian catches a slip of the tongue in real time, they say `"Ой, нет — на сту́ле"`, `"Ой, нет, не в семь — в во́семь"`, `"То есть нет: ..."`, or `"Нет, лу́чше ..."`.
3. **Missing Contrastive `А` and Additive `И` in Two-Voice Exchanges**: In Russian dialogue, if Speaker A says `"У меня́ нет хле́ба"` and Speaker B has bread, B cannot reply `"*У меня́ есть"` without `А` (`"А у меня́ есть"`). Similarly, chiming in with agreement (`"И я то́же"`, `"И мне то́же"`, `"И у меня́ то́же нет"`) strongly prefers initial `И`.
4. **Postural Verbs and Food Request Collocations**: Russian cares intensely about whether an object lies flat (`положи́ хлеб/кни́гу/ло́жку`) or stands upright (`поста́вь торт/ча́шку/таре́лку`), and whether a food item at home is passed (`Дай хлеб / Переда́й соль`) vs poured/served (`Мне чай, пожа́луйста`).
5. **Aspect in Proposals and Plans**: Imperfective `говори́ть` and `быть` were overused where Russian requires bounded perfectives (`Мо́жно поговори́ть с ней у́тром`, `А ве́чером поговори́м`, `Я согла́сен, что нам ну́жно поговори́ть`, `Я хочу́ побы́ть до́ма`).
6. **Systematic Stress Error on `снача́ла`**: Five items across Sets 10, 12, and Weekly Samples 5–6 had `*сна́чала` instead of `снача́ла`.

### B. What Is Missing from a Real Russian Home Soundscape
For an adult learner (week 2) living with a native Russian speaker, the current 12 sets live almost entirely in a quiet kitchen with bread, tea, soup, a table, a chair, and a book. A real Russian home soundscape has six high-frequency acoustic environments that are currently absent or barely touched:

1. **The Phone Call (`телефо́нный разгово́р`)**
   - At home, half of what Ember hears their partner say is one side of a phone call: `"Алло́, да, слу́шаю"`, `"Ты где сейча́с?"`, `"Я пло́хо тебя́ слы́шу, перезвони́"`, `"Дава́й пото́м, я сейча́с не могу́ говори́ть"`, `"Ну всё, дава́й, пока́"`.
   - **Why it matters**: Phone speech is fast, elliptical, and full of boundary markers (`Алло́`, `Ну дава́й`, `Перезвоню́`).

2. **The Shop & Cashier (`в магази́не / на ка́ссе`)**
   - The curriculum goes *toward* the shop (`Я иду́ в магази́н`), but never steps *inside* it or hears the partner calling from the aisle: `"Слу́шай, я в магази́не, что ещё взя́ть?"`, `"Тут нет молока́, взять сок?"`, and the cashier's rapid-fire `"Паке́т ну́жен?"`, `"Ка́ртой и́ли нали́чными?"`, `"Ваш чек"`.

3. **The Courier & Intercom (`курье́р и домофо́н`)**
   - Modern Russian home life involves constant deliveries and entryphone calls: `"Кто там?" — "Э́то курье́р / доста́вка"`, `"Откро́й дверь, пожа́луйста, у меня́ ру́ки за́няты"`, `"Курье́р уже́ прие́хал, возьми́ зака́з"`, `"Оста́вьте у две́ри, пожа́луйста"`.

4. **The Neighbour (`сосе́ди`)**
   - Brief hallway/stairwell exchanges that switch suddenly to polite `вы` (`Здра́вствуйте!`, `До свида́ния`): `"Извини́те, а у вас есть вода́?"`, `"Вы выхо́дите?"`, `"Спаси́бо, хоро́шего дня!"`. Hearing the partner switch between home `ты` and hallway `вы` is a core real-world listening skill.

5. **A Child or Pet at Home (`ребёнок / кот`)**
   - Even though `кот` is in Unit 1, we never hear how Russians actually talk to a pet or child at home: `"Кот опя́ть на столе́!"`, `"Иди́ сюда́"`, `"Не тро́гай!"`, `"Ти́хо, ма́ма спит"`, `"Ты покорми́л кота́?"`.

6. **Background TV, Kettle & Household Micro-Requests (`фо́новый телеви́зор и быт`)**
   - The constant small physical adjustments of shared space: `"Сде́лай поти́ше, пожа́луйста"` (when the TV/video is loud), `"Закро́й окно́, хо́лодно"`, `"Откро́й окно́"`, `"Вы́ключи свет"`, `"Ча́йник уже́ горя́чий?"`, `"Где мой телефо́н? Я не ви́жу его́"`.

---

## 4. Three New Items We Would Add Per Early Set (`listen-01` – `listen-06`)

All 18 proposed items below strictly respect each set's `after_page` grammar/vocabulary gate (with 1–2 glossed preview words where needed) and target the missing domestic soundscape above.

### Additions for `listen-01` (`after_page`: `u3-p4` — Intonation: Statement vs Question)
1. **`listen-01-09` (`question_or_statement` — Pet on the furniture)**
   - `forms`: `[{"ru": "Кот опя́ть там.", "en": "The cat is there again."}, {"ru": "Кот опя́ть там?", "en": "Is the cat there again?"}]` (`played: 1`, `glosses`: `{"опять": "again"}`)
   - `question`: `"Hearing a noise from the kitchen: is the speaker asking or telling?"` → **Question** (`crucial_span`: `"Кот опя́ть там?"`)
   - *Why*: `Кот` is in Unit 1; `опя́ть` is one of the highest-frequency home words with pets and puts the IK-3 pitch peak right on `опя́ть`.
2. **`listen-01-10` (`question_or_statement` — Phone check-in)**
   - `forms`: `[{"ru": "Ты уже́ тут.", "en": "You are already here."}, {"ru": "Ты уже́ тут?", "en": "Are you already here?"}]` (`played: 1`, `glosses`: `{"уже": "already"}`)
   - `question`: `"Answering the phone near the front door: is the speaker asking or telling?"` → **Question** (`crucial_span`: `"Ты уже́ тут?"`)
   - *Why*: `"Ты уже́ тут?"` / `"Ты уже́ до́ма?"` is the #1 real-life phone question a learner hears from their partner.
3. **`listen-01-11` (`question_or_statement` — Finding a misplaced item)**
   - `forms`: `[{"ru": "Вот мой чай.", "en": "Here is my tea."}, {"ru": "Вот мой чай?", "en": "Is this my tea here?"}]` (`played: 0`, `glosses`: `{}`)
   - `question`: `"Looking at the kitchen counter: is the speaker asking or telling?"` → **Statement** (`crucial_span`: `"Вот мой чай."`)
   - *Why*: Contrasts presentational `Вот` with IK-1 falling pitch against echo-question IK-3.

### Additions for `listen-02` (`after_page`: `u4-p4` — Objects & Actions in the Room)
1. **`listen-02-09` (`follow` — Closing vs opening the window)**
   - `ru`: `"Закро́й, пожа́луйста, окно́."` (`en`: `"Close the window, please."`, `glosses`: `{"закрой": "close (informal singular request)", "пожалуйста": "please"}`)
   - `options`: `["Window closed", "Door closed", "Window open"]`, `answer: 0`, `crucial_span`: `"Закро́й, пожа́луйста, окно́"`
   - *Why*: `окно́` and `дверь` are already in `u1–u4`, and `"Закро́й окно́"` / `"Откро́й окно́"` are heard daily at home.
2. **`listen-02-10` (`pick_object` — Doorbell/courier handoff)**
   - `ru`: `"Сок тут, а возьми́, пожа́луйста, торт."` (`en`: `"The juice is here, and take the cake, please."`, `glosses`: `{"возьми": "take (informal singular request)", "пожалуйста": "please"}`)
   - `options`: `["сок", "торт", "сыр"]`, `answer: 1`, `crucial_span`: `"возьми́, пожа́луйста, торт"`
   - *Why*: Introduces `возьми́` ("take [in your hands]") when carrying groceries or a delivery from the hallway.
3. **`listen-02-11` (`correction` — Table vs window ledge)**
   - `ru`: `"Поста́вь сок на стол. Ой, нет — на окно́."` (`en`: `"Put the juice on the table. Oh, no — on the window ledge."`, `glosses`: `{"ой": "oh, oops", "поставь": "put; set upright (informal singular request)"}`)
   - `options`: `["Put the juice on the table", "Put the juice on the window ledge", "Keep the juice in your hand"]`, `answer: 1`, `crucial_span`: `"Ой, нет — на окно́"`
   - *Why*: Pairs upright container verb `поста́вь` (`сок`) with a spatial self-repair (`на стол` → `на окно́`).

### Additions for `listen-03` (`after_page`: `u7-p4` — Small Requests at Home)
1. **`listen-03-09` (`who_needs` — Offering tea before pouring)**
   - `lines`: `A: "Я сейча́с бу́ду чай. А ты?" / B: "И мне чай, пожа́луйста."` (`en`: `"A: I am going to have tea now. And you?\nB: Tea for me too, please."`, `glosses`: `{"буду": "I will have / I will", "мне": "to me; for me", "пожалуйста": "please", "сейчас": "now"}`)
   - `question`: `"Who wants tea?"`, `options`: `["A", "B", "Both"]`, `answer: 2`, `crucial_span`: `"И мне чай"`
   - *Why*: `"Я бу́ду чай"` ("I'll have tea") is how Russians actually announce making/having a drink at home.
2. **`listen-03-10` (`pick_object` — Soup vs tea vs coffee)**
   - `ru`: `"Чай уже́ тут, а мне суп, пожа́луйста."` (`en`: `"The tea is already here, and soup for me, please."`, `glosses`: `{"мне": "to me; for me", "пожалуйста": "please", "уже": "already"}`)
   - `options`: `["чай", "суп", "ко́фе"]`, `answer: 1`, `crucial_span`: `"а мне суп"`
   - *Why*: Forces the learner to ignore the first food word (`Чай уже́ тут`) and pick the requested one (`а мне суп`).
3. **`listen-03-11` (`question_or_statement` — Checking whether the door is closed)**
   - `forms`: `[{"ru": "Э́то наш дом.", "en": "This is our house."}, {"ru": "Э́то наш дом?", "en": "Is this our house?"}]` (`played: 1`, `glosses`: `{}`)
   - `question`: `"Looking at a map or street number: is the speaker asking or telling?"` → **Question** (`crucial_span`: `"Э́то наш дом?"`)
   - *Why*: Uses `наш дом` from `u6–u7` with IK-3 on `наш`/`дом`.

### Additions for `listen-04` (`after_page`: `u14-p5` — At the Table)
1. **`listen-04-09` (`who_needs` — Water vs tea at dinner)**
   - `lines`: `A: "Переда́й, пожа́луйста, во́ду." / B: "Вот вода́. А мне переда́й хлеб."` (`en`: `"A: Pass the water, please.\nB: Here is the water. And pass me the bread."`, `glosses`: `{"вода": "water", "воду": "water (direct object)"}`)
   - `question`: `"Who asks for water?"`, `options`: `["A", "B", "Both"]`, `answer: 0`, `crucial_span`: `"Переда́й, пожа́луйста, во́ду"`
   - *Why*: `вода́ / во́ду` with mobile stress (`вода́` → `во́ду`) is essential table Russian and prepares the `-а → -у` accusative ear.
2. **`listen-04-10` (`correction` — Sugar in coffee)**
   - `ru`: `"Мне ко́фе с са́харом. Ой, нет — без са́хара, пожа́луйста."` (`en`: `"Coffee with sugar for me. Oh, no — without sugar, please."`, `glosses`: `{"без": "without", "ой": "oh, oops", "сахара": "sugar (after without)"}`)
   - `question`: `"How should the coffee be made?"`, `options`: `["With sugar", "Without sugar", "With milk and sugar"]`, `answer: 1`, `crucial_span`: `"Ой, нет — без са́хара"`
   - *Why*: `с са́харом` vs `без са́хара` is one of the most frequent real-life drink repairs at home and in cafés.
3. **`listen-04-11` (`question_or_statement` — Offering a second helping)**
   - `forms`: `[{"ru": "Ты ещё хо́чешь чай.", "en": "You still want tea."}, {"ru": "Ты ещё хо́чешь чай?", "en": "Do you want more tea?"}]` (`played: 1`, `glosses`: `{}`)
   - `question`: `"The kettle is hot in the kitchen. Is the speaker asking or telling?"` → **Question** (`crucial_span`: `"Ты ещё хо́чешь чай?"`)
   - *Why*: Tests `хо́чешь` with IK-3 in the exact phrasing used when refilling cups.

### Additions for `listen-05` (`after_page`: `u10-p4` — What Is Missing / Genitive of Absence)
1. **`listen-05-09` (`who_needs` — Missing spoon vs fork/plate)**
   - `lines`: `A: "У меня́ есть таре́лка, а ло́жки нет." / B: "И у меня́ нет ло́жки."` (`en`: `"A: I have a plate, but no spoon.\nB: And I have no spoon either."`, `glosses`: `{"ложки": "spoon (after нет)"}`)
   - `question`: `"Who has no spoon?"`, `options`: `["A", "B", "Both"]`, `answer: 2`, `crucial_span`: `"И у меня́ нет ло́жки"`
   - *Why*: Contrasts nominative `таре́лка есть` with genitive `ло́жки нет` across both speakers.
2. **`listen-05-10` (`follow` — Setting the cup on the table)**
   - `ru`: `"Поста́вь ча́шку на стол, а ло́жку положи́ тут."` (`en`: `"Set the cup on the table, and lay the spoon here."`, `glosses`: `{"ложку": "spoon (direct object)", "положи": "lay; put flat", "поставь": "set upright", "чашку": "cup (direct object)"}`)
   - `options`: `["Cup on the table", "Cup on the chair", "Spoon on the chair"]`, `answer: 0`, `crucial_span`: `"Поста́вь ча́шку на стол"`
   - *Why*: Directly contrasts `поста́вь ча́шку` (upright vessel) with `положи́ ло́жку` (flat utensil) in a single table-setting instruction.
3. **`listen-05-11` (`correction` — Checking the fridge before shopping)**
   - `ru`: `"До́ма нет молока́. Ой, нет, молоко́ есть — нет сы́ра."` (`en`: `"There is no milk at home. Oh, no, there is milk — there is no cheese."`, `glosses`: `{"ой": "oh, oops", "сыра": "cheese (after нет)"}`)
   - `question`: `"What is actually missing at home?"`, `options`: `["Milk", "Cheese", "Both milk and cheese"]`, `answer: 1`, `crucial_span`: `"молоко́ есть — нет сы́ра"`
   - *Why*: The classic "looking into the fridge while talking" repair (`Ой, нет, молоко́ есть — нет сы́ра`).

### Additions for `listen-06` (`after_page`: `u12-p3` — Understanding, Location & Background Noise)
1. **`listen-06-09` (`who_needs` — Phone connection / speaking louder)**
   - `lines`: `A: "Алло́! Ты меня́ слы́шишь?" / B: "Пло́хо слы́шу. Повтори́, пожа́луйста."` (`en`: `"A: Hello! Can you hear me?\nB: I can barely hear you. Repeat that, please."`, `glosses`: `{"алло": "hello (on the phone)", "плохо": "badly; poorly", "повтори": "repeat (informal singular request)", "слышишь": "you hear", "слышу": "I hear"}`)
   - `question`: `"Who is having trouble hearing on the phone?"`, `options`: `["A", "B", "Both"]`, `answer: 1`, `crucial_span`: `"Пло́хо слы́шу. Повтори́"`
   - *Why*: Introduces the essential phone survival exchange (`Алло́! Ты меня́ слы́шишь? — Пло́хо слы́шу`).
2. **`listen-06-10` (`message` — Leaving a note/voice message from the kitchen)**
   - `ru`: `"Слу́шай, я на ку́хне, тут о́чень гро́мко. Я тебя́ не слы́шу. Скажи́ ещё раз, пожа́луйста."` (`en`: `"Listen, I am in the kitchen; it is very loud here. I cannot hear you. Say it once more, please."`, `glosses`: `{"громко": "loud; noisy", "очень": "very", "скажи": "say (informal singular request)", "слушай": "listen; hey", "слышу": "I hear"}`)
   - `questions`: Q1 (gist): `"Why could the speaker not hear?"` → `"It is loud in the kitchen"` (`crucial_span`: `"тут о́чень гро́мко"`); Q2 (detail): `"What does the speaker ask you to do?"` → `"Say it again"` (`crucial_span`: `"Скажи́ ещё раз"`).
   - *Why*: Captures the real acoustic situation of someone calling from another room while the kettle/water/TV is running.
3. **`listen-06-11` (`correction` — Where the phone/letter is)**
   - `ru`: `"Письмо́ на окне́. Ой, нет — оно́ тут, на столе́."` (`en`: `"The letter is on the window ledge. Oh, no — it is here, on the table."`, `glosses`: `{"ой": "oh, oops", "окне": "on the window ledge (after on)", "письмо": "letter"}`)
   - `question`: `"Where is the letter?"`, `options`: `["On the window ledge", "On the table", "On the chair"]`, `answer: 1`, `crucial_span`: `"Ой, нет — оно́ тут, на столе́"`
   - *Why*: Practices neuter pronoun agreement (`письмо́` → `оно́`) and prepositional `-е` (`на окне́` vs `на столе́`) in a natural domestic search.
