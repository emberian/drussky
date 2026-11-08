import Foundation

/// The tutor's lemma table for a goal: every token outside the course gets its dictionary form
/// and a short gloss, 300 tokens per call, cached in the goal (so a text is lemmatized once).
@MainActor
enum GoalLemmaTable {
    static let batch = 300
    static let model = PageWriter.rescaleModel
    /// Goals whose table is being filled right now.
    private(set) static var filling: Set<String> = []

    static let system = """
    You are a Russian lemmatizer and glossary. You receive Russian word forms, one per line, lowercase, \
    without stress marks. Reply with ONE JSON object and nothing else: each input form as a key, its value a \
    two-element array [lemma, gloss]. The lemma is the dictionary form (nominative singular for nouns and \
    adjectives — masculine for adjectives —, infinitive for verbs including participles and gerunds, with ё \
    where it belongs); the gloss is 1–4 English words for the word's usual meaning. Proper names: lemma is \
    the nominative, gloss "name". Keep every key exactly as given.
    """

    /// The entries missing from the goal's table, from the tutor (empty when there is nothing to
    /// do or no key). The caller merges them into the goal as it is by then (`Book.mergeLemmas`):
    /// the goal passed in is a snapshot and may be stale by the time this returns.
    static func fill(_ goal: Goal, analysis: GoalAnalysis, progress: @escaping (Int, Int) -> Void = { _, _ in }) async throws -> [String: Goal.LemmaEntry] {
        let missing = analysis.tokensNeedingLemmas.filter { goal.lemmaCache?[$0] == nil }
        // Claude's job (OnDevice/TutorBackend.swift: the on-device model lemmatized 12 of 40 forms); offline it waits.
        guard !missing.isEmpty, TutorRouter.route(.lemmatize, .live) == .claude, !filling.contains(goal.id) else { return [:] }
        filling.insert(goal.id)
        defer { filling.remove(goal.id) }
        var table: [String: Goal.LemmaEntry] = [:]
        var done = 0
        for chunk in stride(from: 0, to: missing.count, by: batch).map({ Array(missing[$0..<min($0 + batch, missing.count)]) }) {
            let entries = try await lemmatize(chunk)
            for (k, v) in entries { table[k] = v }
            done += chunk.count
            progress(done, missing.count)
        }
        return table
    }

    /// One call: forms → entries. Forms the tutor skipped are simply absent.
    static func lemmatize(_ forms: [String]) async throws -> [String: Goal.LemmaEntry] {
        var raw = ""
        for try await chunk in Tutor.stream([.init(role: "user", text: forms.joined(separator: "\n"))], system: system,
                                            maxTokens: 12_000, effort: "low", model: model) {
            raw += chunk
        }
        guard let json = PageWriter.extractJSON(raw), let data = json.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { throw PageWriter.Failure.unreadable }
        return entries(in: object, asked: forms)
    }

    /// The contract object (form → [lemma, gloss] or lemma) read into entries; forms not asked for are ignored.
    static func entries(in object: [String: Any], asked forms: [String]) -> [String: Goal.LemmaEntry] {
        var out: [String: Goal.LemmaEntry] = [:]
        let asked = Set(forms)
        for (k, v) in object {
            let key = RussianText.key(k)
            guard asked.contains(key) else { continue }
            if let pair = v as? [Any], let lemma = pair.first as? String, !lemma.isEmpty {
                out[key] = Goal.LemmaEntry(lemma: RussianText.key(lemma), en: pair.count > 1 ? pair[1] as? String : nil)
            } else if let lemma = v as? String, !lemma.isEmpty {
                out[key] = Goal.LemmaEntry(lemma: RussianText.key(lemma), en: nil)
            }
        }
        return out
    }
}
