import Foundation
import LearningCore

/// Pages built locally from morphology.json: word families from roots the learner already has
/// two words of, and the Greek/Latin cognates of mathematics and philosophy.
extension Book {
    /// Lemmas the learner has shown they know, as morphology headwords: the memory model has
    /// evidence on the word (the same criterion the families view draws by). A word a generated
    /// page merely proposes is vocabulary (`customWords`), not knowledge, until an exercise on it
    /// is finished; a word met in an inflected form counts through its lemma.
    var knownLemmas: Set<String> {
        let m = knowledge.domain.morphology, model = knowledge.model, domain = knowledge.domain
        var out = Set(m.lemmas.keys.filter { model.hasEvidence(ComponentID(.lexeme, $0)) })
        for key in targets.keys where key.hasPrefix("word:") {
            for t in String(key.dropFirst(5)).split(separator: " ").map(String.init) {
                let lemma = domain.lemma(of: t) ?? t
                guard model.hasEvidence(domain.wordID(t)) || model.hasEvidence(ComponentID(.lexeme, lemma)),
                      let e = m.entry(word: lemma) else { continue }
                out.insert(e.key)
            }
        }
        return out
    }

    private var metLetters: Set<String> { Set(course.letters.filter { letterKnown($0.upper) }.map(\.upper)) }

    /// Root ids the learner knows at least `atLeast` members of, most members first.
    func familyRoots(from m: Morphology, atLeast: Int = 2) -> [String] {
        let known = knownLemmas
        return m.families.filter { m.members(of: $0).filter(known.contains).count >= atLeast }
            .sorted { a, b in
                let ka = m.members(of: a).filter(known.contains).count, kb = m.members(of: b).filter(known.contains).count
                let ua = m.members(of: a).count - ka, ub = m.members(of: b).count - kb
                return (min(ua, 3), ka) != (min(ub, 3), kb) ? (min(ua, 3), ka) > (min(ub, 3), kb) : a < b
            }
    }

    /// A word-families page: for up to three roots, a card, a new member to guess in a sentence,
    /// words to build from tiles, and a known member to write. Everything in letters already
    /// met; nothing from a root the learner has fewer than two words of. Also returns the new
    /// words the page introduces.
    func wordFamilyExercises(from m: Morphology, rootLimit: Int = 3, seed: UInt64? = nil) -> (exercises: [Exercise], roots: [String], newWords: [Course.Word]) {
        let met = metLetters
        let known = knownLemmas
        var rng = SeededGenerator(seed: seed ?? UInt64(now().timeIntervalSince1970))
        func readable(_ s: String) -> Bool { RussianText.letters(in: s).isSubset(of: met) }
        var out: [Exercise] = []
        var newWords: [Course.Word] = []
        var usedRoots: [String] = []
        for rootID in familyRoots(from: m) where usedRoots.count < rootLimit {
            guard let root = m.root(rootID) else { continue }
            let members = m.members(of: rootID).compactMap { m.lemmas[$0] }.filter { readable($0.ru) }
            let knownMembers = members.filter { known.contains($0.key) }
            let newMembers = members.filter { !known.contains($0.key) }
            guard knownMembers.count >= 2 else { continue }
            usedRoots.append(rootID)
            // The card: the root, its meaning, what is known, what the family adds.
            var card = Exercise(kind: .explain)
            // Only the allomorphs the learner can read yet (кот-, not кош- before Ш is met).
            let label = root.forms.filter(readable).prefix(3).map { $0 + "-" }.joined(separator: "/")
            var text = "Root \(label) means \(root.gloss). You know \(knownMembers.prefix(3).map(\.ru).joined(separator: ", "))."
            if let note = root.note, readable(note) { text += " \(note)" }
            card.text = text
            card.examples = (knownMembers.prefix(3) + newMembers.prefix(3)).map { Course.Example(ru: $0.ru, en: $0.en) }
            out.append(card)
            // Guess a new member from a sentence whose other words are mostly known.
            let guessable = newMembers.filter { e in
                guard let ex = e.example, readable(ex.ru) else { return false }
                let others = RussianText.key(ex.ru).split(separator: " ").map(String.init).filter { $0 != e.key }
                let knownCount = others.filter { known.contains(knowledge.domain.lemma(of: $0) ?? $0) || ($0.count <= 2) }.count
                return others.isEmpty || knownCount * 2 >= others.count
            }.shuffled(using: &rng)
            for e in guessable.prefix(1) {
                var g = Exercise(kind: .familyGuess)
                g.text = e.example!.ru
                g.en = e.example!.en
                g.word = e.ru
                g.root = rootID
                let distractors = m.lemmas.values.filter { $0.family != rootID && $0.family != nil && $0.en != e.en && $0.pos == e.pos }
                    .shuffled(using: &rng).prefix(2).map(\.en)
                var options = [e.en] + distractors
                options.shuffle(using: &rng)
                g.options = options
                g.answerIndex = options.firstIndex(of: e.en)
                out.append(g)
                newWords.append(Course.Word(ru: e.ru, en: e.en, emoji: nil, note: "from the family of \(root.label)"))
            }
            // Build one known and one new member from tiles.
            let buildable = (knownMembers.shuffled(using: &rng).prefix(1) + newMembers.filter { !guessable.prefix(1).contains($0) }.shuffled(using: &rng).prefix(1))
                .filter { m.tiles(for: $0) != nil }
            for e in buildable {
                if let b = buildWord(for: e, from: m, rng: &rng) {
                    out.append(b)
                    if !known.contains(e.key), !newWords.contains(where: { $0.ru == e.ru }) {
                        newWords.append(Course.Word(ru: e.ru, en: e.en, emoji: nil, note: "from the family of \(root.label)"))
                    }
                }
            }
            // Write a known member from its meaning (recorded course words only).
            if let w = knownMembers.compactMap({ word($0.ru) }).filter({ !$0.en.isEmpty }).shuffled(using: &rng).first {
                var t = Exercise(kind: .translate)
                t.en = w.en
                t.answerText = w.ru
                out.append(t)
            }
        }
        return (out, usedRoots, newWords)
    }

    /// A build_word exercise for a lemma: its tiles plus two affix tiles that don't belong, all
    /// in letters already met.
    func buildWord(for e: Morphology.Entry, from m: Morphology, rng: inout SeededGenerator) -> Exercise? {
        guard let tiles = m.tiles(for: e) else { return nil }
        let met = metLetters
        let bare = Set(tiles.map { RussianText.key($0).replacingOccurrences(of: "-", with: "") })
        func fits(_ t: String) -> Bool { !bare.contains(RussianText.key(t).replacingOccurrences(of: "-", with: "")) && RussianText.letters(in: t).isSubset(of: met) }
        let prefixes = m.data.prefixes.map { $0.id + "-" }.filter(fits)
        let suffixes = m.data.suffixes.filter { $0.pos == e.pos || $0.pos == nil }.map { "-" + $0.id }.filter(fits)
        var distractors: [String] = []
        if let p = prefixes.shuffled(using: &rng).first { distractors.append(p) }
        if let s = suffixes.shuffled(using: &rng).first { distractors.append(s) }
        var b = Exercise(kind: .buildWord)
        b.en = e.en
        b.answerText = e.ru
        b.parts = tiles
        b.options = (tiles + distractors).shuffled(using: &rng)
        return b
    }

    /// A cognates page: Greek and Latin words of mathematics and philosophy that are already
    /// almost Russian. A card on the correspondences used, then each word read for its meaning,
    /// built from its parts, and written from English. Only words in letters already met.
    func cognateExercises(from m: Morphology, count: Int = 5, seed: UInt64? = nil) -> (exercises: [Exercise], newWords: [Course.Word]) {
        let met = metLetters
        var rng = SeededGenerator(seed: seed ?? UInt64(now().timeIntervalSince1970))
        let known = knownLemmas
        let pool = m.cognates.filter { RussianText.letters(in: $0.ru).isSubset(of: met) && !known.contains(RussianText.key($0.ru)) }
        let withParts = pool.filter { $0.parts != nil }.shuffled(using: &rng)
        let rest = pool.filter { $0.parts == nil }.shuffled(using: &rng)
        let chosen = Array((withParts + rest).prefix(count))
        guard chosen.count >= 3 else { return ([], []) }
        var out: [Exercise] = []
        var rules: [Morphology.Rule] = []
        for c in chosen { for r in c.rules { if let rule = m.rule(r), !rules.contains(rule), rule.id != "none" { rules.append(rule) } } }
        var card = Exercise(kind: .explain)
        card.text = "Greek and Latin words that are already almost Russian. " + rules.prefix(3).map { "\($0.pattern): \($0.note)" }.joined(separator: " ")
        card.examples = chosen.map { Course.Example(ru: $0.ru, en: $0.en) }
        out.append(card)
        for c in chosen {
            var rm = Exercise(kind: .readMeaning)
            rm.text = c.ru
            let others = pool.filter { $0.en != c.en }.shuffled(using: &rng).prefix(2).map(\.en)
            var options = [c.en] + others
            options.shuffle(using: &rng)
            rm.options = options
            rm.answerIndex = options.firstIndex(of: c.en)
            out.append(rm)
            if let tiles = m.tiles(for: c) {
                var b = Exercise(kind: .buildWord)
                b.en = c.en
                b.answerText = c.ru
                b.parts = tiles
                let foreign = m.cognates.filter { $0.ru != c.ru }.compactMap { $0.parts?.randomElement(using: &rng)?.tile }
                    .filter { t in !tiles.contains(where: { RussianText.key($0) == RussianText.key(t) }) && RussianText.letters(in: t).isSubset(of: met) }
                b.options = (tiles + Array(foreign.shuffled(using: &rng).prefix(2))).shuffled(using: &rng)
                out.append(b)
            }
        }
        for c in chosen.prefix(2) {
            var t = Exercise(kind: .translate)
            t.en = c.en
            t.answerText = c.ru
            out.append(t)
        }
        return (out, chosen.map { Course.Word(ru: $0.ru, en: $0.en, emoji: nil, note: $0.source) })
    }
}

/// A small deterministic generator (SplitMix64), so tests can fix a page's draw.
struct SeededGenerator: RandomNumberGenerator {
    private var state: UInt64
    init(seed: UInt64) { state = seed }
    mutating func next() -> UInt64 {
        state &+= 0x9E37_79B9_7F4A_7C15
        var z = state
        z = (z ^ (z >> 30)) &* 0xBF58_476D_1CE4_E5B9
        z = (z ^ (z >> 27)) &* 0x94D0_49BB_1331_11EB
        return z ^ (z >> 31)
    }
}
