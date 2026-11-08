import LearningCore
import PDFKit
import XCTest
@testable import Drussky

/// Device-shaped features (Drussky/Devices): the notebook on paper, the pocket page for the
/// phone, and typing Russian on a hardware keyboard.
@MainActor
final class DevicesTests: XCTestCase {
    var course: Course {
        let url = Bundle.main.url(forResource: "course", withExtension: "json")!
        return try! JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    func tempStore() -> BookStore {
        BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
    }

    /// A book with `n` pages worked through (synthetic ink on every writing exercise).
    func studied(_ n: Int) -> Book {
        let book = Book(course: course, store: tempStore())
        for p in 0..<n {
            QA.complete(book, page: p, upTo: .max)
            book.turnForward()
        }
        return book
    }

    // MARK: PDF

    func testPaginateKeepsBlocksWholeAndInOrder() {
        // 100 fits with 100 (gap 10 → 210 ≤ 220); the 50 would overflow; the 300 gets its own sheet.
        let sheets = NotebookPDF.paginate([100, 100, 50, 300, 20], usable: 220, gap: 10)
        XCTAssertEqual(sheets, [[0, 1], [2], [3], [4]])
        XCTAssertEqual(NotebookPDF.paginate([], usable: 220), [])
        XCTAssertEqual(NotebookPDF.paginate([500], usable: 220), [[0]], "a block taller than a sheet still gets one")
        XCTAssertEqual(NotebookPDF.paginate([60, 60, 60], usable: 200, gap: 10).count, 1)
    }

    func testNotebookPDFHasTheRightSheetsAndCarriesTheInk() throws {
        let book = studied(2)
        XCTAssertEqual(book.pages.count, 3)
        let paper = NotebookPDF.Paper.a4
        let contentX = paper.margin + NotebookPDF.marginColumn + 14
        let width = paper.size.width - contentX - paper.margin
        let usable = paper.size.height - 2 * paper.margin - 16
        var expectedSheets = 0
        var inkBlocks = 0
        for page in book.pages {
            let blocks = NotebookPDF.blocks(for: page, book: book, width: width, originX: contentX)
            XCTAssertEqual(blocks.count, page.exercises.count + 2 + ((page.notes?.isEmpty ?? true) ? 0 : 1), "header, one per exercise, footer")
            XCTAssertEqual(blocks.first?.kind, .header)
            XCTAssertEqual(blocks.last?.kind, .footer)
            expectedSheets += NotebookPDF.paginate(blocks.map(\.height), usable: usable).count
            for (i, ex) in page.exercises.enumerated() where ex.kind.isWriting && FileManager.default.fileExists(atPath: book.inkURL(page: page, exercise: i).path) {
                let ink = try XCTUnwrap(blocks[i + 1].ink, "exercise \(i) on page \(page.number) was written; its ink prints")
                XCTAssertGreaterThan(ink.size.width, 100)
                XCTAssertGreaterThan(ink.size.height, 20)
                inkBlocks += 1
            }
        }
        XCTAssertGreaterThan(inkBlocks, 5, "the synthetic notebook has handwriting to print")
        XCTAssertGreaterThanOrEqual(expectedSheets, book.pages.count, "at least one sheet per notebook page")

        let data = NotebookPDF.render(book: book, paper: paper)
        XCTAssertTrue(String(decoding: data.prefix(5), as: UTF8.self).hasPrefix("%PDF"))
        let doc = try XCTUnwrap(PDFDocument(data: data))
        XCTAssertEqual(doc.pageCount, expectedSheets)
        let first = try XCTUnwrap(doc.page(at: 0))
        XCTAssertEqual(first.bounds(for: .mediaBox).size.width, paper.size.width, accuracy: 0.5)
        XCTAssertTrue(first.string?.contains("Классная работа") ?? false, "the header is real text")

        // A single page export has only that page's sheets, and is named for it.
        let one = NotebookPDF.render(book: book, pages: [book.pages[0]], paper: paper)
        let oneDoc = try XCTUnwrap(PDFDocument(data: one))
        let oneBlocks = NotebookPDF.blocks(for: book.pages[0], book: book, width: width, originX: contentX)
        XCTAssertEqual(oneDoc.pageCount, NotebookPDF.paginate(oneBlocks.map(\.height), usable: usable).count)

        // The first sheet as a PNG, for looking at (the path is printed).
        let png = first.thumbnail(of: CGSize(width: paper.size.width * 2, height: paper.size.height * 2), for: .mediaBox).pngData()
        let out = FileManager.default.temporaryDirectory.appendingPathComponent("drussky-pdf-sheet1.png")
        try png?.write(to: out)
        print("PDF sheet 1 rendered to \(out.path)")
    }

    // MARK: Pocket

    func testPocketPageHearsAndSpeaksOnlyAndTakesWhatIsDue() throws {
        let book = studied(6)
        book.advanceClock(byDays: 1)
        let due = book.dueReviews()
        XCTAssertGreaterThan(due.count, 0, "yesterday's items are due")
        let voice = Voice()
        let exercises = PocketPage.exercises(book: book, sentences: Library.load().sentences ?? [], hasAudio: voice.hasRecording,
                                             voices: { voice.pairVoices($0, exactStress: $1).count }, minutes: 7)
        XCTAssertGreaterThanOrEqual(exercises.count, 4)
        for ex in exercises {
            XCTAssertTrue(PocketPage.kinds.contains(ex.kind), "\(ex.kind) has no place in a pocket")
            XCTAssertFalse(ex.kind.isWriting)
            XCTAssertNotEqual(ex.kind, .dictation, "a dictation due is said, not written")
            if ex.kind == .readAloud { XCTAssertNotNil(ex.text) }
            if ex.kind == .listenPick { XCTAssertTrue(ex.options?.contains(ex.say ?? "") ?? false) }
        }
        let reviews = exercises.compactMap(\.reviewKey)
        XCTAssertGreaterThan(reviews.count, 0, "due items are on the page")
        XCTAssertTrue(Set(reviews).isSubset(of: Set(due.map(\.id))))
        XCTAssertEqual(Set(reviews).count, reviews.count, "no item twice")
        let seconds = exercises.map(PocketPage.seconds).reduce(0, +)
        XCTAssertLessThanOrEqual(seconds, 10 * 60 + 90, "sized for the pocket, not the desk")
        XCTAssertEqual(exercises.first?.kind, .listenPick, "opens with something to hear")

        // Inserted after the current page, with its own unit, and the items count as reviews.
        let before = book.currentPage
        XCTAssertNil(PocketPage.insert(book: book, voice: voice))
        XCTAssertEqual(book.currentPage, before + 1)
        XCTAssertEqual(book.page?.unitTitle, PocketPage.unit)
        XCTAssertTrue(book.page?.exercises.allSatisfy { PocketPage.kinds.contains($0.kind) } ?? false)
    }

    // MARK: Typing

    func testTransliterationScheme() {
        XCTAssertEqual(Transliteration.russian("privet"), "привет")
        XCTAssertEqual(Transliteration.russian("ya chitayu"), "я читаю")
        XCTAssertEqual(Transliteration.russian("shchi"), "щи")
        XCTAssertEqual(Transliteration.russian("schi"), "щи")
        XCTAssertEqual(Transliteration.russian("koshka"), "кошка")
        XCTAssertEqual(Transliteration.russian("zhizn'"), "жизнь")
        XCTAssertEqual(Transliteration.russian("ob''ekt"), "объект")
        XCTAssertEqual(Transliteration.russian("novyj"), "новый")
        XCTAssertEqual(Transliteration.russian("moj"), "мой")
        XCTAssertEqual(Transliteration.russian("eto"), "ето", "e is always е")
        XCTAssertEqual(Transliteration.russian("ehto"), "это")
        XCTAssertEqual(Transliteration.russian("yolka"), "ёлка")
        XCTAssertEqual(Transliteration.russian("yug"), "юг")
        XCTAssertEqual(Transliteration.russian("Moskva"), "Москва", "a capital carries over")
        XCTAssertEqual(Transliteration.russian("Zhuk"), "Жук")
        XCTAssertEqual(Transliteration.russian("chto? da!"), "что? да!", "punctuation passes through")
        XCTAssertEqual(Transliteration.russian("cvet"), "цвет")
        XCTAssertEqual(Transliteration.russian("xleb"), "хлеб")
        XCTAssertEqual(Transliteration.latin(for: "ш"), "sh")
        XCTAssertEqual(Transliteration.latin(for: "щ"), "sch")
        XCTAssertEqual(Transliteration.latin(for: "ь"), "'")
    }

    func testTypedAnswersAreCheckedLikeHandwriting() {
        var translate = Exercise(kind: .translate); translate.en = "cat"; translate.answerText = "ко́т"
        XCTAssertTrue(TypedCheck.accepts("кот", exercise: translate))
        XCTAssertTrue(TypedCheck.accepts("Кот ", exercise: translate), "case and a trailing space don't matter")
        XCTAssertFalse(TypedCheck.accepts("ток", exercise: translate))
        XCTAssertFalse(TypedCheck.accepts("", exercise: translate))
        var hedgehog = Exercise(kind: .dictation); hedgehog.text = "ёж"
        XCTAssertTrue(TypedCheck.accepts("еж", exercise: hedgehog), "ё may be typed as е")
        var reading = Exercise(kind: .translate); reading.en = "I am reading"; reading.answerText = "читаю"
        XCTAssertTrue(TypedCheck.accepts("я читаю", exercise: reading), "an agreeing subject pronoun is fine when translating")
        var dictation = Exercise(kind: .dictation); dictation.text = "читаю"
        XCTAssertFalse(TypedCheck.accepts("я читаю", exercise: dictation), "but not in a dictation")
        var copy = Exercise(kind: .copy); copy.text = "ма́ма"; copy.times = 2
        XCTAssertTrue(TypedCheck.accepts("мама мама", exercise: copy), "practice typed several times over")
        XCTAssertFalse(TypedCheck.accepts("мам", exercise: copy))
        var meet = Exercise(kind: .meet); meet.letter = "Ж"
        XCTAssertTrue(TypedCheck.accepts("ЖжЖж", exercise: meet))
    }

    func testKeyboardMapNamesTheNextKey() {
        XCTAssertEqual(RussianKeyboard.next(target: "ко́т", typed: ""), .letter("к"))
        XCTAssertEqual(RussianKeyboard.next(target: "ко́т", typed: "ко"), .letter("т"))
        XCTAssertEqual(RussianKeyboard.next(target: "ко́т", typed: "Ко"), .letter("т"), "case doesn't matter")
        XCTAssertEqual(RussianKeyboard.next(target: "ко́т", typed: "кот"), .done)
        XCTAssertEqual(RussianKeyboard.next(target: "ко́т", typed: "ка"), .backspace)
        XCTAssertEqual(RussianKeyboard.next(target: "ко́т", typed: "коты"), .backspace)
        XCTAssertEqual(RussianKeyboard.next(target: "мама кот", typed: "мама"), .space)
        XCTAssertEqual(RussianKeyboard.next(target: "ёж", typed: "е"), .letter("ж"), "ё typed as е is on course")
        XCTAssertEqual(RussianKeyboard.next(target: "", typed: ""), .done)
        XCTAssertEqual(RussianKeyboard.position(of: "ф")?.row, 1)
        XCTAssertEqual(RussianKeyboard.position(of: "ф")?.column, 0)
        XCTAssertEqual(RussianKeyboard.position(of: "Й")?.row, 0)
        XCTAssertNil(RussianKeyboard.position(of: "ё"))
        XCTAssertEqual(RussianKeyboard.finger(row: 1, column: 0), 0, "left little finger on ф")
        XCTAssertEqual(RussianKeyboard.finger(row: 1, column: 10), 7, "right little finger on э")
        XCTAssertEqual(Set(RussianKeyboard.allLetters).count, 33)
    }

    func testTypedAnswersLogUnderTheirOwnModality() throws {
        let book = studied(1)
        var ex = Exercise(kind: .translate); ex.en = "cat"; ex.answerText = "кот"
        book.insertWrittenPage(title: "Typing", intro: nil, unitTitle: "Test", exercises: [ex], newWords: [Course.Word(ru: "кот", en: "cat", emoji: nil, note: nil)])
        let count = book.knowledge.model.events.count
        book.record(page: book.currentPage, exercise: 0) { r in r.attempts += 1; r.status = .correct; r.recognized = "кот"; r.typed = true }
        let event = try XCTUnwrap(book.knowledge.model.events.last)
        XCTAssertGreaterThan(book.knowledge.model.events.count, count)
        XCTAssertEqual(event.modality, .type)
        XCTAssertEqual(event.grade, .good)
        XCTAssertTrue(book.knowledge.domain.modalities.contains(.type))
        XCTAssertEqual(Modality.type.title, "typed")
    }

    func testTypingDrillLinesAndLog() throws {
        var rng = SystemRandomNumberGenerator()
        let home = TypingDrill.line(stage: .homeRow, words: [], using: &rng)
        XCTAssertTrue(home.allSatisfy { $0 == " " || RussianKeyboard.homeRow.contains($0) }, home)
        XCTAssertEqual(home.split(separator: " ").count, 5)
        let words = TypingDrill.line(stage: .words, words: ["кот", "дом", "мама", "вода"], using: &rng)
        XCTAssertTrue((3...4).contains(words.split(separator: " ").count))
        let score = TypingDrill.Score(characters: 25, seconds: 30, errors: 1)
        XCTAssertEqual(score.wpm, 10, accuracy: 0.001)
        XCTAssertEqual(score.accuracy, 0.96, accuracy: 0.001)
        let root = tempStore().root
        var log = TypingDrill.Log()
        log.entries.append(TypingDrill.Entry(at: Date(), stage: .homeRow, score: score))
        log.entries.append(TypingDrill.Entry(at: Date().addingTimeInterval(-86_400), stage: .homeRow, score: TypingDrill.Score(characters: 25, seconds: 60, errors: 0)))
        try log.save(root: root)
        let again = TypingDrill.Log.load(root: root)
        XCTAssertEqual(again.entries.map(\.score), log.entries.map(\.score))
        XCTAssertEqual(again.entries.map(\.stage), log.entries.map(\.stage))
        for (a, b) in zip(again.entries, log.entries) { XCTAssertEqual(a.at.timeIntervalSince1970, b.at.timeIntervalSince1970, accuracy: 1, "ISO-8601 keeps whole seconds") }
        XCTAssertEqual(again.daily(stage: .homeRow).map(\.wpm), [5, 10], "oldest day first")
        XCTAssertEqual(again.best(stage: .homeRow), 10)
        XCTAssertEqual(again.daily(stage: .words).count, 0)
    }
}
