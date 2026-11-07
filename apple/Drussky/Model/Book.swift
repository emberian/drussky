import Foundation
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
}

/// Spaced-retrieval memory for one item ("letter:Ж", "word:кот").
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
}

@MainActor
@Observable
final class Book {
    let course: Course
    private(set) var pages: [BookPage] = []
    private(set) var memories: [String: Memory] = [:]
    /// Index into course.pageSequence of the next authored page to instantiate.
    private(set) var courseCursor: Int = 0
    var currentPage: Int = 0 { didSet { scheduleSave() } }
    var settings = Settings() { didSet { scheduleSave() } }
    /// Bumped when the learner explicitly finishes something (Done, Check, a choice), so the
    /// page moves on. Recognition ticking an exercise by itself never moves the page.
    private(set) var customWords: [Course.Word] = []
    /// While on, the Pencil writes anywhere on the page (margin notes) instead of in exercises.
    var marginMode = false
    private(set) var advanceSignal = 0
    private(set) var advancePage = 0
    func requestAdvance(page: Int) { advancePage = page; advanceSignal += 1 }

    private let store: BookStore
    private var saveTask: Task<Void, Never>?
    var now: () -> Date = Date.init

    init(course: Course, store: BookStore) {
        self.course = course
        self.store = store
        if let saved = store.loadState() {
            pages = saved.pages
            memories = Dictionary(uniqueKeysWithValues: saved.memories.map { ($0.key, $0) })
            courseCursor = saved.courseCursor
            currentPage = min(saved.currentPage, max(0, saved.pages.count - 1))
            settings = saved.settings
            customWords = saved.customWords ?? []
        }
        if pages.isEmpty { _ = appendNextPage() }
    }

    // MARK: Pages

    var page: BookPage? { pages.indices.contains(currentPage) ? pages[currentPage] : nil }
    var courseFinished: Bool { courseCursor >= course.pageSequence.count }

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
        let due = dueMemories()
        let lastWasReview = pages.last?.isReview ?? true
        if (due.count >= 10 && !lastWasReview) || (courseFinished && !due.isEmpty) {
            let items = Array(due.prefix(8))
            let exercises = items.enumerated().map { reviewExercise(for: $1, variant: $0) }
            appendPage(sourceID: nil, unitTitle: "Повторение · Review", title: "Bring it back",
                       intro: "Everything here is something you've met before. Reach for it before you check.",
                       isReview: true, exercises: exercises)
            return true
        }
        guard !courseFinished else { return false }
        let (unit, source) = course.pageSequence[courseCursor]
        courseCursor += 1
        var exercises = source.exercises.map(faded)
        let taughtHere = Set(source.exercises.compactMap { memoryKey(for: $0) })
        let reviews = due.filter { !taughtHere.contains($0.key) }.prefix(3)
        // Spread reviews through the page: after the opening, in the middle, near the end.
        // Insert from the last slot backwards so earlier slot indices stay valid.
        let slots = [min(2, exercises.count), exercises.count / 2 + 1, max(exercises.count - 1, 0)]
        for (i, memory) in reviews.enumerated().reversed() {
            exercises.insert(reviewExercise(for: memory, variant: memory.reps + i), at: min(slots[i], exercises.count))
        }
        appendPage(sourceID: source.id, unitTitle: unit.title, title: source.title, intro: source.intro,
                   isReview: false, exercises: exercises)
        return true
    }

    /// Ear training from human-recorded sentences, mostly made of words the learner knows:
    /// write what you hear, and pick which of three similar sentences was said. Different
    /// speakers each time (high-variability listening).
    func earTraining(from sentences: [Library.Sentence], hasAudio: (String) -> Bool) -> [Exercise] {
        let known = Set(memories.keys.filter { $0.hasPrefix("word:") }.map { String($0.dropFirst(5)) })
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

    /// Expertise reversal: scaffolds that help a novice get in the way once an item is secure.
    /// A word recalled right at least twice without a miss loses its support on later pages:
    /// trace → copy, copy → write it from hearing.
    func faded(_ ex: Exercise) -> Exercise {
        guard [.trace, .copy].contains(ex.kind), let key = memoryKey(for: ex), let m = memories[key],
              m.lapses == 0, m.reps >= 2 else { return ex }
        var e = ex
        if ex.kind == .trace { e.kind = .copy; e.times = 1 } else { e.kind = .dictation; e.times = nil }
        return e
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
        currentPage = at
        scheduleSave()
    }

    // MARK: Results

    func record(page pageIndex: Int, exercise index: Int, _ update: (inout ExerciseResult) -> Void) {
        guard pages.indices.contains(pageIndex), pages[pageIndex].results.indices.contains(index) else { return }
        let before = pages[pageIndex].results[index]
        update(&pages[pageIndex].results[index])
        pages[pageIndex].results[index].updated = now()
        let after = pages[pageIndex].results[index]
        if !before.status.isComplete && after.status.isComplete && after.learned != true {
            learn(from: pages[pageIndex].exercises[index], result: after.status)
            pages[pageIndex].results[index].learned = true
        }
        if !after.status.isComplete { pages[pageIndex].finished = nil }
        if pages[pageIndex].finished == nil && pages[pageIndex].results.allSatisfy({ $0.status.isComplete }) {
            pages[pageIndex].finished = now()
        }
        scheduleSave()
    }

    /// Re-opens a finished exercise so it can be attempted again (ink is kept by the view).
    func reopen(page pageIndex: Int, exercise index: Int) {
        record(page: pageIndex, exercise: index) { r in
            r.status = .open
            r.recognized = nil
            r.chosen = nil
            r.revealed = false
        }
    }

    // MARK: Memory

    func memoryKey(for exercise: Exercise) -> String? {
        if let key = exercise.reviewKey { return key }
        switch exercise.kind {
        case .meet: return exercise.letter.map { "letter:\($0)" }
        // Only what the learner produced becomes something to recall later.
        case .explain, .readMeaning, .passage: return nil
        default:
            guard let target = exercise.target ?? exercise.text, let word = word(target) else { return nil }
            return "word:\(RussianText.key(word.ru))"
        }
    }

    private func learn(from exercise: Exercise, result: ExerciseResult.Status) {
        guard let key = memoryKey(for: exercise) else { return }
        let today = now()
        var m = memories[key] ?? Memory(key: key, stability: 0.6, due: today.addingTimeInterval(0.6 * 86_400), seen: today)
        switch result {
        case .correct:
            // Only reviews that were actually due grow the interval much; same-day repeats grow it a little.
            let wasDue = m.due <= today
            m.stability = min(180, m.stability * (wasDue ? 2.5 : 1.15) + (wasDue ? 0.5 : 0))
            m.reps += 1
        case .missed:
            m.stability = max(0.3, m.stability * 0.4)
            m.lapses += 1
        case .done, .open:
            break
        }
        m.due = today.addingTimeInterval(m.stability * 86_400)
        memories[key] = m
    }

    func dueMemories() -> [Memory] {
        let today = now()
        return memories.values.filter { $0.due <= today }
            .sorted { ($0.due.timeIntervalSince(today) / $0.stability) < ($1.due.timeIntervalSince(today) / $1.stability) }
    }

    var dueCount: Int { dueMemories().count }

    /// English glosses shared by more than one entry can't be asked as "write it in Russian".
    private var ambiguousGlosses: Set<String> {
        var seen: [String: Int] = [:]
        for w in course.lexicon { seen[w.en.lowercased(), default: 0] += 1 }
        return Set(seen.filter { $0.value > 1 }.keys)
    }

    /// A review exercise for a remembered item, varying the modality across repetitions.
    func reviewExercise(for memory: Memory, variant: Int) -> Exercise {
        let parts = memory.key.split(separator: ":", maxSplits: 1).map(String.init)
        var ex: Exercise
        if parts.first == "letter", let letter = course.letter(parts[1]) {
            // Dictate the letter only when its spoken form begins with it (а, ра); signs and
            // letters voiced inside a word (ы in "ты") are recalled by choosing what was heard.
            let sayable = RussianText.key(letter.say).hasPrefix(letter.lower)
            if variant % 2 == 0 && sayable {
                // Hear the letter's sound, write the letter.
                ex = Exercise(kind: .dictation)
                ex.text = letter.lower
                ex.say = letter.say
            } else {
                ex = Exercise(kind: .listenPick)
                ex.say = letter.say
                let others = course.letters.filter { $0.upper != letter.upper && letterKnown($0.upper) }
                    .shuffled().prefix(2).map(\.say)
                ex.options = ([letter.say] + others).shuffled()
            }
        } else if let word = word(parts.count > 1 ? parts[1] : "") {
            let askable = !word.en.isEmpty && !ambiguousGlosses.contains(word.en.lowercased())
            switch variant % 3 {
            case 0 where askable: ex = Exercise(kind: .translate); ex.en = word.en; ex.answerText = word.ru
            case 0, 1: ex = Exercise(kind: .dictation); ex.text = word.ru
            default: ex = Exercise(kind: .readAloud); ex.text = word.ru
            }
        } else {
            // A word no longer in the course lexicon: still recall it by ear.
            ex = Exercise(kind: .dictation)
            ex.text = parts.count > 1 ? parts[1] : ""
        }
        ex.reviewKey = memory.key
        return ex
    }

    /// A word from the course or one the learner kept.
    func word(_ ru: String) -> Course.Word? {
        if let w = course.word(ru) { return w }
        let key = RussianText.key(ru)
        return customWords.first { RussianText.key($0.ru) == key }
    }

    func isKept(_ ru: String) -> Bool { word(ru) != nil && memories["word:\(RussianText.key(ru))"] != nil }

    /// Keeps a word or phrase from a tutor answer: it joins the learner's words and is
    /// reviewed like anything else they produced, starting tomorrow.
    func keep(_ ru: String, en: String) {
        let key = RussianText.key(ru)
        guard !key.isEmpty else { return }
        if course.word(ru) == nil && !customWords.contains(where: { RussianText.key($0.ru) == key }) {
            customWords.append(Course.Word(ru: ru, en: en, emoji: nil, note: "kept from your tutor"))
        }
        let today = now()
        if memories["word:\(key)"] == nil {
            memories["word:\(key)"] = Memory(key: "word:\(key)", stability: 1, due: today.addingTimeInterval(86_400), seen: today)
        }
        scheduleSave()
    }

    func letterKnown(_ upper: String) -> Bool { memories["letter:\(upper)"] != nil }

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
        }
        switch r.status {
        case .open: lines.append(r.attempts > 0 ? "Not finished yet; \(r.attempts) attempt(s)." : "Not attempted yet.")
        case .correct: lines.append("Got it right.")
        case .done: lines.append("Finished.")
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
        return lines.joined(separator: "\n")
    }

    // MARK: Ink & recordings

    func inkURL(page: BookPage, exercise index: Int) -> URL { store.inkURL(pageID: page.id, index: index) }
    func recordingURL(page: BookPage, exercise index: Int) -> URL { store.recordingURL(pageID: page.id, index: index) }
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

    /// Zips the notebook folder (state, ink, recordings) for sharing.
    func exportArchive() -> URL? {
        var output: URL?
        var error: NSError?
        NSFileCoordinator().coordinate(readingItemAt: store.root, options: .forUploading, error: &error) { zipped in
            let dest = FileManager.default.temporaryDirectory.appendingPathComponent("Drussky notebook.zip")
            try? FileManager.default.removeItem(at: dest)
            if (try? FileManager.default.copyItem(at: zipped, to: dest)) != nil { output = dest }
        }
        return output
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
        store.saveState(.init(pages: pages, memories: Array(memories.values), courseCursor: courseCursor,
                              currentPage: currentPage, settings: settings, customWords: customWords))
    }

    #if DEBUG
    /// Test hook: pretend time has passed.
    func shiftMemories(byDays days: Double) {
        for key in memories.keys { memories[key]!.due.addTimeInterval(-days * 86_400) }
    }
    #endif
}

struct BookState: Codable {
    var version = 1
    var pages: [BookPage]
    var memories: [Memory]
    var courseCursor: Int
    var currentPage: Int
    var settings: Settings
    /// Words the learner kept from tutor answers (optional: older notebooks lack it).
    var customWords: [Course.Word]? = nil
}

/// Files under Application Support/Drussky: book.json, ink/, voice/.
struct BookStore: Sendable {
    let root: URL

    @MainActor
    static func standard() -> BookStore {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        return BookStore(root: base.appendingPathComponent(QA.active ? "Drussky-QA" : "Drussky", isDirectory: true))
    }

    init(root: URL) {
        self.root = root
        for dir in ["ink", "voice"] {
            try? FileManager.default.createDirectory(at: root.appendingPathComponent(dir), withIntermediateDirectories: true)
        }
    }

    var stateURL: URL { root.appendingPathComponent("book.json") }
    func inkURL(pageID: String, index: Int) -> URL { root.appendingPathComponent("ink/\(pageID)-\(index).drawing") }
    func recordingURL(pageID: String, index: Int) -> URL { root.appendingPathComponent("voice/\(pageID)-\(index).m4a") }

    /// nil means "no notebook yet". A notebook that exists but can't be decoded is moved aside
    /// (never overwritten) so an update can never silently erase it.
    func loadState() -> BookState? {
        guard let data = try? Data(contentsOf: stateURL) else { return nil }
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        do {
            return try decoder.decode(BookState.self, from: data)
        } catch {
            let stamp = ISO8601DateFormatter().string(from: Date()).replacingOccurrences(of: ":", with: "-")
            let aside = root.appendingPathComponent("book-unreadable-\(stamp).json")
            try? FileManager.default.moveItem(at: stateURL, to: aside)
            try? "\(error)".write(to: aside.appendingPathExtension("error.txt"), atomically: true, encoding: .utf8)
            return nil
        }
    }

    func saveState(_ state: BookState) {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        guard let data = try? encoder.encode(state) else { return }
        try? data.write(to: stateURL, options: .atomic)
    }
}
