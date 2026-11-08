import Foundation

/// Stress marks for the Russian that came from outside the course (Resources/library-stress.json,
/// written offline by scripts/library_stress.py and checked by scripts/validate_stress.py): every
/// sentence of every library text and every Tatoeba sentence beside its form with U+0301 after the
/// stressed vowel of each word of two or more syllables. The source text is never altered: a
/// passage is shown stressed through `Theme.stressed` (the underline convention), while ink,
/// audio and gloss keys keep the bare text. A sentence the file lacks reads bare, as before.
struct StressFile: Codable, Sendable {
    var version: Int
    var model: String?
    var texts: [String: TextEntry]

    struct TextEntry: Codable, Sendable {
        var sentences: [Sentence]
    }
    struct Sentence: Codable, Sendable, Hashable {
        var index: Int
        /// The exact source sentence.
        var text: String
        var stressed: String
    }
}

/// The loaded file, indexed by sentence so any passage made from a library text finds its stressed
/// form by the sentence's text alone, and by word for labels (a goal rung, a chip).
struct LibraryStress: Sendable {
    let file: StressFile
    private let bySentence: [String: String]
    private let byWord: [String: String]

    init(file: StressFile) {
        self.file = file
        var sentences: [String: String] = [:]
        for (_, entry) in file.texts.sorted(by: { $0.key < $1.key }) {
            for s in entry.sentences where RussianStress.problems(source: s.text, stressed: s.stressed).isEmpty {
                sentences[RussianText.unstressed(s.text)] = s.stressed
            }
        }
        bySentence = sentences
        byWord = Self.wordIndex(sentences.values.sorted())
    }

    /// bare lowercased word → its stressed spelling, over stressed sentences: one spelling per written
    /// word, so a word the sentences stress two ways (a homograph) gives no label.
    static func wordIndex(_ stressedSentences: some Sequence<String>) -> [String: String] {
        var words: [String: String?] = [:]
        let mark = RussianText.stressMark.unicodeScalars.first!
        for s in stressedSentences {
            for w in Structure.wordStrings(in: s) where w.unicodeScalars.contains(mark) {
                let low = w.lowercased()
                let bare = RussianText.unstressed(low)
                if let known = words[bare] { if known != low { words[bare] = .some(nil) } } else { words[bare] = low }
            }
        }
        return words.compactMapValues { $0 }
    }

    static let empty = LibraryStress(file: StressFile(version: 1, model: nil, texts: [:]))

    /// Bundled file merged with PrivatePack/library-stress.json when the build carries one.
    static func load(bundle: Bundle = .main) -> LibraryStress {
        func read(_ url: URL?) -> StressFile? {
            guard let url, let data = try? Data(contentsOf: url) else { return nil }
            return try? JSONDecoder().decode(StressFile.self, from: data)
        }
        guard var file = read(bundle.url(forResource: "library-stress", withExtension: "json")) else { return .empty }
        if let pack = read(bundle.url(forResource: "PrivatePack", withExtension: nil)?.appendingPathComponent("library-stress.json")) {
            for (id, entry) in pack.texts where file.texts[id] == nil { file.texts[id] = entry }
        }
        return LibraryStress(file: file)
    }

    /// Loaded once per process; the file is static data.
    static let shared = LibraryStress.load()

    var isEmpty: Bool { bySentence.isEmpty }
    var sentenceCount: Int { bySentence.count }

    /// The stressed form of one sentence as the file stores it (marks the caller's copy may already
    /// carry are ignored), or nil when the file does not have it.
    func stressed(sentence: String) -> String? { bySentence[RussianText.unstressed(sentence)] }

    /// A written word's stressed spelling, when the library stresses it one way (lowercased).
    func stressed(word: String) -> String? { byWord[RussianText.unstressed(word).lowercased()] }

    /// A passage with every sentence the file knows replaced by its stressed form, in place:
    /// whitespace, line breaks and the sentences it does not know stay exactly as they were.
    func stressed(passage: String) -> String {
        Self.apply(to: passage) { stressed(sentence: $0) }
    }

    /// `passage` with each sentence (GoalAnalysis.splitSentences, per line) that `lookup` resolves
    /// replaced by that form, in place.
    static func apply(to passage: String, lookup: (String) -> String?) -> String {
        if let whole = lookup(passage) { return whole }
        var out = ""
        for (i, line) in passage.split(separator: "\n", omittingEmptySubsequences: false).enumerated() {
            if i > 0 { out += "\n" }
            var p = String(line)
            var from = p.startIndex
            for s in GoalAnalysis.splitSentences(p) {
                guard let r = p.range(of: s, range: from..<p.endIndex) else { continue }
                if let st = lookup(s), RussianText.unstressed(st) == RussianText.unstressed(s) {
                    p.replaceSubrange(r, with: st)
                    from = p.index(r.lowerBound, offsetBy: st.count)
                } else {
                    from = r.upperBound
                }
            }
            out += p
        }
        return out
    }
}

extension Library.Text {
    /// A sentence of this text with its stress marks, or the sentence as written when none are stored.
    func stressed(sentence: String) -> String { LibraryStress.shared.stressed(sentence: sentence) ?? sentence }
    /// The whole text with its stress marks where they are stored.
    var stressedText: String { LibraryStress.shared.stressed(passage: text) }
}

/// The mechanical rule a stressed form must pass before the app shows it (scripts/library_stress.py
/// `problems`, repeated here for what the tutor writes at runtime): removing U+0301 gives the source
/// back exactly; every Cyrillic word of two or more syllables carries exactly one mark, after a
/// vowel; one-syllable words and words with ё carry none; in a hyphenated compound each part follows
/// that rule, except that a compound of one-syllable parts may carry one mark in all (кто́-то).
enum RussianStress {
    static let vowels: Set<Character> = ["а", "е", "ё", "и", "о", "у", "ы", "э", "ю", "я"]
    private static let mark = RussianText.stressMark.unicodeScalars.first!

    static func syllables(_ bare: String) -> Int { bare.lowercased().filter { vowels.contains($0) }.count }

    /// A written word that must carry a mark: two or more vowels and no ё (hyphen parts separately).
    static func needsMark(_ bare: String) -> Bool {
        bare.split(separator: "-").contains { syllables(String($0)) >= 2 && !$0.lowercased().contains("ё") }
    }

    /// A string with a Cyrillic word that needs a mark and has none.
    static func lacksStress(_ s: String) -> Bool {
        Structure.wordStrings(in: s).contains { w in
            w.unicodeScalars.contains { (0x0400...0x04FF).contains($0.value) } && !w.unicodeScalars.contains(mark) && needsMark(w)
        }
    }

    /// Offsets (in the unstressed text's scalars) of the letters that carry a mark.
    static func markPositions(_ s: String) -> Set<Int> {
        var out = Set<Int>(), n = 0
        for u in s.unicodeScalars { if u == mark { out.insert(n - 1) } else { n += 1 } }
        return out
    }

    /// Everything wrong with `stressed` as the stress marking of `source`; empty when it is sound. A
    /// mark the source already carries (a Wikisource text marks the odd word: Верти́т) must still be there.
    static func problems(source: String, stressed: String) -> [String] {
        guard RussianText.unstressed(stressed) == RussianText.unstressed(source) else { return ["the text differs from the source once marks are removed"] }
        guard markPositions(source).isSubset(of: markPositions(stressed)) else { return ["a mark the source carries was moved or dropped"] }
        var out: [String] = []
        for w in Structure.wordStrings(in: stressed) {
            let scalars = Array(w.lowercased().unicodeScalars)
            guard scalars.contains(where: { (0x0400...0x04FF).contains($0.value) }) else {
                if scalars.contains(mark) { out.append("\(w): mark on a non-Cyrillic token") }
                continue
            }
            for (i, u) in scalars.enumerated() where u == mark {
                if i == 0 || !vowels.contains(Character(scalars[i - 1])) { out.append("\(w): mark not after a vowel") }
            }
            let parts = w.lowercased().split(separator: "-", omittingEmptySubsequences: false).map(String.init)
            let marks = { (p: String) in p.unicodeScalars.filter { $0 == mark }.count }
            if parts.count > 1, parts.allSatisfy({ syllables(RussianText.unstressed($0)) <= 1 }) {
                if marks(w) > 1 { out.append("\(w): a compound of one-syllable parts takes at most one mark") }
                continue
            }
            for p in parts {
                let plain = RussianText.unstressed(p)
                let n = marks(p)
                if plain.contains("ё") {
                    if n > 0 { out.append("\(w): a word with ё takes no mark") }
                } else if syllables(plain) >= 2 {
                    if n != 1 { out.append("\(w): \(syllables(plain)) syllables need exactly one mark, found \(n)") }
                } else if n > 0 {
                    out.append("\(w): a one-syllable word takes no mark")
                }
            }
        }
        return out
    }
}
