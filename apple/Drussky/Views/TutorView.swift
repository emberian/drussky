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
            .navigationTitle("Ask your tutor")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        }
        .onAppear {
            if let existing = book.notes(page: pageIndex).last(where: { $0.exercise == exercise }) {
                noteID = existing.id
                turns = existing.turns
            }
            if Keychain.apiKey == nil { error = Tutor.Failure.noKey.errorDescription }
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
            Text(Self.markdown(turn.text.isEmpty ? "…" : turn.text))
                .font(.system(size: 17))
                .lineSpacing(3)
                .foregroundStyle(Theme.text)
                .textSelection(.enabled)
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

    private func send(_ text: String) {
        let q = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !q.isEmpty, !streaming else { return }
        draft = ""
        error = nil
        turns.append(.init(role: "user", text: q))
        turns.append(.init(role: "assistant", text: ""))
        streaming = true
        let history = Array(turns.dropLast())
        let context = book.tutorContext(pageIndex: pageIndex, exercise: exercise)
        Task {
            do {
                for try await chunk in Tutor.stream(history, context: context) {
                    turns[turns.count - 1].text += chunk
                }
            } catch {
                self.error = error.localizedDescription
                if turns.last?.text.isEmpty == true { turns.removeLast(); turns.removeLast() }
            }
            streaming = false
            if !turns.isEmpty { noteID = book.saveNote(page: pageIndex, id: noteID, exercise: exercise, turns: turns) }
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
                                Text(TutorSheet.markdown(answer.text))
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
