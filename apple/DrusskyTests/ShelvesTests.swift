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

    func testSerialsDecodeEpisodesInCourseOrder() throws {
        let serials = Serial.load()
        XCTAssertEqual(serials.map(\.id), ["stantsiya", "kukhnya"], "both files, in Serial.files order")
        let order = try pageOrder()
        for serial in serials {
            let count = serial.id == "kukhnya" ? 30 : 24
            XCTAssertEqual(serial.episodes.count, count, serial.id)
            XCTAssertEqual(serial.episodes.map(\.number), Array(1...count), serial.id)
            var last = -1
            for ep in serial.episodes {
                let pos = try XCTUnwrap(order[ep.after_page], "\(ep.id): after_page \(ep.after_page) is no course page")
                XCTAssertGreaterThanOrEqual(pos, last, "\(ep.id) is ready before the one before it")
                last = pos
                XCTAssertFalse(ep.text.isEmpty); XCTAssertFalse(ep.en.isEmpty); XCTAssertFalse(ep.glosses.isEmpty); XCTAssertFalse(ep.prompt.isEmpty)
                XCTAssertEqual(ep.questions.count, 3, ep.id)
                for q in ep.questions {
                    XCTAssertEqual(q.kind, .readMeaning, ep.id)
                    XCTAssertNotNil(q.text, ep.id)
                    XCTAssertEqual(q.options?.count, 3, ep.id)
                    XCTAssertTrue((0..<3).contains(q.answerIndex ?? -1), ep.id)
                }
                if let notice = ep.notice { XCTAssertTrue((1...3).contains(notice.count), ep.id) }
            }
        }
        let station = try XCTUnwrap(serials.first)
        XCTAssertEqual(station.title, "Ста́нция")
        XCTAssertEqual(station.place(of: station.episodes[2]), "Ста́нция · эпизо́д 3")
        XCTAssertTrue(station.episodes.allSatisfy { $0.notice == nil })
        let kitchen = try XCTUnwrap(serials.last)
        XCTAssertEqual(kitchen.title, "Ку́хня")
        XCTAssertEqual(kitchen.place(of: kitchen.episodes[0]), "Ку́хня · диало́г 1")
        XCTAssertEqual(kitchen.itemWordEn, "dialogue")
        XCTAssertTrue(kitchen.episodes.allSatisfy { $0.notice?.isEmpty == false }, "every dialogue says what to hear")
        XCTAssertTrue(kitchen.episodes.allSatisfy { $0.text.hasPrefix("— ") }, "dialogue lines start with a dash")
    }

    func testDialogueCarriesWhatToHearOnItsPassage() throws {
        let kitchen = try XCTUnwrap(Shelves.serials.first { $0.id == "kukhnya" })
        let ep = kitchen.episodes[0]
        let page = Shelves.page(for: ep, in: kitchen)
        XCTAssertEqual(page.sourceID, "episode:kukhnya-01")
        XCTAssertEqual(page.unitTitle, "Ку́хня · диало́г 1")
        XCTAssertEqual(page.exercises.map(\.kind), [.passage, .readMeaning, .readMeaning, .readMeaning, .diary])
        XCTAssertEqual(page.exercises[0].notice, ep.notice)
        XCTAssertTrue(page.intro.hasPrefix("Cheese and Tima. Read it"), page.intro)
        XCTAssertEqual(Shelves.lead("The Milk Boiled Over!"), "The Milk Boiled Over! ")
        XCTAssertEqual(Shelves.lead(nil), "")
        let data = try JSONEncoder().encode(page.exercises[0])
        XCTAssertEqual(try JSONDecoder().decode(Exercise.self, from: data).notice, ep.notice)
        // Both serials are on the shelves, in order, with the kitchen's items named as dialogues.
        let items = Shelves.items
        XCTAssertTrue(items.contains { $0.sourceID == "episode:stantsiya-01" && $0.place == "Ста́нция · эпизо́д 1" })
        XCTAssertTrue(items.contains { $0.sourceID == "episode:kukhnya-30" && $0.place == "Ку́хня · диало́г 30" })
        XCTAssertEqual(Set(items.map(\.sourceID)).count, items.count)
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
        // The kitchen's first dialogue follows u4-p2, so it is not ready yet and nothing else is offered.
        XCTAssertTrue(book.shelfOffers().isEmpty)
        // The notebook keeps the offers through a save and load.
        book.saveNow()
        let again = Book(course: book.course, store: BookStore(root: book.store.root))
        XCTAssertEqual(again.pages.compactMap(\.offers).count, 1)
        XCTAssertTrue(again.hasOpened(first))
    }
}
