import LearningCore
import XCTest
@testable import Drussky

@MainActor
final class KnowledgeTests: XCTestCase {
    var course: Course {
        let url = Bundle.main.url(forResource: "course", withExtension: "json")!
        return try! JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    func tempStore() -> BookStore {
        BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
    }

    func testComponentsExtractedFromTheCourse() {
        let d = RussianDomain(course: course, lemmas: Lemmas.load(), pairs: SoundPairs.load())
        let g = d.graph
        XCTAssertEqual(g.components(of: .letter).count, 33)
        XCTAssertNotNil(g[ComponentID(.lexeme, "читать")])
        XCTAssertEqual(g.edges(from: ComponentID(.form, "читаю"), .formOf).map(\.to), [ComponentID(.lexeme, "читать")])
        XCTAssertTrue(g.edges(from: ComponentID(.form, "читаю"), .exemplifies).map(\.to).contains(ComponentID(.construction, "pres-1sg")))
        XCTAssertTrue(g.edges(from: ComponentID(.form, "книгу"), .exemplifies).map(\.to).contains(ComponentID(.construction, "acc-u")))
        XCTAssertTrue(g.edges(from: ComponentID(.lexeme, "кот"), .uses).map(\.to).contains(ComponentID(.letter, "К")))
        let phrase = ComponentID(.phrase, "у меня есть")
        XCTAssertTrue(g.edges(to: phrase, .partOf).map(\.from).contains(ComponentID(.form, "меня")))
        XCTAssertTrue(g.edges(from: phrase, .exemplifies).map(\.to).contains(ComponentID(.construction, "have")))
        XCTAssertEqual(g.prerequisites(of: ComponentID(.construction, "pres-2sg")), [ComponentID(.construction, "pres-1sg")])
        XCTAssertEqual(g.components(of: .sound).count, SoundPairs.load().contrasts.count)
        // Every lexicon entry is a component.
        for w in course.lexicon {
            XCTAssertNotNil(d.utteranceID(w.ru).flatMap { g[$0] }, w.ru)
        }
        // Letters come out in the order the course meets them.
        let a = g[ComponentID(.letter, "А")]!.order, zh = g[ComponentID(.letter, "Ж")]!.order
        XCTAssertLessThan(a, zh)
    }

    func testExerciseTagging() {
        let d = RussianDomain(course: course, lemmas: Lemmas.load(), pairs: SoundPairs.load())
        var ex = Exercise(kind: .fill)
        ex.before = "Я читаю "; ex.answerText = "кни́гу"; ex.after = "."
        XCTAssertEqual(d.components(for: ex), [ComponentID(.form, "книгу"), ComponentID(.lexeme, "книга"), ComponentID(.construction, "acc-u")])
        var past = Exercise(kind: .dictation); past.text = "Она́ не зна́ла."
        let c = d.components(for: past)
        XCTAssertTrue(c.contains(ComponentID(.construction, "past")))
        XCTAssertTrue(c.contains(ComponentID(.construction, "negation")))
        var tagged = past; tagged.skills = ["gram:gender"]
        XCTAssertEqual(d.components(for: tagged).filter { $0.kind == .construction }, [ComponentID(.construction, "gender")],
                       "an authored skills list replaces the tagger")
        var meet = Exercise(kind: .meet); meet.letter = "Ж"
        XCTAssertEqual(d.components(for: meet), [ComponentID(.letter, "Ж")])
        var syllable = Exercise(kind: .trace); syllable.text = "ма"
        XCTAssertEqual(Set(d.components(for: syllable)), [ComponentID(.letter, "М"), ComponentID(.letter, "А")])
        // The skills field round-trips and is optional.
        let json = try! JSONEncoder().encode(tagged)
        XCTAssertEqual(try! JSONDecoder().decode(Exercise.self, from: json).skills, ["gram:gender"])
    }

    func testMissedAnswersMakeConfusionEdges() {
        let book = Book(course: course, store: tempStore())
        var written = Exercise(kind: .dictation); written.text = "оно́"
        var pick = Exercise(kind: .listenPick); pick.say = "кот"; pick.options = ["кот", "рот"]
        var letters = Exercise(kind: .dictation); letters.text = "сок"
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [written, pick, letters], newWords: [])
        let p = book.currentPage
        // Wrote она for оно, checked, then was shown it.
        book.record(page: p, exercise: 0) { $0.attempts += 1; $0.recognized = "она" }
        book.record(page: p, exercise: 0) { $0.revealed = true; $0.status = .missed }
        // Picked рот, then кот.
        book.record(page: p, exercise: 1) { $0.chosen = 1; $0.attempts += 1 }
        book.record(page: p, exercise: 1) { $0.chosen = 0; $0.attempts += 1; $0.status = .done }
        // Wrote сак: a letter for a letter.
        book.record(page: p, exercise: 2) { $0.attempts += 1; $0.recognized = "сак" }
        let m = book.knowledge.model
        let edges = m.confusionEdges(at: Date())
        func has(_ a: ComponentID, _ b: ComponentID) -> Bool { edges.contains { $0.from == a && $0.to == b } }
        XCTAssertTrue(has(ComponentID(.lexeme, "оно"), ComponentID(.lexeme, "она")), "\(edges)")
        XCTAssertTrue(has(ComponentID(.lexeme, "кот"), ComponentID(.lexeme, "рот")), "\(edges)")
        XCTAssertTrue(has(ComponentID(.letter, "О"), ComponentID(.letter, "А")), "\(edges)")
        XCTAssertEqual(edges.first { $0.from == ComponentID(.lexeme, "оно") }?.weight ?? 0, 1, accuracy: 0.01,
                       "the reveal after a failed check does not count the same misreading twice")
        XCTAssertEqual(m.trace(ComponentID(.lexeme, "оно"), .hearWrite)?.reviews.first?.grade, .again)
        XCTAssertEqual(m.trace(ComponentID(.lexeme, "кот"), .hearPick)?.reviews.first?.grade, .hard)
        // The log on disk says the same thing.
        let reread = KnowledgeModel(events: EventLog(url: book.knowledge.logURL).load())
        XCTAssertEqual(reread.confusionEdges(at: Date()).count, edges.count)
        XCTAssertTrue(book.knowledge.summary().contains("оно ↔ она") || book.knowledge.summary().contains("она ↔ оно"))
    }

    /// A notebook written before the knowledge model existed: no `learned`, no customWords,
    /// no soundStats, no skills, no events.jsonl.
    static let oldNotebook = """
    {"version":1,"courseCursor":1,"currentPage":0,
     "settings":{"fingerDrawing":false,"cursiveModels":false,"showStress":true},
     "memories":[{"key":"word:кот","stability":1.5,"due":"2026-10-02T09:00:00Z","reps":1,"lapses":0,"seen":"2026-10-01T09:00:00Z"}],
     "pages":[{"id":"p0001-abc123","number":1,"sourceID":"u1-p1","unitTitle":"Familiar faces","title":"First",
       "isReview":false,"created":"2026-10-01T08:50:00Z","finished":"2026-10-01T09:05:00Z",
       "exercises":[{"kind":"meet","letter":"К"},{"kind":"dictation","text":"кот"},
                    {"kind":"listen_pick","say":"кот","options":["кот","рот"]},{"kind":"explain","text":"Hi"},
                    {"kind":"translate","en":"cat","answer":"кот"}],
       "results":[{"status":"done","attempts":0,"revealed":false,"updated":"2026-10-01T08:55:00Z"},
                  {"status":"missed","attempts":1,"recognized":"кат","revealed":true,"updated":"2026-10-01T09:00:00Z"},
                  {"status":"correct","attempts":1,"chosen":0,"revealed":false,"updated":"2026-10-01T09:01:00Z"},
                  {"status":"done","attempts":0,"revealed":false,"updated":"2026-10-01T09:02:00Z"},
                  {"status":"open","attempts":0,"revealed":false}]}]}
    """

    func testOldNotebookDecodesAndBackfillsOnce() throws {
        let store = tempStore()
        try Data(Self.oldNotebook.utf8).write(to: store.stateURL)
        let book = Book(course: course, store: store)
        XCTAssertEqual(book.pages.count, 1, "the old notebook was read, not moved aside")
        XCTAssertNil(try FileManager.default.contentsOfDirectory(atPath: store.root.path).first { $0.hasPrefix("book-unreadable") })
        XCTAssertEqual(book.memories["word:кот"]?.reps, 1)
        let events = EventLog(url: book.knowledge.logURL).load()
        // meet, dictation, listen_pick finished and graded; explain is ungraded; translate open.
        XCTAssertEqual(events.map(\.item), ["p0001-abc123#0", "p0001-abc123#1", "p0001-abc123#2"])
        XCTAssertTrue(events.allSatisfy { $0.source == "backfill" })
        XCTAssertEqual(events[1].t, ISO8601DateFormatter().date(from: "2026-10-01T09:00:00Z"))
        XCTAssertEqual(events[1].grade, .again)
        XCTAssertEqual(events[1].confusions?.first?.target, ComponentID(.letter, "О"))
        book.saveNow()

        // Opening the same notebook again adds nothing.
        let again = Book(course: course, store: store)
        XCTAssertEqual(EventLog(url: again.knowledge.logURL).load().count, 3)
        XCTAssertEqual(again.knowledge.backfill(pages: again.pages), 0)
        // Finishing the open exercise is logged live, once.
        again.record(page: 0, exercise: 4) { $0.attempts = 1; $0.recognized = "кот"; $0.status = .correct }
        let third = Book(course: course, store: store)
        let all = EventLog(url: third.knowledge.logURL).load()
        XCTAssertEqual(all.count, 4)
        XCTAssertEqual(all.last?.source, "live")
        XCTAssertNotNil(third.knowledge.model.trace(ComponentID(.lexeme, "кот"), .meaningWrite))
    }

    func testReadability() {
        let book = Book(course: course, store: tempStore())
        var ex = Exercise(kind: .dictation); ex.text = "Я чита́ю кни́гу."
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [ex], newWords: [])
        XCTAssertEqual(book.knowledge.readability(of: "Я читаю книгу."), 0)
        book.record(page: book.currentPage, exercise: 0) { $0.attempts = 1; $0.status = .correct }
        // я, читаю (form), книгу (form) known; книге reached through its lemma; стол not.
        XCTAssertEqual(book.knowledge.readability(of: "Я читаю книгу."), 1)
        XCTAssertEqual(book.knowledge.readability(of: "Я читаю книгу. Книга на столе."), 4.0 / 6.0, accuracy: 1e-9)
    }
}
