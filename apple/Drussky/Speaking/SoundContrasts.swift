import Foundation

/// The sound contrasts of pairs.json, as the speaking report names them.
enum SoundContrasts {
    /// The pairs.json contrast id for a tip's `contrast`, however the analyzer spelled it
    /// ("ы/и", "y-i", "ы or и", "hard/soft", "stress", …). nil when there's no such page.
    static func id(for name: String) -> String? {
        let n = name.lowercased().replacingOccurrences(of: " ", with: "")
        switch n {
        case "y-i", "ы/и", "и/ы", "ыori", "ы-и": return "y-i"
        case "hard-soft", "hard/soft", "soft/hard", "hardorsoft", "soft", "palatalization", "palatalisation": return "hard-soft"
        case "sh-shch", "ш/щ", "щ/ш", "шorщ": return "sh-shch"
        case "zh-sh", "ж/ш", "ш/ж", "жorш": return "zh-sh"
        case "reduction", "unstressedо", "unstressedo", "о/а", "а/о", "akanye": return "reduction"
        case "stress", "wherethestressfalls": return "stress"
        default: return nil
        }
    }

    /// The contrast a letter-for-letter substitution belongs to (expected, heard; lowercase).
    static func id(expected: Character, heard: Character?) -> String? {
        let pair: Set<Character> = heard.map { [expected, $0] } ?? [expected]
        if pair == ["ы", "и"] { return "y-i" }
        if pair == ["ш", "щ"] { return "sh-shch" }
        if pair == ["ж", "ш"] { return "zh-sh" }
        let softPairs: [Set<Character>] = [["а", "я"], ["у", "ю"], ["о", "ё"], ["э", "е"]]
        if softPairs.contains(pair) || expected == "ь" || heard == "ь" { return "hard-soft" }
        return nil
    }

    nonisolated(unsafe) private static var loaded: SoundPairs?
    private static let lock = NSLock()
    static var pairs: SoundPairs {
        lock.withLock {
            if let loaded { return loaded }
            let p = SoundPairs.load()
            loaded = p
            return p
        }
    }

    /// The contrast's teaching note, first sentence only (what to do with the mouth).
    static func advice(_ id: String) -> String? {
        guard let note = pairs.contrast(id)?.note else { return nil }
        if let dot = note.firstIndex(of: ".") { return String(note[...dot]) }
        return note
    }

    static func name(_ id: String) -> String? { pairs.contrast(id)?.name }
}

/// Rough IPA for a letter, for the word-level report's phones (never shown to the learner).
enum LetterSounds {
    static let ipa: [Character: String] = [
        "а": "a", "б": "b", "в": "v", "г": "ɡ", "д": "d", "е": "je", "ё": "jo", "ж": "ʐ", "з": "z", "и": "i",
        "й": "j", "к": "k", "л": "ɫ", "м": "m", "н": "n", "о": "o", "п": "p", "р": "r", "с": "s", "т": "t",
        "у": "u", "ф": "f", "х": "x", "ц": "ts", "ч": "tɕ", "ш": "ʂ", "щ": "ɕː", "ъ": "", "ы": "ɨ", "ь": "ʲ",
        "э": "e", "ю": "ju", "я": "ja",
    ]
    static func phone(_ c: Character) -> Phone { Phone(ipa: ipa[c] ?? String(c)) }
}
