import SwiftUI

/// «Исто́чники · Sources»: everyone whose recordings, sentences, texts, fonts, reference
/// material and models the workbook bundles, from credits.json (Model/Credits.swift). Every
/// entry shows its licence and a tappable source; a gap the generator found is said plainly.
struct CreditsView: View {
    @State private var credits: Credits?
    @State private var failed = false

    var body: some View {
        Group {
            if let credits {
                CreditsList(credits: credits)
            } else if failed {
                Text("credits.json is missing from this build. Regenerate it with apple/scripts/build_credits.py.")
                    .foregroundStyle(Theme.redPen).padding(24)
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .background(GridPaper(marginX: nil))
        .navigationTitle("Исто́чники · Sources")
        .task {
            // credits.json is large (one entry per recording); decode it off the main actor.
            let loaded = await Task.detached(priority: .userInitiated) { try? Credits.load() }.value
            if let loaded { credits = loaded } else { failed = true }
        }
    }
}

private struct CreditsList: View {
    let credits: Credits

    var body: some View {
        List {
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Everything in this workbook that somebody else made, with its licence and where it came from. Generated from the provenance records on \(credits.audit_date). A record, not a legal clearance.")
                    OpenItemsSummary(unresolved: credits.unresolved)
                }
                .font(.system(size: 15)).foregroundStyle(Theme.text)
            }
            .listRowBackground(Theme.card)

            RecordingsSection(entries: credits.group("human_audio"))
            NarrationsSection(entries: credits.group("narration"))
            SentencesSection(entries: credits.group("tatoeba_text"))
            SyntheticSection(entries: credits.group("synthetic_audio"))
            ModelsSection(entries: credits.group("models_and_algorithms"))
            FontsSection(entries: credits.group("fonts"))
            ReferenceSection(entries: credits.group("reference_material"))
            Section {
                Text(credits.model_authored_content).font(.system(size: 15)).foregroundStyle(Theme.text)
            } header: { SectionTitle("Written with language models") }
            .listRowBackground(Theme.card)
            OpenItemsSection(unresolved: credits.unresolved)
        }
        .listStyle(.insetGrouped)
        .scrollContentBackground(.hidden)
    }
}

// MARK: - Sections

/// Human recordings of words and sentences, one row per contributor, their clips behind a disclosure.
private struct RecordingsSection: View {
    let entries: [Credits.Entry]

    private struct Contributor: Identifiable {
        var id: String { name }
        let name: String
        let entries: [Credits.Entry]
        var clipCount: Int { entries.reduce(0) { $0 + ($1.assets?.count ?? 0) } }
        var licences: [String] { Array(Set(entries.compactMap(\.licenceName))).sorted() }
        var incomplete: Bool { entries.contains { !$0.isReady } }
    }

    private var contributors: [Contributor] {
        Dictionary(grouping: entries) { $0.author ?? "Contributor not recorded" }
            .map { Contributor(name: $0.key, entries: $0.value.sorted { $0.title < $1.title }) }
            .sorted { ($1.clipCount, $0.name) < ($0.clipCount, $1.name) }
    }

    var body: some View {
        let contributors = contributors
        Section {
            ForEach(contributors) { c in
                DisclosureGroup {
                    ForEach(c.entries) { e in ClipRow(entry: e) }
                } label: {
                    VStack(alignment: .leading, spacing: 3) {
                        HStack(alignment: .firstTextBaseline, spacing: 8) {
                            Text(c.name).font(.system(size: 16, weight: .semibold)).foregroundStyle(Theme.text)
                            if c.incomplete { Flag("contributor not yet recorded") }
                        }
                        Text("\(c.clipCount) recording\(c.clipCount == 1 ? "" : "s") · \(c.licences.joined(separator: ", "))")
                            .font(.system(size: 13)).foregroundStyle(Theme.muted)
                    }
                }
            }
        } header: {
            SectionTitle("Recorded by people", detail: "\(contributors.count) contributors, \(contributors.reduce(0) { $0 + $1.clipCount }) recordings. Each clip was trimmed, levelled and re-encoded by Drussky; share-alike licences apply to those adapted clips.")
        }
        .listRowBackground(Theme.card)
    }
}

/// One credited recording: what is said, its licence, the page it came from.
private struct ClipRow: View {
    let entry: Credits.Entry

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(entry.title).font(Theme.russian(17)).foregroundStyle(Theme.text)
            HStack(spacing: 6) {
                LicenceLabel(entry: entry)
                if let n = entry.assets?.count, n > 1 { Text("· \(n) clips").font(.system(size: 13)).foregroundStyle(Theme.muted) }
            }
            if let url = entry.source_url { SourceLink(url) }
            if !entry.isReady { Flag(entry.author == nil || entry.author == "Contributor not recorded" ? "contributor not yet recorded" : "credit incomplete: \(entry.credit_status)") }
        }
        .padding(.vertical, 2)
    }
}

/// The human readings of library texts: who read, under what terms, what was cut.
private struct NarrationsSection: View {
    let entries: [Credits.Entry]

    var body: some View {
        Section {
            ForEach(entries) { e in
                VStack(alignment: .leading, spacing: 4) {
                    Text(e.title).font(Theme.russian(17)).foregroundStyle(Theme.text)
                    Text("Read by \(e.author ?? "a reader not yet recorded")").font(.system(size: 14)).foregroundStyle(Theme.text)
                    LicenceLabel(entry: e)
                    if let url = e.source_url { SourceLink(url) }
                    if let changes = e.changes { Text(changes).font(.system(size: 12)).foregroundStyle(Theme.muted) }
                    if let note = e.license_note { Text(note).font(.system(size: 12)).foregroundStyle(Theme.redPen) }
                    if !e.isReady { Flag("credit incomplete: \(e.credit_status)") }
                }
                .padding(.vertical, 2)
            }
        } header: { SectionTitle("Narrations", detail: "\(entries.count) readings of library texts") }
        .listRowBackground(Theme.card)
    }
}

/// Tatoeba sentences: the author of the Russian and of the English translation, per sentence.
private struct SentencesSection: View {
    let entries: [Credits.Entry]
    @State private var expanded = false

    var body: some View {
        let missing = entries.filter { !$0.isReady }.count
        Section {
            VStack(alignment: .leading, spacing: 6) {
                Text("Sentence pairs from Tatoeba, CC BY 2.0 FR. The speaker of each recording is credited above; the people who wrote the Russian sentence and its English translation are credited here.")
                    .font(.system(size: 14)).foregroundStyle(Theme.text)
                HStack(spacing: 8) {
                    SourceLink("https://tatoeba.org/", label: "tatoeba.org")
                    SourceLink("https://creativecommons.org/licenses/by/2.0/fr/", label: "CC BY 2.0 FR")
                }
                if missing > 0 { Flag("\(missing) of \(entries.count) sentences: contributor not yet recorded") }
            }
            DisclosureGroup(isExpanded: $expanded) {
                ForEach(entries) { e in SentenceRow(entry: e) }
            } label: {
                Text("All \(entries.count) sentences").font(.system(size: 15, weight: .semibold)).foregroundStyle(Theme.text)
            }
        } header: { SectionTitle("Sentences", detail: "\(entries.count) from Tatoeba") }
        .listRowBackground(Theme.card)
    }
}

private struct SentenceRow: View {
    let entry: Credits.Entry

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(entry.title).font(Theme.russian(17)).foregroundStyle(Theme.text)
            ContributorLine(role: "Russian", names: entry.ru_contributors ?? [], url: entry.source_url)
            ContributorLine(role: "English", names: entry.en_contributors ?? [], url: entry.translation_source_url)
        }
        .padding(.vertical, 2)
    }
}

/// "Russian: marafon ↗" or the plain gap when Tatoeba has not told us who wrote it.
private struct ContributorLine: View {
    let role: String
    let names: [String]
    let url: String?

    var body: some View {
        HStack(spacing: 6) {
            Text("\(role):").font(.system(size: 13)).foregroundStyle(Theme.muted)
            if names.isEmpty {
                Flag("contributor not yet recorded")
            } else {
                Text(names.joined(separator: ", ")).font(.system(size: 13)).foregroundStyle(Theme.text)
            }
            if let url { SourceLink(url, label: "sentence") }
        }
    }
}

/// The synthetic voice: the model, its licence, and what that licence does and does not settle.
private struct SyntheticSection: View {
    let entries: [Credits.Entry]

    var body: some View {
        Section {
            ForEach(entries) { e in
                VStack(alignment: .leading, spacing: 4) {
                    Text(e.title).font(.system(size: 16, weight: .semibold)).foregroundStyle(Theme.text)
                    Text(e.attribution).font(.system(size: 14)).foregroundStyle(Theme.text)
                    if let model = e.model, let sha = e.model_sha256 {
                        Text("Model \(model), SHA-256 \(sha.prefix(16))…").font(.system(size: 12, design: .monospaced)).foregroundStyle(Theme.muted)
                    }
                    HStack(spacing: 10) {
                        if let url = e.source_url { SourceLink(url, label: "source") }
                        if let url = e.license_url { SourceLink(url, label: e.licenceName ?? "licence") }
                    }
                    Text("\(e.assets?.count ?? 0) clips").font(.system(size: 13)).foregroundStyle(Theme.muted)
                    if !e.isReady { Flag(e.credit_status) }
                }
                .padding(.vertical, 2)
            }
        } header: { SectionTitle("Synthetic voice") }
        .listRowBackground(Theme.card)
    }
}

/// Models and algorithms the app runs: ZIPA only when the phone recognizer is bundled.
private struct ModelsSection: View {
    let entries: [Credits.Entry]

    private var shown: [Credits.Entry] {
        let zipaBundled = Bundle.main.url(forResource: "Models", withExtension: nil)
            .map { FileManager.default.fileExists(atPath: $0.appendingPathComponent("\(PhoneRecognizer.modelName).mlmodelc").path) } ?? false
        return entries.filter { $0.id != "zipa" || zipaBundled }
    }

    var body: some View {
        Section {
            ForEach(shown) { e in
                VStack(alignment: .leading, spacing: 4) {
                    Text(e.title).font(.system(size: 16, weight: .semibold)).foregroundStyle(Theme.text)
                    Text(e.attribution).font(.system(size: 14)).foregroundStyle(Theme.text)
                    HStack(spacing: 10) {
                        if let url = e.source_url { SourceLink(url, label: "source") }
                        if let url = e.license_url { SourceLink(url, label: e.licenceName ?? "licence") }
                    }
                }
                .padding(.vertical, 2)
            }
        } header: { SectionTitle("Models and algorithms") }
        .listRowBackground(Theme.card)
    }
}

/// The handwriting fonts, each shown in itself, with the complete OFL text behind a disclosure.
private struct FontsSection: View {
    let entries: [Credits.Entry]

    var body: some View {
        Section {
            ForEach(entries) { e in
                VStack(alignment: .leading, spacing: 4) {
                    Text(e.title).font(.custom(String(e.id.dropFirst("font-".count)), size: 26)).foregroundStyle(Theme.text)
                    if let author = e.author { Text(author).font(.system(size: 13)).foregroundStyle(Theme.text) }
                    HStack(spacing: 10) {
                        if let url = e.source_url { SourceLink(url, label: "source") }
                        if let url = e.license_url { SourceLink(url, label: "SIL Open Font License 1.1") }
                    }
                    if let text = e.license_text {
                        DisclosureGroup("Full licence text") {
                            Text(text).font(.system(size: 11, design: .monospaced)).foregroundStyle(Theme.text).textSelection(.enabled)
                        }
                        .font(.system(size: 13)).foregroundStyle(Theme.muted)
                    }
                }
                .padding(.vertical, 2)
            }
        } header: { SectionTitle("Fonts") }
        .listRowBackground(Theme.card)
    }
}

/// Wiktionary and the handwriting references: what was consulted, entry by entry where known.
private struct ReferenceSection: View {
    let entries: [Credits.Entry]

    var body: some View {
        Section {
            ForEach(entries) { e in
                VStack(alignment: .leading, spacing: 4) {
                    Text(e.title).font(.system(size: 16, weight: .semibold)).foregroundStyle(Theme.text)
                    Text(e.attribution).font(.system(size: 14)).foregroundStyle(Theme.text)
                    HStack(spacing: 10) {
                        if let url = e.source_url { SourceLink(url, label: "source") }
                        ForEach(e.source_urls ?? [], id: \.self) { SourceLink($0) }
                        if let url = e.license_url { SourceLink(url, label: "licence") }
                    }
                    if let refs = e.references, !refs.isEmpty {
                        DisclosureGroup("\(refs.count) entries consulted") {
                            ForEach(refs, id: \.self) { r in
                                HStack(spacing: 8) {
                                    Text(r.lemma).font(Theme.russian(16)).foregroundStyle(Theme.text)
                                    SourceLink(r.source_url, label: "entry")
                                    SourceLink(r.history_url, label: "history")
                                }
                            }
                        }
                        .font(.system(size: 13)).foregroundStyle(Theme.muted)
                    }
                    if let note = e.note { Text(note).font(.system(size: 12)).foregroundStyle(Theme.muted) }
                    if !e.isReady { Flag(e.credit_status.replacingOccurrences(of: "-", with: " ")) }
                }
                .padding(.vertical, 2)
            }
        } header: { SectionTitle("Reference material") }
        .listRowBackground(Theme.card)
    }
}

/// What the generator could not credit, by kind, with the items behind a disclosure.
private struct OpenItemsSection: View {
    let unresolved: [String: [String]]

    static let labels: [(key: String, label: String)] = [
        ("human_audio_without_author_credit", "Recordings whose contributor is not yet recorded"),
        ("tatoeba_text_missing_contributors", "Sentences whose author or translator is not yet recorded"),
        ("audio_references_without_provenance", "Clips used without a provenance record"),
        ("unidentified_synthesis", "Synthetic clips whose engine is not identified"),
    ]

    var body: some View {
        let open = Self.labels.compactMap { l in (unresolved[l.key] ?? []).isEmpty ? nil : (l.label, unresolved[l.key] ?? []) }
        Section {
            if open.isEmpty {
                Text("Every bundled item has a contributor and a licence recorded.").font(.system(size: 14)).foregroundStyle(Theme.good)
            } else {
                ForEach(open, id: \.0) { label, items in
                    DisclosureGroup {
                        ForEach(items, id: \.self) { Text($0).font(.system(size: 12, design: .monospaced)).foregroundStyle(Theme.muted) }
                    } label: {
                        HStack(spacing: 8) {
                            Text(label).font(.system(size: 14)).foregroundStyle(Theme.text)
                            Spacer()
                            Text("\(items.count)").font(.system(size: 14, weight: .semibold)).foregroundStyle(Theme.redPen)
                        }
                    }
                }
            }
        } header: { SectionTitle("Not yet resolved") }
        .listRowBackground(Theme.card)
    }
}

private struct OpenItemsSummary: View {
    let unresolved: [String: [String]]

    var body: some View {
        let total = unresolved.values.reduce(0) { $0 + $1.count }
        if total == 0 {
            Text("Every bundled item has a contributor and a licence recorded.").foregroundStyle(Theme.good)
        } else {
            Text("\(total) item\(total == 1 ? "" : "s") still without a complete credit; they are marked in red below and listed at the end.").foregroundStyle(Theme.redPen)
        }
    }
}

// MARK: - Pieces

private struct SectionTitle: View {
    let title: String
    var detail: String?

    init(_ title: String, detail: String? = nil) { self.title = title; self.detail = detail }

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(title).font(Theme.label(12)).tracking(1.2).foregroundStyle(Theme.accent)
            if let detail { Text(detail).font(.system(size: 12)).foregroundStyle(Theme.muted).textCase(nil) }
        }
        .textCase(nil)
    }
}

/// A tappable source, shown as its host and path so a long Commons title does not swallow the row.
private struct SourceLink: View {
    let url: URL?
    let label: String

    init(_ string: String, label: String? = nil) {
        url = URL(string: string)
        self.label = label ?? SourceLink.short(string)
    }

    static func short(_ string: String) -> String {
        guard let url = URL(string: string), let host = url.host() else { return string }
        let path = url.path().removingPercentEncoding ?? url.path()
        return host.replacingOccurrences(of: "www.", with: "") + (path == "/" ? "" : path)
    }

    var body: some View {
        if let url {
            Link(destination: url) {
                Label(label, systemImage: "arrow.up.right").labelStyle(.titleAndIcon)
                    .font(.system(size: 13)).foregroundStyle(Theme.accent).lineLimit(1).truncationMode(.middle)
            }
            .buttonStyle(.plain)
        } else {
            Text(label).font(.system(size: 13)).foregroundStyle(Theme.muted)
        }
    }
}

/// The recorded licence, linked to its text when it has one; "licence not recorded" otherwise.
private struct LicenceLabel: View {
    let entry: Credits.Entry

    var body: some View {
        if let url = entry.license_url, let licence = entry.licenceName {
            SourceLink(url, label: licence)
        } else if let licence = entry.licenceName {
            Text(licence).font(.system(size: 13)).foregroundStyle(Theme.muted)
        } else {
            Flag("licence not recorded")
        }
    }
}

/// A gap, said in red pen.
private struct Flag: View {
    let text: String
    init(_ text: String) { self.text = text }
    var body: some View {
        Text(text).font(.system(size: 12, weight: .semibold)).foregroundStyle(Theme.redPen)
    }
}
