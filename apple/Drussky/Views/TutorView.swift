import SwiftUI

/// A question asked from the page, kept on the page.
struct PageNote: Codable, Sendable, Hashable, Identifiable {
    var id = UUID()
    var exercise: Int?
    var turns: [Tutor.Turn]
    var date: Date
}

/// "Ask the tutor" — a short conversation about one exercise (or the page), saved as a note.
struct TutorSheet: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    @Environment(\.dismiss) private var dismiss
    let pageIndex: Int
    let exercise: Int?

    @State private var noteID: UUID?
    @State private var turns: [Tutor.Turn] = []
    @State private var draft = ""
    @State private var streaming = false
    @State private var error: String?
    @FocusState private var focused: Bool

    private var page: BookPage { book.pages[pageIndex] }

    var body: some View {
        NavigationStack {
            ScrollViewReader { proxy in
                ScrollView {
                    VStack(alignment: .leading, spacing: 16) {
                        contextCard
                        if turns.isEmpty { suggestions }
                        ForEach(Array(turns.enumerated()), id: \.offset) { i, turn in
                            bubble(turn).id(i)
                        }
                        if let error {
                            Text(error).font(.system(size: 15)).foregroundStyle(Theme.redPen)
                        }
                        Color.clear.frame(height: 1).id("bottom")
                    }
                    .padding(20)
                }
                .onChange(of: turns) { _, _ in withAnimation { proxy.scrollTo("bottom", anchor: .bottom) } }
            }
            .safeAreaInset(edge: .bottom) { composer }
            .background(Theme.paper.ignoresSafeArea())
            .environment(\.openURL, OpenURLAction { url in
                guard url.scheme == "drussky-say",
                      let t = URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems?.first(where: { $0.name == "t" })?.value
                else { return .systemAction }
                voice.play(t)
                return .handled
            })
            .navigationTitle("Ask your tutor")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        }
        .onAppear {
            if let existing = book.notes(page: pageIndex).last(where: { $0.exercise == exercise }) {
                noteID = existing.id
                turns = existing.turns
            }
            if case .unavailable(let why) = TutorRouter.route(.quickAnswer, .live) { error = why }
        }
    }

    private var contextCard: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(exercise.map { "Exercise \($0 + 1) · \(page.title)" } ?? page.title)
                .font(Theme.label(12)).foregroundStyle(Theme.accent)
            if let i = exercise, let line = book.describe(pageIndex: pageIndex, exercise: i).first {
                Text(line).font(.system(size: 15)).foregroundStyle(Theme.muted)
            }
        }
    }

    private var suggestions: some View {
        let options = exercise == nil
            ? ["What is this page teaching?", "Give me a memory trick for these letters", "How would I use this at dinner?"]
            : ["Why is it this way?", "How do I pronounce it?", "Give me a memory trick", "Show me two more examples"]
        return FlowRow(spacing: 8) {
            ForEach(options, id: \.self) { q in
                Button(q) { send(q) }
                    .font(.system(size: 15, weight: .medium))
                    .padding(.horizontal, 14).padding(.vertical, 9)
                    .background(Capsule().fill(Theme.accent.opacity(0.1)))
                    .foregroundStyle(Theme.accent)
                    .buttonStyle(.plain)
            }
        }
    }

    @ViewBuilder private func bubble(_ turn: Tutor.Turn) -> some View {
        if turn.role == "user" {
            Text(turn.text)
                .font(Theme.cursive(26))
                .foregroundStyle(Theme.ink)
                .frame(maxWidth: .infinity, alignment: .trailing)
        } else {
            VStack(alignment: .leading, spacing: 12) {
                Text(Self.markdown(Tutor.displayMarkdown(turn.text.isEmpty ? "…" : turn.text)))
                    .font(.system(size: 17))
                    .lineSpacing(3)
                    .foregroundStyle(Theme.text)
                    .tint(Theme.accent)
                    .textSelection(.enabled)
                let phrases = streaming && turn == turns.last ? [] : Tutor.phrases(in: turn.text)
                if !phrases.isEmpty {
                    Divider()
                    FlowRow(spacing: 8) {
                        ForEach(phrases, id: \.self) { PhraseChip(phrase: $0) }
                    }
                }
                if turn.provenance == .onDevice, let i = turns.firstIndex(of: turn), i > 0 {
                    HStack(spacing: 10) {
                        ProvenanceTag(.onDevice)
                        Button("Ask Claude for more") { send(turns[i - 1].text, via: .claude) }
                            .font(Theme.label(12)).foregroundStyle(Theme.accent).buttonStyle(.plain)
                            .disabled(streaming)
                    }
                }
            }
            .padding(16)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
        }
    }

    private var composer: some View {
        HStack(spacing: 10) {
            TextField("Ask anything — in English or Russian", text: $draft, axis: .vertical)
                .lineLimit(1...4)
                .focused($focused)
                .padding(12)
                .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
                .onSubmit { send(draft) }
            Button { send(draft) } label: {
                Image(systemName: streaming ? "ellipsis" : "arrow.up")
                    .font(.system(size: 18, weight: .bold))
                    .frame(width: 44, height: 44)
                    .background(Circle().fill(Theme.accent))
                    .foregroundStyle(.white)
            }
            .disabled(streaming || draft.trimmingCharacters(in: .whitespaces).isEmpty)
        }
        .padding(.horizontal, 16).padding(.vertical, 10)
        .background(.bar)
    }

    /// `via` forces a tutor ("Ask Claude for more"); by default the router picks one.
    private func send(_ text: String, via forced: TutorRoute? = nil) {
        let q = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !q.isEmpty, !streaming else { return }
        let route = forced ?? TutorRouter.route(.quickAnswer, .live)
        if case .unavailable(let why) = route { error = why; return }
        draft = ""
        error = nil
        turns.append(.init(role: "user", text: q))
        turns.append(.init(role: "assistant", text: ""))
        streaming = true
        let history = Array(turns.dropLast())
        let context = book.tutorContext(pageIndex: pageIndex, exercise: exercise)
        Task {
            var failed = false
            do {
                if route == .onDevice {
                    let answer = try await OnDeviceBackend().quickAnswer(question: q, context: context)
                    turns[turns.count - 1] = .init(role: "assistant", text: answer, provenance: .onDevice)
                } else {
                    for try await chunk in Tutor.stream(history, context: context) {
                        turns[turns.count - 1].text += chunk
                    }
                    turns[turns.count - 1].provenance = .claude
                }
            } catch {
                // A partial or refused answer is not an answer: drop it and give the question back.
                failed = true
                self.error = error.localizedDescription
                turns.removeLast(2)
                draft = q
            }
            streaming = false
            if !failed && !turns.isEmpty { noteID = book.saveNote(page: pageIndex, id: noteID, exercise: exercise, turns: turns) }
        }
    }

    static func markdown(_ s: String) -> AttributedString {
        (try? AttributedString(markdown: s, options: .init(interpretedSyntax: .inlineOnlyPreservingWhitespace))) ?? AttributedString(s)
    }
}

/// Questions asked on this page, shown at its foot like notes in the margin.
struct PageNotesView: View {
    @Environment(Book.self) private var book
    let pageIndex: Int
    var onOpen: (Int?) -> Void

    var body: some View {
        let notes = book.notes(page: pageIndex)
        if !notes.isEmpty {
            VStack(alignment: .leading, spacing: 14) {
                Text("Вопросы").font(Theme.cursive(30)).foregroundStyle(Theme.ink)
                ForEach(notes) { note in
                    Button { onOpen(note.exercise) } label: {
                        VStack(alignment: .leading, spacing: 6) {
                            if let first = note.turns.first {
                                Text((note.exercise.map { "№\($0 + 1) · " } ?? "") + first.text)
                                    .font(Theme.cursive(22)).foregroundStyle(Theme.ink)
                            }
                            if let answer = note.turns.last(where: { $0.role == "assistant" }) {
                                Text(TutorSheet.markdown(Tutor.displayMarkdown(answer.text)))
                                    .font(.system(size: 15)).foregroundStyle(Theme.text.opacity(0.85))
                                    .lineLimit(4)
                                    .multilineTextAlignment(.leading)
                            }
                        }
                        .padding(14)
                        .frame(maxWidth: 680, alignment: .leading)
                        .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card))
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.top, 28)
        }
    }
}

/// A Russian phrase from an answer: hear it, keep it.
struct PhraseChip: View {
    @Environment(Book.self) private var book
    let phrase: Tutor.Phrase

    var body: some View {
        let kept = book.isKept(phrase.ru)
        HStack(spacing: 8) {
            HearButton(text: phrase.ru, size: 30)
            VStack(alignment: .leading, spacing: 0) {
                Text(Theme.stressed(phrase.ru, show: book.settings.showStress)).font(Theme.russian(19, weight: .medium))
                    .fixedSize()
                if !phrase.en.isEmpty { Text(phrase.en).font(.system(size: 12)).foregroundStyle(Theme.muted) }
            }
            Button { book.keep(phrase.ru, en: phrase.en) } label: {
                Label(kept ? "Kept" : "Keep", systemImage: kept ? "checkmark" : "plus")
                    .font(Theme.label(12))
                    .fixedSize()
                    .padding(.horizontal, 10).padding(.vertical, 6)
                    .background(Capsule().fill(kept ? Theme.good.opacity(0.15) : Theme.accent.opacity(0.12)))
                    .foregroundStyle(kept ? Theme.good : Theme.accent)
            }
            .buttonStyle(.plain)
            .disabled(kept)
        }
        .padding(.vertical, 4).padding(.horizontal, 8)
        .background(RoundedRectangle(cornerRadius: 12).stroke(Theme.rule.opacity(0.4)))
    }
}
