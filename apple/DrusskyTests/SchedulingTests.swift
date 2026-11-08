import LearningCore
import XCTest
@testable import Drussky

/// Review timing and fading come from LearningCore's FSRS traces, graded honestly.
@MainActor
final class SchedulingTests: XCTestCase {
    var course: Course {
        let url = Bundle.main.url(forResource: "course", withExtension: "json")!
        return try! JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    func newBook() -> Book {
        Book(course: course, store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
    }

    /// A one-exercise page reviewing `key` by dictation, answered with `update`.
    func review(_ book: Book, _ text: String, key: String, _ update: (inout ExerciseResult) -> Void) {
        var d = Exercise(kind: .dictation); d.text = text; d.reviewKey = key
        book.insertWrittenPage(title: "r", intro: nil, unitTitle: "r", exercises: [d], newWords: [])
        book.record(page: book.currentPage, exercise: 0, update)
    }

    func trace(_ book: Book, _ key: String) -> MemoryTrace? {
        book.knowledge.domain.reviewComponents(forTarget: key).first.flatMap { book.knowledge.model.trace($0, .hearWrite) }
    }

    /// The old scheduler sent a lapse at 180-day stability 72 days away. Now it comes back
    /// after the relearning step, and not before.
    func testALapseComesBackSoonWhateverTheStability() throws {
        let book = newBook()
        book.keep("кот", en: "cat")
        for _ in 0..<6 {
            let s = trace(book, "word:кот")?.state?.stability ?? 1
            book.advanceClock(byDays: s)
            review(book, "кот", key: "word:кот") { $0.status = .correct }
        }
        XCTAssertGreaterThan(try XCTUnwrap(trace(book, "word:кот")?.state?.stability), 60)
        XCTAssertFalse(book.dueReviews().contains { $0.id == "word:кот" })
        review(book, "кот", key: "word:кот") { $0.revealed = true; $0.status = .missed }
        book.advanceClock(byDays: 5.0 / 1440)
        XCTAssertFalse(book.knowledge.model.dueReviews(book.reviewCandidates(), at: book.now()).contains { $0.id == "word:кот" })
        book.advanceClock(byDays: 10.0 / 1440)
        let due = try XCTUnwrap(book.knowledge.model.dueReviews(book.reviewCandidates(), at: book.now()).first { $0.id == "word:кот" })
        XCTAssertTrue(due.relearning)
        XCTAssertEqual(due.modality, .hearWrite)
    }

    /// Leaving a review unanswered (skipping the page) changes nothing: it stays due.
    func testMovingOnDoesNotPostpone() throws {
        let book = newBook()
        book.keep("кот", en: "cat")
        book.advanceClock(byDays: 2)
        let first = try XCTUnwrap(book.dueReviews().first { $0.id == "word:кот" })
        var d = book.reviewExercise(for: first.id, modality: first.modality, variant: 0)
        d.reviewKey = first.id
        book.insertWrittenPage(title: "r", intro: nil, unitTitle: "r", exercises: [d], newWords: [])
        book.turnForward()
        book.turnForward()
        book.advanceClock(byDays: 1)
        XCTAssertTrue(book.knowledge.model.dueReviews(book.reviewCandidates(), at: book.now()).contains { $0.id == "word:кот" })
    }

    /// A failed check followed by an automatic success is not "right first time".
    func testRightFirstTimeOnlyWhenItWas() throws {
        let book = newBook()
        var d = Exercise(kind: .dictation); d.text = "кот"
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [d, d], newWords: [])
        let p = book.currentPage
        book.record(page: p, exercise: 0) { $0.attempts += 1; $0.failedChecks = 1; $0.recognized = "кат" }
        book.record(page: p, exercise: 0) { $0.status = .correct; $0.recognized = "кот" }
        XCTAssertFalse(book.pages[p].results[0].isRightFirstTime)
        XCTAssertEqual(RussianDomain.grade(d, book.pages[p].results[0]), .hard)
        book.record(page: p, exercise: 1) { $0.status = .correct }
        XCTAssertTrue(book.pages[p].results[1].isRightFirstTime)
        XCTAssertEqual(RussianDomain.grade(d, book.pages[p].results[1]), .good)
        // Right after being shown it, in a later round: practice, not recall.
        var shown = ExerciseResult(); shown.status = .correct; shown.assisted = true; shown.round = 1
        XCTAssertNil(RussianDomain.grade(d, shown))
        var pick = Exercise(kind: .listenPick); pick.say = "кот"; pick.options = ["кот", "рот"]
        var second = ExerciseResult(); second.status = .done; second.attempts = 2
        XCTAssertEqual(RussianDomain.grade(pick, second), .again, "the right pick after a wrong one")
    }

    /// Fading looks at recall now, not at lifetime counts: one old, recovered lapse doesn't
    /// block it, and strength that has decayed doesn't allow it.
    func testFadingFollowsPresentRecall() throws {
        let book = newBook()
        var copy = Exercise(kind: .copy); copy.text = "ма́ма"; copy.times = 2
        book.keep("ма́ма", en: "mom")
        let key = try XCTUnwrap(book.memoryKey(for: copy))
        book.advanceClock(byDays: 1)
        review(book, "ма́ма", key: key) { $0.revealed = true; $0.status = .missed }
        XCTAssertEqual(book.faded(copy).kind, .copy)
        for days in [0.01, 2.0, 6.0] {
            book.advanceClock(byDays: days)
            review(book, "ма́ма", key: key) { $0.status = .correct }
        }
        XCTAssertEqual(book.faded(copy).kind, .dictation, "secure now despite the early lapse")
        book.advanceClock(byDays: 400)
        XCTAssertEqual(book.faded(copy).kind, .copy, "a year later the support comes back")
    }

    /// A format-1 notebook: its memories become targets, its evidence schedules them, and it is
    /// saved as format 2 with the old memories kept.
    func testFormatOneNotebookMigrates() throws {
        let store = BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
        try Data(KnowledgeTests.oldNotebook.utf8).write(to: store.stateURL)
        let book = Book(course: course, store: store)
        XCTAssertEqual(book.targets["word:кот"]?.introduced, ISO8601DateFormatter().date(from: "2026-10-01T09:00:00Z"))
        // Missed in dictation (backfilled as "again"): due again, relearning.
        let due = try XCTUnwrap(book.dueReviews().first { $0.id == "word:кот" })
        XCTAssertTrue(due.relearning)
        book.saveNow()
        let json = try JSONSerialization.jsonObject(with: Data(contentsOf: store.stateURL)) as? [String: Any]
        XCTAssertEqual(json?["version"] as? Int, 2)
        XCTAssertEqual((json?["memories"] as? [Any])?.count, 1, "format-1 memories are written back unchanged")
        XCTAssertEqual((json?["targets"] as? [Any])?.count, 1)
        let again = Book(course: course, store: store)
        XCTAssertEqual(again.targets.keys.sorted(), book.targets.keys.sorted())
    }

    /// Review exercises test the modality that is due.
    func testReviewExerciseMatchesTheDueModality() throws {
        let book = newBook()
        book.keep("кот", en: "cat")
        let key = "word:кот"
        XCTAssertEqual(book.reviewExercise(for: key, modality: .hearWrite, variant: 0).kind, .dictation)
        XCTAssertEqual(book.reviewExercise(for: key, modality: .say, variant: 0).kind, .readAloud)
        let pick = book.reviewExercise(for: key, modality: .hearPick, variant: 0)
        XCTAssertEqual(pick.kind, .listenPick)
        XCTAssertTrue(pick.options?.contains(pick.say ?? "") ?? false)
        XCTAssertEqual(pick.options?.count, 3)
        let meaning = book.reviewExercise(for: key, modality: .readPick, variant: 0)
        XCTAssertEqual(meaning.kind, .readMeaning)
        XCTAssertEqual(meaning.answerIndex.flatMap { meaning.options?[$0] }, "cat")
        for m in book.reviewModalities(for: key) {
            let ex = book.reviewExercise(for: key, modality: m, variant: 0)
            XCTAssertEqual(RussianDomain.modality(for: ex.kind), m, "a review of \(m) is graded in \(m)")
        }
    }

    /// Prepositions, conjunctions and particles are met inside phrases. Known on their own, they
    /// are only ever recognised by meaning: no "write it in Russian: about", no dictation of «о»,
    /// no "say it: с"; phrases that contain them keep every modality.
    func testFunctionWordsAreNeverRecallItemsOnTheirOwn() throws {
        for w in ["о", "с", "в", "у", "и", "а", "не", "же", "бы", "на", "Об", "к", "ни", "ли", "я"] { XCTAssertTrue(RussianText.isFunctionWord(w), w) }
        for w in ["да", "нет", "кот", "до за́втра", "А у тебя́?", ""] { XCTAssertFalse(RussianText.isFunctionWord(w), w) }
        let book = newBook()
        let words = ["о", "с", "в", "у", "и", "а", "не", "же", "бы", "на"]
        for w in words { book.keep(w, en: "") }   // all in the lexicon: their glosses come from there
        book.keep("до за́втра", en: "see you tomorrow")
        for w in words {
            let key = "word:\(w)"
            XCTAssertEqual(book.reviewModalities(for: key), [.readPick], w)
            for v in 0..<3 {
                let ex = book.reviewExercise(for: key, modality: nil, variant: v)
                XCTAssertEqual(ex.kind, .readMeaning, "\(w), first recall \(v)")
                XCTAssertEqual(ex.answerIndex.flatMap { ex.options?[$0] }, book.word(w)?.en)
            }
        }
        let phrase = book.reviewModalities(for: "word:до завтра")
        XCTAssertTrue(phrase.isSuperset(of: [.hearWrite, .say]), "a phrase with a preposition stays a full recall item: \(phrase)")
        // Due as first recalls two days on: every one of them comes back as recognition.
        book.advanceClock(byDays: 2)
        let due = book.dueReviews().filter { words.contains(String($0.id.dropFirst(5))) }
        XCTAssertEqual(due.count, words.count, "\(due.map(\.id))")
        for d in due {
            XCTAssertEqual(book.reviewExercise(for: d.id, modality: d.modality, variant: 0).kind, .readMeaning, d.id)
        }
        for c in book.reviewCandidates() where words.contains(String(c.id.dropFirst(5))) {
            XCTAssertEqual(c.modalities, [.readPick], c.id)
        }
        // A kept function word with no gloss anywhere is nothing to review at all.
        book.keep("ж", en: "")
        XCTAssertEqual(book.reviewModalities(for: "word:ж"), [])
        XCTAssertFalse(book.reviewCandidates().contains { $0.id == "word:ж" })
    }

    func testGrammarCellsAndRootsEnterReviewScheduleAndWordsMetFollowsKnowledgeModel() throws {
        let book = newBook()
        // Recognition-only evidence in KnowledgeModel (not in book.targets) still counts in wordsMetCount.
        var rm = Exercise(kind: .readMeaning)
        rm.text = "кни́га"; rm.options = ["book", "table"]; rm.answerIndex = 0
        var tf = Exercise(kind: .transform)
        tf.word = "кни́га"; tf.paradigm = "decl1"; tf.cue = "gen-sg"; tf.answerText = "кни́ги"
        var bw = Exercise(kind: .buildWord)
        bw.en = "a writer"; bw.answerText = "писа́тель"; bw.parts = ["пис", "-а́-", "-тель"]; bw.options = bw.parts
        book.insertWrittenPage(title: "g", intro: nil, unitTitle: "g", exercises: [rm, tf, bw], newWords: [])
        let p = book.currentPage
        book.record(page: p, exercise: 0) { $0.attempts = 1; $0.chosen = 0; $0.status = .correct }
        XCTAssertNil(book.targets["word:книга"])
        XCTAssertGreaterThanOrEqual(book.wordsMetCount, 1, "wordsMetCount includes words with evidence in KnowledgeModel")

        // Practising a grammar transform and a root buildWord schedules gram:decl1-gen-sg and root:пис.
        book.record(page: p, exercise: 1) { $0.attempts = 1; $0.revealed = true; $0.status = .missed }
        book.record(page: p, exercise: 2) { $0.attempts = 1; $0.recognized = "писатель"; $0.status = .correct }
        XCTAssertTrue(book.reviewCandidates().contains { $0.id == "gram:decl1-gen-sg" })
        XCTAssertTrue(book.reviewCandidates().contains { $0.id == "root:пис" })

        book.advanceClock(byDays: 10)
        let dueIDs = Set(book.dueReviews().map(\.id))
        XCTAssertTrue(dueIDs.contains("gram:decl1-gen-sg"), "due grammar cell is included in dueReviews/dueCount")
        XCTAssertTrue(dueIDs.contains("root:пис"), "due root is included in dueReviews/dueCount")
        XCTAssertGreaterThanOrEqual(book.dueCount, 2)

        let gramEx = book.reviewExercise(for: "gram:decl1-gen-sg", modality: .transform, variant: 0)
        XCTAssertEqual(gramEx.kind, .transform)
        XCTAssertEqual(gramEx.reviewKey, "gram:decl1-gen-sg")
        XCTAssertTrue( book.knowledge.domain.components(for: gramEx).contains(ComponentID(.construction, "decl1-gen-sg")) )

        let classifyEx = book.reviewExercise(for: "gram:decl1-gen-sg", modality: .classify, variant: 0)
        XCTAssertEqual(classifyEx.kind, .paradigmFill)
        XCTAssertNotNil(classifyEx.options)

        let rootEx = book.reviewExercise(for: "root:пис", modality: .build, variant: 0)
        XCTAssertEqual(rootEx.kind, .buildWord)
        XCTAssertEqual(rootEx.reviewKey, "root:пис")
        XCTAssertTrue( book.knowledge.domain.components(for: rootEx).contains(ComponentID(.root, "пис")) )

        // Weakest words ranks a lapsed word ahead of a strong word.
        book.keep("стол", en: "table")
        review(book, "стол", key: "word:стол") { $0.attempts = 1; $0.status = .correct }
        review(book, "кни́га", key: "word:книга") { $0.attempts = 1; $0.status = .correct }
        book.advanceClock(byDays: 1)
        review(book, "кни́га", key: "word:книга") { $0.attempts = 1; $0.revealed = true; $0.status = .missed }
        let weakest = book.weakestWords(limit: 4).map { RussianText.key($0.ru) }
        let idxKniga = try XCTUnwrap(weakest.firstIndex(of: "книга"))
        let idxStol = try XCTUnwrap(weakest.firstIndex(of: "стол"))
        XCTAssertLessThan(idxKniga, idxStol, "the relearning word comes ahead of the strong word in weakestWords")
    }
}
