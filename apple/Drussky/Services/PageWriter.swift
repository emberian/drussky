import Foundation

/// Has the tutor write a workbook page in the course's own format: either practice built from
/// what the learner keeps missing, or a reading page made from a text they bring. The page is
/// validated exercise by exercise before it goes into the book.
@MainActor
enum PageWriter {
    enum Request {
        case practice
        case text(String)
    }

    struct Written: Decodable {
        var title: String
        var intro: String?
        var exercises: [Exercise]
        var new_words: [Course.Word]?
    }

    enum Failure: LocalizedError {
        case unreadable, tooFew
        var errorDescription: String? {
            switch self {
            case .unreadable: "The tutor's page didn't come back in a form I could use. Try again."
            case .tooFew: "The tutor's page had too few usable exercises. Try again."
            }
        }
    }

    /// Streams the tutor's draft (for a progress indicator), then returns the validated page.
    static func write(_ request: Request, book: Book, progress: @escaping (Int) -> Void) async throws -> (Written, [Exercise]) {
        let prompt = Self.prompt(for: request, book: book)
        var raw = ""
        for try await chunk in Tutor.stream([.init(role: "user", text: prompt)], system: system, maxTokens: 16_000, effort: "medium") {
            raw += chunk
            progress(raw.count)
        }
        guard let json = extractJSON(raw), let data = json.data(using: .utf8),
              let written = try? JSONDecoder().decode(Written.self, from: data) else { throw Failure.unreadable }
        let allowPassageLetters: Bool = { if case .text = request { return true } else { return false } }()
        let usable = written.exercises.compactMap { sanitize($0, book: book, passageAnyLetters: allowPassageLetters) }
        guard usable.count >= 4 else { throw Failure.tooFew }
        return (written, usable)
    }

    static let system = """
    You write single pages for Drussky, a Russian exercise book used by one adult English speaker (quick, \
    curious; wants to read Russian science fiction, mathematics and philosophy). Reply with ONE JSON object \
    and nothing else: {"title": "...", "intro": "one or two warm, direct sentences", "exercises": [...], \
    "new_words": [{"ru": "...", "en": "..."}]}.

    Exercise kinds and their fields (use exactly these names):
    - {"kind":"explain","text":"≤3 sentences","examples":[{"ru":"...","en":"..."}]}
    - {"kind":"trace","text":"..."} / {"kind":"copy","text":"...","times":1-3}
    - {"kind":"listen_pick","say":"...","options":["...","..."]}  (say is one of options)
    - {"kind":"dictation","text":"..."}  (hear it, write it)
    - {"kind":"read_aloud","text":"..."}
    - {"kind":"translate","en":"...","answer":"..."}  (English prompt must have exactly one right Russian answer)
    - {"kind":"fill","before":"... ","after":"...","hint":"English of the whole sentence","answer":"..."}
    - {"kind":"read_meaning","text":"...","options":["English", "..."],"answer":0}
    - {"kind":"passage","text":"2–8 sentences; dialogue turns or verse lines separated by \\n","en":"translation","glosses":{"word as written, lowercase, no stress":"meaning here"}}

    Rules: mark stress with U+0301 on every Russian word of 2+ syllables (never on ё, never on one-syllable \
    words). Writing exercises (trace, copy, dictation, translate, fill) should be the majority except on reading \
    pages. Never use letters the learner hasn't met. English prompts must be unambiguous (say "you (one person)" \
    etc.). One-line instructions only; no talk about apps, recordings or accuracy. Russian must be correct and \
    natural. new_words lists every new word or phrase the page teaches that the learner should remember.
    """

    static func prompt(for request: Request, book: Book) -> String {
        let met = book.course.letters.filter { book.letterKnown($0.upper) }.map(\.upper).joined(separator: " ")
        let known = book.memories.keys.filter { $0.hasPrefix("word:") }.compactMap { book.word(String($0.dropFirst(5))) }
            .prefix(160).map { "\($0.ru) = \($0.en)" }.joined(separator: "; ")
        let struggles = book.memories.values.filter { $0.lapses > 0 || $0.stability < 1 }
            .sorted { ($0.lapses, -$0.stability) > ($1.lapses, -$1.stability) }
            .prefix(15).map { $0.key.replacingOccurrences(of: "word:", with: "").replacingOccurrences(of: "letter:", with: "letter ") }
            .joined(separator: ", ")
        var recentMisses: [String] = []
        for page in book.pages.suffix(6) {
            for (i, r) in page.results.enumerated() where r.status == .missed || (r.attempts > 1 && r.status != .correct) {
                if let t = page.exercises[i].target { recentMisses.append(t) }
            }
        }
        let state = """
        Letters met: \(met.isEmpty ? "none" : met)\(met.split(separator: " ").count >= 33 ? " (all 33)" : "").
        Words they know: \(known.isEmpty ? "none yet" : known).
        Hardest items lately: \(struggles.isEmpty ? "none recorded" : struggles).
        Recently missed answers: \(recentMisses.isEmpty ? "none" : recentMisses.joined(separator: ", ")).
        Current course page: \(book.page?.title ?? "") (\(book.page?.unitTitle ?? "")).
        """
        switch request {
        case .practice:
            return """
            \(state)

            Write a practice page of 8–10 exercises that brings back what they find hardest, mixing modalities \
            (hear→write, English→write, see→say) and spacing repeats of the same item apart. Use only known words \
            plus at most two new ones (list them in new_words). No passage.
            """
        case .text(let text):
            return """
            \(state)

            The learner brought this Russian text to read:
            ---
            \(text.prefix(2500))
            ---
            Make a reading page: first ONE passage exercise with (a stretch of) the text — at most 8 sentences, \
            keep the original wording, add stress marks, gloss every word that is not in their known words, and \
            give an English translation. Then 5–7 graded exercises about it: read_meaning questions on what it \
            says, and translate/fill/dictation for its most useful new words (list those in new_words). This page \
            may use letters they haven't met inside the passage only.
            """
        }
    }

    /// The first top-level JSON object in a reply (tolerates code fences and stray prose).
    static func extractJSON(_ s: String) -> String? {
        guard let start = s.firstIndex(of: "{") else { return nil }
        var depth = 0, inString = false, escaped = false
        var i = start
        while i < s.endIndex {
            let c = s[i]
            if inString {
                if escaped { escaped = false } else if c == "\\" { escaped = true } else if c == "\"" { inString = false }
            } else if c == "\"" { inString = true }
            else if c == "{" { depth += 1 }
            else if c == "}" { depth -= 1; if depth == 0 { return String(s[start...i]) } }
            i = s.index(after: i)
        }
        return nil
    }

    /// Drops exercises that are malformed or use letters the learner hasn't met.
    static func sanitize(_ ex: Exercise, book: Book, passageAnyLetters: Bool) -> Exercise? {
        let met = Set(book.course.letters.filter { book.letterKnown($0.upper) }.map(\.upper))
        let allMet = met.count >= book.course.letters.count
        func lettersOK(_ s: String?) -> Bool { allMet || s.map { RussianText.letters(in: $0).isSubset(of: met) } ?? true }
        func nonEmpty(_ s: String?) -> Bool { !(s ?? "").trimmingCharacters(in: .whitespaces).isEmpty }
        switch ex.kind {
        case .meet: return nil  // letters are introduced only by the course
        case .explain:
            guard nonEmpty(ex.text), lettersOK(ex.text) else { return nil }
            var e = ex
            e.examples = ex.examples?.filter { lettersOK($0.ru) }
            return e
        case .trace, .copy, .dictation, .readAloud:
            return nonEmpty(ex.text) && lettersOK(ex.text) ? ex : nil
        case .listenPick:
            guard let say = ex.say, let options = ex.options, options.contains(say), options.count >= 2,
                  options.allSatisfy(lettersOK) else { return nil }
            return ex
        case .translate:
            return nonEmpty(ex.en) && nonEmpty(ex.answerText) && lettersOK(ex.answerText) ? ex : nil
        case .fill:
            guard nonEmpty(ex.answerText), nonEmpty(ex.hint), lettersOK(ex.answerText), lettersOK(ex.before), lettersOK(ex.after) else { return nil }
            return ex
        case .readMeaning:
            guard nonEmpty(ex.text), lettersOK(ex.text), let options = ex.options, let i = ex.answerIndex,
                  options.indices.contains(i), options.count >= 2 else { return nil }
            return ex
        case .passage:
            guard nonEmpty(ex.text), passageAnyLetters || lettersOK(ex.text) else { return nil }
            return ex
        }
    }
}
