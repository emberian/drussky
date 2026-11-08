import PencilKit
import SwiftUI

/// The inside cover: the alphabet in the learner's own handwriting, and the pages so far.
struct ContentsView: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 28) {
                    stats
                    DeskNotes()
                    KnowledgeSection()
                    alphabet
                    if !book.customWords.isEmpty { keptWords }
                    pages
                }
                .padding(24)
                .frame(maxWidth: 900)
                .frame(maxWidth: .infinity)
            }
            .background(GridPaper(marginX: nil))
            .navigationTitle("Алфавит")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } }
            }
        }
    }

    private var lettersMet: Int { book.course.letters.filter { book.letterKnown($0.upper) }.count }
    private var wordsMet: Int { book.targets.keys.filter { $0.hasPrefix("word:") }.count }

    private var stats: some View {
        HStack(spacing: 28) {
            stat("\(lettersMet)", "of 33 letters")
            stat("\(wordsMet)", "words")
            stat("\(book.pages.filter(\.isFinished).count)", "pages done")
            stat("\(book.dueCount)", "ready to review")
        }
    }

    private func stat(_ value: String, _ label: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(value).font(.system(size: 30, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
            Text(label).font(Theme.label(12)).foregroundStyle(Theme.muted)
        }
    }

    private var alphabet: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Your alphabet").font(.system(size: 22, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
            Text("Each letter fills in with your own handwriting when you meet it. Tap to hear.")
                .font(.system(size: 15)).foregroundStyle(Theme.muted)
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 92), spacing: 10)], spacing: 10) {
                ForEach(book.course.letters) { letter in
                    LetterCell(letter: letter, known: book.letterKnown(letter.upper), ink: book.inkForLetter(letter.upper))
                        .onTapGesture { voice.play(letter.say) }
                }
            }
        }
    }

    private var keptWords: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Words you kept").font(.system(size: 22, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
            FlowRow(spacing: 10) {
                ForEach(book.customWords, id: \.self) { w in
                    HStack(spacing: 8) {
                        HearButton(text: w.ru, size: 30)
                        Text(Theme.stressed(w.ru, show: book.settings.showStress)).font(Theme.russian(20, weight: .medium))
                        if !w.en.isEmpty { Text(w.en).font(.system(size: 14)).foregroundStyle(Theme.muted) }
                    }
                    .padding(.vertical, 6).padding(.horizontal, 10)
                    .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card))
                }
            }
        }
    }

    private var pages: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Pages").font(.system(size: 22, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
                .padding(.bottom, 6)
            ForEach(book.pages.indices.reversed(), id: \.self) { i in
                let page = book.pages[i]
                Button {
                    book.goTo(i)
                    dismiss()
                } label: {
                    HStack(spacing: 14) {
                        Text("\(page.number)").font(.system(size: 15, weight: .bold, design: .rounded))
                            .monospacedDigit().foregroundStyle(Theme.muted).frame(width: 34, alignment: .trailing)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(page.title).font(.system(size: 17, weight: .semibold)).foregroundStyle(Theme.text)
                            Text(page.unitTitle).font(.system(size: 13)).foregroundStyle(Theme.muted)
                        }
                        Spacer()
                        if page.isFinished {
                            Text("\(GradeStamp.grade(for: page))").font(Theme.cursive(30)).foregroundStyle(Theme.redPen)
                        } else {
                            Text("\(page.completedCount)/\(page.exercises.count)")
                                .font(.system(size: 14, design: .rounded)).foregroundStyle(Theme.muted)
                        }
                        if i == book.currentPage {
                            Image(systemName: "bookmark.fill").foregroundStyle(Theme.redPen)
                        }
                    }
                    .padding(.vertical, 10).padding(.horizontal, 12)
                    .background(RoundedRectangle(cornerRadius: 12).fill(i == book.currentPage ? Theme.card : .clear))
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        }
    }
}

struct LetterCell: View {
    let letter: Course.Letter
    let known: Bool
    let ink: URL?
    @State private var image: UIImage?

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 12).fill(known ? Theme.card : Theme.card.opacity(0.35))
            if let image {
                Image(uiImage: image).resizable().scaledToFit().padding(10)
            } else {
                Text("\(letter.upper)\(letter.lower)")
                    .font(Theme.russian(34))
                    .foregroundStyle(known ? Theme.text : Theme.muted.opacity(0.35))
            }
        }
        .frame(height: 84)
        .overlay(alignment: .bottomTrailing) {
            if image != nil {
                Text(letter.upper).font(.system(size: 11, weight: .bold, design: .serif))
                    .foregroundStyle(Theme.muted).padding(6)
            }
        }
        .task(id: ink) { image = Self.render(ink) }
        .accessibilityLabel("Letter \(letter.upper)")
    }

    @MainActor
    static func render(_ url: URL?) -> UIImage? {
        guard let url, let data = try? Data(contentsOf: url), let drawing = try? PKDrawing(data: data),
              !drawing.strokes.isEmpty else { return nil }
        return drawing.image(from: drawing.bounds.insetBy(dx: -8, dy: -8), scale: 2)
    }
}

struct SettingsView: View {
    @Environment(Book.self) private var book
    @Environment(\.dismiss) private var dismiss
    @State private var exportURL: URL?
    @State private var graphURL: URL?
    @State private var restoreError: String?
    @State private var apiKey = Keychain.apiKey ?? ""

    var body: some View {
        @Bindable var book = book
        NavigationStack {
            Form {
                Section("Writing") {
                    Toggle("Draw with a finger too", isOn: $book.settings.fingerDrawing)
                    Toggle("Handwritten (cursive) models", isOn: $book.settings.cursiveModels)
                }
                DaySettingsSection()
                Section {
                    Toggle("Show stress", isOn: $book.settings.showStress)
                } footer: {
                    Text("The stressed vowel is underlined in blue. Russian books leave stress unmarked; learners' books mark it.")
                }
                Section {
                    SecureField("Anthropic API key", text: $apiKey)
                        .textContentType(.password)
                        .autocorrectionDisabled()
                        .textInputAutocapitalization(.never)
                        .onChange(of: apiKey) { _, key in Keychain.apiKey = key.trimmingCharacters(in: .whitespacesAndNewlines) }
                } header: {
                    Text("Tutor")
                } footer: {
                    Text("The ? beside each exercise asks Claude (\(Tutor.model)) about it. The key stays in this device's Keychain; questions and answers are written onto the page.")
                }
                Section {
                    if let exportURL {
                        ShareLink(item: exportURL) { Label("Export my notebook", systemImage: "square.and.arrow.up") }
                    } else {
                        Label("Preparing export…", systemImage: "hourglass").foregroundStyle(.secondary)
                    }
                    RestoreButton(error: $restoreError)
                    if let restoreError { Text(restoreError).font(.footnote).foregroundStyle(Theme.redPen) }
                } footer: {
                    Text("Your pages, handwriting, recordings and answer log, as one zip. Restoring checks an export first and keeps the current notebook beside it.")
                }
                Section {
                    if let graphURL {
                        ShareLink(item: graphURL) { Label("Export knowledge graph", systemImage: "point.3.connected.trianglepath.dotted") }
                    }
                } footer: {
                    Text("Every letter, word and pattern with its recall per modality, how they connect, and what you confuse, as JSON. The full answer log is events.jsonl in the notebook export.")
                }
                Section {
                    NavigationLink("Recordings and credits") { CreditsView() }
                }
            }
            .navigationTitle("Settings")
            .task { exportURL = await book.exportArchive(); graphURL = book.exportGraph() }
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        }
    }
}

/// Attribution for the recorded Russian: human recordings (Wikimedia Commons) and the synthetic voice.
struct CreditsView: View {
    struct Source: Decodable {
        var key: String
        var kind: String
        var credit: String?
        var license: String?
        var source_page: String?
        var provider: String?
    }

    private var sources: [Source] {
        guard let url = Bundle.main.url(forResource: "Audio", withExtension: nil)?.appendingPathComponent("sources.json"),
              let data = try? Data(contentsOf: url),
              let map = try? JSONDecoder().decode([String: Source].self, from: data) else { return [] }
        return map.values.sorted { $0.key < $1.key }
    }

    var body: some View {
        let all = sources
        let human = all.filter { $0.kind == "human_recording" }
        let providers = Set(all.compactMap(\.provider)).sorted()
        List {
            Section {
                ForEach(human, id: \.key) { s in
                    VStack(alignment: .leading, spacing: 3) {
                        Text(s.key).font(Theme.russian(20, weight: .medium))
                        Text([s.credit, s.license].compactMap { $0 }.joined(separator: " · "))
                            .font(.system(size: 13)).foregroundStyle(.secondary)
                        if let page = s.source_page, let url = URL(string: page) {
                            Link(page, destination: url).font(.system(size: 12)).lineLimit(1)
                        }
                    }
                }
            } header: { Text("Recorded by people (\(human.count))") }
            Section {
                ForEach(providers, id: \.self) { Text($0) }
                Text("\(all.count - human.count) phrases").foregroundStyle(.secondary)
            } header: { Text("Synthetic voice") }
            Section {
                Text("Marck Script by Denis Masharov, SIL Open Font License 1.1")
            } header: { Text("Handwriting font") }
        }
        .navigationTitle("Credits")
    }
}
