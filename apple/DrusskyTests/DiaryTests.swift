import LearningCore
import XCTest
@testable import Drussky

/// The diary: a new exercise kind, the tutor's correction contract, and corrections as evidence.
@MainActor
final class DiaryTests: XCTestCase {
    private func course() throws -> Course {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        return try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    private func tempStore() -> BookStore {
        BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent("diary-\(UUID().uuidString)"))
    }

    func testDiaryKindDecodesAndIsAuthoredFromUnitThree() throws {
        let ex = try JSONDecoder().decode(Exercise.self, from: Data(#"{"kind":"diary","prompt":"What did you eat today?"}"#.utf8))
        XCTAssertEqual(ex.kind, .diary)
        XCTAssertEqual(ex.prompt, "What did you eat today?")
        XCTAssertNil(ex.target); XCTAssertNil(ex.spoken, "diary prompts are never spoken")
        XCTAssertFalse(ex.kind.isWriting, "QA's synthetic ink has no answer to write")
        let round = try JSONDecoder().decode(Exercise.self, from: JSONEncoder().encode(ex))
        XCTAssertEqual(round.prompt, ex.prompt)

        let c = try course()
        var unitsSeen = 0
        for unit in c.units {
            defer { unitsSeen += 1 }
            for page in unit.pages {
                let diaries = page.exercises.filter { $0.kind == .diary }
                if unitsSeen < 2 {
                    XCTAssertTrue(diaries.isEmpty, "\(page.id): no diary before unit 3")
                } else {
                    XCTAssertEqual(diaries.count, 1, "\(page.id): one diary line")
                    XCTAssertEqual(page.exercises.last?.kind, .diary, "\(page.id): the diary closes the page")
                    XCTAssertFalse((diaries.first?.prompt ?? "").isEmpty)
                }
            }
        }
        let book = Book(course: c, store: tempStore())
        XCTAssertNil(book.memoryKey(for: ex), "a diary line is not itself a thing to recall")
    }

    func testCorrectionParsingIsTolerant() throws {
        let reply = """
        Here you go:
        ```json
        {"corrected": "Я чита́ю кни́гу.",
         "edits": [{"span": "книга", "replacement": "кни́гу", "category": "Case ending", "why": "the object takes -у", "en": "book", "pattern": "gram:acc-u"},
                   {"span": "", "replacement": "x"},
                   {"span": "читаю", "replacement": "чита́ю", "category": "stress"}],
         "praise": "Good word order.",
         "teach": [{"title": "Objects take -у", "note": "кни́га → кни́гу", "pattern": "acc-u"}, "a bare string", {"title": "third"}]}
        ```
        """
        let c = try XCTUnwrap(DiaryTutor.parse(reply, at: Date()))
        XCTAssertEqual(c.corrected, "Я чита́ю кни́гу.")
        XCTAssertEqual(c.edits.count, 1, "an empty span and a stress-only difference are not edits: \(c.edits)")
        XCTAssertEqual(c.edits[0].category, .ending)
        XCTAssertEqual(c.edits[0].pattern, "acc-u")
        XCTAssertEqual(c.edits[0].en, "book")
        XCTAssertEqual(c.teach.count, 2, "at most two teach items")
        XCTAssertEqual(c.teach[1].title, "a bare string")
        XCTAssertNil(DiaryTutor.parse("no json here", at: Date()))
        XCTAssertNil(DiaryTutor.parse(#"{"edits": []}"#, at: Date()), "a correction must carry the corrected text")
        XCTAssertEqual(DiaryTutor.parse(#"{"edits": []}"#, fallbackText: "Кот тут.", at: Date())?.corrected, "Кот тут.")
        // Unknown categories decode as `other` instead of breaking the notebook.
        let stored = try JSONDecoder().decode(DiaryEdit.self, from: Data(#"{"span":"a","replacement":"b","category":"something-new","why":""}"#.utf8))
        XCTAssertEqual(stored.category, .other)
        XCTAssertEqual(DiaryEdit.Category(loose: "word choice"), .wordChoice)
        XCTAssertEqual(DiaryEdit.Category(loose: "Word-Order"), .wordOrder)
    }

    func testRedPenMarksTheSpansInTheLearnersText() {
        let c = DiaryCorrection(corrected: "Моя́ сестра́ чита́ет кни́гу.",
                                edits: [DiaryEdit(span: "Мой сестра", replacement: "Моя́ сестра́", category: .agreement, why: ""),
                                        DiaryEdit(span: "книга", replacement: "кни́гу", category: .ending, why: ""),
                                        DiaryEdit(span: "nowhere", replacement: "x", category: .other, why: "")],
                                praise: "", teach: [], at: Date())
        let segments = c.segments(of: "мой сестра читает книга.")
        XCTAssertEqual(segments.map(\.text), ["мой сестра", " читает ", "книга."], "punctuation after a marked span stays under its underline")
        XCTAssertEqual(segments.map(\.edit), [0, nil, 1])
        XCTAssertEqual(c.unplaced(in: "мой сестра читает книга."), [2])
    }

    func testCorrectionsBecomeEvidenceAndWords() throws {
        let store = tempStore()
        let book = Book(course: try course(), store: store)
        var ex = Exercise(kind: .diary); ex.prompt = "What are you reading?"
        book.insertWrittenPage(title: "Дневник", intro: nil, unitTitle: "My Russian", exercises: [ex], newWords: [])
        let ref = ExerciseRef(pageID: book.pages[book.currentPage].id, index: 0)
        book.submitDiary(ref, entry: DiaryEntry(prompt: ex.prompt!, text: "Я читаю книга.", recognized: "Я читаю кни2а.", submitted: book.now()))
        XCTAssertEqual(book.result(ref)?.status, .open)
        XCTAssertEqual(book.result(ref)?.attempts, 1)
        let correction = DiaryCorrection(corrected: "Я чита́ю кни́гу.",
                                         edits: [DiaryEdit(span: "книга", replacement: "кни́гу", category: .ending, why: "object", en: "book (object)", pattern: "acc-u")],
                                         praise: "Я чита́ю is right.", teach: [DiaryTeach(title: "Objects take -у", note: "", pattern: "acc-u")], at: book.now())
        book.finishDiary(ref, correction: correction)
        let r = try XCTUnwrap(book.result(ref))
        XCTAssertEqual(r.status, .done, "a corrected line is finished, not 'right'")
        XCTAssertEqual(r.diary?.correction?.edits.count, 1)

        let m = book.knowledge.model
        let edges = m.confusionEdges(at: book.now(), minWeight: 0)
        XCTAssertTrue(edges.contains { $0.from == ComponentID(.form, "книгу") && $0.to == ComponentID(.lexeme, "книга") },
                      "what was wanted → what was written: \(edges)")
        XCTAssertEqual(m.trace(ComponentID(.form, "книгу"), .produce)?.reviews.map(\.grade), [.again])
        XCTAssertEqual(m.trace(ComponentID(.construction, "acc-u"), .produce)?.reviews.map(\.grade), [.again], "the pattern behind the slip")
        XCTAssertEqual(m.trace(ComponentID(.form, "читаю"), .produce)?.reviews.map(\.grade), [.good], "words written right are production evidence")
        XCTAssertEqual(m.trace(ComponentID(.lexeme, "книга"), .produce)?.reviews.map(\.grade), [.again], "a wrong form marks its lemma too, as dictation does")
        // The corrected form joins the learner's words and comes back in a recall modality.
        XCTAssertNotNil(book.targets["word:книгу"])
        XCTAssertTrue(book.reviewModalities(for: "word:книгу").contains(.hearWrite))
        book.advanceClock(byDays: 2)
        XCTAssertTrue(book.dueReviews().contains { $0.id == "word:книгу" })

        // Persisted, and the log is not doubled by the next launch's backfill.
        book.saveNow()
        let events = EventLog(url: book.knowledge.logURL).load().filter { $0.item == ref.item }
        XCTAssertEqual(events.filter { $0.type == .outcome }.count, 2, "\(events.map(\.id))")
        let again = Book(course: try course(), store: store)
        XCTAssertEqual(again.result(ref)?.diary?.text, "Я читаю книга.")
        XCTAssertEqual(again.result(ref)?.diary?.recognized, "Я читаю кни2а.")
        XCTAssertEqual(again.knowledge.backfill(pages: again.pages), 0)
        XCTAssertEqual(EventLog(url: again.knowledge.logURL).load().filter { $0.item == ref.item }.count, events.count)
        XCTAssertEqual(again.diaryEntries.count, 1)
        XCTAssertEqual(again.diaryCategoryCounts().first?.0, .ending)
        XCTAssertNotNil(again.exportDiary())

        // A line that needed no change is right; a skipped line is nothing.
        var ex2 = Exercise(kind: .diary); ex2.prompt = "And now?"
        again.insertWrittenPage(title: "Дневник", intro: nil, unitTitle: "My Russian", exercises: [ex2, ex2], newWords: [])
        let clean = ExerciseRef(pageID: again.pages[again.currentPage].id, index: 0)
        again.submitDiary(clean, entry: DiaryEntry(prompt: "And now?", text: "Кот тут.", submitted: again.now()))
        again.finishDiary(clean, correction: DiaryCorrection(corrected: "Кот тут.", edits: [], praise: "Yes.", teach: [], at: again.now()))
        XCTAssertEqual(again.result(clean)?.status, .correct)
        again.record(ExerciseRef(pageID: clean.pageID, index: 1)) { $0.status = .skipped }
        XCTAssertFalse(EventLog(url: again.knowledge.logURL).load().contains { $0.item == "\(clean.pageID)#1" })
    }

    /// A reply whose corrected text differs from the line but whose edits are missing, or do not
    /// apply to what was written, is shown but grades nothing: no word of the tutor's replacement
    /// text is credited as produced, nothing joins the learner's words, and the page mark says so.
    func testAnInconsistentCorrectionGradesNothing() throws {
        let book = Book(course: try course(), store: tempStore())
        var ex = Exercise(kind: .diary); ex.prompt = "Who is at home?"
        book.insertWrittenPage(title: "Дневник", intro: nil, unitTitle: "My Russian", exercises: [ex, ex, ex], newWords: [])
        let pageID = book.pages[book.currentPage].id
        // Changed text, no edits.
        let a = ExerciseRef(pageID: pageID, index: 0)
        book.submitDiary(a, entry: DiaryEntry(prompt: ex.prompt!, text: "Мама дом.", submitted: book.now()))
        book.finishDiary(a, correction: DiaryCorrection(corrected: "Ма́ма до́ма.", edits: [], praise: "", teach: [], at: book.now()))
        XCTAssertEqual(book.result(a)?.status, .done, "not 'right'")
        XCTAssertEqual(book.result(a)?.diary?.correction?.verified, false)
        let doma = book.knowledge.domain.productionComponents("дома").first!
        XCTAssertNil(book.knowledge.model.trace(doma, .produce), "«дома» was never written")
        XCTAssertNil(book.knowledge.model.trace(ComponentID(.lexeme, "мама"), .produce)?.reviews.last?.grade, "nothing graded")
        XCTAssertNil(book.targets["word:дома"])
        let main = book.knowledge.model.events.filter { $0.item == a.item && $0.type == .outcome }
        XCTAssertEqual(main.count, 1)
        XCTAssertNil(main.first?.grade)
        XCTAssertEqual(main.first?.help, ["unverified-correction"])
        // Edits that name words the learner never wrote.
        let b = ExerciseRef(pageID: pageID, index: 1)
        book.submitDiary(b, entry: DiaryEntry(prompt: ex.prompt!, text: "Папа тут.", submitted: book.now()))
        book.finishDiary(b, correction: DiaryCorrection(corrected: "Па́па до́ма.",
                                                         edits: [DiaryEdit(span: "здесь", replacement: "до́ма", category: .wordChoice, why: "", en: "at home")],
                                                         praise: "", teach: [], at: book.now()))
        XCTAssertEqual(book.result(b)?.diary?.correction?.verified, false)
        XCTAssertFalse(book.knowledge.model.events.contains { $0.item == b.item && $0.grade != nil })
        // A consistent correction, stress and punctuation aside, is verified and graded as before.
        let c = ExerciseRef(pageID: pageID, index: 2)
        book.submitDiary(c, entry: DiaryEntry(prompt: ex.prompt!, text: "мама дом", submitted: book.now()))
        book.finishDiary(c, correction: DiaryCorrection(corrected: "Ма́ма до́ма.",
                                                         edits: [DiaryEdit(span: "дом", replacement: "до́ма", category: .ending, why: "", en: "at home")],
                                                         praise: "", teach: [], at: book.now()))
        XCTAssertEqual(book.result(c)?.diary?.correction?.verified, true)
        XCTAssertEqual(book.result(c)?.status, .done)
        XCTAssertEqual(book.knowledge.model.trace(doma, .produce)?.reviews.map(\.grade), [.again])
        XCTAssertNotNil(book.targets["word:дома"])
        // The page: one line verified with one slip, two unverified → assessed by the one, no miss.
        let page = book.pages[book.currentPage]
        XCTAssertTrue(page.isFinished)
        XCTAssertEqual(GradeStamp.grade(for: page), 5)
        var fourSlips = page
        var corr = fourSlips.results[2].diary!.correction!
        corr.edits = Array(repeating: corr.edits[0], count: 4)
        fourSlips.results[2].diary!.correction = corr
        XCTAssertEqual(GradeStamp.grade(for: fourSlips), 4, "many corrections on the only assessed line: a miss")
        var nothingAssessed = page
        nothingAssessed.results[2].diary!.correction!.verified = false
        XCTAssertNil(GradeStamp.grade(for: nothingAssessed))
        XCTAssertEqual(GradeStamp.mark(for: nothingAssessed), "✓")
        XCTAssertEqual(GradeStamp.praise(for: nothingAssessed), "Готово!")
        // A page finished by the learner's own judgement alone is complete, not graded.
        var said = Exercise(kind: .readAloud); said.text = "кот"
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [said], newWords: [])
        book.record(ExerciseRef(pageID: book.page!.id, index: 0)) { $0.status = .done; $0.selfAssessed = true }
        XCTAssertNil(GradeStamp.grade(for: book.page!))
        // A correction from before the check existed reads as consistent.
        let data = try JSONEncoder().encode(DiaryCorrection(corrected: "x", edits: [], praise: "", teach: [], at: book.now()))
        var object = try XCTUnwrap(try JSONSerialization.jsonObject(with: data) as? [String: Any])
        object.removeValue(forKey: "verified")
        let decoded = try JSONDecoder().decode(DiaryCorrection.self, from: JSONSerialization.data(withJSONObject: object))
        XCTAssertNil(decoded.verified)
    }

    /// A diary line's evidence is several lines of the log (one per edit, then the main outcome).
    /// A write that stopped after the first edit is completed edit by edit on the next opening:
    /// the lines already there are not written again, the missing ones are, and only once.
    func testAPartlyWrittenDiaryIsRepairedEditByEdit() throws {
        let store = tempStore()
        let book = Book(course: try course(), store: store)
        var ex = Exercise(kind: .diary); ex.prompt = "What is on the table?"
        book.insertWrittenPage(title: "Дневник", intro: nil, unitTitle: "My Russian", exercises: [ex], newWords: [])
        let ref = ExerciseRef(pageID: book.pages[book.currentPage].id, index: 0)
        book.submitDiary(ref, entry: DiaryEntry(prompt: ex.prompt!, text: "На стол книга и чашка.", submitted: book.now()))
        let correction = DiaryCorrection(corrected: "На столе́ кни́га и ча́шка.",
                                         edits: [DiaryEdit(span: "стол", replacement: "столе́", category: .ending, why: "location", en: "on the table"),
                                                 DiaryEdit(span: "чашка", replacement: "ча́шка", category: .stress, why: "stress", en: "cup")],
                                         praise: "Good.", teach: [], at: book.now())
        book.finishDiary(ref, correction: correction)
        book.saveNow()
        let log = EventLog(url: book.knowledge.logURL)
        let all = log.load()
        let mine = all.filter { $0.item == ref.item }.map(\.id)
        XCTAssertEqual(mine, ["diary:\(ref.item)#0:0", "diary:\(ref.item)#0:1", "out:\(ref.item)#0"])
        // The write stopped after the first edit.
        let kept = all.filter { $0.item != ref.item || $0.id == "diary:\(ref.item)#0:0" }
        try FileManager.default.removeItem(at: log.url)
        XCTAssertTrue(log.append(kept))
        let again = Book(course: try course(), store: store)
        let repaired = EventLog(url: again.knowledge.logURL).load().filter { $0.item == ref.item }
        XCTAssertEqual(repaired.map(\.id).sorted(), mine.sorted(), "\(repaired.map(\.id))")
        XCTAssertEqual(repaired.filter { $0.id == "diary:\(ref.item)#0:0" }.first?.source, "live", "what was there stays as it was")
        XCTAssertEqual(repaired.filter { $0.id == "out:\(ref.item)#0" }.first?.source, "backfill")
        XCTAssertEqual(repaired.filter { $0.type == .outcome && $0.grade == .again }.count, 2, "both edits count against their forms")
        XCTAssertEqual(Book(course: try course(), store: store).knowledge.backfill(pages: again.pages), 0, "nothing left to repair")
    }

    /// A result from before the diary existed decodes, and so does one that a diary was crashed
    /// out of (the entry saved, the outcome never logged): the next launch logs it once.
    func testOldAndInterruptedNotebooksDecode() throws {
        let store = tempStore()
        try Data(KnowledgeTests.oldNotebook.utf8).write(to: store.stateURL)
        guard case .success(let old) = Book.open(course: try course(), store: store) else { return XCTFail("old notebook refused") }
        XCTAssertNil(old.pages[0].results[0].diary)

        let store2 = tempStore()
        let json = """
        {"version":2,"courseCursor":0,"currentPage":0,"memories":[],
         "settings":{"fingerDrawing":false,"cursiveModels":false,"showStress":true},
         "pages":[{"id":"wdiary01","number":1,"unitTitle":"My Russian","title":"Дневник","isReview":false,"created":"2026-11-01T08:50:00Z",
           "finished":"2026-11-01T09:05:00Z",
           "exercises":[{"kind":"diary","prompt":"Who is at home?"}],
           "results":[{"status":"done","attempts":1,"revealed":false,"updated":"2026-11-01T09:00:00Z",
                       "diary":{"prompt":"Who is at home?","text":"Мама дома. Кот тоже.","submitted":"2026-11-01T09:00:00Z",
                                "correction":{"corrected":"Ма́ма до́ма. Кот то́же.","edits":[],"praise":"Yes!","teach":[],"at":"2026-11-01T09:00:30Z"}}}]}]}
        """
        try Data(json.utf8).write(to: store2.stateURL)
        guard case .success(let book) = Book.open(course: try course(), store: store2) else { return XCTFail("diary notebook refused") }
        XCTAssertEqual(book.diaryEntries.count, 1)
        let logged = EventLog(url: book.knowledge.logURL).load()
        XCTAssertEqual(logged.map(\.type), [.outcome], "the missing outcome is reconstructed once")
        XCTAssertEqual(logged.first?.source, "backfill")
        XCTAssertEqual(logged.first?.modality, .produce)
        XCTAssertEqual(Book(course: try course(), store: store2).knowledge.backfill(pages: book.pages), 0)
    }

    /// One live correction through the app's own client (runs only with TEST_RUNNER_DRUSSKY_TUTOR_KEY).
    func testDiaryTutorCorrectsLive() async throws {
        let key = ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"] ?? ""
        try XCTSkipIf(key.isEmpty, "no tutor key supplied")
        let previous = Keychain.apiKey
        Keychain.apiKey = key
        defer { Keychain.apiKey = previous }
        let book = Book(course: try course(), store: tempStore())
        for p in 0..<6 { QA.complete(book, page: p, upTo: .max); book.turnForward() }
        let c = try await DiaryTutor.correct(text: "Я читаю книга. Мой сестра тоже читает.", prompt: "What are you reading?", book: book)
        print("DIARY:", c.corrected, "|", c.edits.map { "\($0.span)→\($0.replacement) [\($0.category.rawValue)]" }, "|", c.praise, "|", c.teach.map(\.title))
        XCTAssertTrue(RussianText.key(c.corrected).contains("книгу"), c.corrected)
        XCTAssertTrue(c.edits.contains { RussianText.key($0.span) == "книга" }, "\(c.edits)")
        XCTAssertTrue(c.edits.contains { RussianText.key($0.span).contains("мой") }, "\(c.edits)")
        XCTAssertFalse(c.praise.isEmpty)
        XCTAssertFalse(c.unplaced(in: "Я читаю книга. Мой сестра тоже читает.").count == c.edits.count, "spans are verbatim")
    }
}
