import Foundation

/// Roots, affixes, the segmentation of every course lemma and the Greek/Latin cognate bridge
/// (Resources/morphology.json, built by scripts/build_morphology.py and checked by
/// scripts/validate_morphology.py).
struct Morphology: Sendable {
    struct Root: Codable, Sendable, Hashable, Identifiable {
        var id: String
        /// The allomorphs as they appear inside words: пис, пиш.
        var forms: [String]
        var gloss: String
        var note: String?
        /// "пис-/пиш-"
        var label: String
    }
    struct Affix: Codable, Sendable, Hashable, Identifiable {
        var id: String
        var forms: [String]
        var gloss: String
        var pos: String?
        var examples: [String]
    }
    /// One lemma: its morphemes (stress on one of them), the roots they realise, and its family.
    struct Entry: Codable, Sendable, Hashable {
        var ru: String
        var morphemes: [String]
        var roots: [String]
        var prefixes: [String]
        var suffixes: [String]
        var pos: String
        var en: String
        var family: String?
        /// The lemma it is derived from (переписать → писать).
        var base: String?
        var note: String?
        var loan: String?
        var example: Course.Example?

        /// The headword without stress, the key of `lemmas`.
        var key: String { RussianText.key(ru) }
    }
    struct Cognate: Codable, Sendable, Hashable {
        struct Part: Codable, Sendable, Hashable {
            /// "фило-", "-со́фия": a hyphen marks the open side.
            var tile: String
            var gloss: String
        }
        var ru: String
        var en: String
        var source: String
        var rules: [String]
        var parts: [Part]?
        var note: String?
    }
    struct Rule: Codable, Sendable, Hashable, Identifiable {
        var id: String
        var pattern: String
        var note: String
        var examples: [String]
    }
    struct Data: Codable, Sendable {
        var version: Int
        var roots: [Root]
        var prefixes: [Affix]
        var suffixes: [Affix]
        var lemmas: [String: Entry]
        /// An inflected or variant form as written → headword.
        var forms: [String: String]
        var cognates: [Cognate]
        var rules: [Rule]
        var names: [String]?
    }

    let data: Data
    private let rootByID: [String: Root]
    private let prefixByForm: [String: Affix]
    private let suffixByForm: [String: Affix]
    private let membersByFamily: [String: [String]]
    private let ruleByID: [String: Rule]

    init(data: Data) {
        self.data = data
        rootByID = Dictionary(data.roots.map { ($0.id, $0) }, uniquingKeysWith: { a, _ in a })
        prefixByForm = Dictionary(data.prefixes.flatMap { p in (p.forms + [p.id]).map { ($0, p) } }, uniquingKeysWith: { a, _ in a })
        suffixByForm = Dictionary(data.suffixes.flatMap { p in (p.forms + [p.id]).map { ($0, p) } }, uniquingKeysWith: { a, _ in a })
        var members: [String: [String]] = [:]
        for (key, e) in data.lemmas { if let f = e.family { members[f, default: []].append(key) } }
        membersByFamily = members.mapValues { $0.sorted { (data.lemmas[$0]!.ru.count, $0) < (data.lemmas[$1]!.ru.count, $1) } }
        ruleByID = Dictionary(data.rules.map { ($0.id, $0) }, uniquingKeysWith: { a, _ in a })
    }

    static let empty = Morphology(data: Data(version: 1, roots: [], prefixes: [], suffixes: [], lemmas: [:], forms: [:], cognates: [], rules: [], names: nil))

    static func load(bundle: Bundle = .main) -> Morphology {
        guard let url = bundle.url(forResource: "morphology", withExtension: "json"), let bytes = try? Foundation.Data(contentsOf: url),
              let d = try? JSONDecoder().decode(Data.self, from: bytes) else { return .empty }
        return Morphology(data: d)
    }

    // MARK: Lookups

    var roots: [Root] { data.roots }
    var cognates: [Cognate] { data.cognates }
    var lemmas: [String: Entry] { data.lemmas }

    func root(_ id: String) -> Root? { rootByID[id] }
    func rule(_ id: String) -> Rule? { ruleByID[id] }

    /// The entry of a lemma (headword without stress).
    func entry(lemma: String) -> Entry? { data.lemmas[RussianText.key(lemma)] }

    /// The entry a word as written belongs to: a headword, or a form the data maps.
    func entry(word: String) -> Entry? {
        let k = RussianText.key(word)
        if let e = data.lemmas[k] { return e }
        if let head = data.forms[k] { return data.lemmas[head] }
        return nil
    }

    /// Headwords of a family, shortest first.
    func members(of family: String) -> [String] { membersByFamily[family] ?? [] }
    var families: [String] { membersByFamily.keys.sorted() }

    func prefix(form: String) -> Affix? { prefixByForm[RussianText.key(form).replacingOccurrences(of: "-", with: "")] }
    func suffix(form: String) -> Affix? { suffixByForm[RussianText.key(form).replacingOccurrences(of: "-", with: "")] }

    /// The component key of an affix tile as the data names it: `пере-` → "пере-", `-тель` → "-тель".
    /// nil for a tile that is a root or an unlisted piece.
    func affixKey(tile: String) -> String? {
        let bare = RussianText.key(tile).replacingOccurrences(of: "-", with: "")
        if tile.hasSuffix("-"), let p = prefixByForm[bare] { return p.id + "-" }
        if tile.hasPrefix("-"), let s = suffixByForm[bare] { return "-" + s.id }
        return nil
    }

    /// Where the root morpheme sits in an entry (the first morpheme that is a form of one of its roots).
    func rootIndex(of e: Entry) -> Int? {
        let forms = Set(e.roots.flatMap { rootByID[$0]?.forms ?? [] })
        return e.morphemes.firstIndex { forms.contains(RussianText.key($0).replacingOccurrences(of: "-", with: "")) }
    }

    /// The tiles of a build_word exercise: each prefix "X-", the root bare, everything after
    /// the root as one "-rest" tile (пере-, пис, -а́ть). nil when the word has fewer than two.
    func tiles(for e: Entry) -> [String]? {
        guard let r = rootIndex(of: e) else { return nil }
        var out: [String] = []
        for m in e.morphemes[..<r] { out.append(m.hasSuffix("-") ? m : m + "-") }
        out.append(e.morphemes[r])
        let rest = e.morphemes[(r + 1)...].joined()
        if !rest.isEmpty { out.append("-" + rest) }
        return out.count >= 2 ? out : nil
    }

    /// "пере- (across; again, re-) + пис- (write) + -а́ть": the word taken apart with glosses.
    func analysis(of e: Entry) -> [(piece: String, gloss: String?)] {
        guard let tiles = tiles(for: e), let r = rootIndex(of: e) else { return [(e.ru, nil)] }
        let rootGloss = e.roots.first.flatMap { rootByID[$0]?.gloss }
        return tiles.enumerated().map { i, t in
            if i == r { return (t + "-", rootGloss) }
            if t.hasSuffix("-") { return (t, prefix(form: t)?.gloss) }
            return (t, e.suffixes.last.flatMap { suffixByForm[$0]?.gloss })
        }
    }

    /// The tiles of a cognate's Greek/Latin parts, when it has any.
    func tiles(for c: Cognate) -> [String]? {
        guard let parts = c.parts, parts.count >= 2 else { return nil }
        return parts.map(\.tile)
    }
}

/// The build_word checker: tiles in the order the learner placed them spell the word (or an
/// accepted alternative) once hyphens and stress are dropped.
enum BuildWordCheck {
    static func assemble(_ tiles: [String]) -> String {
        RussianText.key(tiles.joined()).replacingOccurrences(of: "-", with: "").replacingOccurrences(of: " ", with: "")
    }

    static func matches(_ tiles: [String], answer: String, alternatives: [String]? = nil) -> Bool {
        let built = assemble(tiles)
        guard !built.isEmpty else { return false }
        return ([answer] + (alternatives ?? [])).contains { assemble([$0]) == built }
    }

    /// Whether `tiles` can spell `answer` at all (in some order): a sanity check for authors.
    static func canSpell(_ tiles: [String], answer: String) -> Bool {
        let target = assemble([answer])
        let pieces = tiles.map { assemble([$0]) }
        func go(_ remaining: Substring, _ used: Set<Int>) -> Bool {
            if remaining.isEmpty { return used.count == pieces.count }
            for (i, p) in pieces.enumerated() where !used.contains(i) && remaining.hasPrefix(p) {
                if go(remaining.dropFirst(p.count), used.union([i])) { return true }
            }
            return false
        }
        return go(Substring(target), [])
    }
}
