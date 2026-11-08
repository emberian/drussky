import Foundation

/// Claude as a TutorBackend: the existing calls (Tutor.stream, DiaryTutor, GoalLemmaTable),
/// collected behind the protocol so the router can hand out either tutor.
struct ClaudeBackend: TutorBackend {
    var provenance: TutorProvenance { .claude }

    static let glossSystem = TutorIdentity.glossSystem

    private func collect(_ stream: AsyncThrowingStream<String, Error>) async throws -> String {
        var raw = ""
        for try await chunk in stream { raw += chunk }
        return raw
    }

    func gloss(word: String, context: String) async throws -> TutorGloss {
        let raw = try await collect(Tutor.stream([.init(role: "user", text: "Sentence: \(context)\nWord: \(word)")],
                                                 system: Self.glossSystem, maxTokens: 300, effort: "low"))
        guard let json = await PageWriter.extractJSON(raw), let data = json.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let en = object["gloss"] as? String, !en.isEmpty else { throw PageWriter.Failure.unreadable }
        return TutorGloss(en: en, lemma: object["lemma"] as? String, provenance: .claude)
    }

    func quickAnswer(question: String, context: String) async throws -> String {
        try await collect(Tutor.stream([.init(role: "user", text: question)], context: context))
    }

    func correctDiary(text: String, prompt: String, briefing: String, at date: Date) async throws -> DiaryCorrection {
        let user = "\(briefing)\n\nThe diary prompt was: “\(prompt)”\nThey wrote:\n---\n\(text)\n---"
        let raw = try await collect(Tutor.stream([.init(role: "user", text: user)], system: DiaryTutor.system,
                                                 maxTokens: 4000, effort: "medium"))
        guard var c = await DiaryTutor.parse(raw, at: date) else { throw DiaryTutor.Failure.unreadable }
        c.model = Tutor.opusModel
        return c
    }

    func lemmatize(_ forms: [String]) async throws -> [String: Goal.LemmaEntry] {
        try await GoalLemmaTable.lemmatize(forms)
    }
}
