import Foundation
import LearningCore

/// Russian inflection as a visible system (Resources/paradigms.json, built by
/// scripts/build_paradigms.py and checked by scripts/validate_paradigms.py): the tables (three
/// noun declensions, adjectives, pronouns, two conjugations, irregular verbs, past, imperative,
/// the case uses), every course word's full set of forms, and an index from any written form to
/// the cells it fills.
struct Paradigms: Sendable {
    struct Component: Codable, Sendable, Hashable {
        var id: String
        var name: String
        var note: String
        var requires: [String]?
    }
    struct Row: Codable, Sendable, Hashable, Identifiable {
        var id: String
        var label: String
        var gloss: String?
    }
    struct Col: Codable, Sendable, Hashable, Identifiable {
        var id: String
        var label: String
    }
    struct Cell: Codable, Sendable, Hashable, Identifiable {
        var id: String
        var row: String
        var col: String
        /// "-ы / -и", or the form itself for a word-cell (pronouns, irregular verbs).
        var ending: String
        /// Full component ids ("gram:decl1-gen-sg", "form:меня"); the first is the cell's own.
        var components: [String]
        /// Keys into `words` of the words with this cell.
        var words: [String]
        var componentIDs: [ComponentID] { components.compactMap { ComponentID($0) } }
        var component: ComponentID? { componentIDs.first }
    }
    struct Table: Codable, Sendable, Hashable, Identifiable {
        var id: String
        var kind: String
        var title: String
        var subtitle: String
        var note: String
        var rows: [Row]
        var cols: [Col]
        var cells: [Cell]
        var words: [String]

        func cell(_ id: String) -> Cell? { cells.first { $0.id == id } }
        func cell(row: String, col: String) -> Cell? { cells.first { $0.row == row && $0.col == col } }
        /// The cells of a row, in column order.
        func row(of cellID: String) -> [Cell] {
            guard let c = cell(cellID) else { return [] }
            return cols.compactMap { cell(row: c.row, col: $0.id) }
        }
    }
    struct Word: Codable, Sendable, Hashable {
        var ru: String
        var en: String
        var table: String
        var pos: String
        var gender: String?
        var animate: Bool?
        var number: String?
        var aspect: String?
        var note: String?
        /// cell id → stressed form ("но́вый / но́вого" for an adjective's animate accusative).
        var forms: [String: String]

        var key: String { RussianText.key(ru) }
        /// The accepted spellings of a cell (both halves of a "/" cell).
        func answers(_ cell: String) -> [String] { (forms[cell] ?? "").split(separator: "/").map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty } }
    }
    struct Example: Codable, Sendable, Hashable {
        var ru: String
        var en: String
        var word: String?
        var cell: String?
    }
    struct Gap: Codable, Sendable, Hashable {
        var before: String
        var answer: String
        var after: String
        var hint: String
        var word: String
        var cell: String
    }
    struct Use: Codable, Sendable, Hashable, Identifiable {
        var id: String
        var `case`: String
        var title: String
        var component: String
        var legacy: [String]
        var rule: String
        var examples: [Example]
        var gaps: [Gap]
        var componentID: ComponentID? { ComponentID(component) }
    }
    struct Data: Codable, Sendable {
        var version: Int
        var components: [Component]
        var tables: [Table]
        var words: [String: Word]
        var uses: [Use]
        var exceptions: [String: String]
        /// written form (lowercase, unstressed) → [[word key, cell id], …]
        var forms: [String: [[String]]]
    }

    let data: Data
    private let tableByID: [String: Table]
    private let useByID: [String: Use]

    init(data: Data) {
        self.data = data
        tableByID = Dictionary(data.tables.map { ($0.id, $0) }, uniquingKeysWith: { a, _ in a })
        useByID = Dictionary(data.uses.map { ($0.id, $0) }, uniquingKeysWith: { a, _ in a })
    }

    static let empty = Paradigms(data: Data(version: 1, components: [], tables: [], words: [:], uses: [], exceptions: [:], forms: [:]))

    static func load(bundle: Bundle = .main) -> Paradigms {
        guard let url = bundle.url(forResource: "paradigms", withExtension: "json"), let bytes = try? Foundation.Data(contentsOf: url),
              let d = try? JSONDecoder().decode(Data.self, from: bytes) else { return .empty }
        return Paradigms(data: d)
    }

    var isEmpty: Bool { data.tables.isEmpty }
    var tables: [Table] { data.tables }
    var words: [String: Word] { data.words }
    var uses: [Use] { data.uses }

    func table(_ id: String) -> Table? { tableByID[id] }
    func use(_ id: String) -> Use? { useByID[id] }
    /// A word by its headword (any stress, any case).
    func word(_ ru: String) -> Word? { data.words[RussianText.key(ru)] }
    /// The table a word's forms fill.
    func table(of word: Word) -> Table? { tableByID[word.table] }

    /// Every (word, cell) a written form fills: кни́ги → (книга, gen-sg), (книга, nom-pl), (книга, acc-pl).
    func cells(of form: String) -> [(word: Word, cell: String)] {
        (data.forms[RussianText.key(form)] ?? []).compactMap { pair in
            guard pair.count == 2, let w = data.words[pair[0]] else { return nil }
            return (w, pair[1])
        }
    }

    /// The cells a token fills, narrowed by the lemma map's construction tags when they decide
    /// between homographs (книги as acc-u is the accusative plural, not the genitive singular),
    /// and by the lemma when the token belongs to one.
    func cells(of token: String, lemma: String?, tags: [String]) -> [(word: Word, cell: String)] {
        var hits = cells(of: token)
        if let lemma { hits = hits.filter { $0.word.key == RussianText.key(lemma) || $0.word.table.hasPrefix("poss") && lemma == $0.word.key } }
        let narrow: [String: (String) -> Bool] = [
            "acc-u": { $0 == "acc-sg" }, "prep-e": { $0 == "prep-sg" }, "plural": { $0.hasSuffix("-pl") }, "genitive": { $0.hasPrefix("gen-") },
            "time-instr": { $0 == "instr-sg" }, "past": { $0.hasPrefix("past-") }, "inf": { $0 == "inf" },
            "pres-1sg": { $0 == "1sg" }, "pres-2sg": { $0 == "2sg" }, "pres-3sg": { $0 == "3sg" },
            "pres-1pl": { $0 == "1pl" }, "pres-2pl": { $0 == "2pl" }, "pres-3pl": { $0 == "3pl" },
        ]
        for tag in tags {
            guard let test = narrow[tag] else { continue }
            let kept = hits.filter { test($0.cell) }
            if !kept.isEmpty { hits = kept }
        }
        return hits
    }

    /// The component ids a (word, cell) exercises: the table cell's own and the legacy patterns it
    /// also exemplifies (nothing for the infinitive and other cells no table draws).
    func components(word: Word, cell: String) -> [ComponentID] {
        guard let t = table(of: word) else { return [] }
        if let c = t.cell(cell) { return c.componentIDs }
        // Verb words sit in a conjugation table, but their past and imperative cells live in the
        // past and imperative tables.
        if cell.hasPrefix("past-"), let c = tableByID["past"]?.cell(String(cell.dropFirst(5))) { return c.componentIDs }
        if cell.hasPrefix("imp-"), let c = tableByID["imp"]?.cell(String(cell.dropFirst(4))) { return c.componentIDs }
        return []
    }

    /// The table that draws a word's cell (the conjugation, past or imperative table for a verb).
    func table(word: Word, cell: String) -> Table? {
        if word.pos == "verb" {
            if cell.hasPrefix("past-") { return tableByID["past"] }
            if cell.hasPrefix("imp-") { return tableByID["imp"] }
        }
        return table(of: word)
    }

    /// The id a cell of a verb has in the table that draws it (past-f → f).
    static func cellID(inTable cell: String) -> String {
        if cell.hasPrefix("past-") { return String(cell.dropFirst(5)) }
        if cell.hasPrefix("imp-") { return String(cell.dropFirst(4)) }
        return cell
    }

    /// "genitive singular", "the ты form", "past · она́": what a cue asks for, in words.
    func cueLabel(table: Table, cell: String) -> String {
        let id = Self.cellID(inTable: cell)
        guard let c = table.cell(id) else { return cell }
        let row = table.rows.first { $0.id == c.row }?.label ?? c.row
        let col = table.cols.first { $0.id == c.col }?.label ?? c.col
        switch table.id {
        case "past": return "past · \(row)"
        case "imp": return "imperative · \(row)"
        case "conj1", "conj2", "irregular": return "the \(row) form"
        default: return table.cols.count <= 1 ? row : "\(row) \(col)"
        }
    }

    /// Everything the course can ask about a word: the cells with a form, in table order.
    func cellOrder(_ table: Table) -> [String] {
        table.rows.flatMap { r in table.cols.compactMap { c in table.cell(row: r.id, col: c.id)?.id } }
    }
}

/// The transform / paradigm_fill checker: a written form matches when the handwriting reads as
/// the answer with or without stress marks, with ё as е, through the same lookalike tolerance
/// as every other written answer.
enum TransformCheck {
    static func matches(_ written: String, answer: String) -> Bool {
        let alternatives = answer.split(separator: "/").map { String($0).trimmingCharacters(in: .whitespaces) }
        return alternatives.contains { !$0.isEmpty && RussianText.answerMatches(written, $0) }
    }
}

/// cue_sort items: a phrase and the index of the bucket (option) it belongs in.
struct SortItem: Codable, Sendable, Hashable {
    var text: String
    var answer: Int
}
