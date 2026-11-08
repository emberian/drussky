import Foundation

/// Stress marks for a goal text (the learner's own book, brought in at runtime): the tutor marks the
/// sentences the library does not already know, every form is checked by `RussianStress` before it
/// is kept, and the table is cached in the notebook (goals/<id>.stress.json) so a text is marked
/// once. Offline, or until the tutor answers, the text reads bare.
@MainActor
enum GoalStress {
    static let batch = 14
    static let model = PageWriter.rescaleModel
    /// Goals whose table is being filled right now.
    private(set) static var filling: Set<String> = []

    static let system = """
    You mark word stress in Russian sentences for an adult beginner. You receive numbered sentences, one per \
    line as "N: sentence". Reply with ONE JSON object and nothing else: each number as a string key, its value the \
    sentence exactly as given with U+0301 (COMBINING ACUTE ACCENT) inserted immediately after the stressed vowel of \
    every word of two or more syllables. Change nothing else: not a letter, spelling, punctuation, spacing or case. \
    One mark per word of two or more syllables; none on a one-syllable word; never on a word containing ё; a \
    hyphenated compound of one-syllable parts takes one mark in all (кто́-то), otherwise each part follows the rule \
    (по-ру́сски). Homographs (за́мок/замо́к, бо́льшая/больша́я, ру́ки/руки́) take the stress the context asks for. \
    Leave Latin words, numbers and symbols alone.
    """

    /// A goal's table in the notebook: sentence as written → its stressed form (and, derived, each
    /// written word's stressed spelling for labels).
    struct Table: Codable, Sendable, Equatable {
        var version = 1
        var model: String?
        var sentences: [String: String] = [:] { didSet { words = LibraryStress.wordIndex(sentences.values.sorted()) } }
        private(set) var words: [String: String] = [:]

        enum CodingKeys: String, CodingKey { case version, model, sentences }

        init() {}

        init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            version = try c.decodeIfPresent(Int.self, forKey: .version) ?? 1
            model = try c.decodeIfPresent(String.self, forKey: .model)
            sentences = try c.decodeIfPresent([String: String].self, forKey: .sentences) ?? [:]
            words = LibraryStress.wordIndex(sentences.values.sorted())
        }
    }

    static func url(goalID: String, root: URL) -> URL { root.appendingPathComponent("goals/\(goalID).stress.json") }

    /// The cached table, or nil when the goal has none yet (or it cannot be read). Forms that fail
    /// the rule are dropped on the way in, so a hand-edited file can never show a bad mark.
    static func load(goalID: String, root: URL) -> Table? {
        guard let data = try? Data(contentsOf: url(goalID: goalID, root: root)),
              var table = try? JSONDecoder().decode(Table.self, from: data) else { return nil }
        table.sentences = table.sentences.filter { RussianStress.problems(source: $0.key, stressed: $0.value).isEmpty }
        return table
    }

    static func save(_ table: Table, goalID: String, root: URL) throws {
        let dir = root.appendingPathComponent("goals", isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys, .prettyPrinted, .withoutEscapingSlashes]
        try encoder.encode(table).write(to: url(goalID: goalID, root: root), options: .atomic)
    }

    static func remove(goalID: String, root: URL) {
        try? FileManager.default.removeItem(at: url(goalID: goalID, root: root))
    }

    /// The sentences of a goal still to mark: those with a word needing a mark that neither the
    /// library nor the goal's table has (in text order, each once).
    static func missing(in analysis: GoalAnalysis, have: Table?) -> [String] {
        var seen = Set<String>()
        return analysis.sentences.map(\.text).filter { s in
            RussianStress.lacksStress(s) && LibraryStress.shared.stressed(sentence: s) == nil && have?.sentences[s] == nil
                && seen.insert(s).inserted
        }
    }

    /// The entries missing from the goal's table, from the tutor (empty when there is nothing to do,
    /// no key, no network, or another fill is running). The caller merges them into the table as it
    /// is by then (`Book.mergeGoalStress`).
    static func fill(_ goal: Goal, analysis: GoalAnalysis, have: Table?, progress: @escaping (Int, Int) -> Void = { _, _ in }) async throws -> [String: String] {
        let todo = missing(in: analysis, have: have)
        guard !todo.isEmpty, TutorRouter.route(.lemmatize, .live) == .claude, !filling.contains(goal.id) else { return [:] }
        filling.insert(goal.id)
        defer { filling.remove(goal.id) }
        var out: [String: String] = [:]
        var done = 0
        for chunk in stride(from: 0, to: todo.count, by: batch).map({ Array(todo[$0..<min($0 + batch, todo.count)]) }) {
            for (k, v) in try await mark(chunk) { out[k] = v }
            done += chunk.count
            progress(done, todo.count)
        }
        return out
    }

    /// One call: sentences → verified stressed forms. Sentences the tutor skipped or got wrong are absent.
    static func mark(_ sentences: [String]) async throws -> [String: String] {
        let prompt = sentences.enumerated().map { "\($0.offset): \($0.element)" }.joined(separator: "\n")
        var raw = ""
        for try await chunk in Tutor.stream([.init(role: "user", text: prompt)], system: system, maxTokens: 12_000, effort: "low", model: model) {
            raw += chunk
        }
        guard let json = PageWriter.extractJSON(raw), let data = json.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { throw PageWriter.Failure.unreadable }
        return entries(in: object, asked: sentences)
    }

    /// The reply read into entries: a value is kept only when it is the asked sentence with sound marks.
    static func entries(in object: [String: Any], asked sentences: [String]) -> [String: String] {
        var out: [String: String] = [:]
        for (k, v) in object {
            guard let i = Int(k.trimmingCharacters(in: .whitespaces)), sentences.indices.contains(i),
                  let stressed = (v as? String)?.precomposedStringWithCanonicalMapping else { continue }
            let source = sentences[i]
            if RussianStress.problems(source: source, stressed: stressed).isEmpty, stressed != source { out[source] = stressed }
        }
        return out
    }
}

extension Book {
    /// The goal's cached stress table (loaded once from the notebook; nil until the tutor has marked anything).
    func goalStressTable(_ goalID: String) -> GoalStress.Table? {
        if let t = goalStress[goalID] { return t }
        guard let loaded = GoalStress.load(goalID: goalID, root: store.root) else { return nil }
        goalStress[goalID] = loaded
        return loaded
    }

    /// Verified entries merged into the goal's table and written to the notebook.
    func mergeGoalStress(goalID: String, _ entries: [String: String]) {
        guard !entries.isEmpty else { return }
        var table = goalStressTable(goalID) ?? GoalStress.Table()
        table.model = GoalStress.model
        table.sentences.merge(entries) { _, new in new }
        goalStress[goalID] = table
        do { try GoalStress.save(table, goalID: goalID, root: store.root) } catch { report("stress: couldn't keep the goal's marks (\(error.localizedDescription))") }
    }

    /// Starts the tutor's stress pass for a goal when something is left to mark; quiet otherwise.
    func ensureGoalStress(_ goal: Goal) {
        let a = analysis(for: goal)
        let have = goalStressTable(goal.id)
        guard !GoalStress.missing(in: a, have: have).isEmpty else { return }
        Task { @MainActor in
            if let entries = try? await GoalStress.fill(goal, analysis: a, have: have) { mergeGoalStress(goalID: goal.id, entries) }
        }
    }

    /// A sentence's stressed form from any goal's table, when a goal text has it.
    func goalStressed(sentence: String) -> String? {
        let bare = RussianText.unstressed(sentence)
        for g in goals { if let s = goalStressTable(g.id)?.sentences[bare] { return s } }
        return nil
    }

    /// A written word's stressed spelling from any goal's table (lowercased), when one goal stresses it one way.
    func goalStressed(word: String) -> String? {
        let bare = RussianText.unstressed(word).lowercased()
        for g in goals { if let s = goalStressTable(g.id)?.words[bare] { return s } }
        return nil
    }

    /// A passage with its stored stress marks: the library's, then the goals' tables; a sentence
    /// nobody has stays as written.
    func stressed(passage: String) -> String {
        LibraryStress.apply(to: passage) { LibraryStress.shared.stressed(sentence: $0) ?? goalStressed(sentence: $0) }
    }

    /// What a passage shows: its stressed form while «Show stress» is on, the text as written otherwise.
    func displayText(_ text: String) -> String { settings.showStress ? stressed(passage: text) : text }
}
