import GameController
import LearningCore
import Observation
import SwiftUI

extension Modality {
    /// An answer typed on a keyboard (the system Russian layout or the phonetic scheme). Its own
    /// memory: knowing how to type a word is not knowing how to write it.
    static let type: Modality = "type"
}

/// Whether a hardware keyboard is attached (always on the Mac). `-qa-keyboard` pretends one is,
/// for simulator screenshots.
@MainActor
@Observable
final class KeyboardPresence {
    static let shared = KeyboardPresence()
    private(set) var present: Bool

    private init() {
        #if targetEnvironment(macCatalyst)
        present = true
        #else
        present = GCKeyboard.coalesced != nil || QA.has("-qa-keyboard")
        let center = NotificationCenter.default
        for name in [Notification.Name.GCKeyboardDidConnect, .GCKeyboardDidDisconnect] {
            center.addObserver(forName: name, object: nil, queue: .main) { _ in
                Task { @MainActor in Self.shared.present = GCKeyboard.coalesced != nil || QA.has("-qa-keyboard") }
            }
        }
        #endif
    }
}

/// The checker a typed answer goes through: the same matching as handwriting (homoglyphs, ё/е,
/// an agreeing subject pronoun dropped when translating), and for practice kinds the target
/// written whole one or more times.
enum TypedCheck {
    static func accepts(_ typed: String, exercise ex: Exercise) -> Bool {
        guard let target = ex.target, !typed.trimmingCharacters(in: .whitespaces).isEmpty else { return false }
        if RussianText.answerMatches(typed, target, subjectDrop: ex.kind == .translate) { return true }
        let practice = [.meet, .trace, .copy].contains(ex.kind)
        return practice && RussianText.isRepetition(typed, of: target)
    }
}

/// Type the answer instead of writing it. Shown under a writing strip when a hardware keyboard
/// is attached; the Pencil stays the first choice on the iPad (the strip is still there).
struct TypedAnswer: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    let slot: ExerciseSlot
    let isActive: Bool
    @AppStorage("typing.open") private var open = false
    @AppStorage("typing.phonetic") private var phonetic = false
    @State private var text = ""
    @State private var latin = ""
    @State private var miss: String?
    @FocusState private var focused: Bool

    private var ex: Exercise { slot.exercise }
    private var result: ExerciseResult { slot.result }
    private var isRecall: Bool { [.dictation, .translate, .fill].contains(ex.kind) }
    private var target: String { ex.target ?? "" }
    private var shown: String { phonetic ? Transliteration.russian(latin) : text }

    var body: some View {
        if result.status.isComplete {
            if result.typed == true, let typed = result.recognized {
                HStack(spacing: 8) {
                    Image(systemName: "keyboard").foregroundStyle(Theme.muted)
                    Text(typed).russianFont(24, weight: .medium).foregroundStyle(Theme.text)
                    Text("typed").font(Theme.label(12)).foregroundStyle(Theme.muted)
                }
                .accessibilityLabel("Typed \(typed)")
            }
        } else if !open {
            Button { open = true; focused = true } label: {
                Label("Type it instead", systemImage: "keyboard").font(Theme.label(13)).foregroundStyle(Theme.muted)
            }
            .buttonStyle(.plain)
            .accessibilityIdentifier("type-it-\(slot.index)")
        } else {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 10) {
                    field
                    if phonetic, !latin.isEmpty {
                        Text(shown).russianFont(26, weight: .medium).foregroundStyle(Theme.text)
                            .accessibilityLabel("reads \(shown)")
                    }
                    PillButton(title: "Check", systemImage: "checkmark", prominent: true) { check() }
                        .disabled(shown.trimmingCharacters(in: .whitespaces).isEmpty)
                        .accessibilityIdentifier("type-check-\(slot.index)")
                }
                KeyboardMapView(next: RussianKeyboard.next(target: target, typed: shown), phonetic: phonetic)
                HStack(spacing: 14) {
                    Toggle(isOn: $phonetic) { Text("Phonetic (sh → ш, ya → я, ' → ь)") }
                        .toggleStyle(.switch).font(Theme.label(12)).foregroundStyle(Theme.muted)
                        .fixedSize()
                    Spacer(minLength: 0)
                    Button { open = false } label: {
                        Label("Use the Pencil", systemImage: "pencil").font(Theme.label(12)).foregroundStyle(Theme.muted)
                    }
                    .buttonStyle(.plain)
                }
                if let miss {
                    HStack(spacing: 10) {
                        Text("«\(miss)» isn't it yet.").scaledFont(15, weight: .medium).foregroundStyle(Theme.text)
                        PillButton(title: "Show me", systemImage: "eye", tint: Theme.redPen) {
                            slot.record { $0.revealed = true; $0.status = .missed; $0.typed = true }
                            if let s = ex.spoken { voice.play(s) }
                        }
                    }
                }
            }
            .onAppear { if isActive { focused = true } }
            .onChange(of: phonetic) { _, _ in latin = ""; text = ""; miss = nil }
        }
    }

    private var field: some View {
        Group {
            if phonetic {
                TextField("type phonetically", text: $latin)
            } else {
                TextField("напиши́те", text: $text)
            }
        }
        .textFieldStyle(.plain)
        .font(phonetic ? .system(size: 20, design: .monospaced) : Theme.russian(26, weight: .medium))
        .foregroundStyle(phonetic ? Theme.muted : Theme.text)
        .autocorrectionDisabled()
        .textInputAutocapitalization(.never)
        .focused($focused)
        .onSubmit { check() }
        .onChange(of: text) { _, _ in miss = nil }
        .onChange(of: latin) { _, _ in miss = nil }
        .padding(.horizontal, 12).padding(.vertical, 8)
        .frame(minWidth: 180, maxWidth: 420)
        .background(RoundedRectangle(cornerRadius: 10).fill(Theme.card))
        .overlay(RoundedRectangle(cornerRadius: 10).stroke(Theme.rule.opacity(0.5), lineWidth: 1))
        .accessibilityIdentifier("type-field-\(slot.index)")
    }

    private func check() {
        let answer = shown.trimmingCharacters(in: .whitespaces)
        guard !answer.isEmpty, !result.status.isComplete else { return }
        if TypedCheck.accepts(answer, exercise: ex) {
            let recall = isRecall
            slot.finish { r in
                r.attempts += 1
                r.status = recall ? .correct : .done
                r.recognized = answer
                r.typed = true
            }
            miss = nil
        } else {
            slot.record { r in
                r.attempts += 1
                r.failedChecks = (r.failedChecks ?? 0) + 1
                r.recognized = answer
                r.typed = true
            }
            miss = answer
        }
    }
}

/// A small ЙЦУКЕН map with the next needed key lit, so the layout is learned by looking down
/// at the screen rather than at the keys. In phonetic mode the Latin spelling is shown too.
struct KeyboardMapView: View {
    let next: RussianKeyboard.Next
    var phonetic = false
    var keyWidth: CGFloat = 22

    private var needed: Character? { if case .letter(let c) = next { return c } else { return nil } }

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            ForEach(Array(RussianKeyboard.rows.enumerated()), id: \.offset) { r, row in
                HStack(spacing: 3) {
                    Spacer().frame(width: CGFloat(r) * keyWidth * 0.45)
                    ForEach(Array(row.enumerated()), id: \.offset) { _, key in
                        cap(String(key), lit: needed == key)
                    }
                    if r == 0 { cap("ё", lit: needed == "ё") }
                }
            }
            HStack(spacing: 8) {
                RoundedRectangle(cornerRadius: 4)
                    .fill(next == .space ? Theme.accent : Theme.card)
                    .frame(width: keyWidth * 5, height: keyWidth * 0.8)
                    .overlay(RoundedRectangle(cornerRadius: 4).stroke(Theme.rule.opacity(0.4), lineWidth: 0.8))
                    .padding(.leading, keyWidth * 3)
                if next == .backspace {
                    Label("delete", systemImage: "delete.left").font(Theme.label(11)).foregroundStyle(Theme.redPen)
                } else if next == .done {
                    Label("return", systemImage: "return").font(Theme.label(11)).foregroundStyle(Theme.good)
                } else if phonetic, let needed, let latin = Transliteration.latin(for: needed) {
                    Text("\(String(needed)) = \(latin)").font(.system(size: 12, weight: .semibold, design: .monospaced)).foregroundStyle(Theme.accent)
                }
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(label)
    }

    private var label: String {
        switch next {
        case .letter(let c): "Next key: \(c)"
        case .space: "Next key: space"
        case .backspace: "Next key: delete"
        case .done: "Done, press return"
        }
    }

    private func cap(_ key: String, lit: Bool) -> some View {
        Text(key)
            .font(.system(size: keyWidth * 0.55, weight: lit ? .bold : .medium, design: .rounded))
            .foregroundStyle(lit ? Color.white : Theme.text)
            .frame(width: keyWidth, height: keyWidth * 0.95)
            .background(RoundedRectangle(cornerRadius: 4).fill(lit ? Theme.accent : Theme.card))
            .overlay(RoundedRectangle(cornerRadius: 4).stroke(lit ? Theme.accent : Theme.rule.opacity(0.4), lineWidth: lit ? 1.5 : 0.8))
            .scaleEffect(lit ? 1.12 : 1)
            .animation(.easeOut(duration: 0.12), value: lit)
    }
}
