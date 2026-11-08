import LearningCore
import XCTest
@testable import Drussky

@MainActor
final class ConstructionChoiceTests: XCTestCase {
    func testChoicesFeedTheirOwnTraceAndScheduleNewContexts() throws {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        let book = Book(course: try JSONDecoder().decode(Course.self, from: Data(contentsOf: url)),
                        store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        let start = Date(timeIntervalSince1970: 1_800_000_000)
        book.now = { start }
        var taught = Exercise(kind: .dictation); taught.text = "кни́га"
        book.introduce(taught)
        let choices = ConstructionChoice.bank.filter { !ConstructionChoice.reservedForProbe($0) }
        book.insertWrittenPage(title: "Choice", intro: nil, unitTitle: "Test", exercises: Array(choices.prefix(3)), newWords: [], sourceID: ConstructionChoice.key)
        for i in 0..<3 {
            book.record(page: book.currentPage, exercise: i) { $0.chosen = book.page?.exercises[i].answerIndex; $0.attempts = 1; $0.status = .correct }
        }
        XCTAssertEqual(book.knowledge.model.trace(ConstructionChoice.component, .constructionChoice)?.reviews.count, 3)
        XCTAssertNil(book.knowledge.model.trace(ConstructionChoice.component, .meaningWrite))
        XCTAssertNil(book.knowledge.model.trace(ComponentID(.form, "книгу"), .meaningWrite))
        let probes = book.day.probes.probes.filter { $0.id.hasPrefix("construction-transfer:") }
        XCTAssertEqual(probes.map { Int($0.due.timeIntervalSince($0.drawn) / 86_400) }, [7, 30])
        XCTAssertEqual(book.reviewModalities(for: ConstructionChoice.key), [.constructionChoice])
        book.now = { start.addingTimeInterval(7 * 86_400) }
        let used = Set(book.pages.flatMap { $0.exercises.compactMap(\.word) })
        let review = book.reviewExercise(for: ConstructionChoice.key, modality: .constructionChoice, variant: 0)
        XCTAssertFalse(used.contains(try XCTUnwrap(review.word)))
        XCTAssertFalse(ConstructionChoice.reservedForProbe(review))
        let transfer = book.probeExercises(pageID: "new-page", among: [])
        let probe = try XCTUnwrap(transfer.first { $0.kind == .constructionChoice })
        XCTAssertFalse(used.contains(try XCTUnwrap(probe.word)))
        XCTAssertNil(probe.spoken, "these are visual decisions, without a missing-audio dependency")
    }

    func testFirstChoiceEndsTheTaskWithoutRetryCredit() {
        let ex = ConstructionChoice.bank[0]
        var wrong = ExerciseResult(); wrong.status = .missed; wrong.attempts = 1
        XCTAssertEqual(RussianDomain.grade(ex, wrong), .again)
        var right = ExerciseResult(); right.status = .correct; right.attempts = 1
        XCTAssertEqual(RussianDomain.grade(ex, right), .good)
        XCTAssertEqual(Set(ConstructionChoice.bank.compactMap(\.word)).count, ConstructionChoice.bank.count)
        XCTAssertTrue(ConstructionChoice.bank.allSatisfy { $0.options?.count == 3 && $0.answerIndex != nil })
        XCTAssertTrue(ConstructionChoice.bank.filter { $0.answerIndex == 1 }.allSatisfy { $0.en?.contains("(one)") == true },
                      "singular meaning must be stated: книги could otherwise be a valid plural object")
    }
}
