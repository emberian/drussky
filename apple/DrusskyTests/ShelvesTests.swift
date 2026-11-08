import XCTest
@testable import Drussky

/// The shelves beside the course (Model/Shelves.swift): series.json and math.json decode with
/// their authored support, become pages of ordinary exercises, and are offered once by the course.
@MainActor
final class ShelvesTests: XCTestCase {
    private func course() throws -> Course {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        return try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    private func freshBook() throws -> Book {
        Book(course: try course(), store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
    }

    /// Course pages in order, so `after_page`s can be compared by position (units are not numbered in order).
    private func pageOrder() throws -> [String: Int] {
        Dictionary(uniqueKeysWithValues: try course().pageSequence.enumerated().map { ($1.page.id, $0) })
    }

    func testSerialDecodesEpisodesInCourseOrder() throws {
        let serials = Serial.load()
        let serial = try XCTUnwrap(serials.first { $0.id == "stantsiya" })
        XCTAssertEqual(serial.title, "Ста́нция")
        XCTAssertEqual(serial.episodes.count, 24)
        XCTAssertEqual(serial.episodes.map(\.number), Array(1...24))
        let order = try pageOrder()
        var last = -1
        for ep in serial.episodes {
            let pos = try XCTUnwrap(order[ep.after_page], "\(ep.id): after_page \(ep.after_page) is no course page")
            XCTAssertGreaterThanOrEqual(pos, last, "\(ep.id) is ready before the episode before it")
            last = pos
            XCTAssertFalse(ep.text.isEmpty); XCTAssertFalse(ep.en.isEmpty); XCTAssertFalse(ep.glosses.isEmpty); XCTAssertFalse(ep.prompt.isEmpty)
            XCTAssertEqual(ep.questions.count, 3, ep.id)
            for q in ep.questions {
                XCTAssertEqual(q.kind, .readMeaning, ep.id)
                XCTAssertNotNil(q.text, ep.id)
                XCTAssertEqual(q.options?.count, 3, ep.id)
                XCTAssertTrue((0..<3).contains(q.answerIndex ?? -1), ep.id)
            }
        }
    }

    func testMathShelfDecodesWithItsSupport() throws {
        let lib = Library.load()
        let shelf = try XCTUnwrap(lib.shelves?.first { $0.id == "mathematics" })
        XCTAssertEqual(shelf.title, "Матема́тика")
        let texts = lib.texts(onShelf: shelf.id)
        XCTAssertEqual(texts.count, 24)
        XCTAssertEqual(texts.map { $0.rung ?? 0 }, Array(1...24))
        let order = try pageOrder()
        var last = -1
        for t in texts {
            let pos = try XCTUnwrap(order[try XCTUnwrap(t.after_page, t.id)], "\(t.id): after_page is no course page")
            XCTAssertGreaterThanOrEqual(pos, last, t.id)
            last = pos
            XCTAssertTrue(t.isAuthoredReading, t.id)
            XCTAssertEqual(t.questions?.count, 3, t.id)
            XCTAssertFalse(t.how_read?.isEmpty ?? true, t.id)
            for q in t.questions ?? [] {
                XCTAssertTrue(["gist", "detail", "inference"].contains(q.kind), t.id)
                XCTAssertTrue((0..<q.options.count).contains(q.answer), t.id)
            }
        }
        // The library proper keeps decoding, without any of the shelf's fields.
        let loose = lib.loose
        XCTAssertGreaterThanOrEqual(loose.count, 40)
        XCTAssertTrue(loose.allSatisfy { $0.shelf == nil && $0.en == nil && $0.glosses == nil && $0.after_page == nil })
        XCTAssertEqual(Set(lib.texts.map(\.id)).count, lib.texts.count, "ids are unique across the library and its shelves")
    }

    func testEpisodeBecomesAPageOfOrdinaryExercises() throws {
        let serial = try XCTUnwrap(Shelves.serials.first)
        let ep = serial.episodes[0]
        let page = Shelves.page(for: ep, in: serial)
        XCTAssertEqual(page.sourceID, "episode:stantsiya-01")
        XCTAssertEqual(page.exercises.map(\.kind), [.passage, .readMeaning, .readMeaning, .readMeaning, .diary])
        let passage = page.exercises[0]
        XCTAssertEqual(passage.text, ep.text)
        XCTAssertEqual(passage.en, ep.en)
        XCTAssertEqual(passage.glosses, ep.glosses)
        XCTAssertNil(passage.frames)
        XCTAssertEqual(page.exercises[4].prompt, ep.prompt)
        // Pages are frozen into the notebook: the exercises round-trip.
        let data = try JSONEncoder().encode(page.exercises)
        XCTAssertEqual(try JSONDecoder().decode([Exercise].self, from: data), page.exercises)
    }

    func testReadingBecomesAPassageWithFramesAndQuestions() throws {
        let shelf = try XCTUnwrap(Shelves.library.shelves?.first)
        let text = try XCTUnwrap(Shelves.library.texts(onShelf: shelf.id).first)
        let page = Shelves.page(for: text, on: shelf)
        XCTAssertEqual(page.sourceID, "reading:\(text.id)")
        XCTAssertEqual(page.exercises.map(\.kind), [.passage, .readMeaning, .readMeaning, .readMeaning])
        let passage = page.exercises[0]
        XCTAssertEqual(passage.frames?.count, (text.how_read?.count ?? 0) + (text.notation?.count ?? 0))
        XCTAssertEqual(passage.frames?.last?.symbol, text.notation?.last?.symbol)
        let q = page.exercises[1]
        XCTAssertEqual(q.text, text.questions?[0].prompt)
        XCTAssertEqual(q.answerIndex, text.questions?[0].answer)
        XCTAssertEqual(q.hint, text.questions?[0].explanation)
        XCTAssertTrue(RussianText.letters(in: q.text ?? "").isEmpty || q.text!.contains("«"), "an English question, Russian only in quotation")
        let data = try JSONEncoder().encode(passage)
        XCTAssertEqual(try JSONDecoder().decode(Exercise.self, from: data).frames, passage.frames)
    }

    func testPassageTokensLoseEveryKindOfQuote() {
        XCTAssertEqual(PassageExercise.bare("„Что“"), "Что")
        XCTAssertEqual(PassageExercise.bare("“так”,"), "так")
        XCTAssertEqual(PassageExercise.bare("’вот’"), "вот")
        XCTAssertEqual(PassageExercise.bare("'да'"), "да")
        XCTAssertEqual(PassageExercise.bare("(A)"), "A")
        XCTAssertEqual(PassageExercise.bare("звезду́."), "звезду́")
        XCTAssertEqual(PassageExercise.bare("по-пре́жнему"), "по-пре́жнему")
        XCTAssertEqual(PassageExercise.bare("—"), "")
        XCTAssertEqual(PassageExercise.bare("∅"), "∅", "notation stays tappable")
    }

    func testAWordTaughtInsideAPhraseIsGlossedFromThePhrase() throws {
        let book = try freshBook()
        let phrase = try XCTUnwrap(book.course.lexicon.first { RussianText.key($0.ru).split(separator: " ").count == 2
            && $0.ru.split(separator: " ").allSatisfy { book.course.word(String($0)) == nil || RussianText.key(String($0)).count == 1 } })
        let word = try XCTUnwrap(phrase.ru.split(separator: " ").map(String.init).first { RussianText.key($0).count > 1 && book.course.word($0) == nil })
        XCTAssertNil(book.word(word))
        let gloss = try XCTUnwrap(book.phraseGloss(for: word), "\(word) from «\(phrase.ru)»")
        XCTAssertTrue(gloss.hasPrefix(phrase.en))
        XCTAssertTrue(gloss.contains(phrase.ru))
        XCTAssertNil(book.phraseGloss(for: "в"), "one-letter words are not glossed from phrases")
    }

    func testTheCourseOffersAnEpisodeOnceWhenItsPageIsDone() throws {
        let book = try freshBook()
        let serial = try XCTUnwrap(Shelves.serials.first)
        let first = Shelves.item(serial.episodes[0], in: serial)
        XCTAssertFalse(book.isReady(first))
        XCTAssertTrue(book.shelfOffers().isEmpty)
        XCTAssertNil(book.pages.last?.offers)
        // The episode's course page, finished.
        var ex = Exercise(kind: .copy); ex.text = "кот"
        book.appendPage(sourceID: first.afterPage, unitTitle: "u", title: "t", intro: nil, isReview: false, exercises: [ex])
        let p = book.pages.count - 1
        book.record(page: p, exercise: 0) { $0.status = .done }
        XCTAssertTrue(book.pages[p].isFinished)
        XCTAssertTrue(book.isReady(first))
        XCTAssertEqual(book.shelfOffers(), [first.sourceID])
        // The next composed page carries the offer; the one after does not repeat it.
        book.appendPage(sourceID: nil, unitTitle: "u", title: "next", intro: nil, isReview: false, exercises: [ex])
        XCTAssertEqual(book.pages.last?.offers, [first.sourceID])
        book.appendPage(sourceID: nil, unitTitle: "u", title: "after", intro: nil, isReview: false, exercises: [ex])
        XCTAssertNil(book.pages.last?.offers)
        // Opening it makes a page with the episode's source id; the serial moves on to episode 2, not yet ready.
        XCTAssertEqual(book.nextEpisode(of: serial)?.number, 1)
        book.open(first)
        XCTAssertTrue(book.hasOpened(first))
        XCTAssertEqual(book.page?.sourceID, first.sourceID)
        XCTAssertEqual(book.page?.exercises.count, 5)
        XCTAssertEqual(book.nextEpisode(of: serial)?.number, 2)
        XCTAssertTrue(book.shelfOffers().isEmpty)
        // The notebook keeps the offers through a save and load.
        book.saveNow()
        let again = Book(course: book.course, store: BookStore(root: book.store.root))
        XCTAssertEqual(again.pages.compactMap(\.offers).count, 1)
        XCTAssertTrue(again.hasOpened(first))
    }
}
