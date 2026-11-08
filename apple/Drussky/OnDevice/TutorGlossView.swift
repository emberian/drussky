import SwiftUI

/// The meaning under a tapped passage word: the page's own gloss when it has one, else one a
/// tutor gives now (Claude online, the on-device model as the offline fallback), tagged with
/// who said it.
struct GlossLine: View {
    @Environment(Book.self) private var book
    let word: String
    /// The passage the word is in, for the tutor.
    let context: String
    /// The gloss the page or the lexicon already has.
    let known: String?

    @State private var fetched: TutorGloss?
    @State private var failed = false

    /// Meanings already asked for this session, so re-tapping a word is free.
    @MainActor private static var cache: [String: TutorGloss] = [:]

    var body: some View {
        HStack(spacing: 8) {
            Text([book.knowledge.rootHint(for: word), meaning].compactMap { $0 }.joined(separator: " → "))
                .scaledFont(17).foregroundStyle(Theme.muted)
            if let fetched { ProvenanceTag(fetched.provenance) }
        }
        .task(id: word) {
            fetched = nil
            failed = false
            guard known == nil else { return }
            let key = RussianText.key(word)
            if let hit = Self.cache[key] { fetched = hit; return }
            guard let backend = TutorRouter.backend(for: TutorRouter.route(.gloss, .live)) else { failed = true; return }
            do {
                let g = try await backend.gloss(word: word, context: context)
                Self.cache[key] = g
                if !Task.isCancelled { fetched = g }
            } catch {
                if !Task.isCancelled { failed = true }
            }
        }
    }

    private var meaning: String {
        if let known { return known }
        if let fetched { return fetched.en }
        return failed ? "Meaning unavailable. Try again when your tutor is available." : "Looking up the meaning…"
    }
}

/// The small tag that says who answered. Claude is the default tutor and goes unlabelled.
struct ProvenanceTag: View {
    let provenance: TutorProvenance
    init(_ provenance: TutorProvenance) { self.provenance = provenance }

    var body: some View {
        if provenance == .onDevice {
            Text(provenance.label)
                .font(Theme.label(10))
                .padding(.horizontal, 7).padding(.vertical, 3)
                .background(Capsule().fill(Theme.muted.opacity(0.12)))
                .foregroundStyle(Theme.muted)
                .accessibilityIdentifier("provenance-on-device")
        }
    }
}
