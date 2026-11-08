import Foundation

/// Sends a diary line to the tutor and reads back a structured red-pen correction.
@MainActor
enum DiaryTutor {
    enum Failure: LocalizedError {
        case unreadable
        /// No tutor can correct right now (offline, no key); the text says why.
        case unavailable(String)
        var errorDescription: String? {
            switch self {
            case .unreadable: "The tutor's correction didn't come back in a form I could use. Try again."
            case .unavailable(let why): why
            }
        }
    }

    /// The correction's JSON contract. The reply is parsed tolerantly (`parse`): missing fields
    /// get defaults, unknown categories become `other`, prose around the object is ignored.
    static let system = """
    You are the tutor inside Drussky, a Russian exercise book used by one adult English speaker, Ember: quick, \
    curious, a beginner in Russian. They have just written a few words of their OWN Russian in their diary, \
    with a pencil. Correct it the way a kind, exact teacher marks homework in red pen. Reply with ONE JSON \
    object and nothing else:
    {"corrected": "the whole text, corrected, with stress marks (U+0301) on every word of 2+ syllables, never on ё",
     "edits": [{"span": "their words exactly as written", "replacement": "the right form, stress-marked ('' to delete)",
                "category": "spelling|ending|agreement|word-choice|word-order|stress|punctuation|other",
                "why": "one line in English saying what differs and why", "en": "English meaning of the replacement",
                "pattern": "a construction id from the list below when one explains it, else omit"}],
     "praise": "one specific, warm sentence about what they got right",
     "teach": [{"title": "the pattern behind the biggest error, in 3-6 words",
                "note": "at most 2 sentences teaching it, with one example marked [[ру́сский|english]]",
                "pattern": "construction id when one fits, else a short slug"}]}

    Rules: keep their meaning and their words wherever they are right; fix what is wrong, never restyle. \
    Every `span` must be a verbatim substring of their text (a word or a few words), and the edits must not overlap. \
    Handwriting recognition may have mangled a letter: if the error looks like a recognizer slip (a Latin \
    look-alike, a lost soft sign), still correct it, category spelling, and say so in `why`. If the text is \
    already correct, `edits` is [] and `teach` is []. 0–2 teach items; teach the pattern behind the biggest \
    slip, not a list of rules. No preamble, no sign-off.
    """

    /// What the tutor is told about where the learner is.
    static func prompt(entry text: String, prompt: String, book: Book) -> String {
        let met = book.course.letters.filter { book.letterKnown($0.upper) }.map(\.upper).joined(separator: " ")
        let known = book.targets.keys.sorted().filter { $0.hasPrefix("word:") }.compactMap { book.word(String($0.dropFirst(5))) }
            .prefix(160).map { "\($0.ru) = \($0.en)" }.joined(separator: "; ")
        let patterns = Lemmas.load().constructions.map { "\($0.id) (\($0.name))" }.joined(separator: ", ")
        return """
        Letters met: \(met.isEmpty ? "none" : met).
        Words they know: \(known.isEmpty ? "none yet" : known).
        Construction ids: \(patterns).
        \(book.knowledge.summary(limit: 5))

        The diary prompt was: “\(prompt)”
        They wrote:
        ---
        \(text)
        ---
        """
    }

    /// Asks for the correction and parses it. `progress` gets the streamed character count.
    static func correct(text: String, prompt: String, book: Book, progress: @escaping (Int) -> Void = { _ in }) async throws -> DiaryCorrection {
        // Corrections are Claude's alone (OnDevice/TutorBackend.swift says why); offline they wait.
        if case .unavailable(let why) = TutorRouter.route(.diaryCorrection, .live) { throw Failure.unavailable(why) }
        var raw = ""
        for try await chunk in Tutor.stream([.init(role: "user", text: Self.prompt(entry: text, prompt: prompt, book: book))],
                                            system: system, maxTokens: 4000, effort: "medium") {
            raw += chunk
            progress(raw.count)
        }
        guard var c = parse(raw, at: book.now()) else { throw Failure.unreadable }
        c.model = Tutor.model
        return c
    }

    /// The JSON object in a reply, read leniently: `corrected` is the one field that must be
    /// there (a missing one falls back to the learner's text when given); everything else
    /// defaults. Edits with no span are dropped; whitespace is trimmed.
    static func parse(_ reply: String, fallbackText: String? = nil, at date: Date) -> DiaryCorrection? {
        guard let json = PageWriter.extractJSON(reply), let data = json.data(using: .utf8),
              let any = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return nil }
        func str(_ v: Any?) -> String? {
            if let s = v as? String { return s.trimmingCharacters(in: .whitespacesAndNewlines) }
            if let n = v as? NSNumber { return n.stringValue }
            return nil
        }
        guard let corrected = str(any["corrected"]).flatMap({ $0.isEmpty ? nil : $0 }) ?? fallbackText else { return nil }
        var edits: [DiaryEdit] = []
        for item in (any["edits"] as? [Any]) ?? [] {
            guard let e = item as? [String: Any], let span = str(e["span"] ?? e["from"] ?? e["original"]), !span.isEmpty else { continue }
            let replacement = str(e["replacement"] ?? e["to"] ?? e["correct"]) ?? ""
            if RussianText.key(span) == RussianText.key(replacement), RussianText.unstressed(span) == RussianText.unstressed(replacement) { continue }
            let pattern = str(e["pattern"]).flatMap { $0.isEmpty ? nil : $0.replacingOccurrences(of: "gram:", with: "") }
            edits.append(DiaryEdit(span: span, replacement: replacement,
                                   category: DiaryEdit.Category(loose: str(e["category"] ?? e["type"]) ?? "other"),
                                   why: str(e["why"] ?? e["reason"] ?? e["note"]) ?? "",
                                   en: str(e["en"]).flatMap { $0.isEmpty ? nil : $0 }, pattern: pattern))
        }
        var teach: [DiaryTeach] = []
        for item in (any["teach"] as? [Any]) ?? [] {
            if let t = item as? [String: Any], let title = str(t["title"] ?? t["name"]), !title.isEmpty {
                let pattern = str(t["pattern"]).flatMap { $0.isEmpty ? nil : $0.replacingOccurrences(of: "gram:", with: "") }
                teach.append(DiaryTeach(title: title, note: str(t["note"] ?? t["text"] ?? t["explanation"]) ?? "", pattern: pattern))
            } else if let s = item as? String, !s.isEmpty {
                teach.append(DiaryTeach(title: s, note: ""))
            }
        }
        return DiaryCorrection(corrected: corrected, edits: edits, praise: str(any["praise"] ?? any["encouragement"]) ?? "",
                               teach: Array(teach.prefix(2)), at: date)
    }
}
