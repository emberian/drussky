import PencilKit
import Speech
import XCTest
@testable import Drussky

@MainActor
final class DrusskyTests: XCTestCase {
    func testTextKeys() {
        XCTAssertEqual(RussianText.key("Кни́га."), "книга")
        XCTAssertEqual(RussianText.key("Что э́то?"), "что это")
        XCTAssertTrue(RussianText.matches("KOT", "кот"))
        XCTAssertTrue(RussianText.matches("ёж", "еж"))
        XCTAssertTrue(RussianText.isRepetition("Аа Аа", of: "А"))
        XCTAssertFalse(RussianText.isRepetition("Ма", of: "А"))
        XCTAssertEqual(String(Theme.stressed("они́", show: true).characters), "они")
        XCTAssertTrue(RussianText.matches("Sno Kom", "Это кот"))
        XCTAssertFalse(RussianText.matches("оно", "она"), "pronouns stay distinct")
        XCTAssertFalse(RussianText.matches("OHO", "она"))
        XCTAssertFalse(RussianText.matches("mama", "папа"))
        XCTAssertTrue(RussianText.answerMatches("читаю", "Я чита́ю."))
        XCTAssertTrue(RussianText.answerMatches("я читаю", "чита́ю"))
        XCTAssertFalse(RussianText.answerMatches("ты читаешь", "Я чита́ю."))
    }

    func testCourseDecodesAndIsWellFormed() throws {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        let course = try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
        XCTAssertEqual(course.letters.count, 33)
        XCTAssertFalse(course.units.isEmpty)
        for (_, page) in course.pageSequence {
            for ex in page.exercises {
                switch ex.kind {
                case .meet: XCTAssertNotNil(course.letter(ex.letter ?? ""), page.id)
                case .translate, .fill: XCTAssertNotNil(ex.answerText, page.id)
                case .readMeaning: XCTAssertNotNil(ex.answerIndex, page.id)
                case .listenPick: XCTAssertTrue(ex.options?.contains(ex.say ?? "") ?? false, page.id)
                default: break
                }
            }
        }
    }

    func testEverySpokenStringHasARecording() throws {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        let course = try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
        let voice = Voice()
        var missing: [String] = []
        var spoken: [String] = course.letters.map(\.say) + course.lexicon.map(\.ru)
        for (_, page) in course.pageSequence {
            for ex in page.exercises {
                spoken += [ex.spoken].compactMap { $0 } + (ex.options ?? []).filter { _ in ex.kind == .listenPick }
                spoken += (ex.examples ?? []).map(\.ru)
            }
        }
        // Generated reviews speak too.
        let book = Book(course: course, store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        for (i, letter) in course.letters.enumerated() {
            let m = Memory(key: "letter:\(letter.upper)", stability: 1, due: .now, seen: .now)
            spoken += [book.reviewExercise(for: m, variant: i).spoken].compactMap { $0 }
        }
        for word in course.lexicon {
            let m = Memory(key: "word:\(RussianText.key(word.ru))", stability: 1, due: .now, seen: .now)
            for v in 0..<3 { spoken += [book.reviewExercise(for: m, variant: v).spoken].compactMap { $0 } }
        }
        for s in Set(spoken) where !voice.hasRecording(s) { missing.append(s) }
        XCTAssertTrue(missing.isEmpty, "no recording for: \(missing.sorted())")
    }

    /// Vision reads Cyrillic ink. Synthetic ink is font outlines, so this is a lower bound on
    /// the pipeline working, not a measure of accuracy on real handwriting.
    func testInkReaderReadsCursiveCyrillic() async throws {
        var hits: [String] = [], misses: [String] = []
        for word in ["кот", "мама", "окно", "она", "книга", "спасибо", "молоко", "Это кот", "где", "читаю", "хорошо", "звезда"] {
            let drawing = SyntheticInk.drawing(for: word, height: 132, cursive: true)
            let reading = await InkReader.read(drawing, target: word)
            if reading?.matches == true { hits.append(word) } else { misses.append("\(word)→\(reading?.candidates ?? [])") }
        }
        print("CURSIVE hits:", hits.count, hits, "misses:", misses)
        XCTAssertGreaterThanOrEqual(hits.count, 6, "misses: \(misses)")
    }

    func testInkReaderReadsSyntheticCyrillic() async throws {
        var hits: [String] = []
        var misses: [String] = []
        for word in ["кот", "мама", "окно", "она", "книга", "Это кот", "спасибо", "Я", "Ж", "ы", "щ", "Ф", "и"] {
            let drawing = SyntheticInk.drawing(for: word, height: 132)
            let reading = await InkReader.read(drawing, target: word)
            if reading?.matches == true { hits.append(word) } else { misses.append("\(word)→\(reading?.best ?? "nil") \(reading?.candidates ?? [])") }
        }
        print("INK hits:", hits, "misses:", misses)
        XCTAssertGreaterThanOrEqual(hits.count, 6, "misses: \(misses)")
    }

    func testBookSchedulesReviewsAndPersists() throws {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        let course = try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let store = BookStore(root: root)
        let book = Book(course: course, store: store)
        XCTAssertEqual(book.pages.count, 1)
        for p in 0..<3 {
            QA.complete(book, page: p, upTo: .max)
            XCTAssertTrue(book.pages[p].isFinished)
            book.turnForward()
        }
        XCTAssertFalse(book.memories.isEmpty)
        book.shiftMemories(byDays: 3)
        XCTAssertGreaterThan(book.dueCount, 0)
        book.turnForward()
        let page = book.pages.last!
        XCTAssertTrue(page.exercises.contains { $0.reviewKey != nil }, "due items woven into next page")
        book.saveNow()
        let reopened = Book(course: course, store: store)
        XCTAssertEqual(reopened.pages.count, book.pages.count)
        XCTAssertEqual(reopened.currentPage, book.currentPage)
    }

    private func course() throws -> Course {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        return try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    func testUnreadableNotebookIsMovedAsideNotOverwritten() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let store = BookStore(root: root)
        try Data("{ not a notebook".utf8).write(to: store.stateURL)
        let book = Book(course: try course(), store: store)
        book.saveNow()
        let files = try FileManager.default.contentsOfDirectory(atPath: root.path)
        XCTAssertTrue(files.contains { $0.hasPrefix("book-unreadable-") && $0.hasSuffix(".json") }, "\(files)")
        let aside = try XCTUnwrap(files.first { $0.hasPrefix("book-unreadable-") && $0.hasSuffix(".json") })
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent(aside), encoding: .utf8), "{ not a notebook")
    }

    func testRewritingAMissedAnswerHidesItAndCountsOnce() throws {
        let book = Book(course: try course(), store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        let i = try XCTUnwrap(book.pages[0].exercises.firstIndex { [.dictation, .translate, .fill].contains($0.kind) && book.memoryKey(for: $0) != nil })
        let key = try XCTUnwrap(book.memoryKey(for: book.pages[0].exercises[i]))
        for j in book.pages[0].exercises.indices { book.record(page: 0, exercise: j) { $0.status = j == i ? .missed : .done; $0.revealed = j == i } }
        XCTAssertTrue(book.pages[0].isFinished)
        let lapses = book.memories[key]?.lapses
        book.reopen(page: 0, exercise: i)
        XCTAssertFalse(book.pages[0].results[i].revealed)
        XCTAssertFalse(book.pages[0].isFinished, "an open exercise un-finishes the page")
        book.record(page: 0, exercise: i) { $0.status = .correct }
        XCTAssertEqual(book.memories[key]?.lapses, lapses)
        XCTAssertEqual(book.memories[key]?.reps, 0, "a redo after being shown the answer is not a recall")
    }

    /// The speech path, end to end, on recorded Russian: the same transcription call the
    /// read-aloud exercise makes after you speak.
    func testSpeechRecognizerHearsRecordedRussian() async throws {
        // Never prompt from a test (a prompt would hang it); runs once the app has been allowed.
        let status = SFSpeechRecognizer.authorizationStatus()
        try XCTSkipUnless(status == .authorized, "speech recognition not authorized in this environment (\(status.rawValue))")
        let voice = Voice()
        var hits: [String] = [], misses: [String] = []
        for phrase in ["Э́то кот.", "спаси́бо", "Где ма́ма?", "Я чита́ю кни́гу.", "Мы тут.", "молоко́", "Как дела́?", "Она́ там."] {
            guard let url = voice.recordingURL(for: phrase) else { misses.append("\(phrase): no file"); continue }
            let t0 = Date()
            let heard = await SpeechCheck.transcribe(url, hint: RussianText.unstressed(phrase))
            print("SPEECH-TIME", phrase, String(format: "%.1fs", Date().timeIntervalSince(t0)))
            if let heard, RussianText.matches(heard, phrase) { hits.append(phrase) } else { misses.append("\(phrase)→\(heard ?? "nil")") }
        }
        print("SPEECH hits:", hits, "misses:", misses)
        XCTAssertGreaterThanOrEqual(hits.count, 5, "misses: \(misses)")
    }

    /// One live tutor answer through the app's own streaming client. Runs only when a key is
    /// supplied via TEST_RUNNER_DRUSSKY_TUTOR_KEY; the key is removed from the Keychain after.
    func testTutorAnswersLive() async throws {
        let key = ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"] ?? ""
        try XCTSkipIf(key.isEmpty, "no tutor key supplied")
        let previous = Keychain.apiKey
        Keychain.apiKey = key
        defer { Keychain.apiKey = previous }  // never wipe a key the app already has
        var answer = ""
        let context = "Workbook page 3: “Н: looks like H”. Letters met so far: А М О Т К Н.\nWriting the Russian for “she” (answer «она́»). Missed it and was shown the answer. Handwriting recognition read: «оно»."
        for try await chunk in Tutor.stream([.init(role: "user", text: "Why is it она and not оно?")], context: context) {
            answer += chunk
        }
        print("TUTOR:", answer)
        XCTAssertGreaterThan(answer.count, 40)
        XCTAssertTrue(answer.contains("он"), "answer discusses the Russian forms")
    }

    /// Single-line pen strokes (Hershey Cyrillic) with per-variant slant, scale, drift and
    /// wobble: closer to a hand than font outlines. 52 items × 3 variants.
    func testInkReaderReadsPenStrokes() async throws {
        let url = try XCTUnwrap(Bundle(for: Self.self).url(forResource: "hershey-ink", withExtension: "json"))
        let fixture = try JSONDecoder().decode([String: [[[[Double]]]]].self, from: Data(contentsOf: url))
        let ink = PKInk(.pen, color: .black)
        var hits = 0, total = 0
        var misses: [String] = []
        for (target, variants) in fixture.sorted(by: { $0.key < $1.key }) {
            for strokes in variants {
                let pk = strokes.map { pts in
                    PKStroke(ink: ink, path: PKStrokePath(controlPoints: pts.enumerated().map { i, p in
                        PKStrokePoint(location: CGPoint(x: p[0], y: p[1]), timeOffset: Double(i) * 0.01,
                                      size: CGSize(width: 3.5, height: 3.5), opacity: 1, force: 1, azimuth: 0, altitude: .pi / 2)
                    }, creationDate: Date()))
                }
                let reading = await InkReader.read(PKDrawing(strokes: pk), target: target)
                total += 1
                if reading?.matches == true { hits += 1 } else { misses.append("\(target)→\(reading?.best ?? "nil")") }
            }
        }
        print("PEN hits: \(hits)/\(total) misses:", misses)
        XCTAssertGreaterThanOrEqual(Double(hits) / Double(total), 0.75, "misses: \(misses)")
    }

    func testPassageDecodesAndStripsPunctuation() throws {
        let json = #"{"kind":"passage","text":"Робо́т ви́дел звезду́. «Что э́то?»","en":"The robot saw a star.","glosses":{"ви́дел":"saw","звезду":"star (object)"}}"#
        let ex = try JSONDecoder().decode(Exercise.self, from: Data(json.utf8))
        XCTAssertEqual(ex.kind, .passage)
        XCTAssertEqual(ex.glosses?.count, 2)
        XCTAssertEqual(ex.spoken, ex.text)
        XCTAssertEqual(PassageExercise.bare("звезду́."), "звезду́")
        XCTAssertEqual(PassageExercise.bare("«Что"), "Что")
        XCTAssertEqual(PassageExercise.bare("э́то?»"), "э́то")
        let round = try JSONDecoder().decode(Exercise.self, from: JSONEncoder().encode(ex))
        XCTAssertEqual(round.glosses, ex.glosses)
    }

    func testTutorPhrasesAndKeeping() throws {
        let answer = "Use [[она́|she]] for a woman and [[оно́|it]] for neuter things, e.g. [[окно́|window]]. Not [[она́|she]] twice."
        let phrases = Tutor.phrases(in: answer)
        XCTAssertEqual(phrases.map(\.ru), ["она́", "оно́", "окно́"])
        XCTAssertEqual(phrases.first?.en, "she")
        XCTAssertTrue(Tutor.displayMarkdown(answer).contains("](drussky-say:?t="))
        XCTAssertEqual(Tutor.phrases(in: "Say спаси́бо and до свида́ния.").map(\.ru), ["спаси́бо", "до свида́ния"])

        let book = Book(course: try course(), store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        book.keep("до свида́ния", en: "goodbye")
        XCTAssertTrue(book.isKept("до свидания"))
        XCTAssertEqual(book.word("до свидания")?.en, "goodbye")
        book.shiftMemories(byDays: 2)
        let review = book.reviewExercise(for: try XCTUnwrap(book.memories["word:до свидания"]), variant: 0)
        XCTAssertEqual(review.kind, .translate)
        XCTAssertEqual(review.answerText, "до свида́ния")
    }

    /// The tutor writes real pages (live API; runs only with TEST_RUNNER_DRUSSKY_TUTOR_KEY).
    func testPageWriterLive() async throws {
        let key = ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"] ?? ""
        try XCTSkipIf(key.isEmpty, "no tutor key supplied")
        let previous = Keychain.apiKey
        Keychain.apiKey = key
        defer { Keychain.apiKey = previous }
        let book = Book(course: try course(), store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        for p in 0..<6 { QA.complete(book, page: p, upTo: .max); book.turnForward() }

        let (practice, pex) = try await PageWriter.write(.practice, book: book) { _ in }
        print("PRACTICE:", practice.title, "|", pex.count, "of", practice.exercises.count, "usable |", pex.map(\.kind.rawValue))
        XCTAssertGreaterThanOrEqual(pex.count, 6)

        let story = "Робот проснулся ночью. На станции было тихо. Он посмотрел в окно и увидел звезду, которой вчера не было."
        let (reading, rex) = try await PageWriter.write(.text(story), book: book) { _ in }
        print("READING:", reading.title, "|", rex.count, "usable |", rex.map(\.kind.rawValue), "| new words:", reading.new_words?.map(\.ru) ?? [])
        XCTAssertEqual(rex.first?.kind, .passage)
        XCTAssertGreaterThanOrEqual(rex.count, 4)
        book.insertWrittenPage(title: reading.title, intro: reading.intro, unitTitle: "Your text", exercises: rex, newWords: reading.new_words ?? [])
        XCTAssertEqual(book.page?.unitTitle, "Your text")
        XCTAssertEqual(book.pages.map(\.number), Array(1...book.pages.count))
    }

    func testSecureWordsLoseTheirScaffolds() throws {
        let book = Book(course: try course(), store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        var copy = Exercise(kind: .copy); copy.text = "ма́ма"; copy.times = 2
        XCTAssertEqual(book.faded(copy).kind, .copy, "new words keep their support")
        let key = try XCTUnwrap(book.memoryKey(for: copy))
        book.keep("ма́ма", en: "mom")
        for _ in 0..<2 {
            book.shiftMemories(byDays: 5)
            var d = Exercise(kind: .dictation); d.text = "ма́ма"; d.reviewKey = key
            book.insertWrittenPage(title: "t", intro: nil, unitTitle: "u", exercises: [d], newWords: [])
            book.record(page: book.currentPage, exercise: 0) { $0.status = .correct }
        }
        XCTAssertEqual(book.faded(copy).kind, .dictation, "a secure word is written from hearing instead")
    }

    /// A course revision that inserts a page before the learner's place is picked up next,
    /// and no page is skipped or repeated.
    func testCourseRevisionsNeverSkipOrRepeatPages() throws {
        var c = try course()
        let book1 = Book(course: c, store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        for p in 0..<3 { QA.complete(book1, page: p, upTo: .max); book1.turnForward() }
        let had = book1.pages.compactMap(\.sourceID)
        var inserted = c.units[0].pages[0]; inserted.id = "u1-inserted"
        c.units[0].pages.insert(inserted, at: 1)
        let store = BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
        let b = Book(course: try course(), store: store)
        for p in 0..<3 { QA.complete(b, page: p, upTo: .max); b.turnForward() }
        b.saveNow()
        let revised = Book(course: c, store: store)
        revised.turnForward()
        XCTAssertEqual(revised.pages.last?.sourceID, "u1-inserted")
        revised.turnForward()
        let ids = revised.pages.compactMap(\.sourceID)
        XCTAssertEqual(Set(ids).count, ids.count, "no page repeated")
        XCTAssertTrue(Set(had).isSubset(of: Set(ids)))
    }

    /// Recording a read-aloud result must not read the book while mutating it (the pattern
    /// that crashed on the iPad after stopping a recording).
    func testRecordingAReadAloudResultDoesNotTrap() throws {
        let book = Book(course: try course(), store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        let i = try XCTUnwrap(book.pages[0].exercises.firstIndex { $0.kind == .readAloud })
        let slot = ExerciseSlot(book: book, pageIndex: 0, index: i)
        let file = slot.recordingURL.lastPathComponent
        slot.finish { r in r.attempts += 1; r.recording = file; r.recognized = "мама"; r.status = .correct }
        XCTAssertEqual(book.pages[0].results[i].recording, file)
    }
}
