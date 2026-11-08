import Foundation

/// "Read handwriting" from the ＋ menu: words the learner already has, each in a different hand,
/// to print or to pick among look-alikes.
extension Book {
    /// Single words the learner has produced, in letters already met, stressed as the lexicon
    /// writes them.
    func handwritingWords() -> [String] {
        let met = Set(course.letters.filter { letterKnown($0.upper) }.map(\.upper))
        var out: [String] = []
        for key in targets.keys.sorted() where key.hasPrefix("word:") {
            let token = String(key.dropFirst(5))
            guard !token.contains(" "), token.count >= 2, RussianText.letters(in: token).isSubset(of: met) else { continue }
            out.append(word(token)?.ru ?? token)
        }
        return out
    }

    /// Short phrases (2–3 words) the learner has produced, for a line to read.
    func handwritingLines() -> [String] {
        let met = Set(course.letters.filter { letterKnown($0.upper) }.map(\.upper))
        return targets.keys.sorted().filter { $0.hasPrefix("word:") }.map { String($0.dropFirst(5)) }
            .filter { (2...3).contains($0.split(separator: " ").count) && RussianText.letters(in: $0).isSubset(of: met) }
            .map { word($0)?.ru ?? $0 }
    }

    /// A page of handwriting to read: a card, then `count` words alternating pick and print,
    /// and one short line to print when there is one. Words with the letters a quick hand
    /// confuses (и ш л м т п н, а о, д б) come first.
    func readHandwritingExercises(count: Int = 6, seed: UInt64? = nil) -> [Exercise] {
        let words = handwritingWords()
        guard words.count >= 4 else { return [] }
        var rng = SeededGenerator(seed: seed ?? UInt64(now().timeIntervalSince1970))
        let tricky = Set("ишлмтпнаодб")
        let ranked = words.shuffled(using: &rng).sorted { a, b in
            RussianText.key(a).filter(tricky.contains).count > RussianText.key(b).filter(tricky.contains).count
        }
        var out: [Exercise] = []
        var card = Exercise(kind: .explain)
        card.text = "Not every hand is the school hand: these words are written looped, slanted and hurried. The shapes you know are all there. Read each one, then print it or pick it."
        out.append(card)
        for (i, w) in ranked.prefix(count).enumerated() {
            var ex = Exercise(kind: .readHandwriting)
            ex.text = w
            ex.seed = Int(truncatingIfNeeded: rng.next() & 0x7FFF_FFFF)
            if i % 2 == 0 {
                let traps = CursiveTraps.distractors(for: w, pool: words, count: 2, rng: &rng)
                if traps.count == 2 { ex.options = ([w] + traps).shuffled(using: &rng) }
            }
            out.append(ex)
        }
        if let line = handwritingLines().shuffled(using: &rng).first {
            var ex = Exercise(kind: .readHandwriting)
            ex.text = line
            ex.seed = Int(truncatingIfNeeded: rng.next() & 0x7FFF_FFFF)
            out.append(ex)
        }
        return out
    }
}

/// The ＋ menu's handwriting page and its QA flag.
@MainActor
enum CursivePages {
    /// Inserts a read-handwriting page after the current one; returns a notice when none can be made.
    static func addHandwriting(_ book: Book) -> String? {
        let exercises = book.readHandwritingExercises()
        guard exercises.count >= 4 else { return "A few more words first: this page is made from words you have already written." }
        book.insertWrittenPage(title: "Read handwriting", intro: "Words you know, in other people's hands. Read them the way you would a note on the table.",
                               unitTitle: "Cursive", exercises: exercises, newWords: [])
        return nil
    }
}
