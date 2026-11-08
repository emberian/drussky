import Foundation
import LearningCore

/// How well a cell of a paradigm table is known: the strength of its own component (best
/// modality), and whether the learner has ever *produced* it (written or said it, not only
/// recognized it).
struct CellStrength: Hashable {
    var strength: Double?
    var produced: Bool
    var met: Bool { strength != nil }

    static let producing: Set<Modality> = [.hearWrite, .meaningWrite, .produce, .transform, .say]

    @MainActor static func of(_ cell: Paradigms.Cell, _ k: KnowledgeTracker, at t: Date) -> CellStrength {
        guard let id = cell.component else { return CellStrength(strength: nil, produced: false) }
        let s = k.model.strength(id, at: t)
        let produced = k.model.traces(of: id).keys.contains { producing.contains($0) }
        return CellStrength(strength: s, produced: produced)
    }

    /// Every cell of a table, keyed by cell id.
    @MainActor static func table(_ table: Paradigms.Table, _ k: KnowledgeTracker) -> [String: CellStrength] {
        let t = k.now()
        return Dictionary(uniqueKeysWithValues: table.cells.map { ($0.id, of($0, k, at: t)) })
    }

    /// The share of a table's cells with evidence, and the mean strength of those.
    @MainActor static func summary(_ table: Paradigms.Table, _ k: KnowledgeTracker) -> (filled: Int, total: Int, mean: Double) {
        let all = Self.table(table, k).values
        let met = all.compactMap(\.strength)
        return (met.count, all.count, met.isEmpty ? 0 : met.reduce(0, +) / Double(met.count))
    }
}

extension Book {
    /// Paradigm words the learner has met as words (a review target or any evidence), in letters
    /// already met, so a grammar page never asks for a word before the word.
    func knownParadigmWords(from p: Paradigms) -> [Paradigms.Word] {
        let met = Set(course.letters.filter { letterKnown($0.upper) }.map(\.upper))
        let known = Set(targets.keys.filter { $0.hasPrefix("word:") }.flatMap { String($0.dropFirst(5)).split(separator: " ").map(String.init) })
        return p.words.values.filter { w in
            (known.contains(w.key) || knowledge.model.hasEvidence(ComponentID(.lexeme, w.key)))
                && w.forms.values.allSatisfy { RussianText.letters(in: $0).isSubset(of: met) }
        }.sorted { $0.key < $1.key }
    }

    /// Cells worth asking now: every (table, cell) a known word has a form for, weakest first;
    /// a cell never met counts as weak-ish (0.3) so the tables fill in, below cells that have
    /// lapsed. Each entry carries the known words that have the cell.
    func weakCells(from p: Paradigms, words: [Paradigms.Word]) -> [(table: Paradigms.Table, cell: Paradigms.Cell, score: Double, words: [Paradigms.Word])] {
        let t = knowledge.now()
        var out: [(Paradigms.Table, Paradigms.Cell, Double, [Paradigms.Word])] = []
        for table in p.tables where ["noun", "adj", "verb", "pronoun"].contains(table.kind) && table.id != "irregular" {
            for cell in table.cells {
                let wordCell = table.id == "past" ? "past-\(cell.id)" : table.id == "imp" ? "imp-\(cell.id)" : cell.id
                let have = words.filter { $0.table == table.id || ($0.pos == "verb" && ["past", "imp"].contains(table.id)) }.filter { $0.forms[wordCell] != nil }
                guard !have.isEmpty else { continue }
                let s = CellStrength.of(cell, knowledge, at: t)
                out.append((table, cell, s.strength ?? 0.3, have))
            }
        }
        return out.sorted { $0.2 != $1.2 ? $0.2 < $1.2 : ($0.0.id, $0.1.id) < ($1.0.id, $1.1.id) }.map { ($0.0, $0.1, $0.2, $0.3) }
    }

    /// A grammar page built from the tables: a card naming the systems in play, a table to fill
    /// with the Pencil, forms from cues (in sentences where the course has one), a table to fill
    /// from tiles, phrases to sort by the job their case does, and one line to say aloud.
    /// Cases are interleaved: no two asks in a row drill the same row of a table.
    func grammarExercises(from p: Paradigms, seed: UInt64? = nil) -> (exercises: [Exercise], tables: [String], spoken: [String]) {
        var rng = SeededGenerator(seed: seed ?? UInt64(now().timeIntervalSince1970))
        let words = knownParadigmWords(from: p)
        guard words.count >= 3 else { return ([], [], []) }
        let met = Set(course.letters.filter { letterKnown($0.upper) }.map(\.upper))
        func readable(_ s: String) -> Bool { RussianText.letters(in: s).isSubset(of: met) }
        let ranked = weakCells(from: p, words: words)
        guard !ranked.isEmpty else { return ([], [], []) }
        var out: [Exercise] = []
        var tablesUsed: [String] = []
        var spoken: [String] = []
        var usedRows = Set<String>()  // "table/row" asked so far, to interleave cases
        var usedWords = Set<String>()

        func wordCell(_ table: Paradigms.Table, _ cell: Paradigms.Cell) -> String {
            table.id == "past" ? "past-\(cell.id)" : table.id == "imp" ? "imp-\(cell.id)" : cell.id
        }
        /// The next weak cell whose row (case) has not been asked yet, from tables of the kinds given.
        func next(kinds: Set<String>, excludingWord: Set<String> = []) -> (table: Paradigms.Table, cell: Paradigms.Cell, words: [Paradigms.Word])? {
            let pool = ranked.filter { kinds.contains($0.table.kind) && $0.words.contains { !excludingWord.contains($0.key) } }
            let fresh = pool.first { !usedRows.contains("\($0.table.id)/\($0.cell.row)") } ?? pool.first
            guard let fresh else { return nil }
            usedRows.insert("\(fresh.table.id)/\(fresh.cell.row)")
            if !tablesUsed.contains(fresh.table.id) { tablesUsed.append(fresh.table.id) }
            return (fresh.table, fresh.cell, fresh.words.filter { !excludingWord.contains($0.key) })
        }

        // 1. A table to fill with the Pencil: the weakest noun/adjective table's weakest word.
        if let pick = next(kinds: ["noun", "adj"]) {
            let word = pick.words.shuffled(using: &rng).first!
            usedWords.insert(word.key)
            let cellIDs = p.cellOrder(pick.table).filter { word.forms[$0] != nil }
            let weakOrder = ranked.filter { $0.table.id == pick.table.id }.map(\.cell.id)
            var asked = [pick.cell.id]
            for c in weakOrder where asked.count < 4 && !asked.contains(c) && word.forms[c] != nil && c != "nom-sg" && c != "nom-m" {
                // one ask per row where possible
                let row = pick.table.cell(c)?.row
                if !asked.contains(where: { pick.table.cell($0)?.row == row }) || asked.count >= 3 { asked.append(c) }
            }
            var e = Exercise(kind: .paradigmFill)
            e.word = word.ru
            e.paradigm = pick.table.id
            e.forms = Dictionary(uniqueKeysWithValues: cellIDs.map { ($0, word.forms[$0]!) })
            e.cells = asked.sorted { cellIDs.firstIndex(of: $0)! < cellIDs.firstIndex(of: $1)! }
            out.append(e)
            spoken += cellIDs.compactMap { word.answers($0).first }
            for c in asked { usedRows.insert("\(pick.table.id)/\(pick.table.cell(c)?.row ?? c)") }
        }

        // 2. Forms from cues, in a sentence where the course has one with this word and cell.
        var transforms: [Exercise] = []
        var tries = 0
        while transforms.count < 3 && tries < 12 {
            tries += 1
            guard let pick = next(kinds: ["noun", "adj", "verb", "pronoun"], excludingWord: usedWords) else { break }
            let cell = wordCell(pick.table, pick.cell)
            let candidates = pick.words.shuffled(using: &rng)
            guard let word = candidates.first(where: { !$0.answers(cell).isEmpty && !$0.answers(cell)[0].contains("/") }) ?? candidates.first else { continue }
            let answers = word.answers(cell)
            guard let answer = answers.first else { continue }
            usedWords.insert(word.key)
            var e = Exercise(kind: .transform)
            e.word = word.ru
            e.paradigm = pick.table.id
            e.cue = cell
            e.answerText = answer
            if let gap = p.uses.flatMap(\.gaps).filter({ $0.word == word.key && $0.cell == cell && readable($0.before + $0.after) }).shuffled(using: &rng).first {
                e.before = gap.before
                e.after = gap.after
                e.hint = gap.hint
            }
            transforms.append(e)
            spoken.append(answer)
        }

        // 3. A table to fill from tiles: a conjugation or the pronouns, with two tiles that don't belong.
        if let pick = next(kinds: ["verb", "pronoun"], excludingWord: usedWords) ?? next(kinds: ["verb", "pronoun"]) {
            let word = pick.words.shuffled(using: &rng).first!
            let cellIDs = p.cellOrder(pick.table).map { wordCell(pick.table, pick.table.cell($0)!) }.filter { word.forms[$0] != nil && !(word.forms[$0] ?? "").contains("/") }
            if cellIDs.count >= 3 {
                let shown = Array(cellIDs.prefix(6))
                let asked = Array(shown.dropFirst().shuffled(using: &rng).prefix(min(4, shown.count - 1))).sorted { shown.firstIndex(of: $0)! < shown.firstIndex(of: $1)! }
                let answers = asked.compactMap { word.forms[$0] }
                let others = words.filter { $0.key != word.key && $0.table == word.table }.flatMap { o in asked.compactMap { o.forms[$0] } }
                    .filter { f in !answers.contains(where: { RussianText.comparable($0) == RussianText.comparable(f) }) }
                var e = Exercise(kind: .paradigmFill)
                e.word = word.ru
                e.paradigm = pick.table.id
                e.forms = Dictionary(uniqueKeysWithValues: shown.map { ($0, word.forms[$0]!) })
                e.cells = asked
                e.options = (answers + Array(others.shuffled(using: &rng).prefix(2))).shuffled(using: &rng)
                out.append(e)
                spoken += shown.compactMap { word.forms[$0] }
            }
        }

        // 4. Phrases sorted by the job their case does: two or three uses with known words.
        let known = Set(words.map(\.key))
        func usable(_ e: Paradigms.Example) -> Bool { (e.word.map(known.contains) ?? false) && readable(e.ru) }
        let sortable = p.uses.filter { u in u.examples.filter(usable).count >= 2 }.shuffled(using: &rng)
        let useRanked = sortable.sorted { a, b in
            let sa = a.componentID.flatMap { knowledge.model.strength($0, at: knowledge.now()) } ?? 0.3
            let sb = b.componentID.flatMap { knowledge.model.strength($0, at: knowledge.now()) } ?? 0.3
            return sa < sb
        }
        let chosen = Array(useRanked.prefix(3))
        if chosen.count >= 2 {
            var e = Exercise(kind: .cueSort)
            e.paradigm = "uses"
            e.options = chosen.map(\.title)
            var items: [SortItem] = []
            for (i, u) in chosen.enumerated() {
                for ex in u.examples.filter(usable).shuffled(using: &rng).prefix(2) {
                    items.append(SortItem(text: ex.ru, answer: i))
                }
            }
            e.items = items.shuffled(using: &rng)
            out.append(e)
            spoken += items.map(\.text)
            if !tablesUsed.contains("uses") { tablesUsed.append("uses") }
        }

        // The transforms go between the tables, so no two table fills sit together.
        if !transforms.isEmpty {
            let insertAt = min(1, out.count)
            out.insert(contentsOf: transforms, at: insertAt)
        }

        // 5. Say one of the sentences the sort used (or a use example with a known word).
        if let line = (chosen.flatMap(\.examples) + p.uses.flatMap(\.examples)).filter(usable).shuffled(using: &rng).first {
            var e = Exercise(kind: .readAloud)
            e.text = line.ru
            out.append(e)
            spoken.append(line.ru)
        }

        // 0. The card: which tables are in play and the one rule each turns on.
        if !tablesUsed.isEmpty {
            var card = Exercise(kind: .explain)
            let named = tablesUsed.prefix(2).compactMap { p.table($0) }
            // The one rule each table turns on, in letters the learner can read; a title alone otherwise.
            card.text = named.map { t in
                let sentence = t.note.split(separator: ".").map { $0.trimmingCharacters(in: .whitespaces) }.first { !$0.isEmpty && readable($0) }
                return readable(t.title) ? (sentence.map { "\(t.title): \($0)." } ?? "\(t.title).") : (sentence.map { "\($0)." } ?? "")
            }.filter { !$0.isEmpty }.joined(separator: " ")
            if card.text?.isEmpty ?? true { card.text = "The tables behind the words you know." }
            card.examples = Array(words.shuffled(using: &rng).prefix(3)).map { Course.Example(ru: $0.ru, en: $0.en) }
            out.insert(card, at: 0)
        }
        return (out, tablesUsed, spoken)
    }
}

/// The ＋ menu's grammar page, and its QA flag.
@MainActor
enum GrammarPages {
    /// Inserts a grammar page after the current one; returns a notice when none can be made.
    static func addGrammar(_ book: Book) -> String? {
        let p = book.knowledge.domain.paradigms
        let (exercises, tables, _) = book.grammarExercises(from: p)
        guard exercises.count >= 4 else {
            return "A grammar page needs a few words you can already write. The tables open up once nouns and verbs have started changing their endings."
        }
        let titles = tables.compactMap { p.table($0)?.title }.joined(separator: " · ")
        book.insertWrittenPage(title: "Grammar page", intro: "The tables behind the words: \(titles). Each cell is a pattern; fill the weak ones and the system fills in on the inside cover.",
                               unitTitle: "Grammar", exercises: exercises, newWords: [])
        return nil
    }
}
