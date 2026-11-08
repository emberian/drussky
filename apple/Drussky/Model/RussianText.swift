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
        if !a.isEmpty && (a == b || a.replacingOccurrences(of: " ", with: "") == b.replacingOccurrences(of: " ", with: "")) { return true }
        return shapeMatches(heard, target)
    }

    /// What a handwritten Russian letter can come back as from a recognizer that read it as
    /// Latin (cursive т looks like m, и like u, д like g, а like a/o/G…). Lowercase keys.
    static let lookalikes: [Character: Set<Character>] = [
        "a": ["а", "э"], "b": ["б", "в", "ь"], "c": ["с", "а"], "d": ["д", "б"], "e": ["е", "ё", "э"], "g": ["д", "а"],
        "h": ["н", "ч"], "i": ["і", "и", "г", "т"], "k": ["к"], "l": ["л", "г"], "m": ["т", "м", "ш"], "n": ["п", "л", "т"],
        "o": ["о"], "p": ["р"], "r": ["г", "т"], "s": ["э", "з", "с"], "t": ["т", "э"], "u": ["и", "ц"], "v": ["в", "у"],
        "w": ["ш"], "x": ["х", "ж"], "y": ["у"], "z": ["г", "з"], "3": ["з", "э"], "6": ["б", "в"], "0": ["о"],
        "4": ["ч"], "9": ["д"],
    ]

    /// Position-by-position match of a recognizer's reading against the known answer, allowing
    /// each read glyph to stand for the Russian letters it is commonly confused with.
    /// Readings where several Latin glyphs stand for one Russian letter (bl→ы, III→ш, IO→ю).
    static func foldMultiGlyph(_ s: String) -> String {
        var h = s
        for (pair, letter) in [("bl", "ы"), ("bI", "ы"), ("b|", "ы"), ("bi", "ы"), ("III", "ш"), ("lll", "ш"), ("Ill", "ш"), ("IO", "ю"), ("Io", "ю"), ("lo", "ю")] {
            h = h.replacingOccurrences(of: pair, with: letter)
        }
        return h
    }

    static func shapeMatches(_ heard: String, _ target: String) -> Bool {
        // Multi-glyph readings (bl→ы, III→ш, IO→ю) are tried both applied and not, since the
        // same pair can also be two real letters (IO in "ЧIO" is т+о).
        let h = foldMultiGlyph(heard)
        return shapeMatchesExact(h, target) || (h != heard && shapeMatchesExact(heard, target))
    }

    private static func shapeMatchesExact(_ h: String, _ target: String) -> Bool {
        let a = Array(key(h).replacingOccurrences(of: " ", with: ""))
        let b = Array(key(target).replacingOccurrences(of: "ё", with: "е").replacingOccurrences(of: " ", with: ""))
        guard !a.isEmpty, a.count == b.count else { return false }
        for (x, y) in zip(a, b) {
            let x = x == "ё" ? "е" : x
            if x == y { continue }
            if let options = lookalikes[x], options.contains(y) { continue }
            return false
        }
        return true
    }

    /// Cyrillic letters used in a string (uppercased), for letters-before-use logic.
    static func letters(in s: String) -> Set<String> {
        Set(unstressed(s).uppercased().compactMap { ch -> String? in
            guard let v = ch.unicodeScalars.first?.value, (0x0400...0x04FF).contains(v) else { return nil }
            return String(ch)
        })
    }

    /// True when handwriting practice reads as the target written whole, in order, one or more
    /// times ("Аа Аа" for А, "мамамама" for мама). A part ("ам"), a rearrangement ("ток" for
    /// кот) or extra letters don't count.
    static func isRepetition(_ heard: String, of target: String) -> Bool {
        let unit = Array(comparable(target).filter { !$0.isWhitespace })
        let got = Array(comparable(heard).filter { !$0.isWhitespace })
        guard !unit.isEmpty, !got.isEmpty, got.count % unit.count == 0 else { return false }
        return stride(from: 0, to: got.count, by: unit.count).allSatisfy { Array(got[$0..<($0 + unit.count)]) == unit }
    }

    static let pronouns: Set<String> = ["я", "ты", "он", "она", "оно", "мы", "вы", "они"]

    /// Verb endings each subject pronoun agrees with (present/future and past).
    private static let agreement: [String: [String]] = [
        "я": ["ю", "у", "л", "ла"], "ты": ["ешь", "ёшь", "ишь", "л", "ла"],
        "он": ["ет", "ёт", "ит", "л"], "она": ["ет", "ёт", "ит", "ла"], "оно": ["ет", "ёт", "ит", "ло"],
        "мы": ["ем", "ём", "им", "ли"], "вы": ["ете", "ёте", "ите", "ли"], "они": ["ют", "ут", "ят", "ат", "ли"],
    ]

    /// "я читаю" → "читаю": a subject pronoun before a verb that agrees with it, which Russian
    /// may drop. nil for anything else ("ты кот" is not "кот"; "ты читаю" agrees with nothing).
    static func droppingSubjectPronoun(_ s: String) -> String? {
        let words = comparable(s).split(separator: " ").map(String.init)
        guard words.count == 2 || words.count == 3, let pronoun = words.first, let endings = agreement[pronoun] else { return nil }
        let verbIndex = words[1] == "не" && words.count == 3 ? 2 : 1
        let verb = words[verbIndex].replacingOccurrences(of: "ё", with: "е")
        guard verb.count > 2, endings.contains(where: { verb.hasSuffix($0.replacingOccurrences(of: "ё", with: "е")) }) else { return nil }
        return words.dropFirst().joined(separator: " ")
    }

    /// Written answers: the answer itself. With `subjectDrop` (translating from English, where
    /// "I am reading" may be "читаю" or "я читаю"), also with or without an agreeing subject
    /// pronoun. Never for dictation, where the utterance itself is the answer.
    static func answerMatches(_ heard: String, _ target: String, subjectDrop: Bool = false) -> Bool {
        if matches(heard, target) || matches(heard.lowercased(), target) { return true }
        guard subjectDrop else { return false }
        if let t = droppingSubjectPronoun(target), matches(heard, t) { return true }
        if let h = droppingSubjectPronoun(heard), matches(h, target) { return true }
        return false
    }

    // MARK: Audio identity

    /// Question words (and forms) that, opening a question, give it a falling contour (IK-2).
    static let questionWords: Set<String> = Set(("кто кого кому кем ком что чего чему чем чём где куда откуда когда почему зачем "
        + "отчего как какой какая какое какие какого каком какую каким каких какому чей чья чьё чьи сколько который "
        + "которая которое которые").split(separator: " ").map(String.init))
    private static let leadWords: Set<String> = Set("а и но ну так о об обо с со в во на к ко у от до из за по для про"
        .split(separator: " ").map(String.init))

    /// A question said with a rise (yes/no, "А ты?"): it ends in "?" and no question word opens
    /// it. Its recording differs from the statement with the same words; a question-word
    /// question falls like a statement. (Mirrors scripts/validate_course.py rising_question.)
    static func isRisingQuestion(_ s: String) -> Bool {
        let trimmed = unstressed(s).replacingOccurrences(of: "+", with: "")
            .trimmingCharacters(in: CharacterSet(charactersIn: "»\"”) \n"))
        guard trimmed.hasSuffix("?") else { return false }
        let body = String(trimmed.dropLast())
        let last = body.components(separatedBy: CharacterSet(charactersIn: ".!?…")).last ?? body
        var words = key(last).split(separator: " ").map(String.init)
        while let w = words.first, leadWords.contains(w) { words.removeFirst() }
        return !(words.first.map(questionWords.contains) ?? false)
    }

    /// The identity of a recording: `key` plus "?" for a rising question, so "Э́то чай?" and
    /// "Э́то чай." are different clips. (Mirrors validate_course.py audio_key.)
    static func audioKey(_ s: String) -> String { key(s) + (isRisingQuestion(s) ? "?" : "") }
}
