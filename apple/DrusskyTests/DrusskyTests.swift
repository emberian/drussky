import PencilKit
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
}
