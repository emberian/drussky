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
        XCTAssertEqual(g.components(of: .sound).count, SoundPairs.load().contrasts.count + 1, "the pair contrasts and asking-or-telling (RussianDomain+Listening)")
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
        XCTAssertEqual(m.trace(ComponentID(.lexeme, "кот"), .hearPick)?.reviews.first?.grade, .again,
                       "a right pick after a wrong one is not recall")
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
        XCTAssertNotNil(book.targets["word:кот"], "format-1 memories become review targets")
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
        // Only the actual forms produced are covered; neither книга nor столе was produced.
        XCTAssertEqual(book.knowledge.readability(of: "Я читаю книгу."), 1)
        XCTAssertEqual(book.knowledge.readability(of: "Я читаю книгу. Книга на столе."), 3.0 / 6.0, accuracy: 1e-9)
    }

    /// How long the answer took travels from the result into the log (and back through a backfill),
    /// feeds the trace's running average, and is reset when the exercise is reopened.
    func testResponseLatencyTravelsToTheLogAndBack() throws {
        let store = tempStore()
        let book = Book(course: course, store: store)
        var d = Exercise(kind: .dictation); d.text = "кот"
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [d], newWords: [])
        let ref = ExerciseRef(pageID: book.page!.id, index: 0)
        // A failed check first (an attempt, timed), then the right answer (an outcome, timed).
        book.record(ref) { $0.attempts = 1; $0.failedChecks = 1; $0.recognized = "кат"; $0.responseMs = 4200 }
        book.record(ref) { $0.attempts = 2; $0.recognized = "кот"; $0.status = .correct; $0.responseMs = 3100 }
        let events = book.knowledge.model.events.filter { $0.item == ref.item }
        XCTAssertEqual(events.map(\.latencyMs), [4200, 3100])
        let trace = try XCTUnwrap(book.knowledge.model.trace(ComponentID(.lexeme, "кот"), .hearWrite))
        XCTAssertEqual(trace.reviews.last?.latencyMs, 3100)
        XCTAssertEqual(trace.latencySamples, 0, "right after a failed check is `hard`: not how fast the knowledge comes")
        XCTAssertEqual(book.result(ref)?.responseMs, 3100)
        book.saveNow()
        let saved = try BookStore.decoder().decode(BookState.self, from: Data(contentsOf: store.stateURL))
        XCTAssertEqual(saved.pages.last?.results[0].responseMs, 3100, "persisted with the result")
        // A crash between the save and the log: the backfilled outcome carries the latency too.
        let log = EventLog(url: book.knowledge.logURL)
        let kept = log.load().filter { $0.type != .outcome || $0.item != ref.item }
        try FileManager.default.removeItem(at: log.url)
        XCTAssertTrue(log.append(kept))
        let again = Book(course: course, store: store)
        let rebuilt = try XCTUnwrap(again.knowledge.model.events.first { $0.id == KnowledgeTracker.outcomeID(item: ref.item, round: 0) })
        XCTAssertEqual(rebuilt.source, "backfill")
        XCTAssertEqual(rebuilt.latencyMs, 3100)
        // Reopened: a new round, its own timing.
        again.reopen(ref)
        XCTAssertNil(again.result(ref)?.responseMs)
        again.record(ref) { $0.attempts = 1; $0.failedChecks = nil; $0.recognized = "кот"; $0.status = .correct; $0.responseMs = 900 }
        let t = try XCTUnwrap(again.knowledge.model.trace(ComponentID(.lexeme, "кот"), .hearWrite))
        XCTAssertEqual(t.latencySamples, 1)
        XCTAssertEqual(t.latencyEMA ?? 0, 900, accuracy: 1e-9)
        XCTAssertFalse(again.knowledge.isCompiled(ComponentID(.lexeme, "кот")), "one quick answer is not compiled")
        // The margin wording: untimed says nothing; timed and slow says so; compiled says so.
        let policy = again.knowledge.compilation
        XCTAssertFalse(t.hasLatencyEvidence(policy))
        XCTAssertEqual(policy.floor(for: .hearWrite), 8_000)
        XCTAssertEqual(policy.floor(for: .copy), 0, "copying is never compiled")
    }

    func testSnapshotAndTailReplayMatchesFullFold() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let t0 = Date(timeIntervalSince1970: 1_800_000_000)
        func day(_ d: Double) -> Date { t0.addingTimeInterval(d * 86_400) }
        let tracker = KnowledgeTracker(course: course, root: root, customWords: [], now: { t0 })
        XCTAssertEqual(tracker.foldCount, 1)
        XCTAssertFalse(tracker.loadedFromSnapshot)

        let kot = ComponentID(.lexeme, "кот")
        let rot = ComponentID(.lexeme, "рот")
        let dom = ComponentID(.lexeme, "дом")
        let sok = ComponentID(.lexeme, "сок")

        var initialEvents: [LearningEvent] = []
        for i in 0..<22 {
            let comp = i % 2 == 0 ? kot : dom
            let mod: Modality = i % 3 == 0 ? .hearPick : .hearWrite
            let conf: [Confusion]? = i == 3 ? [Confusion(target: kot, produced: rot, channel: "choice")] : nil
            let grade: Grade = i == 3 ? .again : .good
            initialEvents.append(LearningEvent(id: "ev-\(i)", t: day(Double(i)), type: .outcome, item: "p1#\(i)",
                                               modality: mod, components: [comp], grade: grade, confusions: conf,
                                               latencyMs: 1200 + i * 50))
        }
        // Leave a pending help event before the snapshot so tail replay must honour `helped`.
        initialEvents.append(LearningEvent(id: "ev-help", t: day(22), type: .help, item: "p1#helped",
                                           modality: .meaningWrite, components: [sok], help: ["tutor"]))
        tracker.record(initialEvents)
        tracker.saveSnapshot()
        XCTAssertTrue(FileManager.default.fileExists(atPath: tracker.snapshotURL.path))
        XCTAssertEqual(tracker.eventsSinceSnapshot, 0)

        let tailEvents: [LearningEvent] = [
            LearningEvent(id: "tail-0", t: day(23), type: .outcome, item: "p1#helped",
                          modality: .meaningWrite, components: [sok], grade: .easy, latencyMs: 900),
            LearningEvent(id: "tail-1", t: day(24), type: .attempt, item: "p2#0",
                          modality: .hearWrite, components: [dom], correct: false,
                          confusions: [Confusion(target: dom, produced: kot, channel: "writing")]),
            LearningEvent(id: "tail-2", t: day(24.1), type: .outcome, item: "p2#0",
                          modality: .hearWrite, components: [dom], grade: .again,
                          confusions: [Confusion(target: dom, produced: kot, channel: "writing")], latencyMs: 3400),
            LearningEvent(id: "tail-3", t: day(25), type: .outcome, item: "p2#1",
                          modality: .meaningWrite, components: [kot], grade: .good, latencyMs: 1100),
        ]
        tracker.record(tailEvents)
        XCTAssertEqual(tracker.eventsSinceSnapshot, 4)

        let reloaded = KnowledgeTracker(course: course, root: root, customWords: [], now: { day(26) })
        XCTAssertTrue(reloaded.loadedFromSnapshot)
        XCTAssertEqual(reloaded.foldCount, 0, "snapshot load with tail replay does not perform a full fold")
        XCTAssertEqual(reloaded.tailReplayedCount, 4)

        let scratch = KnowledgeModel(events: EventLog(url: reloaded.logURL).load(), fsrs: reloaded.model.fsrs)
        let probeDate = day(26)
        XCTAssertEqual(reloaded.model, scratch)
        XCTAssertEqual(reloaded.model.events, scratch.events)
        XCTAssertEqual(reloaded.model.traces, scratch.traces)
        XCTAssertEqual(reloaded.model.confusions, scratch.confusions)
        XCTAssertEqual(reloaded.model.transfer, scratch.transfer)
        XCTAssertEqual(reloaded.model.confusionEdges(at: probeDate), scratch.confusionEdges(at: probeDate))
        for key in scratch.traces.keys {
            XCTAssertEqual(reloaded.model.retrievability(key.component, key.modality, at: probeDate) ?? -1,
                           scratch.retrievability(key.component, key.modality, at: probeDate) ?? -1, accuracy: 1e-12)
        }
        // Help before snapshot capped the tail outcome from .easy to .hard.
        XCTAssertEqual(reloaded.model.trace(sok, .meaningWrite)?.reviews.first?.grade, .hard)

        // Appending past the 50-event threshold automatically writes a fresh snapshot.
        let more = (0..<46).map { j in
            LearningEvent(id: "auto-\(j)", t: day(30 + Double(j) * 0.1), type: .outcome, item: "p3#\(j)",
                          modality: .hearWrite, components: [kot], grade: .good)
        }
        reloaded.record(more)
        XCTAssertEqual(reloaded.eventsSinceSnapshot, 0)
        let afterAuto = KnowledgeTracker(course: course, root: root, customWords: [], now: { day(40) })
        XCTAssertTrue(afterAuto.loadedFromSnapshot)
        XCTAssertEqual(afterAuto.tailReplayedCount, 0)
        XCTAssertEqual(afterAuto.foldCount, 0)
    }

    func testSnapshotFallbackWhenFSRSWeightsChangeOrLogIsTruncated() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let t0 = Date(timeIntervalSince1970: 1_800_000_000)
        let kot = ComponentID(.lexeme, "кот")
        let tracker = KnowledgeTracker(course: course, root: root, customWords: [], now: { t0 })
        let events = (0..<25).map { i in
            LearningEvent(id: "e-\(i)", t: t0.addingTimeInterval(Double(i) * 86_400), type: .outcome,
                          item: "p#\(i)", modality: .hearWrite, components: [kot], grade: .good)
        }
        tracker.record(events)
        tracker.saveSnapshot()

        // Changing FSRS weights invalidates the snapshot and falls back to a full fold (and writes a new snapshot since count >= 20).
        let scaled = FSRS().scalingStability(by: 0.85)
        let withNewWeights = KnowledgeTracker(course: course, root: root, customWords: [], fsrs: scaled, now: { t0 })
        XCTAssertFalse(withNewWeights.loadedFromSnapshot)
        XCTAssertEqual(withNewWeights.foldCount, 1)
        XCTAssertEqual(withNewWeights.model.fsrs, scaled)
        XCTAssertEqual(withNewWeights.model, KnowledgeModel(events: EventLog(url: withNewWeights.logURL).load(), fsrs: scaled))

        // Subsequent launch with the same scaled FSRS loads from the refreshed snapshot.
        let warmScaled = KnowledgeTracker(course: course, root: root, customWords: [], fsrs: scaled, now: { t0 })
        XCTAssertTrue(warmScaled.loadedFromSnapshot)
        XCTAssertEqual(warmScaled.foldCount, 0)

        // Truncating events.jsonl below the snapshot's byteOffset invalidates the snapshot and falls back to a full fold.
        let log = EventLog(url: tracker.logURL)
        try FileManager.default.removeItem(at: log.url)
        XCTAssertTrue(log.append(Array(events.prefix(5))))
        let afterTruncate = KnowledgeTracker(course: course, root: root, customWords: [], fsrs: scaled, now: { t0 })
        XCTAssertFalse(afterTruncate.loadedFromSnapshot)
        XCTAssertEqual(afterTruncate.foldCount, 1)
        XCTAssertEqual(afterTruncate.model.events.count, 5)
    }

    func testBookLaunchWithStabilityScaleFoldsOnce() {
        let store = tempStore()
        let book = Book(course: course, store: store)
        var d = Exercise(kind: .dictation); d.text = "кот"
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [d], newWords: [])
        book.record(page: book.currentPage, exercise: 0) { $0.attempts = 1; $0.recognized = "кот"; $0.status = .correct }
        // Set a learned stabilityScale in day state without rebuilding in-process, so snapshot on disk has unscaled FSRS.
        book.day.stabilityScale = 0.85
        book.saveNow()

        let reopened = Book(course: course, store: store)
        XCTAssertEqual(reopened.knowledge.foldCount, 1, "Book.init folds once directly with scaled FSRS")
        XCTAssertEqual(reopened.knowledge.model.fsrs, FSRS().scalingStability(by: 0.85))
    }
}
