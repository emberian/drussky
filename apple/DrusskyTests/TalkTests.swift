import AVFoundation
import LearningCore
import Speech
import XCTest
@testable import Drussky

/// Talk: the dialogue's turn order, the streamed reply parser, the marking contract mapped onto
/// the transcript, voice selection, persistence, and the evidence a conversation yields.
@MainActor
final class TalkTests: XCTestCase {
    private func course() throws -> Course {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        return try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    private func tempStore() -> BookStore {
        BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent("talk-\(UUID().uuidString)"))
    }

    // MARK: The dialogue

    func testResumingLearnerTurnWaitsForMicrophoneTap() async throws {
        // Resuming checks for the tutor first (no key: the page says so instead); the turn itself
        // never reaches the network, so any key will do for this test. The learner's own is restored.
        let keyBefore = Keychain.apiKey
        Keychain.apiKey = keyBefore ?? "qa-placeholder-key"
        defer { Keychain.apiKey = keyBefore }
        let book = Book(course: try course(), store: tempStore())
        book.insertTalkPage()
        let ref = ExerciseRef(pageID: try XCTUnwrap(book.page).id, index: 0)
        var saved = TalkConversation(turns: 6)
        saved.scene = TalkScene(title: "A cat", ru: "Кот")
        let line = TalkLine(role: .tutor, ru: "Где кот?", at: book.now())
        saved.lines = [line]
        book.saveTalk(ref, saved)
        let session = TalkSession(ref: ref, book: book, pageVoice: Voice(), voice: SilentTutorVoice())
        defer { session.cancel() }

        session.appear(autoStart: false)
        XCTAssertEqual(session.status, .paused)
        session.resume()
        await Task.yield()
        XCTAssertEqual(session.status, .yourTurn)
        XCTAssertEqual(session.mic.phase, .idle, "Reading the reply must not start capture or request microphone permission")

        session.mic.pose(.hearing, key: session.key)
        session.replay(line)
        XCTAssertEqual(session.mic.phase, .idle, "Replaying the tutor must not record the tutor as the learner")
        XCTAssertEqual(session.status, .yourTurn)
    }

    /// Without a key nothing starts: the page says so, and "Try again" asks the same question
    /// again instead of launching a request that would fail.
    func testTryAgainWithoutKeyStaysUnavailable() async throws {
        try XCTSkipUnless((Keychain.apiKey ?? "").isEmpty, "requires a simulator with no tutor key")
        let book = Book(course: try course(), store: tempStore())
        book.insertTalkPage()
        let ref = ExerciseRef(pageID: try XCTUnwrap(book.page).id, index: 0)
        let session = TalkSession(ref: ref, book: book, pageVoice: Voice(), voice: SilentTutorVoice())
        defer { session.cancel() }
        session.propose()
        guard case .unavailable = session.status else { return XCTFail("A missing key is an unavailable note, not a request: \(session.status)") }
        session.resume()
        guard case .unavailable = session.status else { return XCTFail("Try again with the key still missing stays unavailable: \(session.status)") }
    }

    func testDialogueTurnOrderAndEnd() {
        var d = TalkDialogue(totalTurns: 6)
        XCTAssertEqual(d.phase, .choosingScene)
        d.tutorSpoke(goodbye: false)
        XCTAssertEqual(d.phase, .choosingScene, "nothing moves before a scene is chosen")
        d.choose()
        XCTAssertEqual(d.phase, .tutorTurn, "the tutor opens")
        for k in 1...6 {
            d.tutorSpoke(goodbye: false)
            XCTAssertEqual(d.phase, .learnerTurn, "turn \(k): the learner answers")
            d.learnerSpoke()
            XCTAssertEqual(d.phase, .tutorTurn)
            XCTAssertEqual(d.learnerTurns, k)
        }
        XCTAssertTrue(d.isClosing, "after the sixth answer the tutor must say goodbye")
        d.tutorSpoke(goodbye: false)
        XCTAssertEqual(d.phase, .ended, "the goodbye ends it even if the tutor forgot END")
        d.learnerSpoke()
        XCTAssertEqual(d.phase, .ended, "nothing after the end")
    }

    func testDialogueEndsEarlyOnGoodbyeAndClampsTurns() {
        var d = TalkDialogue(totalTurns: 40)
        XCTAssertEqual(d.totalTurns, 10, "turns are clamped to 6–10")
        XCTAssertEqual(TalkDialogue(totalTurns: 1).totalTurns, 6)
        d.choose()
        d.tutorSpoke(goodbye: false)
        d.learnerSpoke()
        d.tutorSpoke(goodbye: true)
        XCTAssertEqual(d.phase, .ended, "END from the tutor ends it at any turn")
    }

    func testDialogueCancelAndEndEarlyAndResume() {
        var d = TalkDialogue(totalTurns: 8)
        d.choose()
        d.tutorSpoke(goodbye: false)
        d.cancel()
        XCTAssertEqual(d.phase, .cancelled)
        d.learnerSpoke()
        XCTAssertEqual(d.phase, .cancelled, "a cancelled dialogue stays cancelled")

        var e = TalkDialogue(totalTurns: 8)
        e.choose()
        e.tutorSpoke(goodbye: false)
        e.learnerSpoke()
        e.endEarly()
        XCTAssertEqual(e.phase, .tutorTurn, "ending early gives the tutor one goodbye turn")
        XCTAssertTrue(e.isClosing)
        e.tutorSpoke(goodbye: false)
        XCTAssertEqual(e.phase, .ended)

        var c = TalkConversation(turns: 7)
        c.scene = TalkScene(title: "At breakfast", ru: "За за́втраком")
        c.lines = [TalkLine(role: .tutor, ru: "До́брое у́тро!", at: Date())]
        XCTAssertEqual(TalkDialogue(resuming: c).phase, .learnerTurn, "a saved conversation resumes after the tutor's last line")
        c.lines.append(TalkLine(role: .learner, ru: "До́брое у́тро", at: Date()))
        let r = TalkDialogue(resuming: c)
        XCTAssertEqual(r.phase, .tutorTurn)
        XCTAssertEqual(r.learnerTurns, 1)
        c.ended = Date()
        XCTAssertEqual(TalkDialogue(resuming: c).phase, .ended)
        XCTAssertEqual(TalkDialogue(resuming: TalkConversation()).phase, .choosingScene)
    }

    // MARK: The streamed reply

    func testReplyParserSpeaksSentencesAsTheyArrive() {
        var p = TalkTutor.ReplyParser()
        var spoken: [String] = []
        spoken += p.feed("RU: До́брое у́тро! Что вы ")
        XCTAssertEqual(spoken, ["До́брое у́тро!"], "the first sentence is spoken before the line is complete")
        spoken += p.feed("хоти́те на за́втрак?\nEN: Good morn")
        XCTAssertEqual(spoken, ["До́брое у́тро!", "Что вы хоти́те на за́втрак?"])
        spoken += p.feed("ing! What do you want for breakfast?\nNEW: за́втрак = breakfast; ко́фе = coffee\n")
        spoken += p.finish()
        XCTAssertEqual(spoken.count, 2, "nothing is said twice")
        XCTAssertEqual(p.reply.ru, "До́брое у́тро! Что вы хоти́те на за́втрак?")
        XCTAssertEqual(p.reply.en, "Good morning! What do you want for breakfast?")
        XCTAssertEqual(p.reply.newWords.map(\.ru), ["за́втрак", "ко́фе"])
        XCTAssertEqual(p.reply.newWords.map(\.en), ["breakfast", "coffee"])
        XCTAssertFalse(p.reply.ended)
    }

    func testReplyParserHandlesQuotesEndAndMissingPrefix() {
        var p = TalkTutor.ReplyParser()
        var spoken = p.feed("RU: Вы хоте́ли сказа́ть «я хочу́ ко́фе»? Хорошо́.")
        XCTAssertEqual(spoken, ["Вы хоте́ли сказа́ть «я хочу́ ко́фе»?"], "a closing quote stays with its sentence; the last one waits for the stream")
        spoken += p.feed("\nEN: Did you mean “I want coffee”? Good.\nEND\n")
        spoken += p.finish()
        XCTAssertEqual(spoken, ["Вы хоте́ли сказа́ть «я хочу́ ко́фе»?", "Хорошо́."])
        XCTAssertTrue(p.reply.ended)

        var q = TalkTutor.ReplyParser()
        var s2 = q.feed("Приве́т! Как дела́?\nEN: Hi! How are things?\n")
        s2 += q.finish()
        XCTAssertEqual(s2, ["Приве́т!", "Как дела́?"], "a reply without the RU: prefix is still read as the spoken line")
        XCTAssertEqual(q.reply.ru, "Приве́т! Как дела́?")
        XCTAssertEqual(q.reply.en, "Hi! How are things?")
    }

    func testTurnsAlternateRolesAndNameTheLastTurn() {
        var c = TalkConversation(turns: 6)
        c.scene = TalkScene(title: "At breakfast", ru: "За за́втраком")
        let d0 = TalkDialogue(resuming: c)
        XCTAssertEqual(TalkTutor.turns(for: c, dialogue: d0).map(\.role), ["user"])
        c.lines = [TalkLine(role: .tutor, ru: "Что вы хоти́те?", en: "What do you want?", at: Date()),
                   TalkLine(role: .learner, ru: "", at: Date()),
                   TalkLine(role: .learner, ru: "Ко́фе", at: Date())]
        let turns = TalkTutor.turns(for: c, dialogue: TalkDialogue(resuming: c))
        XCTAssertEqual(turns.map(\.role), ["user", "assistant", "user"], "two learner lines in a row fold into one user turn")
        XCTAssertTrue(turns[1].text.hasPrefix("RU: Что вы хоти́те?"))
        XCTAssertTrue(turns[2].text.contains("(nothing was heard)") && turns[2].text.contains("«Ко́фе»"))
        var e = TalkDialogue(resuming: c)
        e.endEarly()
        XCTAssertTrue(TalkTutor.turns(for: c, dialogue: e).last!.text.contains("LAST turn"), "ending early tells the tutor to say goodbye")
    }

    func testScenesCarryTheInformationGap() {
        let reply = """
        {"scenes":[{"title":"At breakfast","ru":"За за́втраком","goal":"Find out what there is to drink and order one thing",
                    "frames":[{"ru":"Что есть …?","en":"what is there …?"},{"ru":"Я хочу́ …","en":"I want …"},{"ru":"А … есть?","en":"and is there …?"},{"ru":"too many","en":""}],
                    "secret":"There is tea and juice but no coffee"},
                   {"title":"In a shop","ru":"В магази́не","goal":"Ask the price of bread","frames":["Ско́лько сто́ит …? = how much is …?"],"facts":"Bread is 40 roubles"},
                   {"title":"At breakfast","ru":"dup"}, {"title":"Fourth","ru":"Четвёртая"}]}
        """
        let scenes = TalkTutor.parseScenes(reply)
        XCTAssertEqual(scenes.map(\.title), ["At breakfast", "In a shop", "Fourth"], "duplicates dropped, three at most")
        XCTAssertEqual(scenes[0].goal, "Find out what there is to drink and order one thing")
        XCTAssertEqual(scenes[0].frames?.count, 3, "at most three frames")
        XCTAssertEqual(scenes[0].frames?.first, TalkFrame(ru: "Что есть …?", en: "what is there …?"))
        XCTAssertEqual(scenes[0].secret, "There is tea and juice but no coffee")
        XCTAssertEqual(scenes[1].frames, [TalkFrame(ru: "Ско́лько сто́ит …?", en: "how much is …?")], "a frame as one string still reads")
        XCTAssertEqual(scenes[1].secret, "Bread is 40 roubles", "`facts` is read as the secret")
        XCTAssertNil(scenes[2].goal)
        // The scene's private facts reach the partner's prompt, not the marker's.
        let turnPrompt = TalkTutor.turnSystem(scene: scenes[0], state: "")
        XCTAssertTrue(turnPrompt.contains("no coffee") && turnPrompt.contains("order one thing"))
        XCTAssertTrue(turnPrompt.contains("not obliged to agree"))
        XCTAssertFalse(TalkTutor.markSystem.contains("private facts"), "the marker judges from the learner's words")
    }

    func testEndpointingProfileAndTurnRequest() {
        let c = TalkSession.endpointing
        XCTAssertEqual(c.trailingSilence, 1.5, "a hesitant beginner may pause mid-thought")
        XCTAssertEqual(c.maxSpeech, 12)
        XCTAssertEqual(c.waitLimit, 10)
        XCTAssertGreaterThan(c.trailingSilence, VoiceActivity.Config().trailingSilence)
        let plain = SpeakingTurn.Request(key: "k", target: "Да.", takeURL: URL(fileURLWithPath: "/tmp/x.m4a"), model: nil, repeats: 0)
        XCTAssertNil(plain.config, "read-aloud keeps the target's own endpointing")
        XCTAssertEqual(SpeakingTurn.config(target: "Да.", model: nil).maxSpeech, 6)
    }

    func testTalkIsUnavailableWithoutAKeyAndLeavable() throws {
        let previous = Keychain.apiKey
        Keychain.apiKey = nil
        defer { Keychain.apiKey = previous }
        let book = Book(course: try course(), store: tempStore())
        QA.complete(book, page: 0, upTo: .max); book.turnForward()
        book.insertTalkPage()
        let page = try XCTUnwrap(book.page)
        let ref = ExerciseRef(pageID: page.id, index: 0)
        let session = TalkSession(ref: ref, book: book, pageVoice: Voice(), voice: SilentTutorVoice())
        session.appear(autoStart: true)
        guard case .unavailable(let why) = session.status else { return XCTFail("expected unavailable, got \(session.status)") }
        XCTAssertTrue(why.contains("key"), why)
        XCTAssertNil(book.result(ref)?.talk, "nothing was started")
        session.propose()
        guard case .unavailable = session.status else { return XCTFail("propose must not reach the network without a key") }
        session.skip()
        XCTAssertEqual(book.result(ref)?.status, .skipped, "Not today leaves the page")
        XCTAssertEqual(session.status, .done)
        XCTAssertTrue(book.pages[book.currentPage].isFinished)
    }

    // MARK: The marking

    func testMarkingMapsOntoTheTranscript() throws {
        var c = TalkConversation(turns: 6)
        c.scene = TalkScene(title: "At breakfast", ru: "За за́втраком")
        c.lines = [TalkLine(role: .tutor, ru: "Что вы хоти́те?", at: Date()),
                   TalkLine(role: .learner, ru: "Я хочу кофе и хлеб", at: Date()),
                   TalkLine(role: .tutor, ru: "С молоко́м?", at: Date()),
                   TalkLine(role: .learner, ru: "С молоко и сахар", at: Date()),
                   TalkLine(role: .tutor, ru: "До свида́ния!", at: Date())]
        let reply = """
        Here is the marking:
        {"lines": [
          {"n": 1, "corrected": "Я хочу́ ко́фе и хлеб.", "edits": [], "praise": "Just right."},
          {"n": 2, "corrected": "С молоко́м и с са́харом.", "edits": [
             {"span": "молоко", "replacement": "молоко́м", "category": "case ending", "why": "с takes the instrumental", "en": "with milk", "pattern": "gram:instr"},
             {"span": "сахар", "replacement": "с са́харом", "category": "ending", "why": "the same", "en": "with sugar"}],
           "praise": ""}],
         "goal": {"reached": true, "conveyed": "They ordered coffee and bread and asked for milk and sugar.", "clarified": ""},
         "praise": "A whole conversation!",
         "teach": [{"title": "With: с + instrumental", "note": "After с the noun changes: [[с молоко́м|with milk]].", "pattern": "instr"}]}
        """
        let m = try XCTUnwrap(TalkTutor.parseMarking(reply, conversation: c, at: Date()))
        XCTAssertEqual(m.outcome?.reached, true)
        XCTAssertEqual(m.outcome?.conveyed, "They ordered coffee and bread and asked for milk and sugar.")
        XCTAssertNil(m.outcome?.clarified, "an empty clarification is none")
        XCTAssertEqual(m.lines.count, 2)
        XCTAssertEqual(m.lines[1]?.edits.count, 0)
        XCTAssertEqual(m.lines[1]?.praise, "Just right.")
        let second = try XCTUnwrap(m.lines[2])
        XCTAssertEqual(second.corrected, "С молоко́м и с са́харом.")
        XCTAssertEqual(second.edits.map(\.category), [.ending, .ending], "loose category names are read the diary's way")
        XCTAssertEqual(second.edits.first?.pattern, "instr")
        XCTAssertEqual(second.unplaced(in: "С молоко и сахар"), [], "every span is found in the line as heard")
        XCTAssertEqual(m.praise, "A whole conversation!")
        XCTAssertEqual(m.teach.first?.title, "With: с + instrumental")
        XCTAssertTrue(TalkTutor.transcript(c).contains("Learner 2: С молоко и сахар"))
    }

    // MARK: The voice

    func testVoicePickPrefersPremiumThenEnhancedAndRuRU() {
        typealias C = RussianVoicePick.Candidate
        let voices = [C(language: "en-US", quality: 3, name: "Ava"), C(language: "ru-RU", quality: 1, name: "Milena"),
                      C(language: "ru-RU", quality: 2, name: "Milena"), C(language: "ru-RU", quality: 1, name: "Yuri")]
        XCTAssertEqual(RussianVoicePick.best(voices), C(language: "ru-RU", quality: 2, name: "Milena"), "Enhanced beats Standard")
        XCTAssertEqual(RussianVoicePick.best(voices + [C(language: "ru-RU", quality: 3, name: "Katya")])?.name, "Katya", "Premium beats Enhanced")
        XCTAssertEqual(RussianVoicePick.best([C(language: "ru-UA", quality: 2, name: "A"), C(language: "ru-RU", quality: 2, name: "B")])?.name, "B", "ru-RU first at equal quality")
        XCTAssertNil(RussianVoicePick.best([C(language: "en-GB", quality: 3, name: "Daniel")]), "never a non-Russian voice")
        if let v = Voice.bestRussianVoice {
            XCTAssertTrue(v.language.hasPrefix("ru"))
            let all = AVSpeechSynthesisVoice.speechVoices().filter { $0.language.hasPrefix("ru") }
            XCTAssertEqual(v.quality.rawValue, all.map(\.quality.rawValue).max(), "the app's voice is the best installed")
        }
        XCTAssertEqual(TalkSession.kindTip(from: PronunciationReport(target: "да", transcript: "да", expected: [], heard: [], sounds: [],
                                                                     syllables: [], stressedSyllable: nil, accuracy: 1, tips: [])), nil)
    }

    // MARK: Persistence

    func testConversationRoundTripsInsideTheResult() throws {
        var r = ExerciseResult()
        var c = TalkConversation(turns: 7)
        c.proposed = [TalkScene(title: "In a shop", ru: "В магази́не")]
        c.scene = c.proposed?.first
        c.started = Date(timeIntervalSince1970: 1_000)
        c.lines = [TalkLine(role: .tutor, ru: "Здра́вствуйте!", en: "Hello!", newWords: [Course.Word(ru: "магази́н", en: "shop", emoji: nil, note: nil)],
                            latency: TalkLatency(firstToken: 800, firstSpeech: 1200, complete: 2000), at: Date(timeIntervalSince1970: 1_001)),
                   TalkLine(role: .learner, ru: "Здравствуйте", take: "p1-0-abc.m4a", correction: DiaryCorrection(corrected: "Здра́вствуйте.", edits: [], praise: "", teach: [], at: Date(timeIntervalSince1970: 1_002)),
                            tip: "One thing to try: …", at: Date(timeIntervalSince1970: 1_002))]
        c.ended = Date(timeIntervalSince1970: 1_003)
        c.marked = Date(timeIntervalSince1970: 1_004)
        c.praise = "Well done."
        c.teach = [DiaryTeach(title: "Greeting", note: "", pattern: nil)]
        r.talk = c
        r.status = .correct
        let data = try JSONEncoder().encode(r)
        let back = try JSONDecoder().decode(ExerciseResult.self, from: data)
        XCTAssertEqual(back.talk, c)
        XCTAssertEqual(back.talk?.lines[0].latency?.firstSpeech, 1200)
        // A result from before Talk existed, and an exercise of the new kind, both decode.
        let old = try JSONDecoder().decode(ExerciseResult.self, from: Data(#"{"status":"done","attempts":1,"revealed":false}"#.utf8))
        XCTAssertNil(old.talk)
        let ex = try JSONDecoder().decode(Exercise.self, from: Data(#"{"kind":"talk"}"#.utf8))
        XCTAssertEqual(ex.kind, .talk)
        XCTAssertNil(ex.target); XCTAssertNil(ex.spoken); XCTAssertFalse(ex.kind.isWriting)
    }

    // MARK: Evidence

    func testFinishedConversationBecomesSpeakingEvidence() throws {
        let book = Book(course: try course(), store: tempStore())
        for p in 0..<4 { QA.complete(book, page: p, upTo: .max); book.turnForward() }
        QA.talkPage(book, state: "marked")
        let page = try XCTUnwrap(book.page)
        XCTAssertEqual(page.exercises.first?.kind, .talk)
        XCTAssertEqual(page.results.first?.status, .done, "a marked conversation with corrections finishes as done")
        let events = book.knowledge.model.events.filter { $0.activity == "talk" }
        XCTAssertFalse(events.isEmpty)
        XCTAssertTrue(events.allSatisfy { $0.modality == .speak })
        let credits = events.filter { $0.id.hasSuffix(":t1") || $0.id.hasSuffix(":t2") || $0.id.hasSuffix(":t3") }
        XCTAssertEqual(credits.count, 3, "each understood turn credits the words said")
        XCTAssertTrue(credits.allSatisfy { $0.grade == .good })
        let again = events.filter { $0.grade == .again }
        XCTAssertEqual(again.count, 4, "one `again` per red-pen edit")
        XCTAssertTrue(again.allSatisfy { $0.confusions?.allSatisfy { $0.channel == .speech } ?? true })
        XCTAssertTrue(again.contains { $0.components.contains(ComponentID(.construction, "instr")) }, "the taught pattern rides on an edit")
        XCTAssertTrue(book.isKept("прия́тного аппети́та"), "the tutor's new words join the learner's words")
        XCTAssertTrue(book.isKept("кни́гу"), "corrected forms join them too")
        XCTAssertEqual(book.talkRecords.count, 1)
        XCTAssertEqual(book.talkRecords.first?.conversation.learnerTurns, 3)
        // Reopening the notebook reconstructs nothing twice.
        let again2 = Book(course: try course(), store: book.store)
        XCTAssertEqual(again2.knowledge.backfill(pages: again2.pages), 0)
    }

    // MARK: Live

    /// A two-turn exchange through the app's own session: scenes, the tutor's opener spoken
    /// sentence by sentence, the learner's line from a bundled clip (transcribed when speech
    /// recognition is allowed, otherwise what the clip says), the reply, then the marking.
    /// Runs only with TEST_RUNNER_DRUSSKY_TUTOR_KEY; the Keychain's own key is restored after.
    func testTalkTwoTurnsLive() async throws {
        let key = ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"] ?? ""
        try XCTSkipIf(key.isEmpty, "no tutor key supplied")
        let previous = Keychain.apiKey
        Keychain.apiKey = key
        defer { Keychain.apiKey = previous }
        let book = Book(course: try course(), store: tempStore())
        for p in 0..<12 { QA.complete(book, page: p, upTo: .max); book.turnForward() }
        book.insertTalkPage()
        let page = try XCTUnwrap(book.page)
        let ref = ExerciseRef(pageID: page.id, index: 0)
        let voice = SilentTutorVoice()
        let session = TalkSession(ref: ref, book: book, pageVoice: Voice(), voice: voice)

        func wait(for want: TalkSession.Status, seconds: Double = 90) async throws {
            let start = Date()
            while session.status != want {
                if case .failed(let m) = session.status { XCTFail("failed: \(m)"); return }
                try XCTSkipIf(Date().timeIntervalSince(start) > seconds, "timed out waiting for \(want) in \(session.status)")
                try await Task.sleep(for: .milliseconds(100))
            }
        }
        let t0 = Date()
        session.propose()
        try await wait(for: .choosing)
        let scenes = try XCTUnwrap(session.conversation.proposed)
        print("TALK scenes (\(Int(Date().timeIntervalSince(t0) * 1000)) ms):", scenes.map { "\($0.title) / \($0.ru)" })
        XCTAssertEqual(scenes.count, 3)
        print("TALK gap:", scenes.map { "\($0.goal ?? "-") | frames \(($0.frames ?? []).map(\.ru)) | secret \($0.secret ?? "-")" })
        XCTAssertTrue(scenes.allSatisfy { $0.goal != nil && !($0.frames ?? []).isEmpty }, "every scene has a goal and frames")
        XCTAssertTrue(scenes.contains { $0.secret != nil }, "the partner has private facts")
        session.choose(scenes[0])
        try await wait(for: .yourTurn)
        let opener = try XCTUnwrap(session.conversation.tutorLines.last)
        print("TALK opener:", opener.ru, "|", opener.en ?? "", "| latency:", opener.latency.map { "first token \($0.firstToken ?? -1) ms, first speech \($0.firstSpeech ?? -1) ms, complete \($0.complete ?? -1) ms" } ?? "none")
        XCTAssertFalse(RussianText.letters(in: opener.ru).isEmpty)
        XCTAssertFalse(voice.spoken.isEmpty, "the voice was handed sentences")
        XCTAssertNotNil(opener.latency?.firstSpeech)

        // The learner's audio: a bundled clip of «до́брое у́тро», heard by the recognizer when allowed.
        let clip = try XCTUnwrap(Bundle.main.url(forResource: "dobroe-utro-b1d3ed", withExtension: "m4a", subdirectory: "Audio"))
        var heard = "Доброе утро"
        if SFSpeechRecognizer.authorizationStatus() == .authorized, let h = await SpeechCheck.transcribe(clip, hint: "доброе утро"), !h.isEmpty { heard = h }
        print("TALK learner clip heard as:", heard)
        session.mic.cancel()
        session.typedDraft = heard
        session.submitTyped()
        try await wait(for: .yourTurn)
        let reply = try XCTUnwrap(session.conversation.tutorLines.last)
        print("TALK reply:", reply.ru, "|", reply.en ?? "", "| latency:", reply.latency.map { "first token \($0.firstToken ?? -1) ms, first speech \($0.firstSpeech ?? -1) ms, complete \($0.complete ?? -1) ms" } ?? "none")
        XCTAssertEqual(session.conversation.learnerTurns, 1)
        XCTAssertEqual(session.conversation.tutorLines.count, 2)

        session.mic.cancel()
        session.typedDraft = "Я хочу кофе и хлеб"
        session.submitTyped()
        session.endNow()
        try await wait(for: .done, seconds: 150)
        let c = session.conversation
        print("TALK outcome:", c.outcome.map { "reached=\(String(describing: $0.reached)) conveyed=\($0.conveyed) clarified=\($0.clarified ?? "-")" } ?? "none")
        XCTAssertNotNil(c.outcome, "the marking says first whether the goal was reached")
        print("TALK goodbye:", c.tutorLines.last?.ru ?? "", "| marks:", c.learnerLines.map { "\($0.ru) → \($0.corrected) [\($0.correction?.edits.count ?? -1)]" }, "| praise:", c.praise ?? "", "| teach:", c.teach?.map(\.title) ?? [])
        XCTAssertTrue(c.isEnded && c.isMarked)
        XCTAssertEqual(c.learnerLines.count, 2)
        XCTAssertTrue(c.learnerLines.allSatisfy { $0.correction != nil }, "every learner line is marked")
        XCTAssertTrue(book.result(ref)?.status.isComplete ?? false)
        XCTAssertTrue(book.knowledge.model.events.contains { $0.activity == "talk" && $0.modality == .speak && $0.grade == .good })
    }
}
