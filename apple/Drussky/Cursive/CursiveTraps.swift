import Foundation

/// Words that could be misread for another in a quick hand: the лишишься problem, where
/// и ш л м т п н all become the same zigzag, о and а one bowl, д and б one oval with a tail.
enum CursiveTraps {
    /// A small letter and the ones a reader mistakes it for in handwriting.
    static let lookalikes: [Character: [Character]] = [
        "и": ["ш", "л", "н", "п"], "ш": ["и", "т", "щ"], "л": ["м", "и", "а"], "м": ["л", "ш"], "т": ["ш", "п", "м"],
        "п": ["т", "н", "и"], "н": ["п", "и", "к"], "ц": ["щ", "и"], "щ": ["ш", "ц"], "й": ["и"],
        "а": ["о", "д"], "о": ["а", "е"], "д": ["б", "у", "а"], "б": ["д", "в"], "у": ["д", "ц"],
        "г": ["ч", "т"], "ч": ["г", "у"], "е": ["с", "в"], "ё": ["е", "с"], "с": ["е", "о"], "э": ["з", "с"], "з": ["э", "у"],
        "в": ["б", "е"], "ж": ["х"], "х": ["ж", "л"], "ы": ["ь", "и"], "ь": ["ы", "в"], "ъ": ["ь"],
        "к": ["н", "л"], "р": ["ф", "у"], "ф": ["р"], "ю": ["ы", "о"], "я": ["л", "а"],
    ]

    /// True when `a` and `b` have the same letters except at one or two places where the
    /// letters are handwriting look-alikes.
    static func lookalike(_ a: String, _ b: String) -> Bool {
        let x = Array(RussianText.key(a)), y = Array(RussianText.key(b))
        guard x.count == y.count, x != y else { return false }
        var diffs = 0
        for (p, q) in zip(x, y) where p != q {
            guard lookalikes[p]?.contains(q) == true || lookalikes[q]?.contains(p) == true else { return false }
            diffs += 1
        }
        return (1...2).contains(diffs)
    }

    /// Up to `count` words to show beside `word`: real words from `pool` that are look-alikes
    /// of it first, then invented ones (a look-alike letter swapped in), never the word itself.
    /// Stress marks are kept where the word's letters are kept.
    static func distractors(for word: String, pool: [String], count: Int, rng: inout SeededGenerator) -> [String] {
        let key = RussianText.key(word)
        var out: [String] = []
        let real = pool.filter { RussianText.key($0) != key && lookalike($0, word) }.shuffled(using: &rng)
        for w in real where !out.contains(where: { RussianText.key($0) == RussianText.key(w) }) && out.count < count { out.append(w) }
        var tries = 0
        let chars = Array(word)
        let positions = chars.indices.filter { lookalikes[bare(chars[$0])] != nil }
        while out.count < count && tries < 40 && !positions.isEmpty {
            tries += 1
            var c = chars
            let i = positions[Int(rng.next() % UInt64(positions.count))]
            guard let options = lookalikes[bare(c[i])] else { continue }
            var swap = options[Int(rng.next() % UInt64(options.count))]
            if c[i].isUppercase { swap = Character(swap.uppercased()) }
            // Keep the stress mark if the vowel stays a vowel; drop it otherwise.
            let stressed = String(c[i]).unicodeScalars.contains("\u{0301}")
            let vowels = "аеёиоуыэюя"
            c[i] = stressed && vowels.contains(swap) ? Character(String(swap) + "\u{0301}") : swap
            let made = String(c)
            let madeKey = RussianText.key(made)
            guard madeKey != key, !out.contains(where: { RussianText.key($0) == madeKey }), !pool.contains(where: { RussianText.key($0) == madeKey }) else { continue }
            out.append(made)
        }
        return out
    }

    /// The letter without its stress mark, lowercased ("а́" → "а").
    static func bare(_ ch: Character) -> Character {
        let scalars = String(ch).unicodeScalars.filter { $0 != "\u{0301}" }
        return Character(String(String.UnicodeScalarView(scalars)).lowercased())
    }
}
