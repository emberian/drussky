import LearningCore
import SwiftUI

/// On a word's history card: its hook, or a button that asks the tutor for one. The learner
/// accepts it, rewords it, or writes their own; the hook is theirs to keep or drop.
struct HookCard: View {
    @Environment(Book.self) private var book
    let id: ComponentID

    private enum Phase: Equatable { case idle, asking, proposed(MemoryHook), editing, failed(String) }
    @State private var phase: Phase = .idle
    @State private var draft = ""
    @State private var draftEmoji = ""
    @State private var exportURL: URL?

    private var ru: String { book.knowledge.graph[id]?.label ?? id.key }
    private var word: Course.Word? { book.word(ru) ?? book.word(id.key) }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            switch phase {
            case .idle:
                if let hook = book.hook(for: ru) { existing(hook) } else { none }
            case .asking:
                HStack(spacing: 10) {
                    ProgressView().controlSize(.small)
                    Text("Asking the tutor for a hook…").scaledFont(14).foregroundStyle(Theme.muted)
                }
            case .proposed(let hook):
                hookLine(hook)
                HStack(spacing: 10) {
                    PillButton(title: "Keep it", systemImage: "checkmark", prominent: true) { book.setHook(hook, for: ru); phase = .idle }
                    PillButton(title: "In my words", systemImage: "pencil") { draft = hook.text; draftEmoji = hook.emoji ?? ""; phase = .editing }
                    PillButton(title: "Another", systemImage: "arrow.counterclockwise") { ask() }
                    PillButton(title: "No, thanks", systemImage: "xmark") { phase = .idle }
                }
            case .editing:
                editor
            case .failed(let why):
                Text(why).scaledFont(14).foregroundStyle(Theme.redPen).fixedSize(horizontal: false, vertical: true)
                HStack(spacing: 10) {
                    PillButton(title: "Write my own", systemImage: "pencil") { startOwn() }
                    PillButton(title: "Never mind", systemImage: "xmark") { phase = .idle }
                }
            }
        }
        .padding(14)
        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
        .accessibilityIdentifier("hook-card")
        .animation(.easeOut(duration: 0.2), value: phase)
    }

    private func hookLine(_ hook: MemoryHook) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            if let e = hook.emoji { Text(e).font(.system(size: 26)) }
            Text(hook.text).font(.system(size: 18, design: .serif)).italic().foregroundStyle(Theme.text)
                .fixedSize(horizontal: false, vertical: true)
        }
        .accessibilityLabel("Hook: \(hook.text)")
    }

    private func existing(_ hook: MemoryHook) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Text("HOOK").font(Theme.label(11)).foregroundStyle(Theme.muted)
                Spacer()
                let n = book.recallsSinceHook(for: ru)
                Text(HookFading.opacity(recalls: n) > 0 ? "shown faintly for \(max(0, HookFading.steps.count - n)) more recall\(HookFading.steps.count - n == 1 ? "" : "s")" : "no longer shown before recalls")
                    .font(.system(size: 12)).foregroundStyle(Theme.muted)
            }
            hookLine(hook)
            HStack(spacing: 10) {
                PillButton(title: "Edit", systemImage: "pencil") { draft = hook.text; draftEmoji = hook.emoji ?? ""; phase = .editing }
                PillButton(title: "Another from the tutor", systemImage: "sparkles") { ask() }
                PillButton(title: "Remove", systemImage: "trash", tint: Theme.redPen) { book.setHook(nil, for: ru) }
                Spacer()
                if let exportURL {
                    ShareLink(item: exportURL) { Label("Export hooks", systemImage: "square.and.arrow.up").font(Theme.label(13)).foregroundStyle(Theme.accent) }
                }
            }
            .task { exportURL = book.exportHooks() }
        }
    }

    private var none: some View {
        HStack(spacing: 12) {
            PillButton(title: "Give me a hook", systemImage: "link", prominent: true) { ask() }
                .accessibilityIdentifier("hook-ask")
            PillButton(title: "Write my own", systemImage: "pencil") { startOwn() }
            Text("A sound-alike tied to the meaning, shown faintly before the next few recalls.")
                .scaledFont(13).foregroundStyle(Theme.muted).fixedSize(horizontal: false, vertical: true)
        }
    }

    private var editor: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 10) {
                TextField("emoji", text: $draftEmoji).frame(width: 56).textFieldStyle(.roundedBorder)
                    .onChange(of: draftEmoji) { _, v in if v.count > 1 { draftEmoji = String(v.suffix(1)) } }
                TextField("KEYWORD — the image", text: $draft, axis: .vertical).textFieldStyle(.roundedBorder).lineLimit(1...3)
            }
            HStack(spacing: 10) {
                PillButton(title: "Save", systemImage: "checkmark", prominent: true) {
                    let text = draft.trimmingCharacters(in: .whitespacesAndNewlines)
                    guard !text.isEmpty else { return }
                    let source = Self.source(of: text, proposed: proposedText, existing: book.hook(for: ru))
                    book.setHook(MemoryHook(text: text, emoji: draftEmoji.isEmpty ? nil : draftEmoji, source: source, created: book.now(),
                                            model: source == .mine ? nil : Tutor.model), for: ru)
                    proposedText = nil
                    phase = .idle
                }
                .disabled(draft.trimmingCharacters(in: .whitespaces).isEmpty)
                PillButton(title: "Cancel", systemImage: "xmark") { phase = .idle }
            }
        }
    }

    /// The tutor's text when the editor opened from a proposal (to tell "edited" from "mine").
    @State private var proposedText: String?

    /// Whose words a saved hook is: the tutor's as proposed, the tutor's reworded, or the learner's own.
    static func source(of text: String, proposed: String?, existing: MemoryHook?) -> MemoryHook.Source {
        if let proposed { return text == proposed ? .tutor : .edited }
        if let existing, existing.source != .mine { return text == existing.text ? existing.source : .edited }
        return .mine
    }

    private func startOwn() {
        draft = ""
        draftEmoji = word?.emoji ?? ""
        proposedText = nil
        phase = .editing
    }

    private func ask() {
        let w = word ?? Course.Word(ru: ru, en: book.knowledge.graph[id]?.gloss ?? "", emoji: nil, note: nil)
        guard let key = Keychain.apiKey, !key.isEmpty else { startOwn(); return }
        phase = .asking
        let at = book.now()
        Task {
            do {
                let h = try await HookTutor.suggest(for: w, at: at)
                proposedText = h.text
                phase = .proposed(h)
            } catch {
                phase = .failed(error.localizedDescription)
            }
        }
    }
}

/// Before a recall of a word: its hook, as faint as its schedule says, or nothing.
struct HookLine: View {
    @Environment(Book.self) private var book
    let word: String

    var body: some View {
        let opacity = book.hookOpacity(for: word)
        if opacity > 0, let hook = book.hook(for: word) {
            HStack(spacing: 8) {
                if let e = hook.emoji { Text(e).font(.system(size: 16)) }
                Text(hook.text).font(.system(size: 15, design: .serif)).italic().foregroundStyle(Theme.text)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .opacity(opacity)
            .accessibilityLabel("Hook: \(hook.text)")
            .accessibilityIdentifier("hook-line")
        }
    }
}
