import Foundation

/// Word timings of a human narration: `Audio/narr-<text id>.words.json`, written by
/// `scripts/narration.py build`. `words` are, in order, every whitespace token of the text whose
/// bare form (outer punctuation off, stress marks off) contains a letter -- `Narration.timedTokens`
/// applies the same rule, so a text's tokens zip with this list.
struct WordTimings: Codable, Sendable, Equatable {
    struct Word: Codable, Sendable, Equatable {
        var word: String
        var start: Double
        var end: Double

        init(_ word: String, _ start: Double, _ end: Double) {
            self.word = word; self.start = start; self.end = end
        }

        // On disk a word is the compact array ["слово", 1.23, 1.58].
        init(from decoder: Decoder) throws {
            var c = try decoder.unkeyedContainer()
            word = try c.decode(String.self)
            start = try c.decode(Double.self)
            end = try c.decode(Double.self)
        }

        func encode(to encoder: Encoder) throws {
            var c = encoder.unkeyedContainer()
            try c.encode(word); try c.encode(start); try c.encode(end)
        }
    }

    var version: Int
    var text: String
    var file: String
    var duration: Double
    var reader: String?
    var words: [Word]

    /// Problems that make the timings unusable for `text`; empty when sound.
    func problems(for text: String) -> [String] {
        var out: [String] = []
        let expected = Narration.timedTokens(text).map(\.word)
        if expected != words.map(\.word) {
            out.append("words differ from the text (\(words.count) timed, \(expected.count) in text)")
        }
        var last = 0.0
        for w in words {
            if !(0 <= w.start && w.start <= w.end && w.end <= duration + 0.01) { out.append("\(w.word): \(w.start)–\(w.end) outside 0–\(duration)") }
            if w.start < last - 0.005 { out.append("\(w.word): starts at \(w.start) before the previous word ended (\(last))") }
            last = w.end
        }
        return out
    }
}

/// Everything built from a narration's word timings: which tokens of a text are timed, which word
/// is being said at a playback time, where its sentences are, and the read-along page itself.
enum Narration {
    /// A whitespace token of a text that carries a timing, with its index among all tokens.
    struct TimedToken: Equatable, Sendable {
        var tokenIndex: Int
        var word: String
    }

    static let audioDir = Bundle.main.url(forResource: "Audio", withExtension: nil)

    static func url(_ file: String) -> URL? {
        guard let u = audioDir?.appendingPathComponent(file), FileManager.default.fileExists(atPath: u.path) else { return nil }
        return u
    }

    nonisolated(unsafe) private static var cache: [String: WordTimings] = [:]
    private static let lock = NSLock()

    /// The timings file by name (cached; nil when absent or unreadable).
    static func timings(_ file: String) -> WordTimings? {
        lock.lock(); defer { lock.unlock() }
        if let t = cache[file] { return t }
        guard let u = url(file), let data = try? Data(contentsOf: u), let t = try? JSONDecoder().decode(WordTimings.self, from: data) else { return nil }
        cache[file] = t
        return t
    }

    static let punctuation = Set(".,?!—–-:;…«»\"()*'’“”‘[]")

    /// A token's bare word: outer punctuation off, stress marks off (scripts/narration.py `bare`).
    static func bare(_ token: Substring) -> String {
        let core = token.drop(while: { punctuation.contains($0) }).reversed().drop(while: { punctuation.contains($0) }).reversed()
        return String(String(core).unicodeScalars.filter { $0 != "\u{0301}" }.map(Character.init))
    }

    static func tokens(_ text: String) -> [Substring] {
        text.split(whereSeparator: \.isWhitespace)
    }

    /// The tokens that get a timing: those whose bare form has a letter (digits, ***, dashes do not).
    static func timedTokens(_ text: String) -> [TimedToken] {
        var out: [TimedToken] = []
        for (i, tok) in tokens(text).enumerated() {
            let b = bare(tok)
            if b.contains(where: \.isLetter) { out.append(TimedToken(tokenIndex: i, word: b)) }
        }
        return out
    }

    /// For each token of `text`, the index of its timing in `timings.words`, or nil. When the text is
    /// the narrated text itself the zip is exact; when it is a stretch of it (a passage the tutor cut
    /// from a library text) the stretch is found as the best-matching run of words. Nil when fewer
    /// than two thirds of the text's words can be placed.
    static func map(_ text: String, onto timings: WordTimings) -> [Int?]? {
        let timed = timedTokens(text)
        let count = tokens(text).count
        guard !timed.isEmpty, !timings.words.isEmpty else { return nil }
        var result = [Int?](repeating: nil, count: count)
        let keys = timings.words.map { RussianText.key($0.word) }
        let mine = timed.map { RussianText.key($0.word) }
        if mine == keys {
            for (k, t) in timed.enumerated() { result[t.tokenIndex] = k }
            return result
        }
        // The stretch: the offset where the most words agree, then only the agreeing words are timed.
        var bestOffset = 0, bestHits = -1
        if mine.count <= keys.count {
            for offset in 0...(keys.count - mine.count) {
                var hits = 0
                for (j, m) in mine.enumerated() where keys[offset + j] == m { hits += 1 }
                if hits > bestHits { bestHits = hits; bestOffset = offset }
            }
        }
        guard bestHits * 3 >= mine.count * 2 else { return nil }
        for (j, t) in timed.enumerated() where keys[bestOffset + j] == mine[j] { result[t.tokenIndex] = bestOffset + j }
        return result
    }

    /// The word being said at `time`: the last word that has started, kept through the pause after it
    /// until the next begins; nil before the first word and after the last one has ended.
    static func wordIndex(at time: Double, in words: [WordTimings.Word]) -> Int? {
        guard let first = words.first, time >= first.start else { return nil }
        if let last = words.last, time > last.end { return nil }
        // Binary search for the last start <= time.
        var lo = 0, hi = words.count - 1
        while lo < hi {
            let mid = (lo + hi + 1) / 2
            if words[mid].start <= time { lo = mid } else { hi = mid - 1 }
        }
        return lo
    }

    /// Sentences as ranges of timing indices: a chunk ends at a token with sentence punctuation
    /// (. ! ? …) or at the end of a line (a dialogue turn, a verse line).
    static func chunks(_ text: String) -> [ClosedRange<Int>] {
        var out: [ClosedRange<Int>] = []
        var k = 0, start: Int? = nil
        for line in text.split(separator: "\n", omittingEmptySubsequences: true) {
            let toks = tokens(String(line))
            for (i, tok) in toks.enumerated() {
                let timed = bare(tok).contains(where: \.isLetter)
                if timed { if start == nil { start = k }; k += 1 }
                let ends = tok.contains(where: { ".!?…".contains($0) }) || i == toks.count - 1
                if ends, let s = start, k > s { out.append(s...(k - 1)); start = nil }
            }
        }
        return out
    }

    /// The text of a chunk as written: the tokens from its first timed word to its last, joined.
    static func chunkText(_ text: String, _ range: ClosedRange<Int>) -> String {
        let toks = tokens(text)
        let timed = timedTokens(text)
        guard range.lowerBound >= 0, range.upperBound < timed.count else { return "" }
        return toks[timed[range.lowerBound].tokenIndex...timed[range.upperBound].tokenIndex].joined(separator: " ")
    }

    /// Where a chunk plays in the clip: from a little before its first word to a little after its last.
    static func interval(_ range: ClosedRange<Int>, in timings: WordTimings) -> (start: Double, end: Double) {
        let words = timings.words
        guard range.lowerBound >= 0, range.upperBound < words.count else { return (0, timings.duration) }
        let before = range.lowerBound > 0 ? words[range.lowerBound - 1].end : 0
        let after = range.upperBound + 1 < words.count ? words[range.upperBound + 1].start : timings.duration
        return (max(before, words[range.lowerBound].start - 0.25), min(after, words[range.upperBound].end + 0.35))
    }

    /// The interval of an exercise's narration: its `span` of words, or the whole clip.
    static func interval(of ex: Exercise) -> (file: String, start: Double, end: Double)? {
        guard let file = ex.narration else { return nil }
        if let words = ex.words, let t = timings(words), let span = ex.span, span.count == 2 {
            let i = interval(span[0]...span[1], in: t)
            return (file, i.start, i.end)
        }
        return (file, 0, .infinity)
    }

    // MARK: The read-along page

    /// A page built locally from a narrated library text: the passage with its narration, then two
    /// sentences to write from hearing (chunk dictation) and two to say after the reader (shadowing).
    static func page(for text: Library.Text) -> (title: String, intro: String, exercises: [Exercise]) {
        var passage = Exercise(kind: .passage)
        passage.text = text.text
        passage.narration = text.narration
        passage.words = text.words
        var exercises = [passage]
        if let words = text.words, let timings = timings(words), timings.problems(for: text.text).isEmpty {
            let all = chunks(text.text)
            // Sentences of 3–9 words, no digits or stray symbols, shortest first; alternate dictation and shadowing.
            let usable = all.filter { r in
                let n = r.count
                let t = chunkText(text.text, r)
                return (3...9).contains(n) && !t.contains(where: \.isNumber) && !t.contains("*")
            }.sorted { $0.count < $1.count }
            let picks = Array(usable.prefix(4))
            for (i, r) in picks.enumerated() {
                var ex = Exercise(kind: i % 2 == 0 ? .dictation : .readAloud)
                ex.text = chunkText(text.text, r)
                ex.narration = text.narration
                ex.words = text.words
                ex.span = [r.lowerBound, r.upperBound]
                exercises.append(ex)
            }
        }
        let reader = text.reader.map { " Read by \($0)." } ?? ""
        return (text.title, "Listen to the whole text at the reader's pace, following the words; then write two of its sentences from hearing and say two after the reader.\(reader)", exercises)
    }
}
