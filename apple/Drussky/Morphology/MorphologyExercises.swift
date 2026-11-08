import LearningCore
import SwiftUI

/// build_word: the meaning in English and a tray of morpheme tiles; tap them into order, then check.
/// Right at the first check is recall; a second try that lands is practice; two misses show the word.
struct BuildWordExercise: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    let slot: ExerciseSlot
    let isActive: Bool
    @State private var placed: [Int] = []
    @State private var lastTry: String?
    /// When the exercise came up (response latency runs from here to Check).
    @State private var shownAt: Date?

    private var ex: Exercise { slot.exercise }
    private var result: ExerciseResult { slot.result }
    private var tiles: [String] { ex.options ?? ex.parts ?? [] }
    private var answer: String { ex.answerText ?? "" }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Instruction(text: "Build the word from its parts.")
            HStack(spacing: 12) {
                if let emoji = book.word(answer)?.emoji { Text(emoji).scaledFont(36) }
                Text(ex.en ?? "").scaledFont(26, weight: .semibold, design: .serif).foregroundStyle(Theme.text)
            }
            assembled
            if !result.status.isComplete {
                FlowRow(spacing: 10) {
                    ForEach(tiles.indices, id: \.self) { i in
                        tile(tiles[i], dimmed: placed.contains(i)) { if !placed.contains(i) { placed.append(i) } }
                    }
                }
                HStack(spacing: 10) {
                    if !placed.isEmpty {
                        PillButton(title: "Check", systemImage: "checkmark", prominent: true) { check() }
                        PillButton(title: "Clear", systemImage: "eraser") { placed = []; lastTry = nil }
                    }
                }
                .frame(minHeight: 34)
            }
            feedback
        }
        .animation(.easeOut(duration: 0.2), value: placed)
        .onChange(of: isActive, initial: true) { _, active in if active, shownAt == nil { shownAt = Date() } }
    }

    /// The row the tiles land in; a placed tile taps back out. After the exercise, the word built.
    private var assembled: some View {
        HStack(spacing: 6) {
            if result.status.isComplete {
                let built = result.status == .missed ? answer : (result.recognized ?? answer)
                Text(Theme.stressed(built, show: book.settings.showStress)).russianFont(32, weight: .medium)
                    .foregroundStyle(result.status == .missed ? Theme.redPen : Theme.text)
                HearButton(text: answer, size: 36)
            } else if placed.isEmpty {
                Text("tap the parts in order").scaledFont(15).foregroundStyle(Theme.muted.opacity(0.7))
            } else {
                ForEach(placed, id: \.self) { i in
                    tile(tiles[i], dimmed: false) { placed.removeAll { $0 == i } }
                }
            }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 14).padding(.vertical, 10)
        .frame(maxWidth: .infinity, minHeight: 56, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
        .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Theme.rule.opacity(0.5), style: StrokeStyle(lineWidth: 1, dash: result.status.isComplete ? [] : [6, 4])))
        .accessibilityElement(children: .combine)
        .accessibilityLabel(result.status.isComplete ? "Built: \(RussianText.unstressed(result.recognized ?? answer))"
                            : "Built so far: \(placed.map { RussianText.unstressed(tiles[$0]) }.joined())")
    }

    private func tile(_ text: String, dimmed: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(Theme.stressed(text, show: book.settings.showStress)).russianFont(26, weight: .medium)
                .padding(.horizontal, 16).padding(.vertical, 10)
                .foregroundStyle(dimmed ? Theme.muted.opacity(0.35) : Theme.text)
                .background(RoundedRectangle(cornerRadius: 12).fill(dimmed ? Theme.card.opacity(0.4) : Theme.card))
                .overlay(RoundedRectangle(cornerRadius: 12).stroke(dimmed ? Theme.rule.opacity(0.2) : Theme.accent.opacity(0.5), lineWidth: 1))
        }
        .buttonStyle(.plain)
        .disabled(dimmed)
        .accessibilityLabel(RussianText.unstressed(text))
    }

    @ViewBuilder private var feedback: some View {
        if result.status == .correct || result.status == .done {
            VStack(alignment: .leading, spacing: 6) {
                Text(result.status == .correct ? "Right first time." : "Got it.").font(Theme.label(14)).foregroundStyle(Theme.good)
                WordAnalysis(word: answer)
            }
        } else if result.status == .missed {
            VStack(alignment: .leading, spacing: 6) {
                HStack(spacing: 10) {
                    Text("It's").scaledFont(15, weight: .medium).foregroundStyle(Theme.text)
                    Text(Theme.stressed(answer, show: book.settings.showStress)).russianFont(22, weight: .medium).foregroundStyle(Theme.good)
                    PillButton(title: "Try it again", systemImage: "arrow.counterclockwise") { placed = []; lastTry = nil; slot.book.reopen(slot.ref) }
                        .accessibilityHint("Practice only; it isn't graded")
                }
                WordAnalysis(word: answer)
            }
        } else if let lastTry {
            Text("«\(lastTry)» isn't it. Try another order, or another part.").scaledFont(15, weight: .medium).foregroundStyle(Theme.redPen)
        }
    }

    private func check() {
        let chosen = placed.map { tiles[$0] }
        let built = BuildWordCheck.assemble(chosen)
        let right = BuildWordCheck.matches(chosen, answer: answer, alternatives: ex.alternatives)
        if right { voice.play(answer) }
        let missedNow = !right && result.attempts + 1 >= 2
        let took = shownAt.map { Int(Date().timeIntervalSince($0) * 1000) }
        shownAt = Date()  // a second try is timed from the first
        slot.finish { r in
            r.attempts += 1
            r.recognized = built
            r.status = right ? (r.attempts == 1 ? .correct : .done) : (missedNow ? .missed : .open)
            if missedNow { r.revealed = true }
            r.responseMs = took
        }
        if !right {
            lastTry = built
            placed = []
            if missedNow { voice.play(answer) }
        }
    }
}

/// family_guess: a sentence with one new word from a root the learner knows. Guess its meaning
/// from the root (the pretesting effect), then see how the word is built.
struct FamilyGuessExercise: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    let slot: ExerciseSlot
    let isActive: Bool
    /// When the exercise came up (response latency runs from here to the tap).
    @State private var shownAt: Date?

    private var ex: Exercise { slot.exercise }
    private var result: ExerciseResult { slot.result }
    private var word: String { ex.word ?? "" }

    var body: some View {
        let k = book.knowledge
        let root = ex.root.flatMap { k.domain.morphology.root($0) }
        VStack(alignment: .leading, spacing: 12) {
            Instruction(text: "A new word on a root you know. Guess what it means, then check.")
            HStack(alignment: .firstTextBaseline, spacing: 12) {
                sentence
                HearButton(text: ex.text ?? "", size: 36)
            }
            if let root {
                let known = k.domain.morphology.members(of: root.id).filter { k.model.hasEvidence(ComponentID(.lexeme, $0)) && $0 != RussianText.key(word) }
                HStack(spacing: 8) {
                    Text("root").font(Theme.label(12)).foregroundStyle(Theme.muted)
                    Text(root.label).russianFont(18, weight: .semibold).foregroundStyle(Theme.accent)
                    Text(root.gloss).scaledFont(15).foregroundStyle(Theme.text)
                    if !known.isEmpty {
                        Text("· you know").font(Theme.label(12)).foregroundStyle(Theme.muted)
                        Text(known.prefix(4).joined(separator: ", ")).russianFont(16).foregroundStyle(Theme.muted)
                    }
                }
                .accessibilityElement(children: .combine)
            }
            ChoiceGrid(options: ex.options ?? [], chosen: result.chosen, correct: ex.answerIndex,
                       russian: false, showStress: true, locked: result.status.isComplete) { i in
                let right = i == ex.answerIndex
                if right || result.attempts + 1 >= 2 { voice.play(word) }
                let took = shownAt.map { Int(Date().timeIntervalSince($0) * 1000) }
                shownAt = Date()
                slot.finish { r in
                    r.chosen = i
                    r.attempts += 1
                    r.status = right ? (r.attempts == 1 ? .correct : .done) : (r.attempts >= 2 ? .missed : .open)
                    r.responseMs = took
                }
            }
            if result.status == .missed, let answer = ex.answerIndex.flatMap({ ex.options?[safe: $0] }) {
                ChoiceCorrection(slot: slot) {
                    HStack(spacing: 8) {
                        Text("It means").scaledFont(15, weight: .medium).foregroundStyle(Theme.text)
                        Text(answer).scaledFont(17, weight: .semibold).foregroundStyle(Theme.good)
                    }
                }
            }
            if result.status.isComplete {
                WordAnalysis(word: word)
                if let en = ex.en { Text(en).scaledFont(16).italic().foregroundStyle(Theme.text.opacity(0.85)) }
            }
        }
        .onChange(of: isActive, initial: true) { _, active in if active, shownAt == nil { shownAt = Date() } }
    }

    /// The sentence, the new word set off from it.
    private var sentence: some View {
        let target = RussianText.key(word)
        return FlowRow(spacing: 8) {
            ForEach(Array((ex.text ?? "").split(separator: " ").enumerated()), id: \.offset) { _, token in
                let t = String(token)
                let hit = RussianText.key(PassageExercise.bare(t)) == target
                Text(Theme.stressed(t, show: book.settings.showStress))
                    .russianFont(28, weight: hit ? .semibold : .regular)
                    .foregroundStyle(hit ? Theme.accent : Theme.text)
                    .padding(.horizontal, hit ? 6 : 0).padding(.vertical, 2)
                    .background(RoundedRectangle(cornerRadius: 6).fill(hit ? Theme.accent.opacity(0.12) : .clear))
            }
        }
    }
}

/// "пере- across; again, re- · пис- write · -а́ть": a word taken apart, each piece glossed, from
/// morphology.json or, for a borrowing, from its Greek/Latin parts.
struct WordAnalysis: View {
    @Environment(Book.self) private var book
    let word: String

    var body: some View {
        let m = book.knowledge.domain.morphology
        let pieces: [(String, String?)] = {
            if let e = m.entry(word: word), let r = m.rootIndex(of: e), e.morphemes.count > 1 {
                return m.analysis(of: e).enumerated().map { i, p in (p.piece, i == r ? (p.gloss.map { "root: \($0)" }) : p.gloss) }
            }
            if let c = m.cognates.first(where: { RussianText.key($0.ru) == RussianText.key(word) }), let parts = c.parts {
                return parts.map { ($0.tile, $0.gloss) }
            }
            return []
        }()
        if !pieces.isEmpty {
            FlowRow(spacing: 14) {
                ForEach(Array(pieces.enumerated()), id: \.offset) { i, p in
                    HStack(spacing: 5) {
                        if i > 0 { Text("+").font(Theme.label(13)).foregroundStyle(Theme.muted) }
                        Text(Theme.stressed(p.0, show: book.settings.showStress)).russianFont(18, weight: .medium).foregroundStyle(Theme.text)
                        if let g = p.1 { Text(g).scaledFont(13).foregroundStyle(Theme.muted) }
                    }
                }
            }
            .accessibilityElement(children: .combine)
        }
    }
}
