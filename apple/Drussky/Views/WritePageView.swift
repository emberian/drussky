import SwiftUI

/// Asks the tutor for a new page: practice from what you keep missing, a reading page from a
/// Russian text you bring, or a library text — as written or rescaled to your level.
struct WritePageSheet: View {
    enum Mode: Hashable { case practice, text }
    @Environment(\.dismiss) private var dismiss
    let mode: Mode

    var body: some View {
        NavigationStack {
            WritePageContent(source: mode == .practice ? .practice : .ownText, onDone: { dismiss() })
                .navigationTitle(mode == .practice ? "A page from your mistakes" : "Read your own text")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } } }
        }
    }
}

struct WritePageContent: View {
    enum Source: Hashable { case practice, ownText, library(Library.Text) }
    @Environment(Book.self) private var book
    let source: Source
    var onDone: () -> Void
    @State private var text = ""
    @State private var writing = false
    @State private var progress = 0
    @State private var error: String?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                switch source {
                case .practice:
                    Text("Your tutor writes a fresh page from the things you've been finding hardest, and puts it right after this one.")
                        .font(.system(size: 17)).foregroundStyle(Theme.text)
                case .ownText:
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
                case .library(let t):
                    LibraryTextHeader(text: t)
                }
                if writing {
                    HStack(spacing: 12) {
                        ProgressView()
                        Text(progress == 0 ? "Your tutor is reading…" : "Your tutor is writing the page… \(progress) characters")
                            .font(.system(size: 15)).foregroundStyle(Theme.muted)
                    }
                }
                if let error { Text(error).font(.system(size: 15)).foregroundStyle(Theme.redPen) }
                buttons
            }
            .padding(24)
        }
        .background(Theme.paper.ignoresSafeArea())
        .interactiveDismissDisabled(writing)
    }

    @ViewBuilder private var buttons: some View {
        switch source {
        case .practice:
            bigButton("Write my page", enabled: !writing) { .practice }
        case .ownText:
            bigButton("Make it a page", enabled: !writing && RussianText.letters(in: text).count >= 3) { .text(text) }
        case .library(let t):
            if t.isNarrated { ReadAlongButton(text: t, onDone: onDone).disabled(writing) }
            bigButton("Rescale to my level", enabled: !writing, prominent: !t.isNarrated) { .rescale(t.text, title: "\(t.author), «\(t.title)»") }
            bigButton("Read it as written", enabled: !writing, prominent: false) { .text(t.text) }
        }
    }

    private func bigButton(_ title: String, enabled: Bool, prominent: Bool = true, request: @escaping () -> PageWriter.Request) -> some View {
        Button { Task { await write(request()) } } label: {
            Text(title)
                .font(.system(size: 18, weight: .semibold, design: .rounded))
                .frame(maxWidth: .infinity).padding(.vertical, 14)
                .background(Capsule().fill(prominent ? (enabled ? Theme.accent : Theme.muted.opacity(0.3)) : Theme.accent.opacity(0.1)))
                .foregroundStyle(prominent ? Color.white : Theme.accent)
        }
        .buttonStyle(.plain)
        .disabled(!enabled)
    }

    private func write(_ request: PageWriter.Request) async {
        writing = true
        error = nil
        progress = 0
        defer { writing = false }
        do {
            let (page, exercises) = try await PageWriter.write(request, book: book) { n in progress = n }
            let unit: String = {
                switch source {
                case .practice: "Written for you"
                case .ownText: "Your text"
                case .library(let t): "Library · \(t.level)"
                }
            }()
            book.insertWrittenPage(title: page.title, intro: page.intro, unitTitle: unit,
                                   exercises: exercises, newWords: page.new_words ?? [])
            onDone()
        } catch {
            self.error = error.localizedDescription
        }
    }
}

struct LibraryTextHeader: View {
    let text: Library.Text
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(text.title).font(Theme.russian(30, weight: .semibold)).foregroundStyle(Theme.text)
            Text([text.author, text.year.map(String.init)].compactMap { $0 }.joined(separator: ", "))
                .font(.system(size: 15)).foregroundStyle(Theme.muted)
            if let s = text.summary_en { Text(s).font(.system(size: 17)).foregroundStyle(Theme.text) }
            Text(String(text.text.prefix(420)) + (text.text.count > 420 ? "…" : ""))
                .font(Theme.russian(19)).foregroundStyle(Theme.text.opacity(0.85))
                .padding(14)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card))
            Text("Original level \(text.level) · \(text.license)\(text.isPrivate == true ? " · personal study" : "")"
                 + (text.reader.map { " · read aloud by \($0)" } ?? ""))
                .font(Theme.label(12)).foregroundStyle(Theme.muted)
        }
    }
}

/// Real texts to read, by level. Opening one makes a reading page — as written or rescaled.
struct LibrarySheet: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(Book.self) private var book
    @State private var library = Library.load()
    @State private var topic = "all"

    private var topics: [String] { ["all"] + Array(Set(library.texts.map(\.topic))).sorted() }

    var body: some View {
        NavigationStack {
            List {
                Picker("Topic", selection: $topic) {
                    ForEach(topics, id: \.self) { Text($0 == "all" ? "Everything" : $0.capitalized).tag($0) }
                }
                .pickerStyle(.segmented)
                .listRowBackground(Color.clear)
                ForEach(Library.levels, id: \.self) { level in
                    let texts = library.texts.filter { $0.level == level && (topic == "all" || $0.topic == topic) }
                    if !texts.isEmpty {
                        Section(level) {
                            ForEach(texts) { t in
                                NavigationLink {
                                    WritePageContent(source: .library(t), onDone: { dismiss() })
                                        .navigationTitle(t.title).navigationBarTitleDisplayMode(.inline)
                                } label: {
                                    HStack {
                                        VStack(alignment: .leading, spacing: 3) {
                                            Text(t.title).font(Theme.russian(20, weight: .medium))
                                            Text([t.author, t.title_en].compactMap { $0 }.joined(separator: " · "))
                                                .font(.system(size: 14)).foregroundStyle(.secondary)
                                        }
                                        Spacer()
                                        if t.isNarrated {
                                            Image(systemName: "headphones").foregroundStyle(Theme.accent)
                                                .accessibilityLabel("Read aloud by \(t.reader ?? "a human reader")")
                                        }
                                        ReadabilityBadge(fraction: book.knowledge.readability(of: t.text))
                                    }
                                }
                            }
                        }
                    }
                }
                if library.texts.isEmpty {
                    Text("The library is empty in this build.").foregroundStyle(.secondary)
                }
            }
            .navigationTitle("Library")
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        }
    }
}
