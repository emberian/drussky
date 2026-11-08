import SwiftUI

/// The inside cover's "Distance to your book": one calm panel, or an invitation to choose one.
struct GoalSection: View {
    @Environment(Book.self) private var book
    @State private var picking = false
    @State private var writing = false
    @State private var filling: (Int, Int)?

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            if let goal = book.currentGoal {
                panel(goal)
            } else {
                invitation
            }
        }
        .sheet(isPresented: $picking) { GoalPicker().presentationSizing(.page) }
        .sheet(isPresented: $writing) { WritePageSheet(mode: .goal) }
    }

    private var invitation: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Your book").font(.system(size: 22, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
            Text("Choose a text you want to read for real — a chapter of Zamyatin, a page of analysis, anything pasted or linked. The course then pulls toward it, and this cover shows how far you are.")
                .font(.system(size: 15)).foregroundStyle(Theme.muted).fixedSize(horizontal: false, vertical: true)
            Button { picking = true } label: {
                Label("Choose your book", systemImage: "book")
                    .font(.system(size: 16, weight: .semibold, design: .rounded))
                    .padding(.horizontal, 18).padding(.vertical, 10)
                    .background(Capsule().fill(Theme.accent)).foregroundStyle(.white)
            }
            .buttonStyle(.plain)
            .accessibilityIdentifier("choose-goal")
        }
        .padding(18)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
    }

    @ViewBuilder private func panel(_ goal: Goal) -> some View {
        let d = book.distance(for: goal)
        let a = d.analysis
        let patterns = d.patterns(domain: book.knowledge.domain, model: book.knowledge.model, at: book.now())
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Distance to your book").font(.system(size: 22, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
                    Text(goal.byline).font(Theme.russian(16)).foregroundStyle(Theme.muted)
                }
                Spacer()
                Menu {
                    if book.goals.count > 1 {
                        Picker("Current book", selection: Binding(get: { book.currentGoalID ?? goal.id }, set: { book.currentGoalID = $0 })) {
                            ForEach(book.goals) { g in Text(g.title).tag(g.id) }
                        }
                    }
                    Button { picking = true } label: { Label("Add another text", systemImage: "plus") }
                    Button(role: .destructive) { book.removeGoal(id: goal.id) } label: { Label("Put this book away", systemImage: "trash") }
                } label: {
                    Image(systemName: "ellipsis.circle").font(.system(size: 20)).foregroundStyle(Theme.accent).frame(width: 44, height: 44)
                }
                .accessibilityLabel("Book options")
            }
            HStack(alignment: .top, spacing: 28) {
                stat("\(Int((d.coverage * 100).rounded()))%", "of its forms practiced from memory")
                stat("\(d.firstPageUnknown)", d.firstPageUnknown == 1 ? "word still to practice on the first page" : "words still to practice on the first page")
                stat("\(d.readableSentences) of \(a.sentences.count)", "sentences with 95%+ form coverage")
            }
            Text("Vocabulary coverage comes from writing from memory or saying these forms. Understanding the passage is a separate task.")
                .font(.system(size: 13)).foregroundStyle(Theme.muted)
            firstPage(goal, d)
            if !d.ladder.isEmpty {
                VStack(alignment: .leading, spacing: 8) {
                    Text("The next words worth knowing").font(Theme.label(13)).foregroundStyle(Theme.muted)
                    FlowRow(spacing: 8) {
                        ForEach(d.ladder) { rung in
                            HStack(spacing: 6) {
                                Text(rung.example.lowercased()).font(Theme.russian(17, weight: .medium)).foregroundStyle(Theme.text)
                                if let g = rung.gloss { Text(g).font(.system(size: 13)).foregroundStyle(Theme.muted).lineLimit(1) }
                                if rung.count > 1 { Text("×\(rung.count)").font(Theme.label(11)).foregroundStyle(Theme.accent) }
                            }
                            .padding(.horizontal, 10).padding(.vertical, 5)
                            .background(Capsule().fill(rung.onFirstPage ? Theme.accent.opacity(0.12) : Theme.card))
                            .overlay(Capsule().strokeBorder(Theme.accent.opacity(rung.onFirstPage ? 0.45 : 0.15)))
                        }
                    }
                    if let filling {
                        Text("Looking up meanings… \(filling.0) of \(filling.1)").font(.system(size: 13)).foregroundStyle(Theme.muted)
                    } else if d.ladder.contains(where: { $0.gloss == nil }) {
                        Text(Keychain.apiKey?.isEmpty == false ? "Meanings arrive from your tutor." : "Add a tutor key in Settings and the meanings fill in.")
                            .font(.system(size: 13)).foregroundStyle(Theme.muted)
                    }
                }
            }
            if !patterns.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Patterns it leans on").font(Theme.label(13)).foregroundStyle(Theme.muted)
                    FlowRow(spacing: 8) {
                        ForEach(patterns) { p in
                            HStack(spacing: 6) {
                                Circle().fill(p.strength.map { $0 >= 0.6 ? Theme.good : Theme.redPen } ?? Theme.muted.opacity(0.4)).frame(width: 8, height: 8)
                                Text(p.construction.name).font(Theme.russian(15)).foregroundStyle(Theme.text)
                                Text("×\(p.count)").font(Theme.label(11)).foregroundStyle(Theme.muted)
                            }
                        }
                    }
                }
            }
            HStack(spacing: 12) {
                Button { writing = true } label: {
                    Label("Toward my book", systemImage: "arrow.right.to.line")
                        .font(.system(size: 16, weight: .semibold, design: .rounded))
                        .padding(.horizontal, 18).padding(.vertical, 10)
                        .background(Capsule().fill(Theme.accent)).foregroundStyle(.white)
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("toward-my-book")
                Text(d.passageWindow(avoiding: Set(goal.usedSentences ?? [])) == nil
                     ? "Start with its words in short supported sentences."
                     : "A supported passage from the book, then practice with its new words.")
                    .font(.system(size: 13)).foregroundStyle(Theme.muted).fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(18)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
        .accessibilityIdentifier("goal-panel")
        .task(id: "\(goal.id)-\(goal.lemmaCache?.count ?? 0)") {
            guard filling == nil, !a.tokensNeedingLemmas.isEmpty else { return }
            do {
                let entries = try await GoalLemmaTable.fill(goal, analysis: a) { done, total in filling = (done, total) }
                filling = nil
                book.mergeLemmas(goalID: goal.id, entries)  // into the goal as it is now, not the snapshot
            } catch { filling = nil }
        }
    }

    @ViewBuilder private func firstPage(_ goal: Goal, _ d: GoalDistance) -> some View {
        if let opened = goal.firstPageOpened {
            HStack(spacing: 10) {
                Image(systemName: "seal.fill").foregroundStyle(Theme.redPen)
                Text("You opened its first page on \(PageView.russianDate(opened)).").font(.system(size: 15)).foregroundStyle(Theme.text)
            }
        } else if d.firstPageReady {
            Button { book.openFirstPageIfReady() } label: {
                Label("Try the first page · 98%+ form coverage", systemImage: "book.pages")
                    .font(.system(size: 15, weight: .semibold)).foregroundStyle(Theme.redPen)
            }
            .buttonStyle(.plain)
        } else {
            ProgressView(value: min(1, d.firstPageCoverage / Goal.firstPageThreshold)) {
                Text("First page: \(Int((d.firstPageCoverage * 100).rounded()))% of its least-covered sentence practiced (opens at \(Int(Goal.firstPageThreshold * 100))%)")
                    .font(.system(size: 13)).foregroundStyle(Theme.muted)
            }
            .tint(Theme.redPen)
        }
    }

    private func stat(_ value: String, _ label: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(value).font(.system(size: 30, weight: .bold, design: .serif)).foregroundStyle(Theme.text).monospacedDigit()
            Text(label).font(Theme.label(12)).foregroundStyle(Theme.muted).fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: 220, alignment: .leading)
    }
}

/// Choosing the book: a library text, pasted (or scanned) text, or a link to a page.
struct GoalPicker: View {
    enum Source: Hashable { case library, paste, link }
    @Environment(Book.self) private var book
    @Environment(\.dismiss) private var dismiss
    @State private var source: Source = .library
    @State private var library = Library.load()
    @State private var text = ""
    @State private var title = ""
    @State private var link = ""
    @State private var fetching = false
    @State private var error: String?

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                Picker("Source", selection: $source) {
                    Text("Library").tag(Source.library)
                    Text("Paste").tag(Source.paste)
                    Text("Link").tag(Source.link)
                }
                .pickerStyle(.segmented).padding(16)
                switch source {
                case .library: libraryList
                case .paste: pasteForm
                case .link: linkForm
                }
            }
            .background(Theme.paper.ignoresSafeArea())
            .navigationTitle("Your book")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } } }
        }
    }

    private var libraryList: some View {
        List {
            ForEach(Library.levels, id: \.self) { level in
                let texts = library.texts.filter { $0.level == level }
                if !texts.isEmpty {
                    Section(level) {
                        ForEach(texts) { t in
                            Button {
                                book.addGoal(Goal(library: t, created: book.now()))
                                dismiss()
                            } label: {
                                HStack {
                                    VStack(alignment: .leading, spacing: 3) {
                                        Text(t.title).font(Theme.russian(19, weight: .medium)).foregroundStyle(Theme.text)
                                        Text([t.author, t.title_en].compactMap { $0 }.joined(separator: " · "))
                                            .font(.system(size: 14)).foregroundStyle(.secondary)
                                    }
                                    Spacer()
                                    ReadabilityBadge(fraction: book.knowledge.readability(of: t.text))
                                }
                            }
                            .accessibilityIdentifier("goal-\(t.id)")
                        }
                    }
                }
            }
        }
        .scrollContentBackground(.hidden)
    }

    private var pasteForm: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                Text("Paste the text you want to be able to read — a chapter, a page of a textbook, a proof. On iPad, tap the field and choose Scan Text to read it off paper.")
                    .font(.system(size: 16)).foregroundStyle(Theme.text)
                TextField("Title (optional)", text: $title).textFieldStyle(.roundedBorder).font(.system(size: 17))
                TextEditor(text: $text)
                    .font(Theme.russian(19))
                    .scrollContentBackground(.hidden)
                    .padding(10).frame(minHeight: 260)
                    .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
                    .overlay(alignment: .topLeading) {
                        if text.isEmpty { Text("Вста́вьте текст…").font(Theme.russian(19)).foregroundStyle(Theme.muted).padding(18).allowsHitTesting(false) }
                    }
                makeButton(enabled: RussianText.letters(in: text).count >= 5) {
                    Goal(title: title.isEmpty ? String(text.prefix(40)).trimmingCharacters(in: .whitespacesAndNewlines) : title,
                         origin: .pasted, text: text, created: book.now())
                }
            }
            .padding(20)
        }
    }

    private var linkForm: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                Text("A link to a page on ru.wikisource.org (or any page with Russian paragraphs). The text is fetched and cleaned.")
                    .font(.system(size: 16)).foregroundStyle(Theme.text)
                TextField("https://ru.wikisource.org/wiki/…", text: $link)
                    .textFieldStyle(.roundedBorder).font(.system(size: 16))
                    .keyboardType(.URL).textInputAutocapitalization(.never).autocorrectionDisabled()
                if fetching { ProgressView("Fetching…") }
                if let error { Text(error).font(.system(size: 15)).foregroundStyle(Theme.redPen) }
                Button {
                    Task { await fetch() }
                } label: {
                    Text("Fetch and make it my book")
                        .font(.system(size: 18, weight: .semibold, design: .rounded))
                        .frame(maxWidth: .infinity).padding(.vertical, 14)
                        .background(Capsule().fill(URL(string: link)?.host != nil && !fetching ? Theme.accent : Theme.muted.opacity(0.3)))
                        .foregroundStyle(.white)
                }
                .buttonStyle(.plain)
                .disabled(URL(string: link)?.host == nil || fetching)
            }
            .padding(20)
        }
    }

    private func makeButton(enabled: Bool, _ make: @escaping () -> Goal) -> some View {
        Button {
            book.addGoal(make())
            dismiss()
        } label: {
            Text("Make it my book")
                .font(.system(size: 18, weight: .semibold, design: .rounded))
                .frame(maxWidth: .infinity).padding(.vertical, 14)
                .background(Capsule().fill(enabled ? Theme.accent : Theme.muted.opacity(0.3)))
                .foregroundStyle(.white)
        }
        .buttonStyle(.plain)
        .disabled(!enabled)
    }

    private func fetch() async {
        guard let url = URL(string: link.trimmingCharacters(in: .whitespacesAndNewlines)) else { return }
        fetching = true
        error = nil
        defer { fetching = false }
        do {
            let f = try await Wikisource.fetch(url)
            book.addGoal(Goal(title: f.title, author: f.author, origin: .url, source: url.absoluteString, text: f.text, created: book.now()))
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
    }
}

/// The quiet red stamp on the page that opens the first page of the learner's book.
struct FirstPageStamp: View {
    var body: some View {
        Text("Пе́рвая страни́ца")
            .font(Theme.cursive(30))
            .foregroundStyle(Theme.redPen)
            .padding(.horizontal, 12).padding(.vertical, 4)
            .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(Theme.redPen, lineWidth: 2))
            .rotationEffect(.degrees(-6))
            .padding(.vertical, 8)
            .accessibilityLabel("Stamp: first page of your book")
    }
}

/// The ＋ sheet's content for a page toward the book: what the page will be made of.
struct GoalPullHeader: View {
    @Environment(Book.self) private var book
    let goal: Goal

    var body: some View {
        let d = book.distance(for: goal)
        let window = d.passageWindow(avoiding: Set(goal.usedSentences ?? []))
        VStack(alignment: .leading, spacing: 10) {
            Text(goal.title).font(Theme.russian(28, weight: .semibold)).foregroundStyle(Theme.text)
            if let author = goal.author { Text(author).font(.system(size: 15)).foregroundStyle(Theme.muted) }
            Text("\(Int((d.coverage * 100).rounded()))% of its forms practiced · \(d.readableSentences) of \(d.analysis.sentences.count) sentences at 95%+ coverage")
                .font(.system(size: 15)).foregroundStyle(Theme.muted)
            if let window {
                Text(d.analysis.sentenceText(window))
                    .font(Theme.russian(19)).foregroundStyle(Theme.text.opacity(0.85))
                    .padding(14).frame(maxWidth: .infinity, alignment: .leading)
                    .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card))
                Text("This stretch has at least 95% form coverage. Try it with support, then practice its new words.")
                    .font(.system(size: 15)).foregroundStyle(Theme.text)
            } else {
                Text("No stretch of the text reads easily yet. The page teaches its most valuable words through the sentences they come from — nothing on it is invented.")
                    .font(.system(size: 15)).foregroundStyle(Theme.text)
            }
            let words = d.wordsToTeach(in: window, limit: 6)
            if !words.isEmpty {
                FlowRow(spacing: 8) {
                    ForEach(words, id: \.rung.lemma) { w in
                        Text(w.rung.gloss.map { "\(w.rung.example.lowercased()) · \($0)" } ?? w.rung.example.lowercased())
                            .font(Theme.russian(15)).foregroundStyle(Theme.text)
                            .padding(.horizontal, 10).padding(.vertical, 4)
                            .background(Capsule().fill(Theme.card))
                    }
                }
            }
        }
    }
}
