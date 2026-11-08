import LearningCore
import SwiftUI

/// "Grammar" on the inside cover: one calm entry — a row of chips, one per paradigm table, each
/// showing how many of its cells have been met and glowing by the mean recall of its patterns.
/// A chip opens its table; "All tables" opens every table at once. The tables themselves fill in
/// cell by cell as the learner meets and produces each form: a cell's fill is the recall of its
/// pattern, a dot marks a cell the learner has produced, not only recognized.
struct GrammarSection: View {
    @Environment(Book.self) private var book
    @State private var openCell: CellRef?
    @State private var openTable: TableKey?
    @State private var openAll = false

    struct CellRef: Identifiable, Hashable {
        var table: String
        var cell: String
        var id: String { "\(table)/\(cell)" }
    }
    struct TableKey: Identifiable, Hashable { var id: String }

    var body: some View {
        let p = book.knowledge.domain.paradigms
        let k = book.knowledge
        if !p.isEmpty {
            let summaries = p.tables.map { ($0, CellStrength.summary($0, k)) }
            let met = summaries.reduce(0) { $0 + $1.1.filled }, total = summaries.reduce(0) { $0 + $1.1.total }
            VStack(alignment: .leading, spacing: 10) {
                HStack(alignment: .firstTextBaseline, spacing: 12) {
                    Text("Grammar").font(.system(size: 22, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
                    Text("\(met) of \(total) cells met").font(Theme.label(13)).foregroundStyle(Theme.muted).monospacedDigit()
                    Spacer(minLength: 0)
                    Button { openAll = true } label: {
                        Label("All tables", systemImage: "tablecells").font(Theme.label(14))
                    }
                    .buttonStyle(.plain).foregroundStyle(Theme.accent)
                }
                Text("The system behind the words: declensions, conjugations, what each case does. A table fills in as you meet and produce its forms; tap one to see it.")
                    .font(.system(size: 15)).foregroundStyle(Theme.muted)
                FlowRow(spacing: 8) {
                    ForEach(summaries, id: \.0.id) { table, s in
                        Button { openTable = TableKey(id: table.id) } label: {
                            HStack(spacing: 6) {
                                Text(Theme.passageStressed(table.title, show: book.settings.showStress)).font(.system(size: 14, weight: .medium)).foregroundStyle(s.filled == 0 ? Theme.muted : Theme.text).lineLimit(1)
                                Text("\(s.filled)/\(s.total)").font(Theme.label(11)).foregroundStyle(Theme.muted).monospacedDigit()
                            }
                            .padding(.horizontal, 10).padding(.vertical, 6)
                            .background(Capsule().fill(s.filled == 0 ? Color.clear : Theme.accent.opacity(0.08 + 0.4 * s.mean)))
                            .overlay(Capsule().strokeBorder(s.filled == 0 ? Theme.muted.opacity(0.3) : Theme.accent.opacity(0.25 + 0.5 * s.mean), lineWidth: 1))
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("\(table.title): \(s.filled) of \(s.total) cells met")
                    }
                }
            }
            .sheet(item: $openCell) { ref in
                NavigationStack { CellSheet(tableID: ref.table, cellID: ref.cell) }.environment(book).presentationSizing(.page)
            }
            .sheet(item: $openTable) { key in
                NavigationStack { TableSheet(tableID: key.id) }.environment(book).presentationSizing(.page)
            }
            .sheet(isPresented: $openAll) {
                NavigationStack { GrammarTablesSheet() }.environment(book).presentationSizing(.page)
            }
            .onAppear {
                // QA: -qa-grammar-table decl1 opens that table; -qa-grammar-cell decl1/gen-sg opens that cell; -qa-grammar-all opens every table.
                if let t = QA.string("-qa-grammar-table") { openTable = TableKey(id: t) }
                if let c = QA.string("-qa-grammar-cell") {
                    let parts = c.split(separator: "/").map(String.init)
                    if parts.count == 2 { openCell = CellRef(table: parts[0], cell: parts[1]) }
                }
                if QA.has("-qa-grammar-all") { openAll = true }
            }
        }
    }
}

/// Every table at once, small, cells coloured by recall. Tap a cell for its examples, a title for the table.
struct GrammarTablesSheet: View {
    @Environment(Book.self) private var book
    @Environment(\.dismiss) private var dismiss
    @State private var openCell: GrammarSection.CellRef?
    @State private var openTable: GrammarSection.TableKey?

    var body: some View {
        let p = book.knowledge.domain.paradigms
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                Text("Each table fills in as you meet and produce its forms: a cell glows by how surely you'd recall the pattern, a dot means you have written it yourself. Tap a cell for its examples; tap a title for the whole table.")
                    .font(.system(size: 15)).foregroundStyle(Theme.muted)
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 300), spacing: 12, alignment: .top)], alignment: .leading, spacing: 12) {
                    ForEach(p.tables.filter { $0.kind != "use" }) { table in
                        ParadigmTableCard(table: table, compact: true, onCell: { openCell = GrammarSection.CellRef(table: table.id, cell: $0.id) },
                                          onTitle: { openTable = GrammarSection.TableKey(id: table.id) })
                    }
                }
                if let uses = p.table("uses") {
                    ParadigmTableCard(table: uses, compact: true, onCell: { openCell = GrammarSection.CellRef(table: uses.id, cell: $0.id) },
                                      onTitle: { openTable = GrammarSection.TableKey(id: uses.id) })
                }
            }
            .padding(24)
            .frame(maxWidth: 900, alignment: .leading)
            .frame(maxWidth: .infinity)
        }
        .background(GridPaper(marginX: nil))
        .navigationTitle("Grammar")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        .sheet(item: $openCell) { ref in
            NavigationStack { CellSheet(tableID: ref.table, cellID: ref.cell) }.environment(book).presentationSizing(.page)
        }
        .sheet(item: $openTable) { key in
            NavigationStack { TableSheet(tableID: key.id) }.environment(book).presentationSizing(.page)
        }
    }
}

/// One table as a card: column headers, a row label and the cells, each coloured by recall.
struct ParadigmTableCard: View {
    @Environment(Book.self) private var book
    let table: Paradigms.Table
    var compact: Bool
    var onCell: (Paradigms.Cell) -> Void
    var onTitle: () -> Void = {}

    var body: some View {
        let k = book.knowledge
        let strengths = CellStrength.table(table, k)
        let summary = CellStrength.summary(table, k)
        VStack(alignment: .leading, spacing: 8) {
            Button(action: onTitle) {
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    Text(Theme.passageStressed(table.title, show: book.settings.showStress)).font(.system(size: compact ? 16 : 22, weight: .semibold, design: .serif)).foregroundStyle(Theme.text)
                    if !compact { Text(Theme.passageStressed(table.subtitle, show: book.settings.showStress)).font(.system(size: 14)).foregroundStyle(Theme.muted) }
                    Spacer(minLength: 0)
                    Text("\(summary.filled)/\(summary.total)").font(Theme.label(12)).foregroundStyle(Theme.muted).monospacedDigit()
                }
            }
            .buttonStyle(.plain)
            .accessibilityLabel("\(table.title): \(summary.filled) of \(summary.total) cells met")
            if compact { Text(Theme.passageStressed(table.subtitle, show: book.settings.showStress)).font(.system(size: 12)).foregroundStyle(Theme.muted).lineLimit(1) }
            Grid(alignment: .leading, horizontalSpacing: 4, verticalSpacing: 4) {
                if table.cols.count > 1 {
                    GridRow {
                        Color.clear.frame(width: 1, height: 1)
                        ForEach(table.cols) { col in
                            Text(Theme.passageStressed(col.label, show: book.settings.showStress)).font(Theme.label(compact ? 10 : 12)).foregroundStyle(Theme.muted).lineLimit(1)
                        }
                    }
                }
                ForEach(table.rows) { row in
                    GridRow {
                        Text(Theme.passageStressed(row.label, show: book.settings.showStress)).font(Theme.label(compact ? 10 : 12)).foregroundStyle(Theme.muted).lineLimit(1)
                            .frame(minWidth: compact ? 36 : 60, alignment: .leading)
                        ForEach(table.cols) { col in
                            if let cell = table.cell(row: row.id, col: col.id) {
                                ParadigmCellView(cell: cell, strength: strengths[cell.id] ?? CellStrength(strength: nil, produced: false),
                                                 example: example(cell, k), compact: compact) { onCell(cell) }
                            } else {
                                Color.clear.frame(width: 1, height: 1)
                            }
                        }
                    }
                }
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
        .overlay(RoundedRectangle(cornerRadius: 14).stroke(Theme.accent.opacity(0.15 + 0.5 * summary.mean), lineWidth: 1))
    }

    /// The form shown under a filled cell: a word of the cell the learner knows, else none.
    private func example(_ cell: Paradigms.Cell, _ k: KnowledgeTracker) -> String? {
        let p = k.domain.paradigms
        let words = cell.words.compactMap { p.words[$0] }
        let known = words.first { k.model.hasEvidence(ComponentID(.lexeme, $0.key)) } ?? words.first
        let id = table.id == "past" ? "past-\(cell.id)" : table.id == "imp" ? "imp-\(cell.id)" : cell.id
        return known?.forms[id]
    }
}

struct ParadigmCellView: View {
    let cell: Paradigms.Cell
    let strength: CellStrength
    let example: String?
    var compact: Bool
    let onTap: () -> Void

    var body: some View {
        let s = strength.strength
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 1) {
                Text(Theme.stressed(cell.ending, show: true))
                    .font(cell.components.first?.hasPrefix("gram:") == true ? .system(size: compact ? 12 : 15, weight: .semibold, design: .rounded) : Theme.russian(compact ? 13 : 17, weight: .medium))
                    .foregroundStyle(s == nil ? Theme.muted.opacity(0.6) : Theme.text)
                    .lineLimit(1).minimumScaleFactor(0.7)
                if let example, s != nil, !compact || cell.components.first?.hasPrefix("gram:") == true {
                    Text(Theme.stressed(example, show: true)).font(Theme.russian(compact ? 11 : 14)).foregroundStyle(Theme.text.opacity(0.75)).lineLimit(1).minimumScaleFactor(0.7)
                }
            }
            .padding(.horizontal, compact ? 5 : 8).padding(.vertical, compact ? 3 : 6)
            .frame(maxWidth: .infinity, minHeight: compact ? 26 : 44, alignment: .leading)
            .background(RoundedRectangle(cornerRadius: 6).fill(s.map { Theme.accent.opacity(0.08 + 0.42 * $0) } ?? Color.clear))
            .overlay(RoundedRectangle(cornerRadius: 6).stroke(s.map { Theme.accent.opacity(0.25 + 0.5 * $0) } ?? Theme.muted.opacity(0.22), lineWidth: 1))
            .overlay(alignment: .topTrailing) {
                if strength.produced { Circle().fill(Theme.good).frame(width: 5, height: 5).padding(3) }
            }
        }
        .buttonStyle(.plain)
        .accessibilityLabel("\(RussianText.unstressed(cell.ending))" + (example.map { ", as in \(RussianText.unstressed($0))" } ?? ""))
        .accessibilityValue(s.map { "recall \(Int(($0 * 100).rounded())) percent" + (strength.produced ? ", produced" : "") } ?? "not met yet")
    }
}

/// One table, large, with its note.
struct TableSheet: View {
    @Environment(Book.self) private var book
    @Environment(\.dismiss) private var dismiss
    let tableID: String
    @State private var openCell: Paradigms.Cell?

    var body: some View {
        let p = book.knowledge.domain.paradigms
        ScrollView {
            if let table = p.table(tableID) {
                VStack(alignment: .leading, spacing: 16) {
                    Text(Theme.passageStressed(table.note, show: book.settings.showStress)).font(.system(size: 16)).foregroundStyle(Theme.text).fixedSize(horizontal: false, vertical: true)
                    ParadigmTableCard(table: table, compact: false, onCell: { openCell = $0 })
                    if !table.words.isEmpty {
                        Text("Words in this table").font(Theme.label(13)).foregroundStyle(Theme.muted)
                        FlowRow(spacing: 8) {
                            ForEach(table.words, id: \.self) { key in
                                if let w = p.words[key] {
                                    let known = book.knowledge.model.hasEvidence(ComponentID(.lexeme, w.key))
                                    Text(Theme.passageStressed(w.ru, show: book.settings.showStress)).font(Theme.russian(16)).foregroundStyle(known ? Theme.text : Theme.muted.opacity(0.7))
                                        .padding(.horizontal, 8).padding(.vertical, 3)
                                        .background(Capsule().fill(known ? Theme.accent.opacity(0.1) : .clear))
                                        .overlay(Capsule().strokeBorder(Theme.muted.opacity(known ? 0 : 0.3), lineWidth: 1))
                                }
                            }
                        }
                    }
                }
                .padding(24)
                .frame(maxWidth: 820, alignment: .leading)
                .frame(maxWidth: .infinity)
            }
        }
        .background(GridPaper(marginX: nil))
        .navigationTitle(RussianText.unstressed(p.table(tableID)?.title ?? tableID))
        .navigationBarTitleDisplayMode(.inline)
        .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        .sheet(item: $openCell) { cell in
            NavigationStack { CellSheet(tableID: tableID, cellID: cell.id) }.environment(book).presentationSizing(.page)
        }
    }
}

/// One cell: its pattern, recall by modality, every example the course has (known ones first,
/// each with its sound), the rule for a case use, and the way to its history.
struct CellSheet: View {
    @Environment(Book.self) private var book
    @Environment(\.dismiss) private var dismiss
    let tableID: String
    let cellID: String
    @State private var history: ComponentID?

    var body: some View {
        let k = book.knowledge
        let p = k.domain.paradigms
        ScrollView {
            if let table = p.table(tableID), let cell = table.cell(cellID) {
                let t = k.now()
                let strength = CellStrength.of(cell, k, at: t)
                let wordCell = table.id == "past" ? "past-\(cell.id)" : table.id == "imp" ? "imp-\(cell.id)" : cell.id
                let words = cell.words.compactMap { p.words[$0] }.sorted { a, b in
                    let ka = k.model.hasEvidence(ComponentID(.lexeme, a.key)), kb = k.model.hasEvidence(ComponentID(.lexeme, b.key))
                    return ka != kb ? ka : a.key < b.key
                }
                let use = table.kind == "use" ? p.use(cell.id) : nil
                VStack(alignment: .leading, spacing: 20) {
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(alignment: .firstTextBaseline, spacing: 12) {
                            Text(Theme.stressed(cell.ending, show: true)).font(Theme.russian(40, weight: .semibold)).foregroundStyle(Theme.text)
                            Text(Theme.passageStressed(p.cueLabel(table: table, cell: cell.id), show: book.settings.showStress)).font(.system(size: 18)).foregroundStyle(Theme.text)
                        }
                        Text(Theme.passageStressed(table.title, show: book.settings.showStress)).font(Theme.label(13)).foregroundStyle(Theme.muted)
                        if let s = strength.strength {
                            Text("Recall of the pattern today: \(Int((s * 100).rounded()))%" + (strength.produced ? " · you have produced it" : " · met, not yet produced")).font(Theme.label(13)).foregroundStyle(Theme.muted)
                        } else {
                            Text("Not met yet.").font(Theme.label(13)).foregroundStyle(Theme.muted)
                        }
                        if let gloss = table.rows.first(where: { $0.id == cell.row })?.gloss {
                            Text(Theme.passageStressed(gloss, show: book.settings.showStress)).font(.system(size: 15)).foregroundStyle(Theme.text).fixedSize(horizontal: false, vertical: true)
                        }
                    }
                    if let use {
                        Text(Theme.passageStressed(use.rule, show: book.settings.showStress)).font(.system(size: 16)).foregroundStyle(Theme.text).fixedSize(horizontal: false, vertical: true)
                        VStack(alignment: .leading, spacing: 10) {
                            ForEach(use.examples, id: \.ru) { e in
                                HStack(spacing: 12) {
                                    HearButton(text: e.ru, size: 30)
                                    VStack(alignment: .leading, spacing: 2) {
                                        Text(Theme.passageStressed(e.ru, show: book.settings.showStress)).font(Theme.russian(20, weight: .medium)).foregroundStyle(Theme.text)
                                        Text(e.en).font(.system(size: 14)).foregroundStyle(Theme.muted)
                                    }
                                }
                            }
                        }
                    } else if !words.isEmpty {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Examples").font(Theme.label(13)).foregroundStyle(Theme.muted)
                            ForEach(words.prefix(24), id: \.key) { w in
                                if let form = w.forms[wordCell] {
                                    let known = k.model.hasEvidence(ComponentID(.lexeme, w.key))
                                    HStack(spacing: 12) {
                                        HearButton(text: form.split(separator: "/").first.map { String($0).trimmingCharacters(in: .whitespaces) } ?? form, size: 30)
                                        Text(Theme.passageStressed(form, show: book.settings.showStress)).font(Theme.russian(22, weight: .medium)).foregroundStyle(known ? Theme.text : Theme.muted)
                                        Text("← \(RussianText.unstressed(w.ru)) · \(w.en)").font(.system(size: 14)).foregroundStyle(Theme.muted).lineLimit(1)
                                        Spacer(minLength: 0)
                                        if !known { Text("not yet").font(Theme.label(11)).foregroundStyle(Theme.muted.opacity(0.7)) }
                                    }
                                    .padding(8)
                                    .background(RoundedRectangle(cornerRadius: 10).fill(known ? Theme.accent.opacity(0.06) : Theme.card.opacity(0.4)))
                                    .accessibilityElement(children: .combine)
                                }
                            }
                        }
                    }
                    if let id = cell.component {
                        Button { history = id } label: {
                            Label("History of this cell", systemImage: "clock.arrow.circlepath").font(Theme.label(14))
                        }
                        .buttonStyle(.plain).foregroundStyle(Theme.accent)
                    }
                }
                .padding(24)
                .frame(maxWidth: 760, alignment: .leading)
                .frame(maxWidth: .infinity)
            }
        }
        .background(GridPaper(marginX: nil))
        .navigationTitle(RussianText.unstressed(p.table(tableID).map { p.cueLabel(table: $0, cell: cellID) } ?? cellID))
        .navigationBarTitleDisplayMode(.inline)
        .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        .sheet(item: $history) { id in
            NavigationStack { ComponentHistoryView(id: id) }.environment(book).presentationSizing(.page)
        }
    }
}
