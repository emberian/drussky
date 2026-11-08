import Foundation

/// The structure layer of the library (Resources/library-structure.json, written offline by
/// scripts/library_structure.py and checked by scripts/validate_structure.py): for every sentence
/// of every text, its members and clauses, what each pronoun and participle points at, flat Russian
/// for dense constructions, the root x-ray of each content word, and the case frames it uses.
/// Shown by the «Структу́ра» toggle on a reading page (StructureViews.swift); asked by the
/// `structure` exercise (tap the head a pronoun belongs to).
struct StructureFile: Codable, Sendable {
    var version: Int
    var model: String?
    var roles: [String: Role]
    var texts: [String: TextEntry]

    struct Role: Codable, Sendable, Hashable {
        var ru: String
        var en: String
        /// "member" (underlined like a school sentence analysis) or "clause" (bracketed).
        var tier: String
    }
    struct TextEntry: Codable, Sendable {
        var sentences: [StructureSentence]
    }
}

/// One sentence's structure. Word indices refer to `words`, the sentence's words in order
/// (`Structure.words(in:)`, the script's rule: letters, digits and stress marks, hyphenated compounds one word).
struct StructureSentence: Codable, Sendable, Hashable {
    var index: Int
    /// The exact source sentence; the file is stale for a sentence whose text changed.
    var text: String
    /// [start, end) in Unicode scalars of `text`, one per word.
    var words: [[Int]]
    var chunks: [Chunk]
    var referents: [Referent]
    var desugar: [Desugaring]
    var roots: [XRay]
    var governance: [Frame]

    /// A member of the sentence or an embedded clause: words [first, last] inclusive.
    struct Chunk: Codable, Sendable, Hashable {
        var words: [Int]
        var span: [Int]
        var role: String
        var first: Int { words[0] }
        var last: Int { words[1] }
        func contains(_ w: Int) -> Bool { words.count == 2 && first <= w && w <= last }
        var length: Int { last - first + 1 }
    }
    /// A pronoun, participle, adjective or relative word and the head it agrees with or refers to.
    struct Referent: Codable, Sendable, Hashable {
        var word: Int
        var head: Int
        var headSentence: Int
        var kind: String
    }
    /// A dense construction and its flat Russian (then English) paraphrase.
    struct Desugaring: Codable, Sendable, Hashable {
        var words: [Int]
        var span: [Int]
        var kind: String
        var ru: String
        var en: String
        func contains(_ w: Int) -> Bool { words.count == 2 && words[0] <= w && w <= words[1] }
    }
    /// A content word taken apart as written, with the taught words on the same root.
    struct XRay: Codable, Sendable, Hashable {
        var word: Int
        var prefix: String
        var root: String
        var suffix: String
        var inflection: String
        var rootID: String?
        var gloss: String
        var relatives: [String]
        var source: String
        var pieces: [(String, String)] {
            [(prefix, "prefix"), (root, "root"), (suffix, "suffix"), (inflection, "ending")].filter { !$0.0.isEmpty }
        }
    }
    /// A verb, preposition, adjective or noun and the case it asks of its dependents.
    struct Frame: Codable, Sendable, Hashable {
        var head: Int
        var dependents: [Int]
        var frame: String
        var `case`: String
    }

    var wordCount: Int { words.count }

    /// The word as written in the sentence.
    func word(_ i: Int) -> String {
        guard words.indices.contains(i), words[i].count == 2 else { return "" }
        let scalars = Array(text.unicodeScalars)
        guard words[i][0] >= 0, words[i][1] <= scalars.count, words[i][0] <= words[i][1] else { return "" }
        return String(String.UnicodeScalarView(scalars[words[i][0]..<words[i][1]]))
    }

    /// Member chunks containing the word, smallest first.
    func members(of w: Int, roles: [String: StructureFile.Role]) -> [Chunk] {
        chunks.filter { $0.contains(w) && roles[$0.role]?.tier == "member" }.sorted { $0.length < $1.length }
    }
    /// Clause chunks containing the word, innermost first.
    func clauses(of w: Int, roles: [String: StructureFile.Role]) -> [Chunk] {
        chunks.filter { $0.contains(w) && roles[$0.role]?.tier == "clause" }.sorted { $0.length < $1.length }
    }
    func referent(at w: Int) -> Referent? { referents.first { $0.word == w } }
    func desugaring(at w: Int) -> Desugaring? { desugar.filter { $0.contains(w) }.min { ($0.words[1] - $0.words[0]) < ($1.words[1] - $1.words[0]) } }
    func xray(at w: Int) -> XRay? { roots.first { $0.word == w } }
    func frame(headedBy w: Int) -> Frame? { governance.first { $0.head == w } }
    /// Words that are heads of a referent inside this sentence.
    var heads: Set<Int> { Set(referents.filter { $0.headSentence == index }.map(\.head)) }
}

/// The loaded layer, indexed by sentence so any passage made from a library text (as written, read
/// along, the goal's first page) finds its structure by the sentence's text alone.
struct Structure: Sendable {
    let file: StructureFile
    private let bySentence: [String: (text: String, sentence: StructureSentence)]

    init(file: StructureFile) {
        self.file = file
        var index: [String: (String, StructureSentence)] = [:]
        for (id, entry) in file.texts.sorted(by: { $0.key < $1.key }) {
            for s in entry.sentences {
                let k = RussianText.key(s.text)
                if !k.isEmpty, index[k] == nil { index[k] = (id, s) }
            }
        }
        bySentence = index
    }

    static let empty = Structure(file: StructureFile(version: 1, model: nil, roles: [:], texts: [:]))

    /// Bundled layer merged with PrivatePack/library-structure.json when the build carries one.
    static func load(bundle: Bundle = .main) -> Structure {
        func read(_ url: URL?) -> StructureFile? {
            guard let url, let data = try? Data(contentsOf: url) else { return nil }
            return try? JSONDecoder().decode(StructureFile.self, from: data)
        }
        guard var file = read(bundle.url(forResource: "library-structure", withExtension: "json")) else { return .empty }
        if let pack = read(bundle.url(forResource: "PrivatePack", withExtension: nil)?.appendingPathComponent("library-structure.json")) {
            for (id, entry) in pack.texts where file.texts[id] == nil { file.texts[id] = entry }
            for (id, role) in pack.roles where file.roles[id] == nil { file.roles[id] = role }
        }
        return Structure(file: file)
    }

    /// Loaded once per process; the file is static data.
    static let shared = Structure.load()

    var roles: [String: StructureFile.Role] { file.roles }
    func role(_ id: String) -> StructureFile.Role? { file.roles[id] }
    var isEmpty: Bool { file.texts.isEmpty }

    /// The sentences of a library text, in order.
    func sentences(of textID: String) -> [StructureSentence] { file.texts[textID]?.sentences ?? [] }

    /// The structure of a sentence as it appears on a page (stress marks and punctuation spacing
    /// may differ from the source; the lookup key ignores them), when its words line up.
    func sentence(matching s: String) -> StructureSentence? {
        guard let hit = bySentence[RussianText.key(s)], Self.words(in: s).count == hit.sentence.wordCount else { return nil }
        return hit.sentence
    }

    /// Which text a sentence belongs to.
    func textID(ofSentence s: String) -> String? { bySentence[RussianText.key(s)]?.text }

    // MARK: Words

    /// [start, end) Unicode-scalar ranges of the words of a sentence: runs of letters, digits and
    /// stress marks, hyphenated compounds as one word (the script's WORD regex).
    static func words(in s: String) -> [Range<Int>] {
        let scalars = Array(s.unicodeScalars)
        func isWordScalar(_ u: Unicode.Scalar) -> Bool {
            if u == "\u{0301}" { return true }
            switch u.properties.generalCategory {
            case .uppercaseLetter, .lowercaseLetter, .titlecaseLetter, .modifierLetter, .otherLetter,
                 .decimalNumber, .letterNumber, .otherNumber: return true
            default: return false
            }
        }
        var out: [Range<Int>] = []
        var i = 0
        while i < scalars.count {
            guard isWordScalar(scalars[i]) else { i += 1; continue }
            let start = i
            while i < scalars.count {
                if isWordScalar(scalars[i]) { i += 1; continue }
                // A hyphen joins two word parts (кто-то, по-русски).
                if scalars[i] == "-", i + 1 < scalars.count, isWordScalar(scalars[i + 1]), i > start { i += 1; continue }
                break
            }
            out.append(start..<i)
        }
        return out
    }

    /// The words of a sentence as strings.
    static func wordStrings(in s: String) -> [String] {
        let scalars = Array(s.unicodeScalars)
        return words(in: s).map { String(String.UnicodeScalarView(scalars[$0])) }
    }

    // MARK: Exercises

    /// Referent kinds worth asking about: things that point at a noun.
    static let askableKinds: Set<String> = ["pronoun", "participle", "relative", "possessive"]

    /// `structure` exercises for a text: one pronoun or participle each, head in the same sentence,
    /// spread through the text, deterministic. Sentences of 5–24 words; a sentence asked once.
    func exercises(for textID: String, limit: Int = 2, avoiding used: Set<String> = []) -> [Exercise] {
        var candidates: [(StructureSentence, StructureSentence.Referent)] = []
        for s in sentences(of: textID) where (5...24).contains(s.wordCount) && !used.contains(s.text) {
            let refs = s.referents.filter { $0.headSentence == s.index && Self.askableKinds.contains($0.kind) && $0.head != $0.word }
            // Prefer a participle (the construction reading needs most), else the first pronoun.
            if let r = refs.first(where: { $0.kind == "participle" }) ?? refs.first { candidates.append((s, r)) }
        }
        guard !candidates.isEmpty, limit > 0 else { return [] }
        var picks: [(StructureSentence, StructureSentence.Referent)] = []
        let step = max(1, candidates.count / limit)
        var i = 0
        while picks.count < limit && i < candidates.count { picks.append(candidates[i]); i += step }
        return picks.map { s, r in Self.exercise(sentence: s, referent: r) }
    }

    static func exercise(sentence s: StructureSentence, referent r: StructureSentence.Referent) -> Exercise {
        var ex = Exercise(kind: .structure)
        ex.text = s.text
        ex.probe = r.word
        ex.answerIndex = r.head
        ex.word = s.word(r.word)
        ex.skills = [r.kind == "participle" ? "participle" : "anaphora"]
        return ex
    }

    /// The word a `structure` exercise asks about and the one it expects, as written.
    static func probe(of ex: Exercise) -> (probe: String, head: String)? {
        guard ex.kind == .structure, let text = ex.text, let p = ex.probe, let h = ex.answerIndex else { return nil }
        let words = wordStrings(in: text)
        guard words.indices.contains(p), words.indices.contains(h) else { return nil }
        return (words[p], words[h])
    }

    /// A reading page made locally from a library text: the text as written, then two structure
    /// questions on it (no tutor needed).
    func page(for text: Library.Text) -> (title: String, intro: String, exercises: [Exercise]) {
        var passage = Exercise(kind: .passage)
        passage.text = text.text
        let questions = exercises(for: text.id, limit: 2)
        // A text the layer has not reached yet reads plainly; the toggle is not offered for it.
        let intro = sentences(of: text.id).isEmpty ? "Read it as written. Tap any word you want to check."
            : questions.isEmpty ? "Read it as written. Tap any word you want to check; «Структу́ра» shows the skeleton of each sentence."
            : "Read it as written; «Структу́ра» shows the skeleton of each sentence. Then say who is meant: tap the word a pronoun points at."
        return (text.title, intro, [passage] + questions)
    }
}

/// A passage's text laid over its structure: each space-separated display token of each line knows
/// which structure sentence it is in and which of its words it carries.
struct PassageStructure {
    struct Token: Hashable {
        /// Index into `sentences`, nil for a token outside any matched sentence.
        var sentence: Int?
        /// The structure words this token carries (empty for a bare dash or a stray mark).
        var words: Range<Int>
    }
    var sentences: [StructureSentence]
    /// Per line, per display token (`text.split(separator: " ")` of the line).
    var lines: [[Token]]

    /// nil when no sentence of the text has structure.
    init?(text: String, structure: Structure = .shared) {
        guard !structure.isEmpty else { return nil }
        var sentences: [StructureSentence] = []
        var lines: [[Token]] = []
        for line in text.split(separator: "\n", omittingEmptySubsequences: true).map(String.init) {
            let scalars = Array(line.unicodeScalars)
            // Display tokens with their scalar ranges (split on spaces, empties dropped).
            var tokens: [(range: Range<Int>, token: Token)] = []
            var i = 0
            while i < scalars.count {
                if scalars[i].properties.isWhitespace { i += 1; continue }
                let start = i
                while i < scalars.count, !scalars[i].properties.isWhitespace { i += 1 }
                tokens.append((start..<i, Token(sentence: nil, words: 0..<0)))
            }
            // Sentences of the line, located in order.
            var cursor = 0
            for s in GoalAnalysis.splitSentences(line) {
                let sScalars = Array(s.unicodeScalars)
                guard let at = Self.find(sScalars, in: scalars, from: cursor) else { continue }
                cursor = at + sScalars.count
                guard let matched = structure.sentence(matching: s) else { continue }
                let si = sentences.count
                sentences.append(matched)
                let wordStarts = Structure.words(in: s).map { $0.lowerBound + at }
                for t in tokens.indices where tokens[t].range.overlaps(at..<cursor) {
                    let inside = wordStarts.indices.filter { tokens[t].range.contains(wordStarts[$0]) }
                    let words = inside.isEmpty ? 0..<0 : (inside[0])..<(inside[inside.count - 1] + 1)
                    tokens[t].token = Token(sentence: si, words: words)
                }
            }
            lines.append(tokens.map(\.token))
        }
        guard !sentences.isEmpty else { return nil }
        self.sentences = sentences
        self.lines = lines
    }

    private static func find(_ needle: [Unicode.Scalar], in hay: [Unicode.Scalar], from: Int) -> Int? {
        guard !needle.isEmpty, needle.count <= hay.count else { return nil }
        var i = from
        while i + needle.count <= hay.count {
            if hay[i] == needle[0], Array(hay[i..<(i + needle.count)]) == needle { return i }
            i += 1
        }
        return nil
    }

    func token(line: Int, index: Int) -> Token? { lines[safe: line]?[safe: index] }
    func sentence(_ i: Int?) -> StructureSentence? { i.flatMap { sentences[safe: $0] } }

    /// Where a referent's head is on this page: the passage sentence and word, when the head's
    /// sentence (an index within the library text) is one of those shown.
    func head(of r: StructureSentence.Referent) -> (sentence: Int, word: Int)? {
        guard let si = sentences.firstIndex(where: { $0.index == r.headSentence }) else { return nil }
        return (si, r.head)
    }
}
