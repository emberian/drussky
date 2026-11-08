import Foundation
import LearningCore

extension ComponentKind {
    /// A root (family) from morphology.json: `root:пис`.
    static let root: ComponentKind = "root"
    /// A prefix or suffix: `affix:пере-`, `affix:-тель`.
    static let affix: ComponentKind = "affix"
}

extension EdgeKind {
    /// `from` (a word) is built on the root `to`.
    static let containsRoot: EdgeKind = "contains-root"
    /// `from` (a word) carries the affix `to`.
    static let containsAffix: EdgeKind = "contains-affix"
    /// `from` is derived from the word `to` (переписать → писать).
    static let derivedFrom: EdgeKind = "derived-from"
}

extension Modality {
    /// Assembling a word from morpheme tiles for an English meaning (build_word).
    static let build: Modality = "meaning-build"
}

extension RussianDomain {
    /// Root, affix and family-member components with their edges, for the knowledge graph.
    /// Lexemes for words the course doesn't teach come in with `order` .max, so they stay out
    /// of the map until the learner meets them; every word the course knows gets its edges.
    static func morphologyGraph(_ m: Morphology, lexemeIDs: (String) -> ComponentID?) -> (components: [Component], edges: [Edge]) {
        var comps: [Component] = []
        var edges: [Edge] = []
        for r in m.roots {
            comps.append(Component(id: ComponentID(.root, r.id), label: r.label, gloss: r.gloss, unit: "Word families"))
        }
        for p in m.data.prefixes { comps.append(Component(id: ComponentID(.affix, p.id + "-"), label: p.id + "-", gloss: p.gloss, unit: "Word families")) }
        for s in m.data.suffixes { comps.append(Component(id: ComponentID(.affix, "-" + s.id), label: "-" + s.id, gloss: s.gloss, unit: "Word families")) }
        for (key, e) in m.lemmas {
            let id = lexemeIDs(key) ?? ComponentID(.lexeme, key)
            comps.append(Component(id: id, label: key, gloss: e.en, unit: e.family == nil ? nil : "Word families"))
            for r in e.roots { edges.append(Edge(id, ComponentID(.root, r), .containsRoot)) }
            for p in e.prefixes { edges.append(Edge(id, ComponentID(.affix, p + "-"), .containsAffix)) }
            for s in e.suffixes { edges.append(Edge(id, ComponentID(.affix, "-" + s), .containsAffix)) }
            if let base = e.base, let b = m.entry(lemma: base) { edges.append(Edge(id, lexemeIDs(b.key) ?? ComponentID(.lexeme, b.key), .derivedFrom)) }
        }
        return (comps, edges)
    }

    /// The roots the words among `ids` are built on (reading or producing a word is evidence
    /// for its root).
    func rootComponents(for ids: [ComponentID]) -> [ComponentID] {
        var out: [ComponentID] = []
        for id in ids where id.kind == .lexeme || id.kind == .form {
            let lemma = id.kind == .form ? (lemmas.lemma(id.key) ?? id.key) : id.key
            guard let e = morphology.entry(word: lemma) else { continue }
            for r in e.roots where !out.contains(ComponentID(.root, r)) { out.append(ComponentID(.root, r)) }
        }
        return out
    }

    /// The affixes among a build_word exercise's tiles.
    func affixComponents(tiles: [String]) -> [ComponentID] {
        var out: [ComponentID] = []
        for t in tiles {
            if let key = morphology.affixKey(tile: t), !out.contains(ComponentID(.affix, key)) { out.append(ComponentID(.affix, key)) }
        }
        return out
    }

    /// "root зна- (know) + -ние": how a word is built, when its root is one the learner has met
    /// and the word itself is not yet known. `known(id)` answers for any component.
    func rootHint(for word: String, known: (ComponentID) -> Bool) -> String? {
        let key = RussianText.key(word)
        guard !key.isEmpty, !key.contains(" ") else { return nil }
        let lemma = lemmas.lemma(key) ?? morphology.entry(word: key)?.key ?? self.lemma(of: key) ?? key
        guard let e = morphology.entry(word: lemma), let rootID = e.roots.first, let root = morphology.root(rootID),
              known(ComponentID(.root, rootID)), !known(wordID(lemma)), !known(ComponentID(.lexeme, lemma)) else { return nil }
        let pieces = morphology.analysis(of: e)
        guard pieces.count >= 2 else { return "root \(root.label) (\(root.gloss))" }
        return pieces.map { p in
            if RussianText.key(p.piece).replacingOccurrences(of: "-", with: "").isEmpty { return p.piece }
            let bare = RussianText.key(p.piece).replacingOccurrences(of: "-", with: "")
            if root.forms.contains(bare) { return "root \(bare)- (\(root.gloss))" }
            return p.gloss.map { "\(p.piece) (\($0))" } ?? p.piece
        }.joined(separator: " + ")
    }
}

extension KnowledgeTracker {
    /// Decoding help for a passage word: how it is built, when its root is one the learner can
    /// recall today (R ≥ 0.5) and the word itself is not yet.
    func rootHint(for word: String) -> String? {
        let t = now()
        return domain.rootHint(for: word) { (model.strength($0, at: t) ?? 0) >= 0.5 }
    }
}
