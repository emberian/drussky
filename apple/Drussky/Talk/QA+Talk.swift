import Foundation

extension QA {
    /// `-qa-talk STATE`: a Talk page held in one state, no key or microphone needed.
    ///   fresh    nothing yet (`-qa-nokey` / `-qa-offline` pose the one-line note without touching the Keychain or the network)
    ///   scene    the three proposed scenes
    ///   mid      four lines in, the learner's turn
    ///   marked   the finished conversation with its red pen and a tip
    static func talkPage(_ book: Book, state: String) {
        book.insertTalkPage()
        guard state != "fresh", let page = book.page else { return }
        let ref = ExerciseRef(pageID: page.id, index: 0)
        var c = TalkConversation(turns: 6)
        c.proposed = [TalkScene(title: "At breakfast", ru: "За за́втраком", goal: "Find out what there is to drink and order one thing",
                                frames: [TalkFrame(ru: "Что есть …?", en: "what is there …?"), TalkFrame(ru: "Я хочу́ …", en: "I want …"),
                                         TalkFrame(ru: "А … есть?", en: "and is there …?")],
                                secret: "There is tea, juice and coffee, but the milk is finished; there is bread and cheese."),
                      TalkScene(title: "In a shop", ru: "В магази́не", goal: "Find out whether they have bread, and how much it costs",
                                frames: [TalkFrame(ru: "У вас есть …?", en: "do you have …?"), TalkFrame(ru: "Ско́лько сто́ит …?", en: "how much is …?")],
                                secret: "The shop has bread (40 roubles) and milk, but no cheese today."),
                      TalkScene(title: "About a book you read", ru: "О кни́ге, кото́рую вы чита́ли", goal: "Tell them what you are reading and find out what they read",
                                frames: [TalkFrame(ru: "Я чита́ю …", en: "I am reading …"), TalkFrame(ru: "А что вы чита́ете?", en: "and what do you read?")],
                                secret: "You read a book about space; you don't like poetry.")]
        guard state != "scene" else { book.saveTalk(ref, c); return }
        let t0 = Date().addingTimeInterval(-300)
        c.scene = c.proposed?[0]
        c.started = t0
        func tutor(_ ru: String, _ en: String, _ new: [Course.Word] = [], _ s: Double) -> TalkLine {
            TalkLine(role: .tutor, ru: ru, en: en, newWords: new.isEmpty ? nil : new,
                     latency: TalkLatency(firstToken: 900, firstSpeech: 1400, complete: 2600), at: t0.addingTimeInterval(s))
        }
        func me(_ ru: String, _ s: Double) -> TalkLine { TalkLine(role: .learner, ru: ru, at: t0.addingTimeInterval(s)) }
        c.lines = [
            tutor("До́брое у́тро! Что вы хоти́те на за́втрак?", "Good morning! What do you want for breakfast?",
                  [Course.Word(ru: "за́втрак", en: "breakfast", emoji: nil, note: "from a conversation")], 0),
            me("Я хочу кофе и хлеб", 12),
            tutor("Отли́чно. Ко́фе с молоко́м и́ли без?", "Excellent. Coffee with milk or without?", [], 20),
            me("С молоко и сахар", 34),
        ]
        guard state != "mid" else { book.saveTalk(ref, c); return }
        c.lines += [
            tutor("Вы хоте́ли сказа́ть «с молоко́м и с са́харом»? Хорошо́. А что вы чита́ете у́тром?",
                  "Did you mean “with milk and with sugar”? Good. And what do you read in the morning?", [], 42),
            me("Я читаю книга о космос", 58),
            tutor("Интере́сно! До свида́ния, и прия́тного аппети́та!", "Interesting! Goodbye, and enjoy your breakfast!",
                  [Course.Word(ru: "прия́тного аппети́та", en: "enjoy your meal", emoji: nil, note: "from a conversation")], 66),
        ]
        c.ended = t0.addingTimeInterval(70)
        c.marked = t0.addingTimeInterval(80)
        c.outcome = TalkOutcome(reached: true, conveyed: "You ordered coffee and bread, and asked for milk and sugar — your partner understood the order; the milk being finished never came up.",
                                clarified: "whether you meant «с молоко́м и с са́харом»")
        c.praise = "You held a whole conversation, and хочу́ ко́фе came out exactly right."
        c.teach = [DiaryTeach(title: "With: с + instrumental", note: "After с (with) the noun changes its ending: [[с молоко́м|with milk]], [[с са́харом|with sugar]].", pattern: "instr")]
        let date = c.marked!
        c.lines[1].correction = DiaryCorrection(corrected: "Я хочу́ ко́фе и хлеб.", edits: [], praise: "Just right.", teach: [], at: date)
        c.lines[3].correction = DiaryCorrection(corrected: "С молоко́м и с са́харом.",
                                                edits: [DiaryEdit(span: "молоко", replacement: "молоко́м", category: .ending,
                                                                  why: "After с (with) the noun takes the instrumental: молоко́ → молоко́м.", en: "with milk", pattern: "instr"),
                                                        DiaryEdit(span: "сахар", replacement: "с са́харом", category: .ending,
                                                                  why: "The same ending, and с is repeated: с са́харом.", en: "with sugar", pattern: "instr")],
                                                praise: "", teach: [], at: date)
        c.lines[3].tip = "One thing to try: the stress is on «ко́м» in молоко́м — make it longer and louder than «мо»."
        c.lines[5].correction = DiaryCorrection(corrected: "Я чита́ю кни́гу о ко́смосе.",
                                                edits: [DiaryEdit(span: "книга", replacement: "кни́гу", category: .ending,
                                                                  why: "The thing you read is the object: кни́га → кни́гу.", en: "book (object)", pattern: "acc-u"),
                                                        DiaryEdit(span: "космос", replacement: "ко́смосе", category: .ending,
                                                                  why: "After о (about) the noun takes -е: о ко́смосе.", en: "space (about space)", pattern: "prep-e")],
                                                praise: "", teach: [], at: date)
        book.saveTalk(ref, c)
        book.finishTalk(ref, c)
        book.saveNow()
    }
}
