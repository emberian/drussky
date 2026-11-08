import Foundation
import LearningCore

extension Modality {
    /// Writing a form from a grammatical cue (transform, paradigm_fill with the Pencil).
    static let transform: Modality = "cue-write"
    /// Choosing a form or a pattern from a grammatical cue (paradigm_fill with tiles, cue_sort).
    static let classify: Modality = "cue-pick"
}

extension RussianDomain {
    /// The grammar cells as components with their prerequisite edges, for the knowledge graph.
    /// Cells of the unit "Grammar" are drawn by the Grammar section, not in the map's pattern band.
    static func paradigmGraph(_ p: Paradigms) -> (components: [Component], edges: [Edge]) {
        var comps: [Component] = []
        var edges: [Edge] = []
        for c in p.data.components {
            let id = ComponentID(.construction, c.id)
            comps.append(Component(id: id, label: c.name, gloss: c.note, unit: "Grammar"))
            for r in c.requires ?? [] { edges.append(Edge(ComponentID(.construction, r), id, .prerequisite)) }
        }
        return (comps, edges)
    }

    /// The grammar cells a token exercises: every (word, cell) it is a form of, narrowed by the
    /// lemma map's tags (книгу → decl1 acc sg, and acc-u with it).
    func paradigmComponents(token: String) -> [ComponentID] {
        guard !paradigms.isEmpty else { return [] }
        let lemma = lemmas.lemma(token)
        var out: [ComponentID] = []
        for (word, cell) in paradigms.cells(of: token, lemma: lemma, tags: lemmas.tags(token)) {
            for id in paradigms.components(word: word, cell: cell) where !out.contains(id) { out.append(id) }
        }
        return out
    }

    /// Components of the three grammar exercise kinds: the word, the cells asked, their patterns.
    func grammarComponents(for ex: Exercise) -> [ComponentID] {
        var out: [ComponentID] = []
        func add(_ id: ComponentID) { if !out.contains(id) { out.append(id) } }
        switch ex.kind {
        case .transform, .paradigmFill:
            guard let w = ex.word, let word = paradigms.word(w) else { return [] }
            add(wordID(word.key))
            let cells = ex.kind == .transform ? [ex.cue].compactMap { $0 } : (ex.cells ?? [])
            for cell in cells {
                for a in word.answers(cell) {
                    let form = RussianText.key(a)
                    if form != word.key, words.contains(form) { add(ComponentID(.form, form)) }
                }
                for id in paradigms.components(word: word, cell: cell) { add(id) }
            }
        case .cueSort:
            for item in ex.items ?? [] {
                guard let option = ex.options?[safe: item.answer], let use = paradigms.uses.first(where: { $0.title == option || $0.id == option }) else { continue }
                if let id = use.componentID { add(id) }
                for l in use.legacy { if let id = ComponentID(l) { add(id) } }
            }
        default:
            break
        }
        return out
    }

    /// cue_sort: a phrase put in the wrong bucket is a confusion between two patterns.
    func sortConfusions(_ ex: Exercise, response: String) -> [Confusion] {
        // response: "<item index>:<chosen option index>" for the placement that was wrong
        let parts = response.split(separator: ":").compactMap { Int($0) }
        guard parts.count == 2, let item = ex.items?[safe: parts[0]], let options = ex.options,
              let want = options[safe: item.answer], let got = options[safe: parts[1]], want != got,
              let a = paradigms.uses.first(where: { $0.title == want || $0.id == want })?.componentID,
              let b = paradigms.uses.first(where: { $0.title == got || $0.id == got })?.componentID, a != b else { return [] }
        return [Confusion(target: a, produced: b, channel: .choice)]
    }

    /// "genitive singular: кни́ги · книг · …": the paradigm row a missed form sits in, with the
    /// right cell marked, for the hint under a wrong answer. nil when the form is in no table.
    func paradigmRow(forAnswer answer: String, word w: String? = nil, cue: String? = nil) -> ParadigmRowHint? {
        let key = RussianText.key(answer)
        var hits = paradigms.cells(of: key)
        if let w, let word = paradigms.word(w) { hits = hits.filter { $0.word.key == word.key } }
        if let cue { hits = hits.filter { $0.cell == cue } }
        guard let hit = hits.first, let table = paradigms.table(word: hit.word, cell: hit.cell) else { return nil }
        return ParadigmRowHint(table: table, word: hit.word, cell: hit.cell)
    }
}

/// The row of a paradigm table that holds one form: what the hint draws.
struct ParadigmRowHint: Hashable {
    var table: Paradigms.Table
    var word: Paradigms.Word
    /// The word's cell id (past-f for a verb; the table's own id is `Paradigms.cellID(inTable:)`).
    var cell: String

    var tableCellID: String { Paradigms.cellID(inTable: cell) }
    /// The row's cells with the word's form in each, in column order.
    var cells: [(cell: Paradigms.Cell, form: String?)] {
        table.row(of: tableCellID).map { c in
            let wordCell = cell.hasPrefix("past-") ? "past-\(c.id)" : cell.hasPrefix("imp-") ? "imp-\(c.id)" : c.id
            return (c, word.forms[wordCell])
        }
    }
    var rowLabel: String { table.rows.first { $0.id == table.cell(tableCellID)?.row }?.label ?? tableCellID }
}
