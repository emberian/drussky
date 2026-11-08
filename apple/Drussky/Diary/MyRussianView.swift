import SwiftUI

/// Inside the cover: how much of your own Russian you have written, what the red pen keeps
/// finding this week, and the way into the whole corpus.
struct MyRussianSection: View {
    @Environment(Book.self) private var book
    @State private var open = false

    var body: some View {
        let entries = book.diaryEntries
        VStack(alignment: .leading, spacing: 10) {
            Text("My Russian").font(.system(size: 22, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
            if entries.isEmpty {
                Text("Every diary line you write, with its corrections, collects here. The ＋ menu has “Write in my diary”.")
                    .font(.system(size: 15)).foregroundStyle(Theme.muted)
            } else {
                Button { open = true } label: {
                    HStack(alignment: .top, spacing: 16) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("\(entries.count) \(entries.count == 1 ? "entry" : "entries") · \(Self.words(entries)) words of your own")
                                .font(.system(size: 16, weight: .semibold)).foregroundStyle(Theme.text)
                            Text(entries[0].entry.correction?.corrected ?? entries[0].entry.text)
                                .font(Theme.cursive(22)).foregroundStyle(Theme.ink).lineLimit(1)
                            weekly
                        }
                        Spacer()
                        Image(systemName: "chevron.right").foregroundStyle(Theme.muted)
                    }
                    .padding(14)
                    .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card))
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityLabel("My Russian, \(entries.count) entries")
            }
        }
        .navigationDestination(isPresented: $open) { MyRussianView() }
        .onAppear { if QA.has("-qa-myrussian") { open = true } }
    }

    /// This week's red-pen categories: what keeps coming back in their own writing.
    @ViewBuilder private var weekly: some View {
        let counts = book.diaryCategoryCounts(days: 7)
        if counts.isEmpty {
            Text("Nothing marked this week.").font(.system(size: 13)).foregroundStyle(Theme.muted)
        } else {
            HStack(spacing: 8) {
                Text("This week:").font(.system(size: 13)).foregroundStyle(Theme.muted)
                ForEach(counts.prefix(3), id: \.0) { c, n in
                    Text("\(c.title) ×\(n)").font(Theme.label(12)).foregroundStyle(Theme.redPen)
                        .padding(.horizontal, 8).padding(.vertical, 3)
                        .background(Capsule().fill(Theme.redPen.opacity(0.1)))
                }
            }
        }
    }

    static func words(_ entries: [DiaryRecord]) -> Int {
        entries.reduce(0) { $0 + RussianText.key($1.entry.text).split(separator: " ").count }
    }
}

/// Every diary line ever written, newest first, each with its red pen: a growing corpus of the
/// learner's own Russian. Tap an entry to turn to its page; export the lot as Markdown.
struct MyRussianView: View {
    @Environment(Book.self) private var book
    @Environment(\.dismiss) private var dismiss
    @State private var exportURL: URL?

    var body: some View {
        let entries = book.diaryEntries
        ScrollView {
            VStack(alignment: .leading, spacing: 22) {
                summary(entries)
                ForEach(entries) { d in
                    VStack(alignment: .leading, spacing: 10) {
                        HStack(alignment: .firstTextBaseline) {
                            Text(PageView.russianDate(d.entry.submitted)).font(Theme.cursive(24)).foregroundStyle(Theme.ink)
                            Text(d.entry.prompt).font(.system(size: 14)).italic().foregroundStyle(Theme.muted).lineLimit(2)
                            Spacer()
                            Button {
                                if let i = book.index(ofPage: d.ref.pageID) { book.goTo(i) }
                                dismiss()
                            } label: {
                                Text("стр. \(d.pageNumber)").font(Theme.label(12)).foregroundStyle(Theme.accent)
                            }
                            .buttonStyle(.plain)
                            .accessibilityLabel("Go to page \(d.pageNumber)")
                        }
                        if let c = d.entry.correction {
                            RedPen(entry: d.entry, correction: c, compact: true)
                        } else {
                            Text(d.entry.text).russianFont(22).foregroundStyle(Theme.text)
                            Text("Uncorrected.").font(.system(size: 13)).foregroundStyle(Theme.muted)
                        }
                    }
                    .padding(16)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
                }
            }
            .padding(24)
            .frame(maxWidth: 900)
            .frame(maxWidth: .infinity)
        }
        .background(GridPaper(marginX: nil))
        .navigationTitle("My Russian")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                if let exportURL {
                    ShareLink(item: exportURL) { Label("Export", systemImage: "square.and.arrow.up") }
                }
            }
        }
        .task { exportURL = book.exportDiary() }
    }

    private func summary(_ entries: [DiaryRecord]) -> some View {
        let edits = entries.reduce(0) { $0 + ($1.entry.correction?.edits.count ?? 0) }
        let clean = entries.filter { $0.entry.correction?.edits.isEmpty == true }.count
        return HStack(spacing: 28) {
            stat("\(entries.count)", entries.count == 1 ? "entry" : "entries")
            stat("\(MyRussianSection.words(entries))", "words of yours")
            stat("\(clean)", "lines with no marks")
            stat("\(edits)", "corrections")
        }
    }

    private func stat(_ value: String, _ label: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(value).font(.system(size: 30, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
            Text(label).font(Theme.label(12)).foregroundStyle(Theme.muted)
        }
    }
}
