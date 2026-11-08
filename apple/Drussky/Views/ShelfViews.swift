import SwiftUI

/// The shelves in the app (Model/Shelves.swift): a page footer's offer, the serial and the shelf
/// as lists the Library and the inside cover open, and one row for each inside the cover.

/// One calm card at the foot of a page: an episode or reading the course has reached. Written
/// onto the page when it was composed; gone from view once opened.
struct ShelfOfferCard: View {
    @Environment(Book.self) private var book
    let pageIndex: Int

    var body: some View {
        let items = (book.pages[safe: pageIndex]?.offers ?? []).compactMap(Shelves.item(sourceID:)).filter { !book.hasOpened($0) }
        if !items.isEmpty {
            VStack(alignment: .leading, spacing: 12) {
                ForEach(items) { item in
                    HStack(alignment: .firstTextBaseline, spacing: 14) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(Theme.stressed(item.place, show: book.settings.showStress)).font(Theme.label(12)).tracking(1.2).foregroundStyle(Theme.accent)
                            HStack(spacing: 8) {
                                Text(Theme.stressed("«\(item.title)»", show: book.settings.showStress)).russianFont(22, weight: .medium).foregroundStyle(Theme.text)
                                if let en = item.titleEn { Text(en).scaledFont(14).foregroundStyle(Theme.muted) }
                            }
                            Text(item.kind == .listening ? "You can listen to this now." : "You can read this now.")
                                .scaledFont(14).foregroundStyle(Theme.muted)
                        }
                        Spacer(minLength: 8)
                        PillButton(title: item.kind == .listening ? "Listen" : "Read it", systemImage: item.kind == .listening ? "ear" : "book") { book.open(item) }
                    }
                }
            }
            .padding(16)
            .frame(maxWidth: 640, alignment: .leading)
            .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
            .accessibilityElement(children: .contain)
        }
    }
}

/// «Ста́нция», episode by episode: each ready after its course page, every one openable now.
struct SerialView: View {
    @Environment(Book.self) private var book
    let serial: Serial
    var onDone: () -> Void

    var body: some View {
        List {
            Section {
                VStack(alignment: .leading, spacing: 6) {
                    if let en = serial.title_en { Text(en).font(.system(size: 15, weight: .semibold)).foregroundStyle(Theme.text) }
                    if let s = serial.summary_en { Text(s).font(.system(size: 15)).foregroundStyle(Theme.muted) }
                    Text("Each \(serial.itemWordEn) is ready after one page of the course; the next in line is offered at the foot of a page when you get there. Read ahead whenever you like — the glosses carry you.")
                        .font(.system(size: 13)).foregroundStyle(Theme.muted)
                }
                .listRowBackground(Color.clear)
            }
            Section {
                ForEach(serial.episodes) { ep in
                    ShelfItemRow(item: Shelves.item(ep, in: serial), number: ep.number, onDone: onDone)
                }
            }
        }
        .navigationTitle(RussianText.unstressed(serial.title))
        .navigationBarTitleDisplayMode(.inline)
    }
}

/// A shelf of readings in rungs («Матема́тика»).
struct ShelfView: View {
    @Environment(Book.self) private var book
    let shelf: Library.Collection
    var onDone: () -> Void

    var body: some View {
        let texts = Shelves.library.texts(onShelf: shelf.id)
        List {
            Section {
                VStack(alignment: .leading, spacing: 6) {
                    if let en = shelf.title_en { Text(en).font(.system(size: 15, weight: .semibold)).foregroundStyle(Theme.text) }
                    if let d = shelf.description_en { Text(d).font(.system(size: 15)).foregroundStyle(Theme.muted) }
                }
                .listRowBackground(Color.clear)
            }
            Section {
                ForEach(texts) { t in
                    ShelfItemRow(item: Shelves.item(t, on: shelf), number: t.rung ?? 0, summary: t.summary_en, level: t.level, onDone: onDone)
                }
            }
        }
        .navigationTitle(RussianText.unstressed(shelf.title))
        .navigationBarTitleDisplayMode(.inline)
    }
}

/// One episode or reading: its number and title, whether the course has reached it (and after
/// which page if not), and a tick once opened. Tapping opens it as a page, ready or not.
struct ShelfItemRow: View {
    @Environment(Book.self) private var book
    @Environment(\.dismiss) private var dismiss
    let item: Shelves.Item
    let number: Int
    var summary: String? = nil
    var level: String? = nil
    var onDone: () -> Void

    var body: some View {
        let opened = book.hasOpened(item)
        let ready = book.isReady(item)
        Button {
            if !opened { book.open(item) }
            else if let i = book.pages.firstIndex(where: { $0.sourceID == item.sourceID }) { book.goTo(i) }
            onDone()
        } label: {
            HStack(alignment: .firstTextBaseline, spacing: 14) {
                Text("\(number)").font(.system(size: 15, weight: .bold, design: .rounded)).monospacedDigit()
                    .foregroundStyle(Theme.muted).frame(width: 28, alignment: .trailing)
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 8) {
                        Text(Theme.stressed(item.title, show: book.settings.showStress)).font(Theme.russian(20, weight: .medium))
                            .foregroundStyle(ready || opened ? Theme.text : Theme.text.opacity(0.6))
                        if let en = item.titleEn { Text(en).font(.system(size: 14)).foregroundStyle(.secondary) }
                    }
                    if let summary { Text(summary).font(.system(size: 13)).foregroundStyle(.secondary).lineLimit(2) }
                    Text(status(opened: opened, ready: ready)).font(.system(size: 13)).foregroundStyle(.secondary)
                }
                Spacer()
                if let level { Text(level).font(Theme.label(12)).foregroundStyle(Theme.muted) }
                if opened {
                    Image(systemName: "checkmark").foregroundStyle(Theme.redPen).accessibilityLabel("Opened")
                } else if ready {
                    Image(systemName: "book").foregroundStyle(Theme.accent).accessibilityLabel("Ready")
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier(item.sourceID)
    }

    private func status(opened: Bool, ready: Bool) -> String {
        if opened { return "In your notebook" }
        if ready { return item.kind == .listening ? "Ready to hear" : "Ready to read" }
        let ahead = item.kind == .listening ? "listen ahead" : "read ahead"
        if let name = book.pageName(item.afterPage) { return "After \(name) — or \(ahead)" }
        return ahead.prefix(1).uppercased() + ahead.dropFirst()
    }
}

/// «На слух»: the listening sets in order, each ready after its course page, every one openable now.
struct ListeningShelfView: View {
    @Environment(Book.self) private var book
    var onDone: () -> Void

    var body: some View {
        List {
            Section {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Russian by ear").font(.system(size: 15, weight: .semibold)).foregroundStyle(Theme.text)
                    Text("Short things said once — a request, two people at the table, a voice message, someone correcting themselves. You answer what it meant; the words appear only afterwards. Each set is ready after one page of the course and is offered at the foot of a page when you get there.")
                        .font(.system(size: 13)).foregroundStyle(Theme.muted)
                }
                .listRowBackground(Color.clear)
            }
            Section {
                ForEach(Array(Shelves.listeningItems.enumerated()), id: \.element.id) { i, item in
                    ShelfItemRow(item: item, number: i + 1, summary: Shelves.listening.sets[safe: i].map(Self.summary), onDone: onDone)
                }
            }
        }
        .navigationTitle("На слух")
        .navigationBarTitleDisplayMode(.inline)
    }

    /// "8 cards · which thing, who needs what, follow an instruction"
    static func summary(_ set: ListeningSet) -> String {
        let names: [ListeningItem.Kind: String] = [.pickObject: "which thing", .whoNeeds: "who needs what", .follow: "follow an instruction",
                                                   .questionOrStatement: "asking or telling", .message: "voice messages", .correction: "self-corrections",
                                                   .disagreement: "who wants what"]
        var seen = Set<ListeningItem.Kind>()
        let kinds = set.items.map(\.kind).filter { seen.insert($0).inserted }.compactMap { names[$0] }
        return "\(set.items.count) cards · \(kinds.joined(separator: ", "))"
    }
}

/// Inside the cover: one row per shelf — how far along it is and what is ready — opening the list.
struct ShelvesSection: View {
    @Environment(Book.self) private var book
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Shelves").font(.system(size: 22, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
            ForEach(Shelves.serials) { serial in
                let items = serial.episodes.map { Shelves.item($0, in: serial) }
                NavigationLink {
                    SerialView(serial: serial, onDone: { dismiss() })
                } label: {
                    row(title: serial.title, titleEn: serial.title_en, items: items, unit: serial.itemWordEn)
                }
                .buttonStyle(.plain)
            }
            ForEach(Shelves.library.shelves ?? []) { shelf in
                let items = Shelves.library.texts(onShelf: shelf.id).map { Shelves.item($0, on: shelf) }
                NavigationLink {
                    ShelfView(shelf: shelf, onDone: { dismiss() })
                } label: {
                    row(title: shelf.title, titleEn: shelf.title_en, items: items, unit: "reading")
                }
                .buttonStyle(.plain)
            }
            if !Shelves.listeningItems.isEmpty {
                NavigationLink {
                    ListeningShelfView(onDone: { dismiss() })
                } label: {
                    row(title: "На слух", titleEn: "Listening", items: Shelves.listeningItems, unit: "set")
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("shelf-listening")
            }
        }
    }

    private func row(title: String, titleEn: String?, items: [Shelves.Item], unit: String) -> some View {
        let opened = items.filter { book.hasOpened($0) }.count
        let next = items.first { !book.hasOpened($0) }
        return HStack(spacing: 14) {
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 8) {
                    Text(Theme.stressed(title, show: book.settings.showStress)).font(Theme.russian(20, weight: .medium)).foregroundStyle(Theme.text)
                    if let titleEn { Text(titleEn).font(.system(size: 14)).foregroundStyle(Theme.muted) }
                }
                Text(line(opened: opened, total: items.count, next: next, unit: unit)).font(.system(size: 13)).foregroundStyle(Theme.muted)
            }
            Spacer()
            Image(systemName: "chevron.right").font(.system(size: 13, weight: .semibold)).foregroundStyle(Theme.muted)
        }
        .padding(.vertical, 10).padding(.horizontal, 12)
        .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card))
        .contentShape(Rectangle())
    }

    private func line(opened: Int, total: Int, next: Shelves.Item?, unit: String) -> String {
        var s = opened == 0 ? "\(total) \(unit)s" : "\(opened) of \(total) \(unit)s in your notebook"
        if let next {
            if book.isReady(next) { s += " · \(RussianText.unstressed(next.place)) is ready" }
            else if let name = book.pageName(next.afterPage) { s += " · next after \(name)" }
        }
        return s
    }
}
