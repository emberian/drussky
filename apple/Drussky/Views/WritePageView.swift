import SwiftUI

/// Asks the tutor for a new page: practice from what you keep missing, or a reading page from
/// a Russian text you bring (paste it, or use the keyboard's Scan Text to read it off paper).
struct WritePageSheet: View {
    enum Mode { case practice, text }
    @Environment(Book.self) private var book
    @Environment(\.dismiss) private var dismiss
    let mode: Mode
    @State private var text = ""
    @State private var writing = false
    @State private var progress = 0
    @State private var error: String?

    var body: some View {
        NavigationStack {
            VStack(alignment: .leading, spacing: 16) {
                switch mode {
                case .practice:
                    Text("Your tutor writes a fresh page from the things you've been finding hardest, and puts it right after this one.")
                        .font(.system(size: 17)).foregroundStyle(Theme.text)
                case .text:
                    Text("Paste any Russian — a paragraph of a story, a poem, a menu. On iPad you can also tap the text field and choose Scan Text to read it straight off paper.")
                        .font(.system(size: 17)).foregroundStyle(Theme.text)
                    TextEditor(text: $text)
                        .font(Theme.russian(20))
                        .scrollContentBackground(.hidden)
                        .padding(10)
                        .frame(minHeight: 220)
                        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
                        .overlay(alignment: .topLeading) {
                            if text.isEmpty {
                                Text("Вставьте текст…").font(Theme.russian(20)).foregroundStyle(Theme.muted)
                                    .padding(18).allowsHitTesting(false)
                            }
                        }
                }
                if writing {
                    HStack(spacing: 12) {
                        ProgressView()
                        Text(progress == 0 ? "Your tutor is thinking…" : "Your tutor is writing the page… \(progress) characters")
                            .font(.system(size: 15)).foregroundStyle(Theme.muted)
                    }
                }
                if let error { Text(error).font(.system(size: 15)).foregroundStyle(Theme.redPen) }
                Spacer()
                Button {
                    Task { await write() }
                } label: {
                    Text(mode == .practice ? "Write my page" : "Make it a page")
                        .font(.system(size: 18, weight: .semibold, design: .rounded))
                        .frame(maxWidth: .infinity).padding(.vertical, 14)
                        .background(Capsule().fill(canWrite ? Theme.accent : Theme.muted.opacity(0.3)))
                        .foregroundStyle(.white)
                }
                .buttonStyle(.plain)
                .disabled(!canWrite)
            }
            .padding(24)
            .background(Theme.paper.ignoresSafeArea())
            .navigationTitle(mode == .practice ? "A page from your mistakes" : "Read your own text")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() }.disabled(writing) } }
        }
        .interactiveDismissDisabled(writing)
    }

    private var canWrite: Bool {
        !writing && (mode == .practice || RussianText.letters(in: text).count >= 3)
    }

    private func write() async {
        writing = true
        error = nil
        progress = 0
        defer { writing = false }
        do {
            let request: PageWriter.Request = mode == .practice ? .practice : .text(text)
            let (page, exercises) = try await PageWriter.write(request, book: book) { n in progress = n }
            book.insertWrittenPage(title: page.title, intro: page.intro,
                                   unitTitle: mode == .practice ? "Written for you" : "Your text",
                                   exercises: exercises, newWords: page.new_words ?? [])
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
    }
}
