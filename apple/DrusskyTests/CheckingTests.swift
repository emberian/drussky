import LearningCore
import XCTest
@testable import Drussky

/// What counts as right, which recording plays, and outcomes that aren't graded.
@MainActor
final class CheckingTests: XCTestCase {
    var course: Course {
        let url = Bundle.main.url(forResource: "course", withExtension: "json")!
        return try! JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    // MARK: Written answers

    func testRepetitionMustBeTheWholeTargetInOrder() {
        XCTAssertTrue(RussianText.isRepetition("Аа Аа", of: "А"))
        XCTAssertTrue(RussianText.isRepetition("мама мама", of: "ма́ма"))
        XCTAssertTrue(RussianText.isRepetition("кот", of: "кот"))
        XCTAssertFalse(RussianText.isRepetition("ам", of: "мама"), "part of it")
        XCTAssertFalse(RussianText.isRepetition("ток", of: "кот"), "rearranged")
        XCTAssertFalse(RussianText.isRepetition("мамам", of: "мама"), "an extra letter")
        XCTAssertFalse(RussianText.isRepetition("амма", of: "мама"))
        XCTAssertFalse(RussianText.isRepetition("Ма", of: "А"))
    }

    func testSubjectPronounLeniencyOnlyWhereRussianDropsIt() {
        // Translating "I am reading": with or without я.
        XCTAssertTrue(RussianText.answerMatches("читаю", "Я чита́ю.", subjectDrop: true))
        XCTAssertTrue(RussianText.answerMatches("я не знаю", "не зна́ю", subjectDrop: true))
        // Not a verb, or a pronoun that doesn't agree: wrong.
        XCTAssertFalse(RussianText.answerMatches("ты кот", "кот", subjectDrop: true))
        XCTAssertFalse(RussianText.answerMatches("кот", "Ты кот.", subjectDrop: true))
        XCTAssertFalse(RussianText.answerMatches("ты читаю", "чита́ю", subjectDrop: true))
        XCTAssertFalse(RussianText.answerMatches("он читаю", "Я чита́ю.", subjectDrop: true))
        // Dictation reproduces the utterance: nothing dropped or added.
        XCTAssertFalse(RussianText.answerMatches("читаю", "Я чита́ю."))
        XCTAssertFalse(RussianText.answerMatches("я читаю", "чита́ю"))
        XCTAssertFalse(RussianText.answerMatches("ты кот", "кот"))
        XCTAssertTrue(RussianText.answerMatches("Я читаю", "Я чита́ю."))
    }

    func testConfusionsUseTheSameLeniency() {
        let d = RussianDomain(course: course, lemmas: Lemmas.load(), pairs: SoundPairs.load())
        var dictation = Exercise(kind: .dictation); dictation.text = "Я чита́ю."
        var translate = Exercise(kind: .translate); translate.en = "I am reading"; translate.answerText = "Я чита́ю."
        XCTAssertTrue(d.confusions(translate, response: "читаю").isEmpty, "a right translation has no confusion")
        _ = d.confusions(dictation, response: "читаю")  // wrong, but no word substitution to report: must not trap
    }

    // MARK: Audio identity

    func testQuestionAndStatementAreDifferentRecordings() throws {
        XCTAssertEqual(RussianText.audioKey("Э́то чай?"), "это чай?")
        XCTAssertEqual(RussianText.audioKey("Э́то чай."), "это чай")
        XCTAssertEqual(RussianText.audioKey("Где кот?"), "где кот", "a question-word question falls like a statement")
        XCTAssertEqual(RussianText.audioKey("Ты зна́ешь, где соль?"), "ты знаешь где соль?", "yes/no with an embedded question")
        XCTAssertEqual(RussianText.audioKey("О чём ты?"), "о чём ты")
        XCTAssertEqual(RussianText.audioKey("Приве́т! Как дела́?"), "привет как дела")
        let voice = Voice()
        let q = try XCTUnwrap(voice.recordingURL(for: "Э́то чай?"))
        let s = try XCTUnwrap(voice.recordingURL(for: "Э́то чай."))
        XCTAssertNotEqual(q, s)
        let sources = try JSONSerialization.jsonObject(with: Data(contentsOf: q.deletingLastPathComponent().appendingPathComponent("sources.json"))) as? [String: [String: Any]]
        XCTAssertEqual((sources?[q.lastPathComponent]?["input"] as? String)?.last, "?", "the question clip was made from a question")
        XCTAssertEqual((sources?[s.lastPathComponent]?["input"] as? String)?.last, ".")
    }

    /// Every clip's key is what its own provenance says it says (no statement filed as a question
    /// or the reverse).
    func testEveryClipIsFiledUnderItsOwnIntonation() throws {
        let dir = try XCTUnwrap(Bundle.main.url(forResource: "Audio", withExtension: nil))
        let index = try JSONDecoder().decode([String: String].self, from: Data(contentsOf: dir.appendingPathComponent("index.json")))
        let sources = try JSONSerialization.jsonObject(with: Data(contentsOf: dir.appendingPathComponent("sources.json"))) as! [String: [String: Any]]
        var wrong: [String] = []
        for (k, f) in index {
            guard let s = sources[f], let text = (s["input"] ?? s["text"]) as? String else { continue }
            if k.hasSuffix("?") != RussianText.isRisingQuestion(text) { wrong.append("\(k) ← \(text)") }
        }
        XCTAssertTrue(wrong.isEmpty, "\(wrong.prefix(10))")
    }

    // MARK: Ungraded outcomes

    func testSkippingIsFinishedButNeverGraded() throws {
        let book = Book(course: course, store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        var ex = Exercise(kind: .readAloud); ex.text = "кот"
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [ex, ex], newWords: [])
        let p = book.currentPage
        book.record(page: p, exercise: 0) { $0.status = .skipped }
        XCTAssertTrue(book.pages[p].results[0].status.isComplete)
        XCTAssertNil(RussianDomain.grade(ex, book.pages[p].results[0]))
        XCTAssertNil(book.knowledge.model.trace(ComponentID(.lexeme, "кот"), .say), "a skip moves no memory")
        book.record(page: p, exercise: 1) { $0.attempts = 1; $0.status = .done; $0.selfAssessed = true }
        XCTAssertEqual(book.knowledge.model.trace(ComponentID(.lexeme, "кот"), .say)?.reviews.last?.grade, .hard,
                       "judged right by the learner: counted, but not as verified")
        XCTAssertFalse(book.pages[p].results[1].isRightFirstTime)
        // Skipped and self-assessed: the page is complete, and nothing on it was assessed.
        XCTAssertTrue(book.pages[p].isFinished)
        XCTAssertNil(GradeStamp.grade(for: book.pages[p]))
        XCTAssertEqual(GradeStamp.mark(for: book.pages[p]), "✓")
    }

    func testTryingAMissedChoiceAgainIsPractice() throws {
        let book = Book(course: course, store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        var pick = Exercise(kind: .listenPick); pick.say = "кот"; pick.options = ["кот", "рот"]
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [pick], newWords: [])
        let ref = ExerciseRef(pageID: book.pages[book.currentPage].id, index: 0)
        book.record(ref) { $0.chosen = 1; $0.attempts += 1 }
        book.record(ref) { $0.chosen = 1; $0.attempts += 1; $0.status = .missed }
        let before = book.knowledge.model.trace(ComponentID(.lexeme, "кот"), .hearPick)?.reviews.count
        book.reopen(ref)
        book.record(ref) { $0.chosen = 0; $0.attempts += 1; $0.status = .correct }
        XCTAssertEqual(book.knowledge.model.trace(ComponentID(.lexeme, "кот"), .hearPick)?.reviews.count, before)
    }

    // MARK: Playback sequences

    func testASequenceEndsWhenAnythingElsePlays() throws {
        let voice = Voice()
        let a = try XCTUnwrap(voice.recordingURL(for: "кот"))
        let token = voice.playFile(a, key: "a")
        XCTAssertTrue(voice.continueSequence(a, key: "a", token: token))
        voice.play("мама")  // the next trial
        XCTAssertFalse(voice.continueSequence(a, key: "a", token: token), "a later playback ends the comparison")
        let second = voice.playFile(a, key: "a")
        voice.stop()  // a page turn
        XCTAssertFalse(voice.continueSequence(a, key: "a", token: second))
    }
}
