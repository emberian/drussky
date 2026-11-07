import Foundation

enum RussianText {
    static let stressMark: Character = "\u{0301}"

    /// Display text without stress marks (for checking and for writing models).
    static func unstressed(_ s: String) -> String {
        String(s.unicodeScalars.filter { $0 != "\u{0301}" }.map(Character.init))
    }

    /// Lookup key shared with the audio index (apple/COURSE_SCHEMA.md):
    /// lowercase, no stress marks, no punctuation, collapsed whitespace.
    static func key(_ s: String) -> String {
        let stripped = unstressed(s).lowercased()
        let punctuation = Set(".,?!—–-\"«»:;…()")
        let cleaned = String(stripped.map { punctuation.contains($0) ? " " : $0 })
        return cleaned.split(whereSeparator: \.isWhitespace).joined(separator: " ")
    }

    /// Latin letters that handwriting recognizers commonly return for Cyrillic shapes.
    static let homoglyphs: [Character: Character] = [
        "A": "А", "a": "а", "B": "В", "b": "ь", "C": "С", "c": "с", "E": "Е", "e": "е",
        "H": "Н", "K": "К", "k": "к", "M": "М", "m": "т", "O": "О", "o": "о", "P": "Р",
        "p": "р", "T": "Т", "X": "Х", "x": "х", "y": "у", "Y": "У", "u": "и", "n": "п",
        "r": "г", "3": "З", "6": "б", "0": "О",
    ]

    /// Comparison form for handwriting/speech answers: key() with homoglyphs mapped and ё→е.
    static func comparable(_ s: String) -> String {
        var s = s
        for (pair, letter) in [("bl", "ы"), ("bI", "ы"), ("b|", "ы"), ("bi", "ы"), ("IO", "Ю"), ("Io", "ю"), ("3", "з")] {
            s = s.replacingOccurrences(of: pair, with: letter)
        }
        let mapped = String(s.map { homoglyphs[$0] ?? $0 })
        return key(mapped).replacingOccurrences(of: "ё", with: "е")
    }

    static func matches(_ heard: String, _ target: String) -> Bool {
        let a = comparable(heard), b = comparable(target)
        return !a.isEmpty && (a == b || a.replacingOccurrences(of: " ", with: "") == b.replacingOccurrences(of: " ", with: ""))
    }

    /// Cyrillic letters used in a string (uppercased), for letters-before-use logic.
    static func letters(in s: String) -> Set<String> {
        Set(unstressed(s).uppercased().compactMap { ch -> String? in
            guard let v = ch.unicodeScalars.first?.value, (0x0400...0x04FF).contains(v) else { return nil }
            return String(ch)
        })
    }

    /// True when handwriting practice reads as the target written one or more times
    /// (for example "Аа Аа" for А): every letter read belongs to the target.
    static func isRepetition(_ heard: String, of target: String) -> Bool {
        let wanted = Set(comparable(target).filter { !$0.isWhitespace })
        let got = comparable(heard).filter { !$0.isWhitespace }
        return !got.isEmpty && !wanted.isEmpty && got.allSatisfy { wanted.contains($0) } && wanted.isSubset(of: Set(got))
    }
}
