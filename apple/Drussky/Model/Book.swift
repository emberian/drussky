import Foundation
import LearningCore
import Observation

/// One page of the learner's workbook. Pages are instantiated from the course (plus
/// generated review) when first reached, then frozen: they keep their exercises and results
/// forever, so turning back shows exactly what was written there.
struct BookPage: Codable, Identifiable, Sendable {
    var id: String
    var number: Int
    var sourceID: String?
    var unitTitle: String
    var title: String
    var intro: String?
    var isReview: Bool
    var exercises: [Exercise]
    var results: [ExerciseResult]
    var created: Date
    var finished: Date?
    var notes: [PageNote]? = nil

    var isFinished: Bool { finished != nil }
    var firstOpenIndex: Int? { results.firstIndex { !$0.status.isComplete } }
    var completedCount: Int { results.filter { $0.status.isComplete }.count }
}

struct ExerciseResult: Codable, Sendable, Hashable {
    enum Status: String, Codable, Sendable {
        case open, done, correct, missed
        /// Left for now on purpose (no microphone, not today): finished, never graded.
        case skipped
        var isComplete: Bool { self != .open }
    }
    var status: Status = .open
    var attempts: Int = 0
    var recognized: String? = nil
    var chosen: Int? = nil
    var recording: String? = nil
    var revealed: Bool = false
    var updated: Date? = nil
    /// Set once the result has fed the memory model, so re-doing an exercise can't count twice.
    var learned: Bool? = nil
    /// How many times the exercise has been reopened ("Write it again"); each round is its own attempt.
    var round: Int? = nil
    /// The answer was shown at some point, in this round or an earlier one. Survives reopening.
    var assisted: Bool? = nil
    /// Checks the recognizer could not confirm, however they were triggered.
    var failedChecks: Int? = nil
    /// Earlier recordings (file names in voice/), oldest first; `recording` is the latest.
    var takes: [String]? = nil
    /// Earlier writings kept when the exercise was reopened (ink/<page>-<i>~<n>.drawing, n = 1…).
    var inkAttempts: Int? = nil
    /// Finished on the learner's own judgement ("I wrote it right", "Sounded right") after the
    /// checker couldn't confirm it.
    var selfAssessed: Bool? = nil

    var roundNumber: Int { round ?? 0 }

    /// Right on the first try, by the checker, never having been shown the answer.
    var isRightFirstTime: Bool {
        status == .correct && attempts <= 1 && (failedChecks ?? 0) == 0 && assisted != true && !revealed && selfAssessed != true
    }
}

/// A stable address for one exercise: page ids never change, and a page's exercises are frozen,
/// so this survives pages being inserted while async work (recognition, a check) is pending.
struct ExerciseRef: Hashable, Sendable {
    var pageID: String
    var index: Int
    var item: String { "\(pageID)#\(index)" }
}

/// The scheduler before format 2 (its own stability and due date per item). Kept only to
/// read older notebooks: their items become `ReviewTarget`s and LearningCore schedules them.
struct Memory: Codable, Sendable {
    var key: String
    var stability: Double   // days until the next review
    var due: Date
    var reps: Int = 0
    var lapses: Int = 0
    var seen: Date
}

struct Settings: Codable, Sendable, Equatable {
    var fingerDrawing: Bool = false
    var cursiveModels: Bool = false
    var showStress: Bool = true
    /// Times the model plays before the learner's turn to speak (0: straight to their turn).
    var modelRepeats: Int = 1

    init() {}

    // Every field optional on reading, so notebooks from before a setting existed still open.
    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        fingerDrawing = try c.decodeIfPresent(Bool.self, forKey: .fingerDrawing) ?? false
        cursiveModels = try c.decodeIfPresent(Bool.self, forKey: .cursiveModels) ?? false
        showStress = try c.decodeIfPresent(Bool.self, forKey: .showStress) ?? true
        modelRepeats = try c.decodeIfPresent(Int.self, forKey: .modelRepeats) ?? 1
    }
}

@MainActor
@Observable
final class Book {
    let course: Course
    private(set) var pages: [BookPage] = []
    /// Everything the learner should be able to recall ("letter:Ж", "word:кот") and when they
    /// met it. When each comes back is LearningCore's FSRS over the evidence log, nothing here.
    private(set) var targets: [String: ReviewTarget] = [:] { didSet { candidateCache = nil } }
    /// The pre-format-2 memories, written back unchanged (never used for scheduling).
    private var legacyMemories: [Memory] = []
    @ObservationIgnored var candidateCache: [ReviewCandidate]?
    /// Index into course.pageSequence of the next authored page to instantiate.
    private(set) var courseCursor: Int = 0
    var currentPage: Int = 0 { didSet { scheduleSave() } }
    var settings = Settings() { didSet { scheduleSave() } }
    /// Bumped when the learner explicitly finishes something (Done, Check, a choice), so the
    /// page moves on. Recognition ticking an exercise by itself never moves the page.
    private(set) var customWords: [Course.Word] = []
    /// First-choice accuracy per sound contrast (SoundPairs.swift); written only by noteSoundPair.
    var soundStats: [String: ContrastStat] = [:]
    /// While on, the Pencil writes anywhere on the page (margin notes) instead of in exercises.
    var marginMode = false
    private(set) var advanceSignal = 0
    private(set) var advancePageID: String?
    func requestAdvance(page id: String) { advancePageID = id; advanceSignal += 1 }

    let store: BookStore
    private var saveTask: Task<Void, Never>?
    var now: () -> Date = Date.init
    /// Knowledge components, evidence log (events.jsonl) and per-modality memory (Knowledge/).
    let knowledge: KnowledgeTracker
    /// Set when book.json exists but couldn't be opened: this Book then never writes it.
    let problem: NotebookProblem?
    /// What loading had to repair (the original was copied aside first).
    let repairs: [String]
    /// The last failure writing the notebook or its ink; shown until a later write succeeds.
    private(set) var persistenceError: String?
    private var closed = false
    private var pageIndexByID: [String: Int] = [:]

    /// Opens the notebook in `store`, or reports why it can't be opened without touching it.
    static func open(course: Course, store: BookStore) -> Result<Book, NotebookProblem> {
        let book = Book(course: course, store: store)
        if let problem = book.problem { return .failure(problem) }
        return .success(book)
    }

    init(course: Course, store: BookStore) {
        self.course = course
        self.store = store
        var saved: BookState?
        switch store.load() {
        case .missing: problem = nil; repairs = []
        case .loaded(let state, let fixed): saved = state; problem = nil; repairs = fixed
        case .unavailable(let p): problem = p; repairs = []
        }
        knowledge = KnowledgeTracker(course: course, root: store.root, customWords: saved?.customWords ?? [], now: Date.init)
        if let saved {
            pages = saved.pages
            legacyMemories = saved.memories
            targets = Dictionary((saved.targets ?? []).map { ($0.key, $0) }, uniquingKeysWith: { a, _ in a })
            courseCursor = saved.courseCursor
            currentPage = min(saved.currentPage, max(0, saved.pages.count - 1))
            settings = saved.settings
            customWords = saved.customWords ?? []
            soundStats = saved.soundStats ?? [:]
        }
        reindexPages()
        knowledge.now = { [unowned self] in self.now() }
        let writerErrors: @Sendable (String) -> Void = { [weak self] message in
            Task { @MainActor in self?.persistenceError = message }
        }
        let ink = store.ink
        Task { await ink.setFailureHandler(writerErrors) }
        guard problem == nil else { return }
        knowledge.backfill(pages: pages)
        if pages.isEmpty { _ = appendNextPage() }
    }

    // MARK: Pages

    var page: BookPage? { pages.indices.contains(currentPage) ? pages[currentPage] : nil }
    func index(ofPage id: String) -> Int? { pageIndexByID[id] }
    func page(id: String) -> BookPage? { pageIndexByID[id].map { pages[$0] } }
    private func reindexPages() {
        pageIndexByID = Dictionary(pages.enumerated().map { ($1.id, $0) }, uniquingKeysWith: { a, _ in a })
    }
    /// The next authored page the learner hasn't had, in course order. Tracking pages by id
    /// (not by position) means a course revision that inserts or reorders pages never makes
    /// the learner skip or repeat one.
    private var nextCoursePage: (unit: Course.Unit, page: Course.Page)? {
        let had = Set(pages.compactMap(\.sourceID))
        return course.pageSequence.first { !had.contains($0.page.id) }
    }
    var courseFinished: Bool { nextCoursePage == nil }

    func goTo(_ index: Int) {
        guard pages.indices.contains(index) else { return }
        currentPage = index
    }

    /// Moves to the next page, creating it when the learner is on the last one.
    @discardableResult
    func turnForward() -> Bool {
        if currentPage + 1 < pages.count { currentPage += 1; return true }
        guard appendNextPage() else { return false }
        currentPage = pages.count - 1
        return true
    }

    func turnBack() { if currentPage > 0 { currentPage -= 1 } }

    /// Appends either a review page (when much is due) or the next course page with a few
    /// due reviews woven in. Returns false at the end of the course with nothing due.
    private func appendNextPage() -> Bool {
        let due = dueReviews()
        let lastWasReview = pages.last?.isReview ?? true
        if (due.count >= 10 && !lastWasReview) || (courseFinished && !due.isEmpty) {
            let items = Array(due.prefix(8))
            let exercises = items.enumerated().map { reviewExercise(for: $1.id, modality: $1.modality, variant: $0) }
            appendPage(sourceID: nil, unitTitle: "Повторение · Review", title: "Bring it back",
                       intro: "Everything here is something you've met before. Reach for it before you check.",
                       isReview: true, exercises: exercises)
            return true
        }
        guard !courseFinished else { return false }
        guard let (unit, source) = nextCoursePage else { return false }
        courseCursor += 1
        var exercises = source.exercises.map(faded)
        let taughtHere = Set(source.exercises.compactMap { memoryKey(for: $0) })
        let reviews = due.filter { !taughtHere.contains($0.id) }.prefix(3)
        // Spread reviews through the page: after the opening, in the middle, near the end.
        // Insert from the last slot backwards so earlier slot indices stay valid.
        let slots = [min(2, exercises.count), exercises.count / 2 + 1, max(exercises.count - 1, 0)]
        for (i, item) in reviews.enumerated().reversed() {
            exercises.insert(reviewExercise(for: item.id, modality: item.modality, variant: pages.count + i),
                             at: min(slots[i], exercises.count))
        }
        appendPage(sourceID: source.id, unitTitle: unit.title, title: source.title, intro: source.intro,
                   isReview: false, exercises: exercises)
        return true
    }

    /// Ear training from human-recorded sentences, mostly made of words the learner knows:
    /// write what you hear, and pick which of three similar sentences was said. Different
    /// speakers each time (high-variability listening).
    func earTraining(from sentences: [Library.Sentence], hasAudio: (String) -> Bool) -> [Exercise] {
        let known = Set(targets.keys.filter { $0.hasPrefix("word:") }.map { String($0.dropFirst(5)) })
            .union(customWords.map { RussianText.key($0.ru) })
        func coverage(_ s: Library.Sentence) -> Double {
            let words = RussianText.key(s.ru).split(separator: " ").map(String.init)
            guard !words.isEmpty else { return 0 }
            return Double(words.filter { known.contains($0) }.count) / Double(words.count)
        }
        let met = Set(course.letters.filter { letterKnown($0.upper) }.map(\.upper))
        let pool = sentences.filter { hasAudio($0.ru) && RussianText.letters(in: $0.ru).isSubset(of: met)
                                      && RussianText.key($0.ru).split(separator: " ").count <= 6 }
            .sorted { coverage($0) > coverage($1) }
        let chosen = Array(pool.prefix(14).shuffled().prefix(8))
        var out: [Exercise] = []
        for (i, s) in chosen.enumerated() {
            if i % 2 == 0 {
                var e = Exercise(kind: .dictation); e.text = s.ru
                out.append(e)
            } else {
                let others = pool.filter { $0 != s }.sorted {
                    abs($0.ru.count - s.ru.count) < abs($1.ru.count - s.ru.count) }.prefix(2).map(\.ru)
                var e = Exercise(kind: .listenPick); e.say = s.ru; e.options = ([s.ru] + others).shuffled()
                out.append(e)
            }
        }
        return out
    }

    private func appendPage(sourceID: String?, unitTitle: String, title: String, intro: String?,
                            isReview: Bool, exercises: [Exercise]) {
        let number = pages.count + 1
        // A random suffix keeps ink/voice file names unique even if the notebook is ever restarted.
        let id = String(format: "p%04d-", number) + UUID().uuidString.prefix(6).lowercased()
        pages.append(BookPage(id: id, number: number, sourceID: sourceID,
                              unitTitle: unitTitle, title: title, intro: intro, isReview: isReview,
                              exercises: exercises, results: exercises.map { _ in ExerciseResult() },
                              created: now(), finished: nil))
        reindexPages()
        scheduleSave()
    }

    /// Puts a tutor-written page right after the current one and opens it. Its new words join
    /// the learner's words so the page's exercises feed their reviews.
    func insertWrittenPage(title: String, intro: String?, unitTitle: String, exercises: [Exercise], newWords: [Course.Word]) {
        for w in newWords where word(w.ru) == nil && !w.en.isEmpty {
            customWords.append(Course.Word(ru: w.ru, en: w.en, emoji: w.emoji, note: w.note ?? "from a written page"))
        }
        let at = min(currentPage + 1, pages.count)
        let id = "w" + UUID().uuidString.prefix(8).lowercased()
        pages.insert(BookPage(id: id, number: 0, sourceID: nil, unitTitle: unitTitle, title: title, intro: intro,
                              isReview: false, exercises: exercises, results: exercises.map { _ in ExerciseResult() },
                              created: now(), finished: nil), at: at)
        for i in pages.indices { pages[i].number = i + 1 }
        reindexPages()
        currentPage = at
        knowledge.refresh(customWords: customWords)
        candidateCache = nil
        scheduleSave()
    }

    // MARK: Results

    func record(page pageIndex: Int, exercise index: Int, _ update: (inout ExerciseResult) -> Void) {
        guard pages.indices.contains(pageIndex) else { return }
        record(ExerciseRef(pageID: pages[pageIndex].id, index: index), update)
    }

    /// The one way a result changes. The closure edits a copy, never storage inside `pages`, so
    /// it may read anything on the book (an inout borrow of `pages` here was an exclusivity trap).
    /// A result that finishes is saved before its outcome is logged: if the app dies in between,
    /// the next launch finds the saved result and logs the missing outcome (never the reverse).
    func record(_ ref: ExerciseRef, _ update: (inout ExerciseResult) -> Void) {
        guard let p = pageIndexByID[ref.pageID], pages[p].results.indices.contains(ref.index) else { return }
        let before = pages[p].results[ref.index]
        var after = before
        update(&after)
        after.updated = now()
        let finishing = !before.status.isComplete && after.status.isComplete
        if finishing {
            let exercise = pages[p].exercises[ref.index]
            introduce(exercise)
            if after.learned != true {
                noteSoundPair(exercise, status: after.status)
                after.learned = true
            }
        }
        var page = pages[p]
        page.results[ref.index] = after
        if !after.status.isComplete { page.finished = nil }
        if page.finished == nil && page.results.allSatisfy({ $0.status.isComplete }) { page.finished = now() }
        pages[p] = page
        if finishing { saveNow() } else { scheduleSave() }
        knowledge.observe(page: page, index: ref.index, before: before, after: after)
    }

    func result(_ ref: ExerciseRef) -> ExerciseResult? {
        guard let p = pageIndexByID[ref.pageID] else { return nil }
        return pages[p].results[safe: ref.index]
    }

    /// Re-opens a finished exercise as a new round. Its ink so far is kept as an earlier attempt
    /// (the view archives it), and having been shown the answer stays on record.
    func reopen(page pageIndex: Int, exercise index: Int) {
        guard pages.indices.contains(pageIndex) else { return }
        reopen(ExerciseRef(pageID: pages[pageIndex].id, index: index))
    }

    func reopen(_ ref: ExerciseRef, keptInk: Bool = false) {
        record(ref) { r in
            if r.revealed || r.status == .missed { r.assisted = true }
            if keptInk { r.inkAttempts = (r.inkAttempts ?? 0) + 1 }
            r.round = r.roundNumber + 1
            r.status = .open
            r.recognized = nil
            r.chosen = nil
            r.revealed = false
            r.selfAssessed = nil
        }
    }

    // MARK: Memory

    func memoryKey(for exercise: Exercise) -> String? {
        if let key = exercise.reviewKey { return key }
        switch exercise.kind {
        case .meet: return exercise.letter.map { "letter:\($0)" }
        // Only what the learner produced becomes something to recall later.
        case .explain, .readMeaning, .passage, .soundPair: return nil
        default:
            guard let target = exercise.target ?? exercise.text else { return nil }
            guard let word = word(target) else {
                // Recall answers that aren't lexicon entries (inflected forms like чита́ем)
                // still come back — by ear, since there's no single English prompt for them.
                let isRecall = [.dictation, .translate, .fill].contains(exercise.kind)
                let short = RussianText.key(target).split(separator: " ").count <= 3
                return isRecall && short ? "word:\(RussianText.key(target))" : nil
            }
            return "word:\(RussianText.key(word.ru))"
        }
    }

    /// A word from the course or one the learner kept.
    func word(_ ru: String) -> Course.Word? {
        if let w = course.word(ru) { return w }
        let key = RussianText.key(ru)
        return customWords.first { RussianText.key($0.ru) == key }
    }

    func isKept(_ ru: String) -> Bool { word(ru) != nil && targets["word:\(RussianText.key(ru))"] != nil }

    /// Keeps a word or phrase from a tutor answer: it joins the learner's words and is
    /// reviewed like anything else they produced, starting tomorrow.
    func keep(_ ru: String, en: String) {
        let key = RussianText.key(ru)
        guard !key.isEmpty else { return }
        if course.word(ru) == nil && !customWords.contains(where: { RussianText.key($0.ru) == key }) {
            customWords.append(Course.Word(ru: ru, en: en, emoji: nil, note: "kept from your tutor"))
            candidateCache = nil
        }
        if targets["word:\(key)"] == nil {
            targets["word:\(key)"] = ReviewTarget(key: "word:\(key)", introduced: now(), kept: true)
        }
        knowledge.refresh(customWords: customWords)
        scheduleSave()
    }

    func letterKnown(_ upper: String) -> Bool { targets["letter:\(upper)"] != nil }

    /// Something the learner produced becomes a thing to recall (first time only).
    func introduce(_ exercise: Exercise) {
        guard let key = memoryKey(for: exercise), targets[key] == nil else { return }
        targets[key] = ReviewTarget(key: key, introduced: now())
    }

    // MARK: Tutor notes

    func notes(page index: Int) -> [PageNote] { pages.indices.contains(index) ? (pages[index].notes ?? []) : [] }

    @discardableResult
    func saveNote(page index: Int, id: UUID?, exercise: Int?, turns: [Tutor.Turn]) -> UUID {
        var notes = pages[index].notes ?? []
        if let id, let i = notes.firstIndex(where: { $0.id == id }) {
            notes[i].turns = turns
            pages[index].notes = notes
            scheduleSave()
            return id
        }
        let note = PageNote(exercise: exercise, turns: turns, date: now())
        notes.append(note)
        knowledge.askedTutor(page: pages[index], exercise: exercise)
        pages[index].notes = notes
        scheduleSave()
        return note.id
    }

    /// Plain-language description of an exercise and the learner's attempt, for the tutor.
    func describe(pageIndex: Int, exercise i: Int) -> [String] {
        let page = pages[pageIndex]
        let ex = page.exercises[i], r = page.results[i]
        var lines: [String] = []
        switch ex.kind {
        case .meet: lines.append("Meeting the letter \(ex.letter ?? "") and tracing it.")
        case .explain: lines.append("Reading a note: \(ex.text ?? "")")
        case .trace: lines.append("Tracing «\(ex.text ?? "")».")
        case .copy: lines.append("Copying «\(ex.text ?? "")».")
        case .listenPick: lines.append("Hearing «\(ex.say ?? "")» and choosing among \((ex.options ?? []).joined(separator: ", ")).")
        case .dictation: lines.append("Writing from dictation: «\(ex.text ?? "")».")
        case .readAloud: lines.append("Reading aloud: «\(ex.text ?? "")».")
        case .translate: lines.append("Writing the Russian for “\(ex.en ?? "")” (answer «\(ex.answerText ?? "")»).")
        case .fill: lines.append("Filling the gap: \(ex.before ?? "")___\(ex.after ?? "") — “\(ex.hint ?? "")” (answer «\(ex.answerText ?? "")»).")
        case .readMeaning: lines.append("Reading «\(ex.text ?? "")» and choosing its meaning from: \((ex.options ?? []).joined(separator: " / ")).")
        case .passage: lines.append("Reading a passage: «\(ex.text ?? "")» (translation: \(ex.en ?? "none")).")
        case .soundPair: lines.append(ex.say.map { "Hearing «\($0)» and telling it from its minimal pair: \((ex.options ?? []).joined(separator: " / "))." }
                                      ?? "Comparing minimal pairs: \((ex.examples ?? []).map(\.ru).joined(separator: ", ")).")
        }
        switch r.status {
        case .open: lines.append(r.attempts > 0 ? "Not finished yet; \(r.attempts) attempt(s)." : "Not attempted yet.")
        case .correct: lines.append("Got it right.")
        case .done: lines.append(r.selfAssessed == true ? "Finished; they judged it right themselves." : "Finished.")
        case .skipped: lines.append("Skipped for now (not graded).")
        case .missed: lines.append("Missed it and was shown the answer.")
        }
        if let rec = r.recognized, !rec.isEmpty {
            lines.append(ex.kind == .readAloud ? "Speech recognition heard: «\(rec)»." : "Handwriting recognition read: «\(rec)».")
        }
        if let c = r.chosen, let opts = ex.options, opts.indices.contains(c) { lines.append("Chose: \(opts[c]).") }
        return lines
    }

    func tutorContext(pageIndex: Int, exercise: Int?) -> String {
        let page = pages[pageIndex]
        let met = course.letters.filter { letterKnown($0.upper) }.map(\.upper).joined(separator: " ")
        var lines = ["Workbook page \(page.number): “\(page.title)” (\(page.unitTitle)).",
                     "Letters met so far: \(met.isEmpty ? "none yet" : met)."]
        if let intro = page.intro { lines.append("Page intro: \(intro)") }
        if let exercise { lines += describe(pageIndex: pageIndex, exercise: exercise) }
        else { lines.append("Exercises on the page: " + page.exercises.indices.map { describe(pageIndex: pageIndex, exercise: $0)[0] }.joined(separator: " ")) }
        lines.append(knowledge.summary(limit: 5))
        return lines.joined(separator: "\n")
    }

    // MARK: Ink & recordings

    func inkURL(page: BookPage, exercise index: Int) -> URL { store.inkURL(pageID: page.id, index: index) }
    /// An earlier writing kept when the exercise was reopened (n = 1 is the first).
    func inkAttemptURL(_ ref: ExerciseRef, attempt n: Int) -> URL {
        store.root.appendingPathComponent("ink/\(ref.pageID)-\(ref.index)~\(n).drawing")
    }
    /// The latest recording: the file the result names (older notebooks used one fixed name).
    func recordingURL(page: BookPage, exercise index: Int) -> URL {
        if let file = page.results[safe: index]?.recording { return voiceURL(file: file) }
        return store.recordingURL(pageID: page.id, index: index)
    }
    /// A fresh file for a new take; nothing is committed to the result until it succeeds.
    func newTakeURL(_ ref: ExerciseRef) -> URL {
        voiceURL(file: "\(ref.pageID)-\(ref.index)-\(UUID().uuidString.prefix(8).lowercased()).m4a")
    }
    func voiceURL(file: String) -> URL { store.root.appendingPathComponent("voice/\(file)") }
    func marginURL(page: BookPage) -> URL { store.root.appendingPathComponent("ink/\(page.id)-margin.drawing") }

    /// The learner's own handwriting of a letter: the ink from its `meet` exercise.
    func inkForLetter(_ upper: String) -> URL? {
        for page in pages {
            for (i, ex) in page.exercises.enumerated() where ex.kind == .meet && ex.letter == upper {
                let url = store.inkURL(pageID: page.id, index: i)
                if FileManager.default.fileExists(atPath: url.path) { return url }
            }
        }
        return nil
    }

    // MARK: Persistence

    func report(_ message: String) { persistenceError = message }

    /// Writes everything still pending: the notebook, ink, and any events the log couldn't take.
    func flush() async {
        saveNow()
        await PadController.submitAll()
        let failures = await store.ink.flush()
        if let f = failures.first { persistenceError = f }
        knowledge.retryPendingEvents()
    }

    /// Zips the notebook folder (state, ink, recordings, events) for sharing, after flushing.
    func exportArchive() async -> URL? {
        await flush()
        return NotebookArchive.export(root: store.root)
    }

    /// Stops all writing (before the folder is replaced by a restore).
    func close() async {
        await flush()
        await store.ink.close()
        saveTask?.cancel()
        closed = true
    }

    private func scheduleSave() {
        saveTask?.cancel()
        saveTask = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(400))
            guard !Task.isCancelled else { return }
            self?.saveNow()
        }
    }

    func saveNow() {
        // A notebook that exists but couldn't be opened is never written over.
        guard problem == nil, !closed else { return }
        saveTask?.cancel()
        do {
            try store.saveState(.init(pages: pages, memories: legacyMemories, courseCursor: courseCursor,
                                      currentPage: currentPage, settings: settings, customWords: customWords,
                                      soundStats: soundStats.isEmpty ? nil : soundStats,
                                      targets: targets.values.sorted { $0.key < $1.key }))
            if persistenceError?.hasPrefix("notebook") == true { persistenceError = nil }
        } catch {
            persistenceError = "notebook: \((error as NSError).localizedDescription)"
        }
    }

    #if DEBUG
    /// Test hook: from now on the book's clock runs `days` ahead.
    func advanceClock(byDays days: Double) {
        let base = now
        now = { base().addingTimeInterval(days * 86_400) }
    }
    #endif
}

struct BookState: Codable {
    /// 2: review timing moved to LearningCore; `targets` replaces `memories` (kept, unused).
    static let currentVersion = 2
    var version = BookState.currentVersion
    var pages: [BookPage]
    /// Format 1's scheduler state; read to migrate, then written back unchanged.
    var memories: [Memory]
    var courseCursor: Int
    var currentPage: Int
    var settings: Settings
    /// Words the learner kept from tutor answers (optional: older notebooks lack it).
    var customWords: [Course.Word]? = nil
    /// Sound-pair accuracy per contrast (optional: older notebooks lack it).
    var soundStats: [String: ContrastStat]? = nil
    /// What to recall and since when (format 2; migrated from `memories`).
    var targets: [ReviewTarget]? = nil
}

/// Something the learner should be able to recall, and when they met it.
struct ReviewTarget: Codable, Sendable, Hashable {
    var key: String
    var introduced: Date
    /// Kept from a tutor answer (first recall the next day).
    var kept: Bool? = nil
}

/// Files under Application Support/Drussky: book.json, ink/, voice/, events.jsonl.
struct BookStore: Sendable {
    let root: URL
    /// The single serialized writer for this notebook's ink.
    let ink: InkWriter

    @MainActor
    static func standard() -> BookStore { BookStore(root: standardRoot) }

    @MainActor
    static var standardRoot: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        return base.appendingPathComponent(QA.active ? "Drussky-QA" : "Drussky", isDirectory: true)
    }

    init(root: URL, ink: InkWriter = InkWriter()) {
        self.root = root
        self.ink = ink
        for dir in ["ink", "voice"] {
            try? FileManager.default.createDirectory(at: root.appendingPathComponent(dir), withIntermediateDirectories: true)
        }
    }

    var stateURL: URL { root.appendingPathComponent("book.json") }
    func inkURL(pageID: String, index: Int) -> URL { root.appendingPathComponent("ink/\(pageID)-\(index).drawing") }
    func recordingURL(pageID: String, index: Int) -> URL { root.appendingPathComponent("voice/\(pageID)-\(index).m4a") }
}
