import Foundation

/// A syllable of a written Russian target: its letters (Character offsets into the target, the
/// same offsets `PhoneJudgement.letterRange` uses), which word it belongs to, and whether the
/// text marks it stressed.
struct TextSyllable: Sendable, Hashable {
    var text: String
    var letters: Range<Int>
    var word: Int
    var isStressed: Bool
}

enum Syllables {
    static let vowels: Set<Character> = Set("аеёиоуыэюяАЕЁИОУЫЭЮЯ")
    private static let sonorants: Set<Character> = Set("рлмнРЛМН")
    private static let signs: Set<Character> = Set("ьъЬЪ")
    /// Unstressed little words that lean on their neighbour (`RussianText.clitics`).
    private static let clitics: Set<String> = RussianText.clitics

    static func base(_ c: Character) -> Character { Character(String(c.unicodeScalars.prefix(1))) }
    static func isVowel(_ c: Character) -> Bool { vowels.contains(base(c)) }
    static func isLetter(_ c: Character) -> Bool {
        guard let s = c.unicodeScalars.first else { return false }
        return (0x0400...0x04FF).contains(s.value)
    }
    static func isMarkedStressed(_ c: Character) -> Bool {
        c.unicodeScalars.contains("\u{0301}") || base(c) == "ё" || base(c) == "Ё"
    }

    /// Syllables in reading order. Each vowel is a nucleus; between two vowels one consonant
    /// starts the next syllable, й (and a sonorant before another consonant) closes the one
    /// before, ь and ъ stay with their consonant. Words without a vowel (в, с, к) join the next.
    static func split(_ target: String) -> [TextSyllable] {
        let chars = Array(target)
        // Words: maximal runs of Cyrillic letters.
        var words: [Range<Int>] = []
        var i = 0
        while i < chars.count {
            guard isLetter(chars[i]) else { i += 1; continue }
            var j = i
            while j < chars.count && isLetter(chars[j]) { j += 1 }
            words.append(i..<j)
            i = j
        }
        var out: [TextSyllable] = []
        var carry: Int?  // start of a vowelless word waiting for the next
        var wordNumber = 0
        for w in words {
            let nuclei = w.filter { isVowel(chars[$0]) }
            guard !nuclei.isEmpty else { if carry == nil { carry = w.lowerBound }; continue }
            var starts = [carry ?? w.lowerBound]
            carry = nil
            for k in nuclei.indices.dropFirst() {
                let a = nuclei[k - 1], b = nuclei[k]
                let cluster = Array((a + 1)..<b)
                var split = a + 1
                if cluster.count == 1 {
                    split = cluster[0]
                } else if cluster.count > 1 {
                    // Consonants (with their signs) between the vowels.
                    let first = chars[cluster[0]]
                    let firstEnd = cluster.count > 1 && signs.contains(chars[cluster[1]]) ? cluster[0] + 2 : cluster[0] + 1
                    if base(first) == "й" || base(first) == "Й" || (sonorants.contains(first) && firstEnd < b) {
                        split = firstEnd
                    } else {
                        split = cluster[0]
                    }
                }
                starts.append(split)
            }
            let word = String(chars[w].map { $0 })
            let plain = RussianText.key(word)
            let marked = nuclei.contains { isMarkedStressed(chars[$0]) }
            for (k, s) in starts.enumerated() {
                let e = k + 1 < starts.count ? starts[k + 1] : w.upperBound
                let nucleus = nuclei[k]
                let stressed = marked ? isMarkedStressed(chars[nucleus]) : (nuclei.count == 1 && !clitics.contains(plain))
                out.append(TextSyllable(text: String(chars[s..<e]), letters: s..<e, word: wordNumber, isStressed: stressed))
            }
            wordNumber += 1
        }
        return out
    }
}
