import PencilKit
import XCTest
@testable import Drussky

@MainActor
final class BaselineTests: XCTestCase {
    private func book() throws -> Book {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        return Book(course: try JSONDecoder().decode(Course.self, from: Data(contentsOf: url)),
                    store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
    }

    func testBaselineHasFiveDistinctRecordedSpeakersAndNoTrainingAnswers() throws {
        let book = try book()
        XCTAssertTrue(book.openBaseline())
        let page = try XCTUnwrap(book.page)
        let tasks = page.exercises.compactMap(\.baseline)
        XCTAssertEqual(tasks.map(\.kind), [.reading] + Array(repeating: .listening, count: 5) + [.speaking, .writing])
        XCTAssertEqual(Set(tasks.compactMap(\.speaker)).count, 5)
        XCTAssertEqual(tasks.first?.questions.count, 4)
        for task in tasks.filter({ $0.kind == .listening }) {
            let file = try XCTUnwrap(task.audio)
            let root = try XCTUnwrap(Bundle.main.url(forResource: "Audio", withExtension: nil))
            XCTAssertTrue(FileManager.default.fileExists(atPath: root.appendingPathComponent(file).path), file)
            XCTAssertTrue(Library.load().sentences?.contains { $0.audio == file && $0.speaker == task.speaker } == true)
        }
        XCTAssertTrue(page.exercises.allSatisfy { $0.spoken == nil && $0.target == nil })
        XCTAssertTrue(book.openBaseline())
        XCTAssertEqual(book.page?.id, page.id, "resume rather than consume another unseen sample")
    }

    func testFirstResponsesAndOriginalArtifactsSurviveReloadWithoutKnowledgeCredit() async throws {
        let book = try book()
        XCTAssertTrue(book.openBaseline())
        let page = try XCTUnwrap(book.page)
        let ref = ExerciseRef(pageID: page.id, index: 0)
        let ink = PKDrawing().dataRepresentation()
        let inkURL = book.inkURL(page: page, exercise: 0)
        try ink.write(to: inkURL)
        let file = "\(page.id)-6-first.m4a"
        let voiceURL = book.voiceURL(file: file)
        try Data([1, 2, 3, 4]).write(to: voiceURL)
        book.record(ExerciseRef(pageID: page.id, index: 6)) { $0.recording = file }
        let evidenceBefore = book.knowledge.model.events.count
        var response = BaselineResponse(started: book.now(), text: "She finds the book.")
        response.helped = true
        XCTAssertTrue(book.sealBaseline(ref, response: response))
        response.text = "Overwritten"
        XCTAssertFalse(book.sealBaseline(ref, response: response))
        book.reopen(ref)
        XCTAssertTrue(book.result(ref)?.status.isComplete == true)
        XCTAssertEqual(book.knowledge.model.events.count, evidenceBefore)
        await book.flush()
        let reopened = Book(course: book.course, store: book.store)
        XCTAssertEqual(reopened.result(ref)?.baseline?.text, "She finds the book.")
        XCTAssertEqual(reopened.result(ref)?.baseline?.helped, true)
        XCTAssertEqual(try Data(contentsOf: inkURL), ink)
        XCTAssertEqual(try Data(contentsOf: voiceURL), Data([1, 2, 3, 4]))
        XCTAssertFalse(reopened.knowledge.model.events.contains { $0.item.hasPrefix(page.id) })
    }

    func testParallelReadingMaterialIsNeverReusedAsUnseen() throws {
        let book = try book()
        var readingIDs: [String] = []
        for _ in 0..<2 {
            XCTAssertTrue(book.openBaseline())
            let page = try XCTUnwrap(book.page)
            readingIDs.append(try XCTUnwrap(page.exercises.first?.baseline?.id))
            for i in page.exercises.indices {
                XCTAssertTrue(book.sealBaseline(.init(pageID: page.id, index: i), response: .init(started: book.now(), text: "sample")))
            }
        }
        XCTAssertEqual(Set(readingIDs).count, 2)
        XCTAssertFalse(book.openBaseline())
    }

    func testOldExerciseAndResultDecodeWithoutSampleFields() throws {
        let ex = try JSONDecoder().decode(Exercise.self, from: Data(#"{"kind":"dictation","text":"кот"}"#.utf8))
        XCTAssertNil(ex.baseline)
        let old = #"{"status":"open","attempts":0,"revealed":false}"#
        XCTAssertNil(try JSONDecoder().decode(ExerciseResult.self, from: Data(old.utf8)).baseline)
    }
}
