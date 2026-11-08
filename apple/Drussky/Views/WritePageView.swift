import SwiftUI

/// Asks the tutor for a new page: practice from what you keep missing, a reading page from a
/// Russian text you bring, or a library text — as written or rescaled to your level.
struct WritePageSheet: View {
    /// `.goalAuto`: opened by the book itself (policy in Goal/Goal.swift); writing starts at once.
    enum Mode: Hashable { case practice, text, goal, goalAuto }
    @Environment(\.dismiss) private var dismiss
    @Environment(Book.self) private var book
    let mode: Mode

    private var source: WritePageContent.Source? {
        switch mode {
        case .practice: .practice
        case .text: .ownText
        case .goal: book.currentGoal.map { .goal($0, auto: false) }
        case .goalAuto: book.currentGoal.map { .goal($0, auto: true) }
        }
    }

    var body: some View {
        NavigationStack {
            if let source {
                WritePageContent(source: source, onDone: { dismiss() })
                    .navigationTitle(mode == .practice ? "A page from your mistakes" : mode == .text ? "Read your own text" : "Toward your book")
                    .navigationBarTitleDisplayMode(.inline)
                    .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } } }
            }
        }
    }
}

struct WritePageContent: View {
    enum Source: Hashable { case practice, ownText, library(Library.Text), goal(Goal, auto: Bool) }
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
                case .goal(let g, _):
                    GoalPullHeader(goal: g)
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
        .task {
            if case .goal(let g, auto: true) = source, !writing { await write(.goal(brief(g))) }
        }
    }

    @ViewBuilder private var buttons: some View {
        switch source {
        case .practice:
            bigButton("Write my page", enabled: !writing) { .practice }
        case .ownText:
            bigButton("Make it a page", enabled: !writing && RussianText.letters(in: text).count >= 3) { .text(text) }
        case .library(let t):
            bigButton("Rescale to my level", enabled: !writing) { .rescale(t.text, title: "\(t.author), «\(t.title)»") }
            bigButton("Read it as written", enabled: !writing, prominent: false) { .text(t.text) }
        case .goal(let g, _):
            bigButton("Write a page toward my book", enabled: !writing && Keychain.apiKey?.isEmpty == false) { .goal(brief(g)) }
            Button { makeLocalGoalPage(g) } label: {
                Text("Without the tutor: its words in their own sentences")
                    .font(.system(size: 15, weight: .medium)).foregroundStyle(Theme.accent)
            }
            .buttonStyle(.plain).disabled(writing)
        }
    }

    private func brief(_ g: Goal) -> GoalPull.Brief {
        let d = book.distance(for: g)
        return GoalPull.Brief(goal: g, distance: d,
                              patterns: d.patterns(domain: book.knowledge.domain, model: book.knowledge.model, at: book.now()))
    }

    private func makeLocalGoalPage(_ g: Goal) {
        let b = brief(g)
        let met = Set(book.course.letters.filter { book.letterKnown($0.upper) }.map(\.upper))
        let page = GoalPull.localPage(b, met: met, allMet: met.count >= book.course.letters.count)
        guard page.exercises.count >= 2 else { error = "Not enough of the text is within reach yet — a few more pages and try again."; return }
        book.insertGoalPage(PageWriter.Written(title: page.title, intro: page.intro, exercises: page.exercises, new_words: nil),
                            exercises: page.exercises, goal: g, sentences: b.sentences)
        onDone()
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
            if case .goal(let brief) = request {
                book.insertGoalPage(page, exercises: exercises, goal: brief.goal, sentences: brief.sentences)
                onDone()
                return
            }
            let unit: String = {
                switch source {
                case .practice: "Written for you"
                case .ownText: "Your text"
                case .library(let t): "Library · \(t.level)"
                case .goal: "Toward your book"
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
            Text("Original level \(text.level) · \(text.license)\(text.isPrivate == true ? " · personal study" : "")")
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
