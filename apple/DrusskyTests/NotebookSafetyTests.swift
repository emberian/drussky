import LearningCore
import PencilKit
import XCTest
@testable import Drussky

/// Failure scenarios for the learner's notebook: nothing here may lose, overwrite or
/// misplace what was written, said or answered.
@MainActor
final class NotebookSafetyTests: XCTestCase {
    var course: Course {
        let url = Bundle.main.url(forResource: "course", withExtension: "json")!
        return try! JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    func tempRoot() -> URL { FileManager.default.temporaryDirectory.appendingPathComponent("nb-\(UUID().uuidString)") }

    private func files(_ root: URL) -> [String] {
        (try? FileManager.default.contentsOfDirectory(atPath: root.path)) ?? []
    }

    // MARK: Opening

    func testUndecodableNotebookIsNeverReplacedAndGetsAVerifiedCopy() throws {
        let store = BookStore(root: tempRoot())
        let original = Data("{ not a notebook".utf8)
        try original.write(to: store.stateURL)
        guard case .failure(let problem) = Book.open(course: course, store: store) else { return XCTFail("opened garbage") }
        XCTAssertEqual(problem.kind, .undecodable)
        let backup = try XCTUnwrap(problem.backup)
        XCTAssertEqual(try Data(contentsOf: backup), original)
        // Even a Book built over it never writes.
        let book = Book(course: course, store: store)
        XCTAssertTrue(book.pages.isEmpty, "no fresh notebook is started over an existing one")
        book.saveNow()
        XCTAssertEqual(try Data(contentsOf: store.stateURL), original)
    }

    /// Quarantine fails (the folder can't be written): still nothing is overwritten.
    func testFailedQuarantineStillLeavesTheNotebookUntouched() throws {
        let root = tempRoot()
        let store = BookStore(root: root)
        let original = Data(#"{"version":1,"pages":"#.utf8)
        try original.write(to: store.stateURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o555], ofItemAtPath: root.path)
        defer { try? FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: root.path) }
        guard case .failure(let problem) = Book.open(course: course, store: store) else { return XCTFail() }
        XCTAssertNil(problem.backup, "the copy could not be written")
        let book = Book(course: course, store: store)
        book.saveNow()
        XCTAssertEqual(try Data(contentsOf: store.stateURL), original)
        XCTAssertFalse(files(root).contains { $0.hasPrefix("book-unreadable") })
    }

    /// A file that exists but can't be read is not "no notebook".
    func testUnreadableFileIsNotTreatedAsMissing() throws {
        let store = BookStore(root: tempRoot())
        try Data(KnowledgeTests.oldNotebook.utf8).write(to: store.stateURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o000], ofItemAtPath: store.stateURL.path)
        defer { try? FileManager.default.setAttributes([.posixPermissions: 0o644], ofItemAtPath: store.stateURL.path) }
        guard case .unavailable(let problem) = store.load() else { return XCTFail("unreadable file loaded or counted as missing") }
        XCTAssertEqual(problem.kind, .unreadable)
    }

    func testNotebookFromANewerAppIsRefused() throws {
        let store = BookStore(root: tempRoot())
        let newer = KnowledgeTests.oldNotebook.replacingOccurrences(of: #""version":1"#, with: #""version":99"#)
        try Data(newer.utf8).write(to: store.stateURL)
        guard case .failure(let problem) = Book.open(course: course, store: store) else { return XCTFail() }
        XCTAssertEqual(problem.kind, .newerVersion)
        XCTAssertEqual(try String(contentsOf: store.stateURL, encoding: .utf8), newer)
    }

    func testStartingFreshMovesTheOldNotebookAsideIntact() throws {
        let store = BookStore(root: tempRoot())
        let original = Data("garbage".utf8)
        try original.write(to: store.stateURL)
        let aside = try store.setAside()
        XCTAssertEqual(try Data(contentsOf: aside), original)
        guard case .success(let book) = Book.open(course: course, store: store) else { return XCTFail() }
        XCTAssertEqual(book.pages.count, 1)
        book.saveNow()
        XCTAssertEqual(try Data(contentsOf: aside), original)
    }

    /// Duplicate and malformed memory keys and misaligned results decode, are repaired without
    /// trapping, and the original is copied aside before anything is saved.
    func testBrokenInvariantsAreRepairedNotTrapped() throws {
        let store = BookStore(root: tempRoot())
        var json = KnowledgeTests.oldNotebook
        json = json.replacingOccurrences(of: #""memories":["#, with: #""memories":[{"key":"letter","stability":1,"due":"2026-10-02T09:00:00Z","reps":0,"lapses":0,"seen":"2026-10-01T09:00:00Z"},{"key":"word:кот","stability":3,"due":"2026-10-03T09:00:00Z","reps":2,"lapses":0,"seen":"2026-10-01T10:00:00Z"},"#)
        // Drop the last result: five exercises, four results.
        let lastResult = #"{"status":"open","attempts":0,"revealed":false}]"#
        let cut = try XCTUnwrap(json.range(of: lastResult))
        let comma = try XCTUnwrap(json[..<cut.lowerBound].lastIndex(of: ","))
        json.replaceSubrange(comma..<cut.upperBound, with: "]")
        let original = Data(json.utf8)
        try original.write(to: store.stateURL)
        guard case .success(let book) = Book.open(course: course, store: store) else { return XCTFail("repairable notebook refused") }
        XCTAssertEqual(book.pages[0].results.count, book.pages[0].exercises.count)
        XCTAssertFalse(book.repairs.isEmpty)
        let backup = try XCTUnwrap(files(store.root).first { $0.hasPrefix("book-before-repair") && $0.hasSuffix(".json") })
        XCTAssertEqual(try Data(contentsOf: store.root.appendingPathComponent(backup)), original)
        // A malformed key never reaches review generation, and review generation can't trap on one.
        _ = book.reviewExercise(for: "letter", modality: nil, variant: 0)
        XCTAssertNil(book.targets["letter"])
        _ = ExerciseSlot(book: book, pageIndex: 0, index: 4).result
    }

    // MARK: Results

    /// The closure may read the book (the pattern that trapped on the iPad).
    func testRecordClosureMayReadTheBook() throws {
        let book = Book(course: course, store: BookStore(root: tempRoot()))
        book.record(page: 0, exercise: 0) { r in
            r.recognized = "\(book.pages.count) \(book.pages[0].results[0].attempts)"
            r.status = .done
        }
        XCTAssertEqual(book.pages[0].results[0].status, .done)
    }

    /// A finished exercise is on disk at once, before its outcome is logged.
    func testFinishingIsSavedBeforeTheOutcomeIsLogged() throws {
        let store = BookStore(root: tempRoot())
        let book = Book(course: course, store: store)
        book.record(page: 0, exercise: 0) { $0.status = .done }
        let reopened = Book(course: course, store: store)
        XCTAssertEqual(reopened.pages[0].results[0].status, .done)
    }

    /// An attempt was logged, then the completion's append failed: the next launch reconstructs
    /// the missing outcome (an attempt never counts as one), once.
    func testCompletionWhoseAppendFailedIsBackfilledOnce() throws {
        let store = BookStore(root: tempRoot())
        let book = Book(course: course, store: store)
        let i = try XCTUnwrap(book.pages[0].exercises.firstIndex { $0.kind == .listenPick })
        let ex = book.pages[0].exercises[i]
        let wrong = try XCTUnwrap(ex.options?.firstIndex { $0 != ex.say })
        book.record(page: 0, exercise: i) { $0.chosen = wrong; $0.attempts += 1 }
        let log = book.knowledge.logURL
        XCTAssertEqual(EventLog(url: log).load().filter { $0.item.hasSuffix("#\(i)") }.map(\.type), [.attempt])
        try FileManager.default.setAttributes([.posixPermissions: 0o444], ofItemAtPath: log.path)
        book.record(page: 0, exercise: i) { $0.chosen = ex.options?.firstIndex(of: ex.say ?? ""); $0.attempts += 1; $0.status = .done }
        XCTAssertNotNil(book.knowledge.logProblem)
        try FileManager.default.setAttributes([.posixPermissions: 0o644], ofItemAtPath: log.path)
        _ = Book(course: course, store: store)
        _ = Book(course: course, store: store)
        let outcomes = EventLog(url: log).load().filter { $0.item.hasSuffix("#\(i)") && $0.type == .outcome }
        XCTAssertEqual(outcomes.count, 1, "\(outcomes)")
    }

    func testRedoAfterRevealKeepsTheAssistanceOnRecord() throws {
        let book = Book(course: course, store: BookStore(root: tempRoot()))
        book.record(page: 0, exercise: 0) { $0.revealed = true; $0.status = .missed }
        book.reopen(page: 0, exercise: 0)
        let r = book.pages[0].results[0]
        XCTAssertEqual(r.assisted, true)
        XCTAssertEqual(r.roundNumber, 1)
        XCTAssertFalse(r.revealed)
    }

    // MARK: Ink

    private func drawingData(_ text: String) -> Data { SyntheticInk.drawing(for: text, height: 132).dataRepresentation() }

    /// Writes arriving out of order: the older one (or an older "empty", which deletes) loses.
    func testReversedInkWritesKeepTheNewest() async throws {
        let root = tempRoot()
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let url = root.appendingPathComponent("a.drawing")
        let writer = InkWriter(delay: .seconds(60))
        let newer = drawingData("кот"), older = drawingData("к")
        await writer.submit(url, revision: 2, data: newer)
        await writer.submit(url, revision: 1, data: older)
        await writer.flush()
        XCTAssertEqual(try Data(contentsOf: url), newer)
        await writer.submit(url, revision: 1, data: nil)
        await writer.flush()
        XCTAssertEqual(try Data(contentsOf: url), newer, "a stale empty drawing must not delete newer ink")
        await writer.submit(url, revision: 3, data: nil)
        await writer.flush()
        XCTAssertFalse(FileManager.default.fileExists(atPath: url.path))
    }

    /// Export drains ink that hasn't reached its file yet.
    func testExportIncludesPendingInk() async throws {
        let root = tempRoot()
        let store = BookStore(root: root, ink: InkWriter(delay: .seconds(60)))
        let book = Book(course: course, store: store)
        let url = book.inkURL(page: book.pages[0], exercise: 0)
        await store.ink.submit(url, revision: 1, data: drawingData("А"))
        XCTAssertFalse(FileManager.default.fileExists(atPath: url.path))
        let exported = await book.exportArchive()
        let zip = try XCTUnwrap(exported)
        let names = try ZipReader(data: Data(contentsOf: zip)).entries().map(\.name)
        XCTAssertTrue(names.contains { $0.hasSuffix("ink/\(url.lastPathComponent)") }, "\(names)")
    }

    /// "Write it again" keeps the first writing as an earlier attempt.
    func testWritingAgainKeepsThePreviousInk() async throws {
        let store = BookStore(root: tempRoot())
        let book = Book(course: course, store: store)
        let ref = ExerciseRef(pageID: book.pages[0].id, index: 0)
        let url = book.inkURL(page: book.pages[0], exercise: 0)
        let first = drawingData("А а")
        try first.write(to: url)
        book.record(ref) { $0.status = .done }
        let pad = PadController(fileURL: url, writer: store.ink)
        let dest = book.inkAttemptURL(ref, attempt: 1)
        try await pad.archive(to: dest)
        book.reopen(ref, keptInk: true)
        await store.ink.flush()
        XCTAssertEqual(try Data(contentsOf: dest), first)
        XCTAssertFalse(FileManager.default.fileExists(atPath: url.path), "the new round starts on a clean strip")
        XCTAssertEqual(book.result(ref)?.inkAttempts, 1)
    }

    // MARK: Speech

    func testRetakeKeepsThePreviousRecording() throws {
        let book = Book(course: course, store: BookStore(root: tempRoot()))
        let i = try XCTUnwrap(book.pages[0].exercises.firstIndex { $0.kind == .readAloud })
        let ref = ExerciseRef(pageID: book.pages[0].id, index: i)
        let target = book.pages[0].exercises[i].text ?? ""
        let a = book.newTakeURL(ref), b = book.newTakeURL(ref)
        XCTAssertNotEqual(a, b)
        try Data("one".utf8).write(to: a)
        XCTAssertEqual(book.commitTake(ref, round: 0, file: a.lastPathComponent, heard: "нет", target: target,
                                       recognitionAllowed: true, timedOut: false), .recorded)
        try Data("two".utf8).write(to: b)
        book.commitTake(ref, round: 0, file: b.lastPathComponent, heard: nil, target: target, recognitionAllowed: true, timedOut: true)
        let r = try XCTUnwrap(book.result(ref))
        XCTAssertEqual(r.recording, b.lastPathComponent)
        XCTAssertEqual(r.takes, [a.lastPathComponent])
        XCTAssertEqual(try String(contentsOf: a, encoding: .utf8), "one")
        XCTAssertEqual(r.failedChecks, 1, "a timeout isn't a failed check")
        XCTAssertNotEqual(book.newTakeURL(ref), book.recordingURL(page: book.pages[0], exercise: i))
    }

    /// Stop recording on a later page, go back, insert a page while recognition is pending: the
    /// result lands on the exercise that was recorded, not on whatever now has its old index.
    func testPageInsertedDuringRecognitionDoesNotMisplaceTheResult() throws {
        let book = Book(course: course, store: BookStore(root: tempRoot()))
        for p in 0..<2 { QA.complete(book, page: p, upTo: .max); book.turnForward() }
        let p = book.currentPage
        let i = try XCTUnwrap(book.pages[p].exercises.firstIndex { $0.kind == .readAloud }
                              ?? book.pages[p].exercises.indices.first)
        let ref = ExerciseRef(pageID: book.pages[p].id, index: i)
        let target = book.pages[p].exercises[i].text ?? "кот"
        let before = book.pages[p].results[i]
        // …recognition pending; the learner goes back and a page is written in.
        book.goTo(0)
        var ex = Exercise(kind: .readAloud); ex.text = "кот"
        book.insertWrittenPage(title: "inserted", intro: nil, unitTitle: "t", exercises: [ex], newWords: [])
        XCTAssertNotEqual(book.pages[p].id, ref.pageID, "indices shifted")
        let shifted = book.pages[p].results
        book.commitTake(ref, round: before.roundNumber, file: "take.m4a", heard: RussianText.unstressed(target), target: target,
                        recognitionAllowed: true, timedOut: false)
        XCTAssertEqual(book.result(ref)?.recording, "take.m4a")
        XCTAssertEqual(book.pages[p].results, shifted, "the page now at the old index is untouched")
    }

    /// A result for an earlier round never changes the current one.
    func testLateResultForAnEarlierRoundIsKeptButNotGraded() throws {
        let book = Book(course: course, store: BookStore(root: tempRoot()))
        let i = try XCTUnwrap(book.pages[0].exercises.firstIndex { $0.kind == .readAloud })
        let ref = ExerciseRef(pageID: book.pages[0].id, index: i)
        book.record(ref) { $0.status = .done }
        book.reopen(ref)
        XCTAssertEqual(book.commitTake(ref, round: 0, file: "late.m4a", heard: "x", target: "x", recognitionAllowed: true, timedOut: false), .keptOnly)
        XCTAssertEqual(book.result(ref)?.status, .open)
        XCTAssertEqual(book.result(ref)?.takes, ["late.m4a"])
    }

    /// A take analysed across a restore: the old book is closed, and whatever lands on it late
    /// (the take's result, its sound evidence) must not reach the folder, which by then holds the
    /// restored notebook.
    func testAClosedNotebookRefusesLateTakesAndEvidence() async throws {
        let store = BookStore(root: tempRoot())
        let book = Book(course: course, store: store)
        var ex = Exercise(kind: .readAloud); ex.text = "был"
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [ex], newWords: [])
        let page = try XCTUnwrap(book.page)
        let ref = ExerciseRef(pageID: page.id, index: 0)
        await book.close()
        let state = try Data(contentsOf: store.stateURL)
        let log = try? Data(contentsOf: book.knowledge.logURL)
        XCTAssertEqual(book.commitTake(ref, round: 0, file: "late.m4a", heard: "был", target: "был", recognitionAllowed: true, timedOut: false), .closed)
        XCTAssertEqual(book.result(ref)?.status, .open)
        XCTAssertNil(book.result(ref)?.recording)
        let sub = PhoneJudgement(expected: Phone(ipa: "ɨ"), heard: Phone(ipa: "i"), verdict: .substituted, letterRange: 1..<2)
        let report = PronunciationReport(target: "был", transcript: "бил", expected: [], heard: [Phone(ipa: "b"), Phone(ipa: "i"), Phone(ipa: "l")],
                                         sounds: [sub], syllables: [], stressedSyllable: nil, accuracy: 0.6, tips: [])
        book.knowledge.heard(page: page, index: 0, take: "late.m4a", report: report, attempt: 1)
        XCTAssertFalse(book.knowledge.model.events.contains { $0.id == "speech:late.m4a" })
        book.record(ref) { $0.status = .done }
        XCTAssertEqual(book.result(ref)?.status, .open, "nothing is recorded in a closed book")
        book.saveNow()
        XCTAssertEqual(try Data(contentsOf: store.stateURL), state)
        XCTAssertEqual(try? Data(contentsOf: book.knowledge.logURL), log)
    }

    // MARK: Restore

    func testExportRestoresIntoAnotherFolderWithEverything() async throws {
        let store = BookStore(root: tempRoot())
        let book = Book(course: course, store: store)
        QA.complete(book, page: 0, upTo: .max)
        let i = try XCTUnwrap(book.pages[0].exercises.firstIndex { $0.kind == .readAloud })
        let ref = ExerciseRef(pageID: book.pages[0].id, index: i)
        let take = book.newTakeURL(ref)
        try Data("voice".utf8).write(to: take)
        book.reopen(ref)
        book.commitTake(ref, round: 1, file: take.lastPathComponent, heard: nil, target: "", recognitionAllowed: false, timedOut: false)
        let exported = await book.exportArchive()
        let zip = try XCTUnwrap(exported)
        let events = EventLog(url: book.knowledge.logURL).load().count

        let target = tempRoot()
        let current = BookStore(root: target)
        let mine = Book(course: course, store: current)
        mine.saveNow()
        let mineBytes = try Data(contentsOf: current.stateURL)
        let candidate = try NotebookArchive.candidate(from: zip, scratch: tempRoot())
        XCTAssertEqual(candidate.pages, book.pages.count)
        XCTAssertEqual(candidate.events, events)
        let kept = try XCTUnwrap(try NotebookArchive.install(candidate, at: target))
        XCTAssertEqual(try Data(contentsOf: kept.appendingPathComponent("book.json")), mineBytes, "the notebook it replaced is kept")
        guard case .success(let restored) = Book.open(course: course, store: BookStore(root: target)) else { return XCTFail() }
        XCTAssertEqual(restored.pages.map(\.id), book.pages.map(\.id))
        let enc = BookStore.encoder()
        XCTAssertEqual(try enc.encode(restored.pages[0].results), try enc.encode(book.pages[0].results))
        XCTAssertEqual(try String(contentsOf: restored.recordingURL(page: restored.pages[0], exercise: i), encoding: .utf8), "voice")
        let ink = try FileManager.default.contentsOfDirectory(atPath: target.appendingPathComponent("ink").path)
        XCTAssertFalse(ink.isEmpty)
        XCTAssertEqual(EventLog(url: restored.knowledge.logURL).load().count, events)
    }

    func testDamagedExportIsRefusedAndNothingIsReplaced() async throws {
        let source = BookStore(root: tempRoot())
        let book = Book(course: course, store: source)
        book.saveNow()
        try Data("{ broken".utf8).write(to: source.stateURL)
        let zip = try XCTUnwrap(NotebookArchive.export(root: source.root, to: tempRoot().appendingPathExtension("zip")))
        let target = tempRoot()
        let mine = Book(course: course, store: BookStore(root: target))
        mine.saveNow()
        let before = try Data(contentsOf: mine.store.stateURL)
        XCTAssertThrowsError(try NotebookArchive.candidate(from: zip, scratch: tempRoot()))
        XCTAssertEqual(try Data(contentsOf: mine.store.stateURL), before)
    }
}
