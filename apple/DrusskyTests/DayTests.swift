import LearningCore
import XCTest
@testable import Drussky

/// The shape of a day in the notebook: morning and welcome-back pages, the daily cap, probes,
/// the desk shortening a page, and notebooks from before any of this existed.
@MainActor
final class DayTests: XCTestCase {
    var course: Course {
        let url = Bundle.main.url(forResource: "course", withExtension: "json")!
        return try! JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    func tempStore() -> BookStore {
        BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
    }

    /// A book with `n` course pages worked through today, standing on an unfinished page after them.
    func studied(_ n: Int, store: BookStore? = nil) -> Book {
        let book = Book(course: course, store: store ?? tempStore())
        for p in 0..<n {
            QA.complete(book, page: p, upTo: .max)
            book.turnForward()
        }
        return book
    }

    /// Indices of the page's review exercises (probes are not reviews) and of its probes.
    func reviews(_ page: BookPage, _ book: Book) -> [Int] {
        page.exercises.indices.filter { page.exercises[$0].reviewKey != nil && !book.isProbe(page: page, index: $0) }
    }
    func probes(_ page: BookPage, _ book: Book) -> [Int] { page.exercises.indices.filter { book.isProbe(page: page, index: $0) } }
    func reviewCount(_ page: BookPage, _ book: Book) -> Int { reviews(page, book).count }

    func testMorningPageOpensANewDay() throws {
        let book = studied(3)
        let sameDay = book.pages.count
        XCTAssertNil(book.day.lastOpeningKind, "pages made during the day are ordinary")
        book.advanceClock(byDays: 1)
        XCTAssertTrue(book.turnForward())
        let page = try XCTUnwrap(book.pages.last)
        XCTAssertEqual(book.pages.count, sameDay + 1)
        XCTAssertEqual(book.day.lastOpeningKind, "morning")
        XCTAssertNotNil(page.sourceID, "the morning page carries the next course page's material")
        XCTAssertFalse(page.isReview)
        let n = reviewCount(page, book)
        XCTAssertGreaterThan(n, 0, "yesterday's items are due for their first recall")
        XCTAssertLessThanOrEqual(n, book.dayPolicy.morningReviews)
        XCTAssertEqual(reviews(page, book), Array(0..<n), "reviews first, then the course")
        let course = page.exercises.indices.filter { page.exercises[$0].reviewKey == nil }
        XCTAssertFalse(course.isEmpty)
        XCTAssertTrue(course.allSatisfy { $0 >= n })
        XCTAssertTrue(probes(page, book).allSatisfy { $0 > course.last! }, "probes, if any, come last")
        XCTAssertTrue(page.intro?.hasPrefix("A new day.") ?? false, page.intro ?? "")
        let keys = page.exercises.compactMap(\.reviewKey)
        XCTAssertEqual(Set(keys).count, keys.count, "no item twice, probes included")
        // The course advanced by id: the page it took is not offered again.
        XCTAssertNotEqual(book.nextCoursePage?.page.id, page.sourceID)
        // The next page the same day is an ordinary one.
        book.turnForward()
        XCTAssertFalse(book.pages.last?.intro?.hasPrefix("A new day.") ?? false)
        XCTAssertEqual(book.day.lastOpeningKind, "morning")
    }

    func testWelcomeBackAfterALongAbsence() throws {
        let book = studied(3)
        book.advanceClock(byDays: 12)
        let due = book.dueReviews().count
        XCTAssertGreaterThan(due, 0)
        XCTAssertTrue(book.turnForward())
        let page = try XCTUnwrap(book.pages.last)
        XCTAssertEqual(page.unitTitle, Book.welcomeBackUnit)
        XCTAssertTrue(page.isReview)
        XCTAssertNil(page.sourceID, "nothing new on the first page back")
        XCTAssertEqual(reviewCount(page, book), min(due, book.dayPolicy.welcomeBackSample), "a sample, not everything")
        XCTAssertTrue(page.intro?.contains("12 days away") ?? false, page.intro ?? "")
        XCTAssertEqual(book.day.lastOpeningKind, "welcome-back")
        XCTAssertTrue(book.coverLine?.hasPrefix("Welcome back") ?? false, book.coverLine ?? "nil")
        // Then the course goes on.
        book.turnForward()
        XCTAssertNotNil(book.pages.last?.sourceID)
    }

    func testDailyCapHoldsAcrossTheDaysPages() throws {
        let book = studied(4)
        book.settings.dailyReviewCap = 5
        book.advanceClock(byDays: 3)
        XCTAssertGreaterThan(book.dueReviews().count, 5)
        book.turnForward()
        let morning = try XCTUnwrap(book.pages.last)
        XCTAssertEqual(reviewCount(morning, book), 5, "the morning page takes what the day allows")
        XCTAssertEqual(book.reviewAllowance(), 0)
        XCTAssertGreaterThan(book.day.deferredAtOpening ?? 0, 0)
        XCTAssertTrue(book.coverLine?.contains("wait for tomorrow") ?? false, book.coverLine ?? "nil")
        book.turnForward()
        XCTAssertEqual(reviewCount(book.pages.last!, book), 0, "the cap holds for the rest of the day")
        XCTAssertNotNil(book.pages.last?.sourceID, "the course still goes on")
        // Tomorrow the allowance is back and the deferred items come first.
        book.advanceClock(byDays: 1)
        XCTAssertEqual(book.reviewAllowance(), 5)
        book.turnForward()
        XCTAssertEqual(reviewCount(book.pages.last!, book), 5)
    }

    func testRetentionTargetDrivesWhenReviewsComeDue() throws {
        let book = studied(2)
        book.advanceClock(byDays: 1.5)
        let strict = book.dueReviews().count
        book.settings.desiredRetention = 0.95
        XCTAssertEqual(book.reviewPolicy.retention, 0.95)
        XCTAssertGreaterThanOrEqual(book.dueReviews().count, strict, "aiming higher brings reviews sooner")
        book.settings.desiredRetention = 0.8
        XCTAssertLessThanOrEqual(book.dueReviews().count, strict)
    }

    func testProbesArePlacedOnPagesAndObserved() throws {
        let store = tempStore()
        let book = studied(4, store: store)
        // The first probe set was drawn on the first page made once anything had been recalled.
        XCTAssertNotNil(book.day.probes.lastDrawn)
        let drawn = book.day.probes.probes
        XCTAssertFalse(drawn.isEmpty)
        XCTAssertTrue(drawn.allSatisfy(\.isPending), "the first probes come a day later")
        XCTAssertTrue(probes(book.pages.last!, book).isEmpty)
        book.advanceClock(byDays: 1.1)
        book.turnForward()
        let page = try XCTUnwrap(book.pages.last)
        let placed = book.day.probes.probes.filter { $0.item?.hasPrefix(page.id + "#") ?? false }
        XCTAssertFalse(placed.isEmpty, "the one-day probes are on the page")
        XCTAssertLessThanOrEqual(placed.count, 3)
        XCTAssertEqual(probes(page, book).count, placed.count)
        let probe = placed[0]
        let index = Int(probe.item!.split(separator: "#")[1])!
        let ex = page.exercises[index]
        XCTAssertEqual(ex.reviewKey, probe.candidate, "a probe looks like an ordinary review of its item")
        XCTAssertEqual(RussianDomain.modality(for: ex.kind), probe.modality)
        let keys = page.exercises.compactMap(\.reviewKey)
        XCTAssertEqual(Set(keys).count, keys.count, "a probe never doubles a review already on the page")
        // Probes aren't reviews for the cap.
        XCTAssertEqual(book.reviewAllowance(), book.dayPolicy.reviewCap - reviewCount(page, book))
        // Answer it right first time.
        book.record(page: book.pages.count - 1, exercise: index) { r in
            r.attempts = 1
            switch ex.kind {
            case .listenPick: r.chosen = ex.options?.firstIndex(of: ex.say ?? "")
            case .readMeaning: r.chosen = ex.answerIndex
            case .readAloud: r.recognized = ex.text
            default: r.recognized = ex.target
            }
            r.status = .correct
        }
        let answered = try XCTUnwrap(book.day.probes.probes.first { $0.id == probe.id })
        XCTAssertEqual(answered.observed, true)
        let predicted = try XCTUnwrap(answered.predicted)
        XCTAssertTrue(predicted > 0 && predicted < 1, "\(predicted)")
        XCTAssertEqual(book.calibration.overall?.count, 1)
        // The export carries it; the notebook keeps it.
        let json = try JSONSerialization.jsonObject(with: Data(contentsOf: XCTUnwrap(book.exportGraph()))) as? [String: Any]
        XCTAssertNotNil(json?["calibration"])
        book.saveNow()
        let again = Book(course: course, store: store)
        XCTAssertEqual(again.day.probes.probes.count, book.day.probes.probes.count)
        XCTAssertEqual(again.day.probes.observations.count, 1)
        // Switched off: pending probes stay off the page.
        again.settings.probes = false
        again.advanceClock(byDays: 5)
        again.turnForward()
        XCTAssertTrue(again.day.probes.probes.filter { $0.item?.hasPrefix(again.pages.last!.id) ?? false }.isEmpty)
    }

    /// A book on a page with probes placed, and the first probe's index on that page.
    private func withPlacedProbe(store: BookStore) throws -> (book: Book, probe: Probe, index: Int) {
        let book = studied(4, store: store)
        book.advanceClock(byDays: 1.1)
        book.turnForward()
        let page = try XCTUnwrap(book.pages.last)
        let probe = try XCTUnwrap(book.day.probes.probes.first { $0.item?.hasPrefix(page.id + "#") ?? false })
        return (book, probe, Int(probe.item!.split(separator: "#")[1])!)
    }

    private func answerRight(_ book: Book, _ index: Int) {
        let page = book.pages[book.pages.count - 1]
        let ex = page.exercises[index]
        book.record(page: book.pages.count - 1, exercise: index) { r in
            r.attempts = 1
            switch ex.kind {
            case .listenPick: r.chosen = ex.options?.firstIndex(of: ex.say ?? "")
            case .readMeaning: r.chosen = ex.answerIndex
            case .readAloud: r.recognized = ex.text
            default: r.recognized = ex.target
            }
            r.status = .correct
        }
    }

    /// The result's save (synchronous when an exercise finishes) carries the probe's observation:
    /// a crash right after it cannot leave a finished exercise with an unanswered probe.
    func testAProbeObservationIsSavedWithItsResult() throws {
        let store = tempStore()
        let (book, probe, index) = try withPlacedProbe(store: store)
        answerRight(book, index)
        // Straight from disk, before any debounced save could run.
        let saved = try BookStore.decoder().decode(BookState.self, from: Data(contentsOf: store.stateURL))
        let onDisk = try XCTUnwrap(saved.day?.probes.probes.first { $0.id == probe.id })
        XCTAssertNotNil(onDisk.answered)
        XCTAssertEqual(onDisk.observed, true)
        let predicted = try XCTUnwrap(onDisk.predicted)
        XCTAssertTrue(predicted > 0 && predicted < 1, "\(predicted)")
        XCTAssertEqual(saved.pages.last?.results[index].status, .correct)
    }

    /// A notebook from before that: the exercise finished and its outcome logged, the probe still
    /// placed and unanswered. Opening reconciles it from the saved result and the trace.
    func testAStrandedProbeIsReconciledOnOpening() throws {
        let store = tempStore()
        let (book, probe, index) = try withPlacedProbe(store: store)
        answerRight(book, index)
        let live = try XCTUnwrap(book.day.probes.probes.first { $0.id == probe.id })
        var state = try BookStore.decoder().decode(BookState.self, from: Data(contentsOf: store.stateURL))
        var day = try XCTUnwrap(state.day)
        let k = try XCTUnwrap(day.probes.probes.firstIndex { $0.id == probe.id })
        day.probes.probes[k].answered = nil
        day.probes.probes[k].observed = nil
        day.probes.probes[k].predicted = nil
        state.day = day
        try BookStore.encoder().encode(state).write(to: store.stateURL)
        let again = Book(course: course, store: store)
        let fixed = try XCTUnwrap(again.day.probes.probes.first { $0.id == probe.id })
        XCTAssertNotNil(fixed.answered)
        XCTAssertEqual(fixed.observed, true)
        XCTAssertEqual(try XCTUnwrap(fixed.predicted), try XCTUnwrap(live.predicted), accuracy: 1e-6, "the retrievability just before that review")
        XCTAssertEqual(again.calibration.overall?.count, book.calibration.overall?.count)
        // Written back, so the next opening has nothing to do.
        let saved = try BookStore.decoder().decode(BookState.self, from: Data(contentsOf: store.stateURL))
        XCTAssertNotNil(saved.day?.probes.probes.first { $0.id == probe.id }?.answered)
    }

    func testLearnedParametersSurviveReopening() throws {
        let store = tempStore()
        let book = Book(course: course, store: store)
        book.day.stabilityScale = 0.9
        book.applyLearnedParameters()
        XCTAssertEqual(book.knowledge.model.fsrs.initialStability(.good), 0.9 * FSRS().initialStability(.good), accuracy: 1e-9)
        book.saveNow()
        let again = Book(course: course, store: store)
        XCTAssertEqual(again.knowledge.model.fsrs.w[0], 0.9 * FSRS.defaultWeights[0], accuracy: 1e-9)
    }

    func testATiredLearnerGetsAShorterPage() throws {
        let book = Book(course: course, store: tempStore())
        let words = ["кот", "мама", "окно", "она", "книга", "дом", "сок", "нос", "рот", "стол", "ты", "мы", "он", "это"]
        let exercises: [Exercise] = words.map { w in
            var d = Exercise(kind: .dictation); d.text = w; d.reviewKey = "word:\(w)"; return d
        }
        book.insertWrittenPage(title: "long", intro: nil, unitTitle: "t", exercises: exercises, newWords: [])
        let p = book.currentPage
        let start = book.now()
        var clock = start
        book.now = { clock }
        // Six quick right answers…
        for i in 0..<6 {
            clock = clock.addingTimeInterval(6)
            book.record(page: p, exercise: i) { $0.attempts = 1; $0.recognized = words[i]; $0.status = .correct }
        }
        XCTAssertEqual(book.day.fatigue?.level, .sharp, "\(String(describing: book.day.fatigue))")
        XCTAssertNil(book.deskLine(for: p))
        // …then slow, failing ones, half an hour in.
        for i in 6..<9 {
            clock = clock.addingTimeInterval(300)
            book.record(page: p, exercise: i) { $0.attempts += 1; $0.failedChecks = 1; $0.recognized = QA.misreading(words[i]) }
            clock = clock.addingTimeInterval(280)
            book.record(page: p, exercise: i) { $0.attempts += 1; $0.revealed = true; $0.status = .missed }
        }
        let estimate = try XCTUnwrap(book.day.fatigue)
        XCTAssertEqual(estimate.level, .tired, "\(estimate)")
        let page = book.pages[p]
        let line = try XCTUnwrap(book.deskLine(for: p))
        XCTAssertTrue(line.hasPrefix("Shorter page today"), line)
        XCTAssertEqual(book.coverLine, "Shorter page today — you've been at it a while.")
        // Two of the remaining reviews became recognition; the rest were left for later, never graded.
        let remaining = Array(9..<14)
        XCTAssertEqual(remaining.filter { page.exercises[$0].kind == .listenPick || page.exercises[$0].kind == .readMeaning }, [9, 10])
        XCTAssertEqual(remaining.filter { page.results[$0].status == .skipped }, [11, 12, 13])
        XCTAssertTrue([11, 12, 13].allSatisfy { page.exercises[$0].kind == .dictation })
        XCTAssertEqual(page.exercises[9].reviewKey, "word:\(words[9])", "the same item, asked by recognition")
        XCTAssertTrue(page.results[9].status == .open && page.results[10].status == .open)
        XCTAssertFalse(page.isFinished)
        // Answered ones are untouched.
        XCTAssertEqual(page.results[0].status, .correct)
        XCTAssertEqual(page.results[6].status, .missed)
        // Memory for the skipped items didn't move.
        XCTAssertNil(book.knowledge.model.trace(ComponentID(.lexeme, words[12]), .hearWrite))
        // The desk shortens a page once: answering on goes on as usual.
        clock = clock.addingTimeInterval(300)
        book.record(page: p, exercise: 9) { $0.attempts = 1; $0.chosen = 0; $0.status = .done }
        XCTAssertEqual(book.pages[p].results[10].status, .open)
    }

    /// Shortening never takes an exercise the learner has begun: ink on its strip (the writing may be
    /// under the recognizer), a take recorded. A replaced one starts a new round, so a check that
    /// began on the old exercise lands nowhere instead of ticking the new one.
    func testShorteningSparesWorkUnderway() throws {
        let book = Book(course: course, store: tempStore())
        let words = ["кот", "мама", "окно", "она", "книга", "дом", "сок", "нос", "рот", "стол", "ты", "мы", "он", "это"]
        let exercises: [Exercise] = words.map { w in
            var d = Exercise(kind: .dictation); d.text = w; d.reviewKey = "word:\(w)"; return d
        }
        book.insertWrittenPage(title: "long", intro: nil, unitTitle: "t", exercises: exercises, newWords: [])
        let p = book.currentPage
        let page0 = book.pages[p]
        var clock = book.now()
        book.now = { clock }
        // Writing is under way on 9 (ink on disk, Vision pending); a take was recorded on 10.
        try Data("ink".utf8).write(to: book.inkURL(page: page0, exercise: 9))
        book.record(page: p, exercise: 10) { $0.recording = "t.m4a" }
        let pendingRound = book.pages[p].results[11].roundNumber
        for i in 0..<6 {
            clock = clock.addingTimeInterval(6)
            book.record(page: p, exercise: i) { $0.attempts = 1; $0.recognized = words[i]; $0.status = .correct }
        }
        for i in 6..<9 {
            clock = clock.addingTimeInterval(300)
            book.record(page: p, exercise: i) { $0.attempts += 1; $0.failedChecks = 1; $0.recognized = QA.misreading(words[i]) }
            clock = clock.addingTimeInterval(280)
            book.record(page: p, exercise: i) { $0.attempts += 1; $0.revealed = true; $0.status = .missed }
        }
        XCTAssertNotNil(book.deskLine(for: p))
        let page = book.pages[p]
        XCTAssertEqual(page.exercises[9].kind, .dictation, "ink under way: left alone")
        XCTAssertEqual(page.results[9].status, .open)
        XCTAssertEqual(page.exercises[10].kind, .dictation, "a take recorded: left alone")
        XCTAssertEqual(page.results[10].status, .open)
        XCTAssertTrue([.listenPick, .readMeaning].contains(page.exercises[11].kind))
        XCTAssertTrue([.listenPick, .readMeaning].contains(page.exercises[12].kind))
        XCTAssertEqual(page.results[13].status, .skipped)
        XCTAssertEqual(page.results[11].roundNumber, pendingRound + 1, "a replaced exercise is a new round")
        // The handwriting check that began on the old exercise 11 compares rounds and drops out.
        let ref = ExerciseRef(pageID: page.id, index: 11)
        XCTAssertNotEqual(book.result(ref)?.roundNumber, pendingRound)
        XCTAssertEqual(book.commitTake(ref, round: pendingRound, file: "late.m4a", heard: words[11], target: words[11],
                                       recognitionAllowed: true, timedOut: false), .keptOnly)
        XCTAssertEqual(book.result(ref)?.status, .open)
    }

    /// Back after twelve days, the learner answers one thing on the page left open before turning
    /// it: the absence still opens with a welcome-back page. The same for an ordinary morning.
    func testAnAnswerBeforeTheFirstTurnDoesNotHideTheAbsence() throws {
        let store = tempStore()
        let book = studied(3, store: store)
        let open = book.currentPage
        book.advanceClock(byDays: 12)
        book.noteSessionStart()  // the launch (Book.init does this too, with the clock of the device)
        XCTAssertEqual(book.day.pendingOpening, "welcome-back")
        XCTAssertEqual(book.day.pendingOpeningDays, 12)
        QA.complete(book, page: open, upTo: 1)
        XCTAssertTrue(Calendar.current.isDate(try XCTUnwrap(book.lastActivity), inSameDayAs: book.now()), "the answer is today's")
        book.turnForward()
        XCTAssertEqual(book.pages.last?.unitTitle, Book.welcomeBackUnit)
        XCTAssertNil(book.day.pendingOpening, "consumed")
        XCTAssertEqual(book.day.lastOpeningKind, "welcome-back")
        // The next page is ordinary; the decision survives a relaunch until it is used.
        book.turnForward()
        XCTAssertNotEqual(book.pages.last?.unitTitle, Book.welcomeBackUnit)
        book.advanceClock(byDays: 1)
        book.noteSessionStart()
        XCTAssertEqual(book.day.pendingOpening, "morning")
        book.saveNow()
        let again = Book(course: course, store: store, now: book.now)
        XCTAssertEqual(again.day.pendingOpening, "morning", "persisted")
        QA.complete(again, page: again.currentPage, upTo: 2)
        again.turnForward()
        XCTAssertEqual(again.day.lastOpeningKind, "morning")
        XCTAssertTrue(again.pages.last?.intro?.hasPrefix("A new day.") ?? false, again.pages.last?.intro ?? "")
    }

    /// The daily cap holds on a return too: five allowed, eight due, five placed.
    func testWelcomeBackKeepsToTheDailyCap() throws {
        let book = studied(4)
        book.settings.dailyReviewCap = 5
        book.advanceClock(byDays: 9)
        XCTAssertGreaterThan(book.dueReviews().count, 5)
        book.turnForward()
        let page = try XCTUnwrap(book.pages.last)
        XCTAssertEqual(page.unitTitle, Book.welcomeBackUnit)
        XCTAssertEqual(reviewCount(page, book), 5)
        XCTAssertEqual(book.reviewAllowance(), 0)
        book.turnForward()
        XCTAssertEqual(reviewCount(book.pages.last!, book), 0, "the allowance is spent for the day")
    }

    /// A format-2 notebook written before the day's shape existed: no `day`, old settings.
    static let formatTwoNotebook = """
    {"version":2,"courseCursor":1,"currentPage":0,"memories":[],
     "settings":{"fingerDrawing":false,"cursiveModels":true,"showStress":true},
     "targets":[{"key":"word:кот","introduced":"2026-10-01T09:00:00Z"}],
     "pages":[{"id":"p0001-abc123","number":1,"sourceID":"u1-p1","unitTitle":"Familiar faces","title":"First",
       "isReview":false,"created":"2026-10-01T08:50:00Z","finished":"2026-10-01T09:05:00Z",
       "exercises":[{"kind":"dictation","text":"кот"}],
       "results":[{"status":"correct","attempts":1,"recognized":"кот","revealed":false,"updated":"2026-10-01T09:00:00Z"}]}]}
    """

    func testNotebookWithoutDayStateDecodesAndGainsIt() throws {
        let store = tempStore()
        try Data(Self.formatTwoNotebook.utf8).write(to: store.stateURL)
        let book = Book(course: course, store: store)
        XCTAssertNil(book.problem)
        XCTAssertTrue(book.repairs.isEmpty)
        XCTAssertEqual(book.pages.count, 1)
        // Opening a notebook last written weeks ago decides the day's opening at once; nothing else.
        XCTAssertEqual(book.day.pendingOpening, "welcome-back")
        var expected = DayState()
        expected.pendingOpening = book.day.pendingOpening
        expected.pendingOpeningDays = book.day.pendingOpeningDays
        expected.pendingOpeningSince = book.day.pendingOpeningSince
        XCTAssertEqual(book.day, expected)
        XCTAssertTrue(book.settings.cursiveModels)
        XCTAssertNil(book.settings.desiredRetention)
        XCTAssertEqual(book.reviewPolicy.retention, 0.9)
        XCTAssertEqual(book.dayPolicy.reviewCap, 25)
        XCTAssertTrue(book.probePolicy.enabled)
        XCTAssertNil(book.coverLine)
        // The learner was away long enough: a welcome-back page (кот is due), then saved with `day`.
        book.turnForward()
        XCTAssertEqual(book.pages.last?.unitTitle, Book.welcomeBackUnit)
        book.settings.dailyReviewCap = 30
        book.saveNow()
        let json = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(contentsOf: store.stateURL)) as? [String: Any])
        XCTAssertEqual(json["version"] as? Int, 2, "optional fields only: the format did not change")
        XCTAssertNotNil(json["day"])
        XCTAssertEqual((json["settings"] as? [String: Any])?["dailyReviewCap"] as? Int, 30)
        let again = Book(course: course, store: store)
        XCTAssertEqual(again.settings.dailyReviewCap, 30)
        XCTAssertEqual(again.day.lastOpeningKind, "welcome-back")
    }
}
