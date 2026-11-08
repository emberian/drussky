import LearningCore
import SwiftUI

/// "Word families" on the inside cover: every root the learner has a word of, as a hub with its
/// known members (filled) and the rest of the family (grey). Tap a root for the whole family.
struct WordFamiliesSection: View {
    @Environment(Book.self) private var book
    @State private var openRoot: String?

    var body: some View {
        let k = book.knowledge
        let m = k.domain.morphology
        let hubs = Self.hubs(k, m)
        if !hubs.isEmpty {
            VStack(alignment: .leading, spacing: 10) {
                Text("Word families").font(.system(size: 22, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
                Text("Roots you have words on. Grey members are still ahead of you; a root glows as its words do. Tap a root for the family.")
                    .font(.system(size: 15)).foregroundStyle(Theme.muted)
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 250), spacing: 12, alignment: .top)], alignment: .leading, spacing: 12) {
                    ForEach(hubs, id: \.root.id) { hub in
                        Button { openRoot = hub.root.id } label: { FamilyHub(hub: hub) }
                            .buttonStyle(.plain)
                            .accessibilityLabel("Root \(hub.root.label), \(hub.root.gloss): \(hub.known.count) of \(hub.known.count + hub.unknown.count) words known")
                    }
                }
            }
            .sheet(item: Binding(get: { openRoot.map { RootKey(id: $0) } }, set: { openRoot = $0?.id })) { key in
                NavigationStack { FamilyView(rootID: key.id) }
                    .environment(book)
                    .presentationSizing(.page)
            }
            .onAppear {
                // QA: -qa-family пис opens that family.
                let args = ProcessInfo.processInfo.arguments
                if let i = args.firstIndex(of: "-qa-family"), i + 1 < args.count { openRoot = args[i + 1] }
            }
        }
    }

    struct RootKey: Identifiable { var id: String }

    struct Hub {
        var root: Morphology.Root
        var strength: Double?
        /// (headword, strength) for members with evidence, strongest first.
        var known: [(String, Double)]
        var unknown: [String]
    }

    @MainActor static func hubs(_ k: KnowledgeTracker, _ m: Morphology) -> [Hub] {
        let t = k.now()
        return m.families.compactMap { fam -> Hub? in
            guard let root = m.root(fam) else { return nil }
            var known: [(String, Double)] = [], unknown: [String] = []
            for member in m.members(of: fam) {
                let id = ComponentID(.lexeme, member)
                if k.model.hasEvidence(id) { known.append((member, k.model.strength(id, at: t) ?? 0)) } else { unknown.append(member) }
            }
            guard !known.isEmpty else { return nil }
            return Hub(root: root, strength: k.model.strength(ComponentID(.root, fam), at: t), known: known.sorted { $0.1 > $1.1 }, unknown: unknown)
        }
        .sorted { ($0.known.count, $0.known.count + $0.unknown.count) > ($1.known.count, $1.known.count + $1.unknown.count) }
    }
}

/// One root with its family around it.
struct FamilyHub: View {
    let hub: WordFamiliesSection.Hub

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(hub.root.label).font(Theme.russian(20, weight: .semibold)).foregroundStyle(Theme.text)
                Text(hub.root.gloss).font(.system(size: 14)).foregroundStyle(Theme.muted).lineLimit(1)
                Spacer(minLength: 0)
                Text("\(hub.known.count)/\(hub.known.count + hub.unknown.count)").font(Theme.label(12)).foregroundStyle(Theme.muted).monospacedDigit()
            }
            FlowRow(spacing: 6) {
                ForEach(hub.known, id: \.0) { word, s in
                    Text(word).font(Theme.russian(15)).foregroundStyle(Theme.text)
                        .padding(.horizontal, 8).padding(.vertical, 3)
                        .background(Capsule().fill(Theme.accent.opacity(0.1 + 0.4 * s)))
                }
                ForEach(hub.unknown.prefix(6), id: \.self) { word in
                    Text(word).font(Theme.russian(15)).foregroundStyle(Theme.muted.opacity(0.7))
                        .padding(.horizontal, 8).padding(.vertical, 3)
                        .overlay(Capsule().strokeBorder(Theme.muted.opacity(0.3), lineWidth: 1))
                }
                if hub.unknown.count > 6 {
                    Text("+\(hub.unknown.count - 6)").font(Theme.label(12)).foregroundStyle(Theme.muted).padding(.vertical, 4)
                }
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
        .overlay(RoundedRectangle(cornerRadius: 14).stroke(Theme.accent.opacity(0.15 + 0.5 * (hub.strength ?? 0)), lineWidth: 1))
    }
}

/// A whole family: the root, what it means, and every member taken apart, with its gloss and sound.
struct FamilyView: View {
    @Environment(Book.self) private var book
    @Environment(\.dismiss) private var dismiss
    let rootID: String
    @State private var history: ComponentID?

    var body: some View {
        let k = book.knowledge
        let m = k.domain.morphology
        let t = k.now()
        ScrollView {
            if let root = m.root(rootID) {
                VStack(alignment: .leading, spacing: 20) {
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(alignment: .firstTextBaseline, spacing: 14) {
                            Text(root.label).font(Theme.russian(40, weight: .semibold)).foregroundStyle(Theme.text)
                            Text(root.gloss).font(.system(size: 20)).foregroundStyle(Theme.text)
                        }
                        if let note = root.note {
                            Text(Theme.stressed(note, show: book.settings.showStress)).font(.system(size: 15)).foregroundStyle(Theme.muted)
                                .fixedSize(horizontal: false, vertical: true)
                        }
                        if let s = k.model.strength(ComponentID(.root, rootID), at: t) {
                            Text("Recall of the root today: \(Int((s * 100).rounded()))%").font(Theme.label(13)).foregroundStyle(Theme.muted)
                        }
                    }
                    VStack(alignment: .leading, spacing: 12) {
                        ForEach(m.members(of: rootID), id: \.self) { member in
                            if let e = m.lemmas[member] { row(e, k, t) }
                        }
                    }
                    Button { history = ComponentID(.root, rootID) } label: {
                        Label("History of this root", systemImage: "clock.arrow.circlepath").font(Theme.label(14))
                    }
                    .buttonStyle(.plain).foregroundStyle(Theme.accent)
                }
                .padding(24)
                .frame(maxWidth: 760, alignment: .leading)
                .frame(maxWidth: .infinity)
            }
        }
        .background(GridPaper(marginX: nil))
        .navigationTitle(m.root(rootID)?.label ?? rootID)
        .navigationBarTitleDisplayMode(.inline)
        .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        .sheet(item: $history) { id in
            NavigationStack { ComponentHistoryView(id: id) }.environment(book).presentationSizing(.page)
        }
    }

    private func row(_ e: Morphology.Entry, _ k: KnowledgeTracker, _ t: Date) -> some View {
        let id = ComponentID(.lexeme, e.key)
        let strength = k.model.strength(id, at: t)
        let known = k.model.hasEvidence(id)
        return HStack(alignment: .firstTextBaseline, spacing: 12) {
            HearButton(text: e.ru, size: 30)
            VStack(alignment: .leading, spacing: 3) {
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    Text(Theme.stressed(e.ru, show: book.settings.showStress)).font(Theme.russian(22, weight: .medium))
                        .foregroundStyle(known ? Theme.text : Theme.muted)
                    Text(e.en).font(.system(size: 15)).foregroundStyle(known ? Theme.text : Theme.muted)
                    if let s = strength {
                        Text("\(Int((s * 100).rounded()))%").font(Theme.label(12)).foregroundStyle(Theme.muted).monospacedDigit()
                    } else {
                        Text("not yet").font(Theme.label(12)).foregroundStyle(Theme.muted.opacity(0.7))
                    }
                }
                HStack(spacing: 6) {
                    Text(e.morphemes.map { RussianText.unstressed($0) }.joined(separator: " · ")).font(Theme.russian(14)).foregroundStyle(Theme.muted)
                    if let base = e.base, base != e.key { Text("← \(base)").font(.system(size: 13)).foregroundStyle(Theme.muted) }
                }
                if let note = e.note ?? e.loan {
                    Text(Theme.stressed(note, show: book.settings.showStress)).font(.system(size: 13)).foregroundStyle(Theme.muted)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            Spacer(minLength: 0)
        }
        .padding(10)
        .background(RoundedRectangle(cornerRadius: 12).fill(known ? Theme.accent.opacity(0.06 + 0.3 * (strength ?? 0)) : Theme.card.opacity(0.5)))
        .accessibilityElement(children: .combine)
    }
}

/// The ＋ menu's two morphology pages, and their QA flags.
@MainActor
enum MorphologyPages {
    /// Inserts a word-families page after the current one; returns a notice when none can be made.
    static func addFamilies(_ book: Book) -> String? {
        let m = book.knowledge.domain.morphology
        let (exercises, roots, newWords) = book.wordFamilyExercises(from: m)
        guard exercises.count >= 4 else {
            return "A family page needs two words you know on one root. A few more pages and the first roots open up."
        }
        let labels = roots.compactMap { m.root($0)?.label }.joined(separator: " · ")
        book.insertWrittenPage(title: "Word families", intro: "One root, many words: \(labels). Each new word here is an old root with a new prefix or suffix: guess before you look.",
                               unitTitle: "Word families", exercises: exercises, newWords: newWords)
        return nil
    }

    /// Inserts a cognates page after the current one.
    static func addCognates(_ book: Book) -> String? {
        let m = book.knowledge.domain.morphology
        let (exercises, newWords) = book.cognateExercises(from: m)
        guard exercises.count >= 4 else { return "The cognates use letters you haven't met yet. They open up with the alphabet." }
        book.insertWrittenPage(title: "Cognates", intro: "Words that mathematics and philosophy share with Russian, sound for sound. Read each from the Greek or Latin you already know.",
                               unitTitle: "Word families", exercises: exercises, newWords: newWords)
        return nil
    }
}
