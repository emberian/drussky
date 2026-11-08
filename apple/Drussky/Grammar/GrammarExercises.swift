import LearningCore
import PencilKit
import SwiftUI

// MARK: - transform (the prompt; the strip and checking are WritingExercise's)

/// What a transform asks: a word and a grammatical cue («кни́га» → genitive singular), or a
/// sentence with a gap that forces the form.
struct TransformPrompt: View {
    @Environment(Book.self) private var book
    let exercise: Exercise

    var body: some View {
        let p = book.knowledge.domain.paradigms
        let word = exercise.word.flatMap { p.word($0) }
        let table = word.flatMap { w in exercise.cue.flatMap { p.table(word: w, cell: $0) } }
        let cue = table.flatMap { t in exercise.cue.map { p.cueLabel(table: t, cell: $0) } } ?? exercise.cue ?? ""
        VStack(alignment: .leading, spacing: 6) {
            Instruction(text: exercise.before != nil || exercise.after != nil ? "Write the missing form." : "Write the form.")
            if exercise.before != nil || exercise.after != nil {
                (Text(Theme.stressed(exercise.before ?? "", show: book.settings.showStress)) + Text(" ____ ").foregroundStyle(Theme.accent)
                    + Text(Theme.stressed(exercise.after ?? "", show: book.settings.showStress)))
                    .russianFont(30, weight: .medium)
                    .foregroundStyle(Theme.text)
                if let hint = exercise.hint { Text(hint).scaledFont(16).italic().foregroundStyle(Theme.muted) }
            }
            HStack(spacing: 10) {
                if let word {
                    Text(Theme.stressed(word.ru, show: book.settings.showStress)).russianFont(exercise.before == nil && exercise.after == nil ? 34 : 22, weight: .medium).foregroundStyle(Theme.text)
                    HearButton(text: word.ru, size: 32)
                    Text(word.en).scaledFont(15).foregroundStyle(Theme.muted)
                }
                Text("→").scaledFont(18).foregroundStyle(Theme.muted)
                CueTag(text: cue)
            }
            .accessibilityElement(children: .combine)
        }
    }
}

/// "genitive singular" in a small rounded tag.
struct CueTag: View {
    let text: String
    var body: some View {
        Text(text).font(Theme.label(14)).foregroundStyle(Theme.accent)
            .padding(.horizontal, 10).padding(.vertical, 5)
            .background(Capsule().fill(Theme.accent.opacity(0.12)))
    }
}

/// The paradigm row a missed form belongs to, the right cell marked: the hint that teaches the
/// system, not just the answer. Shows nothing when the exercise's answer is in no table.
struct GrammarRowHint: View {
    @Environment(Book.self) private var book
    let exercise: Exercise

    var body: some View {
        if let answer = exercise.target,
           let hint = book.knowledge.domain.paradigmRow(forAnswer: answer, word: exercise.word, cue: exercise.cue) {
            ParadigmRowHintView(hint: hint)
        }
    }
}

struct ParadigmRowHintView: View {
    @Environment(Book.self) private var book
    let hint: ParadigmRowHint

    var body: some View {
        // One row: the table and case as a leading chip, then the row's cells — compact, so it
        // stays in view above the next exercise when the page moves on.
        FlowRow(spacing: 8) {
            VStack(alignment: .leading, spacing: 2) {
                Text(hint.rowLabel).font(Theme.label(13)).foregroundStyle(Theme.accent)
                Text(hint.table.title).font(Theme.label(11)).foregroundStyle(Theme.muted).lineLimit(1)
            }
            .padding(.horizontal, 10).padding(.vertical, 6)
            .frame(minHeight: 58, alignment: .leading)
            ForEach(hint.cells, id: \.cell.id) { c, form in
                    let hit = c.id == hint.tableCellID
                    VStack(alignment: .leading, spacing: 2) {
                        Text(hint.table.cols.first { $0.id == c.col }?.label ?? c.col).font(Theme.label(11)).foregroundStyle(Theme.muted)
                        Text(Theme.stressed(form ?? c.ending, show: book.settings.showStress)).russianFont(20, weight: hit ? .semibold : .regular)
                            .foregroundStyle(hit ? Theme.redPen : Theme.text)
                        if form != nil, c.ending != form, !c.ending.hasPrefix("=") {
                            Text(c.ending).font(Theme.label(11)).foregroundStyle(Theme.muted)
                        }
                    }
                    .padding(.horizontal, 10).padding(.vertical, 6)
                    .background(RoundedRectangle(cornerRadius: 10).fill(hit ? Theme.redPen.opacity(0.08) : Theme.card))
                    .overlay(RoundedRectangle(cornerRadius: 10).stroke(hit ? Theme.redPen : Theme.rule.opacity(0.4), lineWidth: hit ? 1.5 : 1))
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(hint.table.title), \(hint.rowLabel): " + hint.cells.map { RussianText.unstressed($0.form ?? $0.cell.ending) }.joined(separator: ", "))
    }
}

// MARK: - paradigm_fill

/// A paradigm table with some cells given and the rest to fill: with the Pencil on a small ruled
/// cell each, or by tapping tiles into them. Every cell shows its ending, so the pattern is in
/// view while the forms are produced. One check grades the whole table; a second miss shows it.
struct ParadigmFillExercise: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    let slot: ExerciseSlot
    let isActive: Bool
    @State private var pads: [String: PadController] = [:]
    @State private var placed: [String: Int] = [:]   // cell → tile index (tile mode)
    @State private var selectedCell: String?
    @State private var right: Set<String> = []
    @State private var readings: [String: String] = [:]
    @State private var checking = false
    @State private var cellWidth: CGFloat = 200

    private var ex: Exercise { slot.exercise }
    private var result: ExerciseResult { slot.result }
    private var p: Paradigms { book.knowledge.domain.paradigms }
    private var word: Paradigms.Word? { ex.word.flatMap { p.word($0) } }
    private var table: Paradigms.Table? { ex.paradigm.flatMap { p.table($0) } ?? word.flatMap { p.table(of: $0) } }
    private var asked: [String] { ex.cells ?? [] }
    private var tileMode: Bool { ex.options != nil }
    private var tiles: [String] { ex.options ?? [] }
    private var forms: [String: String] { ex.forms ?? [:] }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Instruction(text: tileMode ? "Fill the table: tap a cell, then the form that belongs in it." : "Fill the table. Write each missing form in its cell.")
            if let word, let table {
                HStack(spacing: 12) {
                    Text(Theme.stressed(word.ru, show: book.settings.showStress)).russianFont(30, weight: .medium).foregroundStyle(Theme.text)
                    HearButton(text: word.ru, size: 34)
                    Text(word.en).scaledFont(16).foregroundStyle(Theme.muted)
                    Spacer(minLength: 0)
                    Text(table.title).font(Theme.label(13)).foregroundStyle(Theme.muted)
                }
                grid(table, word)
            }
            if tileMode, !result.status.isComplete { tray }
            controls
            feedback
        }
        // The column's width, measured on the root (whose width the cells cannot change), sizes the cells.
        .frame(maxWidth: .infinity, alignment: .leading)
        .onGeometryChange(for: CGFloat.self) { $0.size.width } action: { w in
            let cols = CGFloat(max(1, table.map { t in t.cols.filter { c in t.rows.contains { forms[cellID(row: $0.id, col: c.id, t)] != nil } }.count } ?? 1))
            let width = max(120, (w - 80) / cols - 8)
            if abs(width - cellWidth) > 1 { cellWidth = width }
        }
        .onAppear(perform: restore)
        .animation(.easeOut(duration: 0.2), value: placed)
    }

    /// Rows and columns of the table; only cells with a form are drawn.
    private func grid(_ table: Paradigms.Table, _ word: Paradigms.Word) -> some View {
        let cols = table.cols.filter { col in table.rows.contains { forms[cellID(row: $0.id, col: col.id, table)] != nil } }
        return Grid(alignment: .leading, horizontalSpacing: 8, verticalSpacing: 6) {
            if cols.count > 1 {
                GridRow {
                    Color.clear.frame(width: 1, height: 1)
                    ForEach(cols) { col in Text(col.label).font(Theme.label(12)).foregroundStyle(Theme.muted) }
                }
            }
            ForEach(table.rows.filter { r in cols.contains { forms[cellID(row: r.id, col: $0.id, table)] != nil } }) { row in
                GridRow(alignment: .center) {
                    Text(row.label).font(Theme.label(13)).foregroundStyle(Theme.muted).frame(minWidth: 56, alignment: .leading)
                    ForEach(cols) { col in
                        let id = cellID(row: row.id, col: col.id, table)
                        if let form = forms[id], let c = table.cell(row: row.id, col: col.id) {
                            cell(id: id, form: form, ending: c.ending)
                        } else {
                            Color.clear.frame(width: 1, height: 1)
                        }
                    }
                }
            }
        }
    }

    /// A verb's past and imperative cells carry their table's prefix (past-f); noun cells are row-col.
    private func cellID(row: String, col: String, _ table: Paradigms.Table) -> String {
        if table.id == "past" { return "past-\(row)" }
        if table.id == "imp" { return "imp-\(row)" }
        return table.cell(row: row, col: col)?.id ?? "\(row)-\(col)"
    }

    @ViewBuilder private func cell(id: String, form: String, ending: String) -> some View {
        let isAsked = asked.contains(id)
        let done = result.status.isComplete
        let isRight = right.contains(id)
        ZStack(alignment: .topTrailing) {
            if !isAsked || done && !isRight && !tileMode && result.status != .missed {
                given(form, dim: false)
            } else if tileMode {
                tileCell(id: id, form: form)
            } else {
                writingCell(id: id, form: form)
            }
            if !ending.hasPrefix("="), ending != form {
                Text(ending).font(.system(size: 10, weight: .semibold, design: .rounded)).foregroundStyle(Theme.muted.opacity(0.8))
                    .padding(.horizontal, 5).padding(.vertical, 2)
                    .background(Capsule().fill(Theme.paper.opacity(0.9)))
                    .padding(4)
                    .accessibilityHidden(true)
            }
        }
        .frame(width: cellWidth)
        .overlay(alignment: .bottomTrailing) {
            if isAsked, isRight || done { Image(systemName: isRight ? "checkmark" : "xmark").font(.system(size: 12, weight: .bold)).foregroundStyle(isRight ? Theme.good : Theme.redPen).padding(5) }
        }
    }

    private func given(_ form: String, dim: Bool) -> some View {
        HStack(spacing: 6) {
            Text(Theme.stressed(form, show: book.settings.showStress)).russianFont(22, weight: .medium).foregroundStyle(dim ? Theme.redPen : Theme.text)
            if result.status.isComplete { HearButton(text: form, size: 26) }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 10).padding(.vertical, 8)
        .frame(minHeight: 44)
        .background(RoundedRectangle(cornerRadius: 10).fill(Theme.card.opacity(0.6)))
        .accessibilityLabel(RussianText.unstressed(form))
    }

    private func writingCell(id: String, form: String) -> some View {
        let pad = pad(for: id)
        let locked = right.contains(id) || result.status.isComplete
        return VStack(alignment: .leading, spacing: 2) {
            ZStack(alignment: .topLeading) {
                WritingStrip(controller: pad, height: 72, ghost: nil, ghostTimes: 1, cursive: book.settings.cursiveModels, fingerDrawing: book.settings.fingerDrawing)
                    .frame(height: 72)
                    .allowsHitTesting(!locked)
                    .accessibilityIdentifier("cell-\(slot.index)-\(id)")
                if result.status == .missed && !right.contains(id) {
                    Text(RussianText.unstressed(form)).font(Theme.cursive(26)).foregroundStyle(Theme.redPen).rotationEffect(.degrees(-2)).padding(.leading, 8).allowsHitTesting(false)
                }
            }
            .background(RoundedRectangle(cornerRadius: 10).fill(Theme.card))
            .overlay(RoundedRectangle(cornerRadius: 10).stroke(right.contains(id) ? Theme.good : (readings[id] != nil ? Theme.redPen.opacity(0.6) : Theme.rule.opacity(0.4)), lineWidth: 1))
            if let r = readings[id], !right.contains(id), !result.status.isComplete {
                Text(r.isEmpty ? "can't read that yet" : "reads as «\(r)»").font(Theme.label(11)).foregroundStyle(Theme.redPen).lineLimit(1)
            }
        }
    }

    private func tileCell(id: String, form: String) -> some View {
        let chosen = placed[id].flatMap { tiles[safe: $0] }
        let selected = selectedCell == id
        let locked = right.contains(id) || result.status.isComplete
        return Button {
            if locked { return }
            if placed[id] != nil { placed[id] = nil; selectedCell = id } else { selectedCell = selected ? nil : id }
        } label: {
            HStack(spacing: 6) {
                if result.status == .missed, !right.contains(id) {
                    Text(Theme.stressed(form, show: book.settings.showStress)).russianFont(22, weight: .medium).foregroundStyle(Theme.redPen)
                } else if let chosen {
                    Text(Theme.stressed(chosen, show: book.settings.showStress)).russianFont(22, weight: .medium).foregroundStyle(Theme.text)
                } else {
                    Text(selected ? "tap a tile" : " ").scaledFont(13).foregroundStyle(Theme.muted.opacity(0.7))
                }
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 10).padding(.vertical, 8)
            .frame(minHeight: 44)
            .background(RoundedRectangle(cornerRadius: 10).fill(Theme.card))
            .overlay(RoundedRectangle(cornerRadius: 10).stroke(right.contains(id) ? Theme.good : selected ? Theme.accent : Theme.rule.opacity(0.5),
                                                              style: StrokeStyle(lineWidth: selected ? 2 : 1, dash: chosen == nil && !locked ? [5, 4] : [])))
        }
        .buttonStyle(.plain)
        .accessibilityLabel(chosen.map { "cell holds \(RussianText.unstressed($0))" } ?? "empty cell")
    }

    private var tray: some View {
        FlowRow(spacing: 10) {
            ForEach(tiles.indices, id: \.self) { i in
                let used = placed.values.contains(i)
                Button {
                    guard !used else { return }
                    guard let cell = selectedCell ?? asked.first(where: { placed[$0] == nil && !right.contains($0) }) else { return }
                    placed[cell] = i
                    selectedCell = asked.first { placed[$0] == nil && !right.contains($0) }
                } label: {
                    Text(Theme.stressed(tiles[i], show: book.settings.showStress)).russianFont(22, weight: .medium)
                        .padding(.horizontal, 14).padding(.vertical, 8)
                        .foregroundStyle(used ? Theme.muted.opacity(0.35) : Theme.text)
                        .background(RoundedRectangle(cornerRadius: 10).fill(used ? Theme.card.opacity(0.4) : Theme.card))
                        .overlay(RoundedRectangle(cornerRadius: 10).stroke(used ? Theme.rule.opacity(0.2) : Theme.accent.opacity(0.5), lineWidth: 1))
                }
                .buttonStyle(.plain)
                .disabled(used)
                .accessibilityLabel(RussianText.unstressed(tiles[i]))
            }
        }
    }

    @ViewBuilder private var controls: some View {
        if !result.status.isComplete {
            HStack(spacing: 10) {
                let ready = tileMode ? asked.allSatisfy { placed[$0] != nil || right.contains($0) } : asked.contains { !right.contains($0) && !(pads[$0]?.isEmpty ?? true) }
                if ready {
                    PillButton(title: checking ? "Reading…" : "Check", systemImage: "checkmark", prominent: true) { check() }.disabled(checking)
                }
                if (result.failedChecks ?? 0) >= 1 {
                    PillButton(title: "Show me", systemImage: "eye", tint: Theme.redPen) { reveal() }
                }
                if !tileMode, let cell = asked.first(where: { !right.contains($0) && !(pads[$0]?.isEmpty ?? true) }) {
                    PillButton(title: "Clear", systemImage: "eraser") { pads[cell]?.clear(); readings[cell] = nil }
                }
                Spacer(minLength: 0)
            }
            .frame(minHeight: 34)
        }
    }

    @ViewBuilder private var feedback: some View {
        switch result.status {
        case .correct: Text("Right first time.").font(Theme.label(14)).foregroundStyle(Theme.good)
        case .done: Text("Got it.").font(Theme.label(14)).foregroundStyle(Theme.good)
        case .missed:
            if let table, let word, let cell = asked.first(where: { !right.contains($0) }) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("The missing forms are in red. Copy one under its cell with the pattern in view; the table comes back soon.").scaledFont(15).foregroundStyle(Theme.redPen)
                    ParadigmRowHintView(hint: ParadigmRowHint(table: p.table(word: word, cell: cell) ?? table, word: word, cell: cell))
                }
            }
        default:
            if let wrong = asked.first(where: { readings[$0] != nil && !right.contains($0) }), !tileMode {
                Text("Not all of it yet. «\(readings[wrong] ?? "")» isn't the form for \(cueLabel(wrong)).").scaledFont(15, weight: .medium).foregroundStyle(Theme.redPen)
            } else if tileMode, result.attempts > 0, right.count < asked.count {
                Text("Not all of it yet. The marked cells are right; try the others again.").scaledFont(15, weight: .medium).foregroundStyle(Theme.redPen)
            }
        }
    }

    private func cueLabel(_ cell: String) -> String {
        guard let word, let t = p.table(word: word, cell: cell) else { return cell }
        return p.cueLabel(table: t, cell: cell)
    }

    private func pad(for cell: String) -> PadController {
        if let pad = pads[cell] { return pad }
        let url = slot.inkURL.deletingPathExtension().appendingPathExtension("\(cell).drawing")
        let pad = PadController(fileURL: url, writer: slot.book.store.ink)
        pads[cell] = pad
        return pad
    }

    /// Cells confirmed earlier are kept in `recognized` as "cell=reading", so a reopened page shows them.
    private func restore() {
        guard let rec = result.recognized else { return }
        for part in rec.split(separator: ";") {
            let kv = part.split(separator: "=", maxSplits: 1).map(String.init)
            guard kv.count == 2 else { continue }
            if kv[1].hasPrefix("✓") { right.insert(kv[0]); if tileMode, let i = tiles.firstIndex(of: String(kv[1].dropFirst())) { placed[kv[0]] = i } }
        }
    }

    private func encodeState() -> String {
        asked.map { cell in "\(cell)=" + (right.contains(cell) ? "✓" + (tileMode ? (placed[cell].flatMap { tiles[safe: $0] } ?? "") : (word?.answers(cell).first ?? "")) : (readings[cell] ?? "")) }
            .joined(separator: ";")
    }

    private func check() {
        guard let word, !checking else { return }
        checking = true
        let pending = asked.filter { !right.contains($0) }
        let ref = slot.ref, book = slot.book, round = result.roundNumber
        if tileMode {
            for cell in pending {
                guard let i = placed[cell], let tile = tiles[safe: i] else { continue }
                if TransformCheck.matches(tile, answer: word.forms[cell] ?? "") { right.insert(cell) } else { readings[cell] = tile; placed[cell] = nil }
            }
            settle(book: book, ref: ref, round: round, allRight: right.count == asked.count)
            return
        }
        Task {
            var confirmed = right
            var seen = readings
            for cell in pending {
                guard let pad = pads[cell], !pad.isEmpty else { continue }
                let reading = await InkReader.read(pad.drawing, target: word.answers(cell).first ?? "")
                let ok = reading.map { r in r.matches || word.answers(cell).contains { TransformCheck.matches(r.best, answer: $0) } } ?? false
                if ok { confirmed.insert(cell); seen[cell] = nil } else { seen[cell] = reading?.best ?? "" }
            }
            guard let current = book.result(ref), !current.status.isComplete, current.roundNumber == round else { checking = false; return }
            right = confirmed
            readings = seen
            settle(book: book, ref: ref, round: round, allRight: confirmed.count == asked.count)
        }
    }

    private func settle(book: Book, ref: ExerciseRef, round: Int, allRight: Bool) {
        checking = false
        let slot = ExerciseSlot(book: book, ref: ref)
        let state = encodeState()
        if allRight {
            if let word, let first = asked.first, let f = word.forms[first] { voice.play(f) }
            slot.finish { r in
                r.attempts += 1
                r.recognized = state
                r.status = (r.failedChecks ?? 0) == 0 && r.attempts == 1 ? .correct : .done
            }
        } else {
            slot.record { r in
                r.attempts += 1
                r.failedChecks = (r.failedChecks ?? 0) + 1
                r.recognized = state
            }
        }
    }

    private func reveal() {
        slot.finish { r in
            r.revealed = true
            r.status = .missed
            r.recognized = encodeState()
        }
        if let word, let cell = asked.first(where: { !right.contains($0) }), let f = word.forms[cell] { voice.play(f) }
    }
}

// MARK: - cue_sort

/// Phrases sorted into the case or pattern they show. Tap a phrase, then its bucket; a wrong
/// bucket sends it back once, twice places it for you in red. Flawless is recall; with slips,
/// practice; with a phrase placed for you, missed.
struct CueSortExercise: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    let slot: ExerciseSlot
    let isActive: Bool
    @State private var selected: Int?
    @State private var placed: [Int: Int] = [:]     // item → bucket
    @State private var revealedItems: Set<Int> = []
    @State private var misses: [Int: Int] = [:]
    @State private var shake: Int?

    private var ex: Exercise { slot.exercise }
    private var result: ExerciseResult { slot.result }
    private var items: [SortItem] { ex.items ?? [] }
    private var options: [String] { ex.options ?? [] }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Instruction(text: "Sort each phrase by what its case is doing: tap the phrase, then where it belongs.")
            if !result.status.isComplete {
                FlowRow(spacing: 10) {
                    ForEach(items.indices.filter { placed[$0] == nil }, id: \.self) { i in chip(i, inBucket: false) }
                }
                .frame(minHeight: 44, alignment: .leading)
            }
            HStack(alignment: .top, spacing: 12) {
                ForEach(options.indices, id: \.self) { b in bucket(b) }
            }
            feedback
        }
        .onAppear(perform: restore)
        .animation(.easeOut(duration: 0.2), value: placed)
    }

    private func chip(_ i: Int, inBucket: Bool) -> some View {
        let isSelected = selected == i
        let wrong = revealedItems.contains(i)
        return HStack(spacing: 6) {
            Button {
                if !inBucket, !result.status.isComplete { selected = isSelected ? nil : i }
            } label: {
                Text(Theme.stressed(items[i].text, show: book.settings.showStress)).russianFont(inBucket ? 18 : 22, weight: .medium)
                    .foregroundStyle(wrong ? Theme.redPen : Theme.text)
                    .padding(.horizontal, 12).padding(.vertical, 7)
                    .background(RoundedRectangle(cornerRadius: 10).fill(Theme.card))
                    .overlay(RoundedRectangle(cornerRadius: 10).stroke(isSelected ? Theme.accent : wrong ? Theme.redPen : Theme.rule.opacity(0.5), lineWidth: isSelected ? 2 : 1))
            }
            .buttonStyle(.plain)
            .offset(x: shake == i ? 6 : 0)
            .animation(shake == i ? .spring(response: 0.15, dampingFraction: 0.2) : .default, value: shake)
            HearButton(text: items[i].text, size: 24)
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(RussianText.unstressed(items[i].text))
        .accessibilityValue(isSelected ? "selected" : (inBucket ? (wrong ? "placed for you" : "placed") : ""))
    }

    private func bucket(_ b: Int) -> some View {
        let members = items.indices.filter { placed[$0] == b }
        let use = book.knowledge.domain.paradigms.uses.first { $0.title == options[b] || $0.id == options[b] }
        return Button { drop(into: b) } label: {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 6) {
                    Text(Theme.stressed(options[b], show: book.settings.showStress)).scaledFont(15, weight: .semibold).foregroundStyle(Theme.accent)
                    if let use { Text("· \(use.case)").font(Theme.label(12)).foregroundStyle(Theme.muted) }
                }
                if members.isEmpty {
                    Text(selected != nil ? "put it here" : " ").scaledFont(13).foregroundStyle(Theme.muted.opacity(0.7)).frame(minHeight: 30)
                } else {
                    VStack(alignment: .leading, spacing: 6) { ForEach(members, id: \.self) { chip($0, inBucket: true) } }
                }
            }
            .padding(10)
            .frame(maxWidth: .infinity, minHeight: 80, alignment: .topLeading)
            .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card.opacity(0.5)))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(selected != nil ? Theme.accent.opacity(0.7) : Theme.rule.opacity(0.4),
                                                                   style: StrokeStyle(lineWidth: 1, dash: selected != nil ? [6, 4] : [])))
        }
        .buttonStyle(.plain)
        .disabled(selected == nil || result.status.isComplete)
        .accessibilityLabel("\(options[b]): \(members.count) placed")
    }

    @ViewBuilder private var feedback: some View {
        switch result.status {
        case .correct: Text("Right first time.").font(Theme.label(14)).foregroundStyle(Theme.good)
        case .done: Text("Sorted, with a slip or two on the way.").font(Theme.label(14)).foregroundStyle(Theme.good)
        case .missed:
            if let use = revealedItems.sorted().first.flatMap({ items[safe: $0] }).flatMap({ it in book.knowledge.domain.paradigms.uses.first { $0.title == options[safe: it.answer] } }) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("The red one was placed for you.").scaledFont(15).foregroundStyle(Theme.redPen)
                    Text(Theme.stressed(use.rule, show: book.settings.showStress)).scaledFont(15).foregroundStyle(Theme.text).fixedSize(horizontal: false, vertical: true)
                }
            }
        default:
            if let s = selected { Text("«\(RussianText.unstressed(items[s].text))» — which job is its case doing?").scaledFont(14).foregroundStyle(Theme.muted) }
        }
    }

    private func drop(into b: Int) {
        guard let i = selected, !result.status.isComplete else { return }
        let right = items[i].answer == b
        if right {
            placed[i] = b
            selected = nil
            voice.play(items[i].text)
            finishIfDone()
        } else {
            misses[i, default: 0] += 1
            shake = i
            Task { try? await Task.sleep(for: .milliseconds(250)); shake = nil }
            let wrongPlacement = "\(i):\(b)"
            slot.record { r in r.attempts += 1; r.recognized = wrongPlacement }
            if misses[i, default: 0] >= 2 {
                placed[i] = items[i].answer
                revealedItems.insert(i)
                selected = nil
                finishIfDone()
            }
        }
    }

    private func finishIfDone() {
        guard placed.count == items.count else { return }
        let revealed = !revealedItems.isEmpty
        let slips = misses.values.reduce(0, +)
        let state = items.indices.map { "\($0):\(placed[$0] ?? -1)\(revealedItems.contains($0) ? "!" : "")" }.joined(separator: ",")
        slot.finish { r in
            r.attempts += 1
            r.recognized = state
            if revealed { r.revealed = true; r.status = .missed } else { r.status = slips == 0 ? .correct : .done }
        }
    }

    private func restore() {
        guard result.status.isComplete, let rec = result.recognized, rec.contains(",") || rec.contains("!") || items.count == 1 else { return }
        for part in rec.split(separator: ",") {
            let bang = part.hasSuffix("!")
            let kv = part.trimmingCharacters(in: CharacterSet(charactersIn: "!")).split(separator: ":").compactMap { Int($0) }
            guard kv.count == 2, kv[1] >= 0 else { continue }
            placed[kv[0]] = kv[1]
            if bang { revealedItems.insert(kv[0]) }
        }
    }
}
