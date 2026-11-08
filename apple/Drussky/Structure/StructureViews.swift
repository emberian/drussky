import LearningCore
import SwiftUI

/// The «Структу́ра» layer on a reading page: a teacher's pencil marks over the text. Members of
/// the sentence are underlined the way a Russian school exercise book does it (подлежа́щее one line,
/// сказу́емое two, дополне́ние dashed, определе́ние wavy, обстоя́тельство dash-dot); embedded clauses
/// get light brackets; a pronoun or participle wears a small ring and, tapped, pulses its head; a
/// participial clause, tapped, shows its flat Russian (English on a second tap); a word held down
/// shows its roles and its root x-ray. Nothing moves: every mark is drawn in the word's own padding
/// and the row spacing.
@MainActor
@Observable
final class StructureLayer {
    struct WordRef: Hashable {
        var sentence: Int
        var word: Int
    }
    enum Panel: Equatable {
        case desugar(sentence: Int, StructureSentence.Desugaring, english: Bool)
        case xray(sentence: Int, word: Int)
        case frame(sentence: Int, StructureSentence.Frame)
    }

    private(set) var pulsing: Set<WordRef> = []
    var panel: Panel?
    private var lastLongPress: Date = .distantPast
    private var pulseTask: Task<Void, Never>?

    /// A tap that arrives as a long press ends is the same gesture, not a second one.
    func consumeSuppressedTap() -> Bool { Date().timeIntervalSince(lastLongPress) < 0.5 }

    /// The learner tapped a word with the layer on: pulse what it points at, open its paraphrase or frame.
    func tapped(_ s: StructureSentence, at sentenceIndex: Int, words: Range<Int>) {
        guard let w = words.first else { return }
        var heads: Set<WordRef> = []
        if let r = s.referent(at: w), r.headSentence == s.index { heads.insert(WordRef(sentence: sentenceIndex, word: r.head)) }
        if let d = s.desugaring(at: w) {
            // The flat Russian first; a second tap on the same construction adds the English.
            if case .desugar(sentenceIndex, d, false) = panel { panel = .desugar(sentence: sentenceIndex, d, english: true) }
            else { panel = .desugar(sentence: sentenceIndex, d, english: false) }
        } else if let f = s.frame(headedBy: w) {
            panel = .frame(sentence: sentenceIndex, f)
            for d in f.dependents { heads.insert(WordRef(sentence: sentenceIndex, word: d)) }
        } else if case .xray = panel { panel = nil }
        if !heads.isEmpty { pulse(heads) }
    }

    /// Held down: the word's roles and root x-ray.
    func longPressed(_ s: StructureSentence, at sentenceIndex: Int, words: Range<Int>) {
        lastLongPress = Date()
        guard let w = words.first else { return }
        panel = .xray(sentence: sentenceIndex, word: w)
    }

    func pulse(_ refs: Set<WordRef>) {
        pulseTask?.cancel()
        pulsing = refs
        pulseTask = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(1400))
            guard !Task.isCancelled else { return }
            self?.pulsing = []
        }
    }

    func reset() { panel = nil; pulsing = [] }

    // MARK: Marks

    /// The marks for one display token, given its neighbours on the line (to join underlines).
    func mark(for token: PassageStructure.Token?, previous: PassageStructure.Token?, next: PassageStructure.Token?,
              in passage: PassageStructure, roles: [String: StructureFile.Role]) -> StructureMark {
        guard let token, let si = token.sentence, let s = passage.sentence(si), let w = token.words.first else { return .none }
        var m = StructureMark()
        let member = s.members(of: w, roles: roles).first
        m.underline = member.flatMap { StructureMark.Underline(role: $0.role) }
        if let member {
            m.joinLeft = previous?.sentence == si && previous.map { !$0.words.isEmpty && member.contains($0.words.first!) } == true
            m.joinRight = next?.sentence == si && next.map { !$0.words.isEmpty && member.contains($0.words.first!) } == true
        }
        let clauses = s.chunks.filter { roles[$0.role]?.tier == "clause" }
        m.opens = clauses.filter { token.words.contains($0.first) }.count
        m.closes = clauses.filter { token.words.contains($0.last) }.count
        m.isReferent = s.referent(at: w).map { $0.headSentence == s.index } ?? false
        m.pulsing = token.words.contains { pulsing.contains(WordRef(sentence: si, word: $0)) }
        return m
    }
}

/// What to draw over one word.
struct StructureMark: Equatable {
    enum Underline: Equatable {
        case single, double, dashed, wavy, dashDot
        /// The school convention for the members of a sentence.
        init?(role: String) {
            switch role {
            case "subject": self = .single
            case "predicate": self = .double
            case "object": self = .dashed
            case "attribute", "apposition": self = .wavy
            case "adverbial", "prepositional_phrase": self = .dashDot
            default: return nil
            }
        }
    }
    var underline: Underline?
    var joinLeft = false
    var joinRight = false
    var opens = 0
    var closes = 0
    var isReferent = false
    var pulsing = false
    static let none = StructureMark()
}

/// Draws a word's structure marks in its padding and the row spacing, so the text never moves.
struct StructureMarks: ViewModifier {
    let mark: StructureMark
    let on: Bool
    /// Half the row spacing: how far a joined underline reaches toward its neighbour.
    var reach: CGFloat = 4

    func body(content: Content) -> some View {
        content
            .overlay(alignment: .bottom) {
                if on, let u = mark.underline {
                    UnderlineView(kind: u)
                        .frame(height: 6)
                        .padding(.leading, mark.joinLeft ? -reach : 2)
                        .padding(.trailing, mark.joinRight ? -reach : 2)
                        .offset(y: -1)
                        .allowsHitTesting(false)
                }
            }
            .overlay(alignment: .leading) {
                if on, mark.opens > 0 { BracketView(count: mark.opens, opening: true).offset(x: -3).allowsHitTesting(false) }
            }
            .overlay(alignment: .trailing) {
                if on, mark.closes > 0 { BracketView(count: mark.closes, opening: false).offset(x: 3).allowsHitTesting(false) }
            }
            .overlay(alignment: .topLeading) {
                if on, mark.isReferent {
                    Circle().stroke(Theme.pencil, lineWidth: 1)
                        .frame(width: 4.5, height: 4.5)
                        .offset(x: 0, y: 1)
                        .allowsHitTesting(false)
                }
            }
            .background {
                RoundedRectangle(cornerRadius: 6)
                    .fill(Theme.accent.opacity(mark.pulsing ? 0.22 : 0))
                    .padding(-2)
            }
            .scaleEffect(mark.pulsing ? 1.08 : 1)
            .animation(mark.pulsing ? .easeInOut(duration: 0.3).repeatCount(5, autoreverses: true) : .easeOut(duration: 0.25), value: mark.pulsing)
            .animation(.easeOut(duration: 0.25), value: on)
    }
}

extension View {
    func structureMarks(_ mark: StructureMark, on: Bool) -> some View { modifier(StructureMarks(mark: mark, on: on)) }
}

/// One of the five school underlines, in pencil.
struct UnderlineView: View {
    let kind: StructureMark.Underline

    var body: some View {
        Canvas { ctx, size in
            let y = size.height / 2
            var p = Path()
            let color = GraphicsContext.Shading.color(Theme.pencil)
            switch kind {
            case .single:
                p.move(to: .init(x: 0, y: y)); p.addLine(to: .init(x: size.width, y: y))
                ctx.stroke(p, with: color, lineWidth: 1)
            case .double:
                p.move(to: .init(x: 0, y: y - 1.4)); p.addLine(to: .init(x: size.width, y: y - 1.4))
                p.move(to: .init(x: 0, y: y + 1.4)); p.addLine(to: .init(x: size.width, y: y + 1.4))
                ctx.stroke(p, with: color, lineWidth: 1)
            case .dashed:
                p.move(to: .init(x: 0, y: y)); p.addLine(to: .init(x: size.width, y: y))
                ctx.stroke(p, with: color, style: StrokeStyle(lineWidth: 1, dash: [4, 3]))
            case .dashDot:
                p.move(to: .init(x: 0, y: y)); p.addLine(to: .init(x: size.width, y: y))
                ctx.stroke(p, with: color, style: StrokeStyle(lineWidth: 1, lineCap: .round, dash: [5, 3, 0.1, 3]))
            case .wavy:
                let wave: CGFloat = 6, amp: CGFloat = 1.3
                p.move(to: .init(x: 0, y: y))
                var x: CGFloat = 0
                while x < size.width {
                    p.addQuadCurve(to: .init(x: min(x + wave / 2, size.width), y: y), control: .init(x: x + wave / 4, y: y - amp))
                    p.addQuadCurve(to: .init(x: min(x + wave, size.width), y: y), control: .init(x: x + 3 * wave / 4, y: y + amp))
                    x += wave
                }
                ctx.stroke(p, with: color, lineWidth: 0.9)
            }
        }
    }
}

/// A light square bracket at a clause boundary; nested clauses step outward.
struct BracketView: View {
    let count: Int
    let opening: Bool

    var body: some View {
        Canvas { ctx, size in
            for i in 0..<min(count, 3) {
                let x = opening ? 2 + CGFloat(i) * 2.5 : size.width - 2 - CGFloat(i) * 2.5
                let foot: CGFloat = opening ? 2.5 : -2.5
                var p = Path()
                p.move(to: .init(x: x + foot, y: 3))
                p.addLine(to: .init(x: x, y: 3))
                p.addLine(to: .init(x: x, y: size.height - 3))
                p.addLine(to: .init(x: x + foot, y: size.height - 3))
                ctx.stroke(p, with: .color(Theme.pencil), lineWidth: 0.9)
            }
        }
        .frame(width: 10)
    }
}

/// The quiet toggle in the text's margin.
struct StructureToggle: View {
    @Environment(Book.self) private var book

    var body: some View {
        let on = book.settings.showStructure ?? false
        Button {
            withAnimation(.easeOut(duration: 0.25)) { book.settings.showStructure = !on }
        } label: {
            Text(Theme.stressed("Структу́ра", show: book.settings.showStress, color: on ? Theme.accent : Theme.pencil))
                .font(Theme.russian(14))
                .foregroundStyle(on ? Theme.accent : Theme.pencil)
                .overlay(alignment: .bottom) {
                    Rectangle().fill(on ? Theme.accent : .clear).frame(height: 1).offset(y: 1)
                }
                .padding(.horizontal, 6).padding(.vertical, 4)
        }
        .buttonStyle(.plain)
        .accessibilityLabel(on ? "Hide the structure" : "Show the structure")
        .accessibilityIdentifier("structure-toggle")
    }
}

/// What a tap or a long press opened, under the text: a paraphrase, a word's roles and x-ray, a case frame.
struct StructurePanel: View {
    @Environment(Book.self) private var book
    let layer: StructureLayer
    let passage: PassageStructure
    @State private var englishRole: String?

    var body: some View {
        if let panel = layer.panel {
            VStack(alignment: .leading, spacing: 8) {
                switch panel {
                case .desugar(let si, let d, let english):
                    desugar(d, sentence: si, english: english)
                case .xray(let si, let w):
                    if let s = passage.sentence(si) { xray(s, word: w, sentence: si) }
                case .frame(let si, let f):
                    if let s = passage.sentence(si) { frame(f, in: s) }
                }
            }
            .padding(.vertical, 8).padding(.horizontal, 12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .overlay(alignment: .leading) { Rectangle().fill(Theme.pencil).frame(width: 1).padding(.vertical, 4) }
            .transition(.opacity)
            .accessibilityIdentifier("structure-panel")
        }
    }

    private func label(_ ru: String, _ en: String, id: String) -> some View {
        Button { withAnimation { englishRole = englishRole == id ? nil : id } } label: {
            HStack(spacing: 6) {
                Text(Theme.stressed(ru, show: book.settings.showStress, color: Theme.pencil)).font(Theme.russian(13)).italic()
                if englishRole == id { Text(en).font(Theme.label(11)).foregroundStyle(Theme.muted) }
            }
            .foregroundStyle(Theme.pencil)
        }
        .buttonStyle(.plain)
        .accessibilityLabel("\(RussianText.unstressed(ru)), \(en)")
    }

    @ViewBuilder private func desugar(_ d: StructureSentence.Desugaring, sentence si: Int, english: Bool) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Text(Theme.stressed("про́ще:", show: book.settings.showStress, color: Theme.pencil)).font(Theme.russian(13)).italic().foregroundStyle(Theme.pencil)
            Button {
                withAnimation { layer.panel = .desugar(sentence: si, d, english: !english) }
            } label: {
                Text(Theme.stressed(d.ru, show: book.settings.showStress)).russianFont(21).foregroundStyle(Theme.text)
                    .multilineTextAlignment(.leading).fixedSize(horizontal: false, vertical: true)
            }
            .buttonStyle(.plain)
            .accessibilityIdentifier("structure-paraphrase")
            HearButton(text: d.ru, size: 30)
        }
        if english {
            Text(d.en).scaledFont(15).italic().foregroundStyle(Theme.text.opacity(0.8)).fixedSize(horizontal: false, vertical: true)
        }
    }

    @ViewBuilder private func xray(_ s: StructureSentence, word w: Int, sentence si: Int) -> some View {
        let roles = Structure.shared.roles
        let members = s.members(of: w, roles: roles)
        let clauses = s.clauses(of: w, roles: roles)
        FlowRow(spacing: 10) {
            Text(Theme.stressed(s.word(w), show: book.settings.showStress)).russianFont(21, weight: .medium).foregroundStyle(Theme.text)
            ForEach(Array((members.prefix(1) + clauses.prefix(1)).enumerated()), id: \.offset) { i, c in
                if let r = roles[c.role] { label(r.ru, r.en, id: "\(c.role)-\(i)") }
            }
            if let r = s.referent(at: w), r.headSentence == s.index {
                label("→ \(s.word(r.head))", "points at \(s.word(r.head))", id: "ref")
            }
        }
        if let x = s.xray(at: w) {
            FlowRow(spacing: 12) {
                ForEach(Array(x.pieces.enumerated()), id: \.offset) { i, p in
                    VStack(alignment: .leading, spacing: 1) {
                        HStack(spacing: 6) {
                            if i > 0 { Text("+").font(Theme.label(12)).foregroundStyle(Theme.pencil) }
                            Text(p.0).russianFont(19, weight: p.1 == "root" ? .semibold : .regular)
                                .foregroundStyle(p.1 == "root" ? Theme.accent : Theme.text)
                        }
                        Text(Self.pieceLabel(p.1)).font(Theme.russian(10)).italic().foregroundStyle(Theme.pencil)
                            .padding(.leading, i > 0 ? 14 : 0)
                    }
                }
                if !x.gloss.isEmpty {
                    Text("· \(x.gloss)").scaledFont(14).foregroundStyle(Theme.muted).padding(.top, 2)
                }
            }
            .accessibilityElement(children: .combine)
            let relatives = known(x.relatives)
            if !relatives.isEmpty {
                HStack(spacing: 8) {
                    Text(Theme.stressed("того́ же ко́рня:", show: book.settings.showStress, color: Theme.pencil))
                        .font(Theme.russian(13)).italic().foregroundStyle(Theme.pencil)
                    ForEach(relatives, id: \.word) { r in
                        Text(Theme.stressed(r.word, show: book.settings.showStress)).russianFont(17)
                            .foregroundStyle(r.known ? Theme.text : Theme.pencil)
                    }
                }
            }
        }
        if let f = s.frame(headedBy: w) { frame(f, in: s) }
    }

    @ViewBuilder private func frame(_ f: StructureSentence.Frame, in s: StructureSentence) -> some View {
        HStack(spacing: 10) {
            Text(Theme.stressed("управле́ние:", show: book.settings.showStress, color: Theme.pencil)).font(Theme.russian(13)).italic().foregroundStyle(Theme.pencil)
            Text(Theme.stressed(f.frame, show: book.settings.showStress)).russianFont(18).foregroundStyle(Theme.text)
        }
    }

    /// Relatives the learner has met first (up to two), then one more in pencil if there is nothing known.
    private func known(_ words: [String]) -> [(word: String, known: Bool)] {
        let k = book.knowledge
        let tagged = words.map { w in (word: w, known: k.model.hasEvidence(ComponentID(.lexeme, RussianText.key(w))) || book.word(w) != nil) }
        let met = tagged.filter(\.known).prefix(2)
        if !met.isEmpty { return Array(met) }
        return Array(tagged.prefix(2))
    }

    static func pieceLabel(_ kind: String) -> String {
        switch kind {
        case "prefix": "приста́вка"
        case "root": "ко́рень"
        case "suffix": "су́ффикс"
        default: "оконча́ние"
        }
    }
}

/// Who is meant: a pronoun or participle pulses; tap the word it points to.
struct StructureExercise: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    let slot: ExerciseSlot
    let isActive: Bool
    @State private var beat = false
    @State private var shaking: Int?

    private var ex: Exercise { slot.exercise }
    private var result: ExerciseResult { slot.result }
    private var sentence: String { ex.text ?? "" }
    private var words: [String] { Structure.wordStrings(in: sentence) }

    var body: some View {
        let probe = ex.probe ?? -1
        let head = ex.answerIndex ?? -1
        let finished = result.status.isComplete
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 14) {
                Instruction(text: finished ? "Who is meant." : "Who is meant? Tap the word «\(RussianText.unstressed(ex.word ?? ""))» points to.")
                HearButton(text: sentence, size: 34)
            }
            FlowRow(spacing: 8) {
                ForEach(Array(tokens.enumerated()), id: \.offset) { _, tok in
                    let isProbe = tok.words.contains(probe)
                    let isHead = tok.words.contains(head)
                    let wrong = shaking != nil && tok.words.contains(shaking!)
                    Button { tap(tok) } label: {
                        Text(Theme.stressed(tok.text, show: book.settings.showStress))
                            .russianFont(26, weight: isProbe ? .medium : .regular)
                            .foregroundStyle(wrong ? Theme.redPen : Theme.text)
                            .padding(.vertical, 2).padding(.horizontal, 3)
                            .background(RoundedRectangle(cornerRadius: 6)
                                .fill(isProbe ? Theme.accent.opacity(beat && !finished ? 0.26 : 0.12)
                                      : isHead && finished ? Theme.good.opacity(0.18)
                                      : isHead && result.chosen == head ? Theme.good.opacity(0.18) : .clear))
                            .overlay(alignment: .bottom) {
                                if isHead, finished {
                                    Rectangle().fill(result.status == .missed ? Theme.redPen : Theme.good).frame(height: 1.2).offset(y: -1)
                                }
                            }
                            .overlay(alignment: .topLeading) {
                                if isProbe {
                                    Circle().stroke(Theme.accent, lineWidth: 1).frame(width: 5, height: 5).offset(y: 1)
                                }
                            }
                    }
                    .buttonStyle(.plain)
                    .disabled(finished || tok.words.isEmpty)
                    .accessibilityLabel(RussianText.unstressed(tok.text))
                    .accessibilityIdentifier(isProbe ? "structure-probe" : isHead ? "structure-head" : "structure-word")
                    .accessibilityValue(isProbe ? "the word asked about" : isHead && finished ? "what it points to" : "")
                }
            }
            .padding(18)
            .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
            if result.status == .missed, words.indices.contains(head) {
                ChoiceCorrection(slot: slot) {
                    HStack(spacing: 8) {
                        Text(Theme.stressed(ex.word ?? "", show: book.settings.showStress)).russianFont(19, weight: .medium).foregroundStyle(Theme.text)
                        Text("→").scaledFont(15).foregroundStyle(Theme.muted)
                        Text(Theme.stressed(words[head], show: book.settings.showStress)).russianFont(19, weight: .semibold).foregroundStyle(Theme.good)
                    }
                }
            }
        }
        .onAppear {
            guard !finished else { return }
            withAnimation(.easeInOut(duration: 0.9).repeatForever(autoreverses: true)) { beat = true }
        }
        .animation(.easeOut(duration: 0.2), value: result.status)
    }

    /// Display tokens (split on spaces) with the word indices each carries.
    private var tokens: [(text: String, words: Range<Int>)] {
        let scalars = Array(sentence.unicodeScalars)
        let starts = Structure.words(in: sentence).map(\.lowerBound)
        var out: [(String, Range<Int>)] = []
        var i = 0
        while i < scalars.count {
            if scalars[i].properties.isWhitespace { i += 1; continue }
            let a = i
            while i < scalars.count, !scalars[i].properties.isWhitespace { i += 1 }
            let inside = starts.indices.filter { (a..<i).contains(starts[$0]) }
            let range = inside.isEmpty ? 0..<0 : inside[0]..<(inside[inside.count - 1] + 1)
            out.append((String(String.UnicodeScalarView(scalars[a..<i])), range))
        }
        return out
    }

    private func tap(_ tok: (text: String, words: Range<Int>)) {
        guard let head = ex.answerIndex, let w = tok.words.first else { return }
        let right = tok.words.contains(head)
        voice.play(PassageExercise.bare(tok.text))
        if !right {
            shaking = w
            Task { try? await Task.sleep(for: .milliseconds(600)); if shaking == w { shaking = nil } }
        }
        slot.finish { r in
            r.chosen = w
            r.recognized = words[safe: w]
            r.attempts += 1
            r.status = right ? (r.attempts == 1 ? .correct : .done) : (r.attempts >= 2 ? .missed : .open)
            if r.status == .missed { r.revealed = true }
        }
    }
}
