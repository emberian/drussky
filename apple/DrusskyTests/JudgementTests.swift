import LearningCore
import XCTest
@testable import Drussky

/// Self-judgement first, recognizer second: the evidence mapping, the setting, the notebook.
@MainActor
final class JudgementTests: XCTestCase {
    func course() throws -> Course {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        return try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    func tempStore() -> BookStore {
        BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
    }

    // MARK: Verdict × reader → grade, confidence

    func testVerdictDecidesAndReaderModulates() {
        typealias A = SelfJudgement.Assessment
        XCTAssertEqual(SelfJudgement.assess(verdict: .right, readerAgreed: true, firstTry: true), A(grade: .good, confidence: 1, disagreement: false))
        XCTAssertEqual(SelfJudgement.assess(verdict: .right, readerAgreed: true, firstTry: false), A(grade: .hard, confidence: 1, disagreement: false))
        XCTAssertEqual(SelfJudgement.assess(verdict: .right, readerAgreed: nil, firstTry: true), A(grade: .good, confidence: 0.7, disagreement: false))
        XCTAssertEqual(SelfJudgement.assess(verdict: .right, readerAgreed: false, firstTry: true), A(grade: .hard, confidence: 0.4, disagreement: true),
                       "the reader's doubt costs one notch, never the mark")
        XCTAssertEqual(SelfJudgement.assess(verdict: .notQuite, readerAgreed: true, firstTry: true), A(grade: .again, confidence: 0.6, disagreement: true))
        XCTAssertEqual(SelfJudgement.assess(verdict: .notQuite, readerAgreed: nil, firstTry: true), A(grade: .again, confidence: 0.8, disagreement: false))
        XCTAssertEqual(SelfJudgement.assess(verdict: .notQuite, readerAgreed: false, firstTry: true), A(grade: .again, confidence: 1, disagreement: false))
        // Every verdict yields a grade: a self-judged recall is always evidence.
        for v in [SelfJudgement.Verdict.right, .notQuite] { for r in [true, false, nil] {
            XCTAssertEqual(SelfJudgement.assess(verdict: v, readerAgreed: r, firstTry: true).grade.isSuccess, v == .right)
        } }
    }

    func testDomainGradeReadsTheVerdict() {
        var ex = Exercise(kind: .translate)
        ex.answerText = "кот"
        var r = ExerciseResult()
        r.status = .correct; r.attempts = 1; r.judged = "right"; r.readerAgreed = true
        XCTAssertEqual(RussianDomain.grade(ex, r), .good)
        r.readerAgreed = false
        XCTAssertEqual(RussianDomain.grade(ex, r), .hard)
        r.readerAgreed = nil; r.failedChecks = 1
        XCTAssertEqual(RussianDomain.grade(ex, r), .hard, "an earlier failed check means not first time")
        var miss = ExerciseResult()
        miss.status = .missed; miss.revealed = true; miss.attempts = 1; miss.judged = "not-quite"; miss.readerAgreed = true
        XCTAssertEqual(RussianDomain.grade(ex, miss), .again)
        // The reader's own verdicts are unchanged.
        var reader = ExerciseResult()
        reader.status = .correct; reader.attempts = 1
        XCTAssertEqual(RussianDomain.grade(ex, reader), .good)
        reader.selfAssessed = true
        XCTAssertEqual(RussianDomain.grade(ex, reader), .hard)
        XCTAssertEqual(SelfJudgement.judge(reader), "self")
        XCTAssertEqual(SelfJudgement.confidence(reader), 0.4)
        reader.selfAssessed = nil
        XCTAssertEqual(SelfJudgement.judge(reader), "reader")
        XCTAssertNil(SelfJudgement.confidence(reader))
        XCTAssertNil(SelfJudgement.feedback(reader))
        XCTAssertEqual(SelfJudgement.feedback(r), "Right, by your judgement.")
        r.readerAgreed = false; r.recognized = "кат"
        XCTAssertTrue(SelfJudgement.feedback(r)!.contains("worth another look"))
    }

    func testHintNeverRules() {
        XCTAssertEqual(SelfJudgement.hint(nil, reading: true), "Reading…")
        XCTAssertNil(SelfJudgement.hint(nil, reading: false))
        XCTAssertEqual(SelfJudgement.hint(.init(reading: "кот", matched: true, revision: 1), reading: false), "The reader agrees.")
        XCTAssertEqual(SelfJudgement.hint(.init(reading: "кат", matched: false, revision: 1), reading: false), "The reader saw «кат». You decide.")
        XCTAssertEqual(SelfJudgement.hint(.init(reading: "", matched: false, revision: 1), reading: false), "The reader couldn't make it out — your call.")
    }

    // MARK: The setting

    func testSettingDefaultsToMyselfAndOldNotebooksDecode() throws {
        var s = Settings()
        XCTAssertEqual(s.writingJudge, .myself)
        XCTAssertNil(s.judge)
        s.writingJudge = .reader
        let data = try JSONEncoder().encode(s)
        XCTAssertTrue(String(decoding: data, as: UTF8.self).contains("\"judge\":\"reader\""))
        XCTAssertEqual(try JSONDecoder().decode(Settings.self, from: data).writingJudge, .reader)
        // A notebook from before the setting: no `judge` key at all.
        let old = try JSONDecoder().decode(Settings.self, from: Data("{\"fingerDrawing\":true,\"showStress\":false}".utf8))
        XCTAssertEqual(old.writingJudge, .myself)
        XCTAssertTrue(old.fingerDrawing)
        // A value from a later build reads as the default rather than failing the notebook.
        let future = try JSONDecoder().decode(Settings.self, from: Data("{\"judge\":\"oracle\"}".utf8))
        XCTAssertEqual(future.writingJudge, .myself)
        XCTAssertEqual(future.judge, "oracle", "kept as written")
    }

    func testSettingAndVerdictsSurviveReopeningTheNotebook() throws {
        let store = tempStore()
        let book = Book(course: try course(), store: store)
        book.settings.writingJudge = .reader
        QA.complete(book, page: 0, upTo: .max)
        book.turnForward()
        let page = book.currentPage
        let i = try XCTUnwrap(book.pages[page].exercises.firstIndex { $0.kind == .translate })
        book.record(page: page, exercise: i) { r in r.attempts += 1; r.judged = "right"; r.readerAgreed = false; r.recognized = "кат"; r.status = .correct }
        book.saveNow()
        let again = Book(course: try course(), store: store)
        XCTAssertEqual(again.settings.writingJudge, .reader)
        let r = again.pages[page].results[i]
        XCTAssertEqual(r.judged, "right")
        XCTAssertEqual(r.readerAgreed, false)
        // Reopening the exercise clears the verdict for the next round.
        again.reopen(page: page, exercise: i)
        XCTAssertNil(again.pages[page].results[i].judged)
        XCTAssertNil(again.pages[page].results[i].readerAgreed)
    }

    // MARK: Evidence

    func testSelfVerdictFeedsEvidenceWithJudgeAndConfidence() throws {
        let book = Book(course: try course(), store: tempStore())
        QA.complete(book, page: 0, upTo: .max)
        book.turnForward()
        let page = book.currentPage
        let i = try XCTUnwrap(book.pages[page].exercises.firstIndex { $0.kind == .translate })
        let ex = book.pages[page].exercises[i]
        let word = try XCTUnwrap(book.knowledge.domain.utteranceID(ex.target ?? ""))
        let before = book.knowledge.model.events.count

        // Right, reader disagrees: hard, confidence 0.4, judge self; the word is worth another look.
        book.record(page: page, exercise: i) { r in r.attempts += 1; r.judged = "right"; r.readerAgreed = false; r.recognized = "кат"; r.status = .correct }
        let e = try XCTUnwrap(book.knowledge.model.events.last)
        XCTAssertEqual(book.knowledge.model.events.count, before + 1)
        XCTAssertEqual(e.type, .outcome)
        XCTAssertEqual(e.grade, .hard)
        XCTAssertEqual(e.judge, "self")
        XCTAssertEqual(e.confidence, 0.4)
        XCTAssertEqual(e.response, "кат")
        XCTAssertNil(e.confusions, "a self-judged right answer accuses no letter")
        XCTAssertEqual(book.knowledge.readerDisagreement(word)?.id, e.id)
        XCTAssertTrue(book.knowledge.worthAnotherLook().contains(word))
        XCTAssertEqual(book.knowledge.model.trace(word, .meaningWrite)?.reviews.last?.grade, .hard)

        // Reopen and judge right with the reader agreeing: a second round is never "first time"
        // (attempts carry over), so hard — but sure of it, and no standing disagreement.
        book.reopen(page: page, exercise: i)
        book.record(page: page, exercise: i) { r in r.attempts += 1; r.judged = "right"; r.readerAgreed = true; r.recognized = "кот"; r.status = .correct }
        let e2 = try XCTUnwrap(book.knowledge.model.events.last)
        XCTAssertEqual(e2.grade, .hard)
        XCTAssertEqual(e2.confidence, 1)
        XCTAssertEqual(e2.judge, "self")
        XCTAssertFalse(book.knowledge.worthAnotherLook().contains(word))
        XCTAssertEqual(book.knowledge.model.trace(word, .meaningWrite)?.reviews.count, 2)

        // Not quite: again, whatever the reader thought; the log line carries both fields.
        let j = try XCTUnwrap(book.pages[page].exercises.indices.first { $0 != i && [.dictation, .fill].contains(book.pages[page].exercises[$0].kind) })
        book.record(page: page, exercise: j) { r in r.attempts += 1; r.judged = "not-quite"; r.readerAgreed = true; r.status = .missed; r.revealed = true }
        let e3 = try XCTUnwrap(book.knowledge.model.events.last)
        XCTAssertEqual(e3.grade, .again)
        XCTAssertEqual(e3.judge, "self")
        XCTAssertEqual(e3.confidence, 0.6)
        let line = try XCTUnwrap(String(data: try EventLog.encoder().encode(e3), encoding: .utf8))
        XCTAssertTrue(line.contains("\"judge\":\"self\""), line)
        XCTAssertTrue(line.contains("\"confidence\":0.6"), line)
        // The log on disk reads back with the new fields.
        XCTAssertEqual(EventLog(url: book.knowledge.logURL).load().last?.confidence, 0.6)
    }

    func testReaderJudgedRecallIsMarkedReader() throws {
        let book = Book(course: try course(), store: tempStore())
        QA.complete(book, page: 0, upTo: .max)
        book.turnForward()
        let page = book.currentPage
        let i = try XCTUnwrap(book.pages[page].exercises.firstIndex { $0.kind == .translate })
        book.record(page: page, exercise: i) { r in r.attempts += 1; r.recognized = "кот"; r.status = .correct }
        let e = try XCTUnwrap(book.knowledge.model.events.last)
        XCTAssertEqual(e.judge, "reader")
        XCTAssertNil(e.confidence)
        XCTAssertEqual(e.grade, .good)
        // Copying is nobody's judgement call.
        let c = try XCTUnwrap(book.pages[page].exercises.firstIndex { $0.kind == .copy || $0.kind == .trace })
        book.record(page: page, exercise: c) { r in r.status = .done }
        XCTAssertNil(book.knowledge.model.events.last?.judge)
    }
}
