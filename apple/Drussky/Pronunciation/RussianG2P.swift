import Foundation

/// One sound the target should contain, from `RussianG2P`.
struct ExpectedSound: Sendable, Hashable {
    /// Canonical IPA (espeak/ZIPA style): "tʲ", "ɐ", "ɕː", "ʂ"…
    var ipa: String
    /// Character offsets in the target string (a stress-marked letter is one Character).
    var letters: Range<Int>
    var word: Int
    /// Index into the target's syllables, nil for a consonant-only word such as в or к.
    var syllable: Int?
    var isVowel: Bool
    var stressed: Bool = false
    /// The vowel or consonant the spelling asks for before reduction or devoicing, when it
    /// differs: "o" for the ɐ of дома́, "d" for the t of код. Scoring uses it to name the
    /// learner's likely error (saying the spelling).
    var underlying: String? = nil
    /// Other realisations that are just as correct here (unknown stress, optional
    /// assimilation across a word boundary, soft-before-soft clusters).
    var alsoFine: [String] = []
    /// Natives often drop it (the й of a final -ый/-ий, the в of вс-): never judged missing.
    var optional = false
    /// The word's stress is marked, ё, or the word has one syllable.
    var stressKnown = true
}

struct ExpectedSyllable: Sendable, Hashable {
    var text: String
    var letters: Range<Int>
    var word: Int
    var stressed: Bool
    /// Whether this word's stress is known (marked, ё, or one syllable) and it is a content word.
    var stressKnown: Bool
}

struct Transcription: Sendable {
    var sounds: [ExpectedSound]
    var syllables: [ExpectedSyllable]
    var ipa: String { sounds.map(\.ipa).joined() }
}

/// Russian spelling → phones, with stress-dependent vowel reduction, palatalisation, final
/// devoicing and regressive voicing assimilation (standard Moscow pronunciation, the norm
/// learners' materials and the ZIPA training data follow).
enum RussianG2P {
    static let acute: Unicode.Scalar = "\u{301}"
    static let vowelLetters: Set<Character> = ["а", "о", "у", "ы", "э", "я", "ё", "ю", "и", "е"]
    static let iotated: Set<Character> = ["я", "ё", "ю", "е"]
    static let softening: Set<Character> = ["е", "ё", "и", "ю", "я", "ь"]
    static let alwaysHard: Set<Character> = ["ж", "ш", "ц"]
    static let alwaysSoft: Set<Character> = ["ч", "щ", "й"]
    static let consonantPhone: [Character: String] = [
        "б": "b", "в": "v", "г": "ɡ", "д": "d", "ж": "ʐ", "з": "z", "й": "j", "к": "k", "л": "l", "м": "m",
        "н": "n", "п": "p", "р": "r", "с": "s", "т": "t", "ф": "f", "х": "x", "ц": "ts", "ч": "tɕ", "ш": "ʂ", "щ": "ɕː",
    ]
    /// Unstressed function words; one-letter prepositions join the next word's phonology.
    static let clitics: Set<String> = [
        "в", "во", "к", "ко", "с", "со", "у", "о", "об", "обо", "на", "за", "по", "до", "из", "от", "без", "для", "под",
        "над", "при", "про", "через", "не", "ни", "и", "а", "но", "да", "же", "ли", "бы", "то",
    ]
    static let proclitics: Set<String> = [
        "в", "во", "к", "ко", "с", "со", "у", "о", "об", "обо", "на", "за", "по", "до", "из", "от", "без", "для", "под",
        "над", "при", "про", "через", "не", "ни",
    ]
    /// Whole-word respellings (stress marks are kept from the target).
    static let respell: [String: String] = [
        "что": "што", "чтобы": "штобы", "ничто": "ништо", "конечно": "конешно", "скучно": "скушно",
        "нарочно": "нарошно", "яичница": "яишница", "его": "ево", "сегодня": "севодня", "сегодняшний": "севодняшний",
        "солнце": "сонце", "сейчас": "сечас", "пожалуйста": "пожалуста", "здравствуй": "здраствуй", "здравствуйте": "здраствуйте", "чувство": "чуство",
        "лестница": "лесница", "счастье": "щастье", "счастливый": "щасливый", "мягкий": "мяхкий", "легко": "лехко",
        "лёгкий": "лёхкий", "бог": "бох", "ого": "ого",
    ]
    static let ogoExceptions: Set<String> = ["много", "немного", "строго", "дорого", "убого", "полого", "отлого", "нестрого", "ого", "того-то"]

    /// One letter of a (respelled) word: the base letter, where it came from, whether it carries the stress mark.
    struct Letter { var ch: Character; var range: Range<Int>; var marked: Bool }

    static func transcribe(_ text: String) -> Transcription {
        let words = split(text)
        var sounds: [ExpectedSound] = []
        var syllables: [ExpectedSyllable] = []
        for (w, word) in words.enumerated() {
            let plain = String(word.map(\.ch))
            let isClitic = clitics.contains(plain) && !word.contains(where: \.marked)
            let letters = respelled(word)
            let vowelIdx = letters.indices.filter { vowelLetters.contains(letters[$0].ch) }
            var stressAt: Int? = letters.firstIndex(where: \.marked)
            if stressAt == nil, let yo = letters.firstIndex(where: { $0.ch == "ё" }) { stressAt = yo }
            if stressAt == nil, vowelIdx.count == 1, !isClitic { stressAt = vowelIdx[0] }
            let stressKnown = stressAt != nil && !isClitic
            // Syllables: one per vowel letter; boundaries before the last consonant of a cluster,
            // after a sonorant that closes the syllable (по́лка = пол-ка).
            var syllableOf = [Int](repeating: 0, count: letters.count)
            var bounds: [Int] = [0]
            for k in 1..<max(1, vowelIdx.count) {
                let a = vowelIdx[k - 1], b = vowelIdx[k]
                let cluster = Array((a + 1)..<b)
                var cut = a + 1
                let cons = cluster.filter { !["ь", "ъ"].contains(letters[$0].ch) }
                if cons.count >= 2, ["л", "м", "н", "р", "й"].contains(letters[cons[0]].ch),
                   !["л", "м", "н", "р", "й"].contains(letters[cons[1]].ch) {
                    cut = cons[0] + 1
                    while cut < b && ["ь", "ъ"].contains(letters[cut].ch) { cut += 1 }
                }
                bounds.append(cut)
            }
            for (s, start) in bounds.enumerated() {
                let end = s + 1 < bounds.count ? bounds[s + 1] : letters.count
                for i in start..<end { syllableOf[i] = s }
            }
            let syllableBase = syllables.count
            if !vowelIdx.isEmpty {
                for (s, start) in bounds.enumerated() {
                    let end = s + 1 < bounds.count ? bounds[s + 1] : letters.count
                    let lo = letters[start].range.lowerBound, hi = letters[end - 1].range.upperBound
                    let chars = Array(text)
                    syllables.append(ExpectedSyllable(
                        text: String(chars[lo..<hi]), letters: lo..<hi, word: w,
                        stressed: stressKnown && (stressAt.map { syllableOf[$0] == s } ?? false), stressKnown: stressKnown))
                }
            }
            let stressSyl = stressAt.map { syllableOf[$0] }
            var prevVowel = true   // word start behaves like after a vowel for iotation
            var i = 0
            while i < letters.count {
                let L = letters[i]
                let syl: Int? = vowelIdx.isEmpty ? nil : syllableBase + syllableOf[i]
                if vowelLetters.contains(L.ch) {
                    let afterSign = i > 0 && ["ь", "ъ"].contains(letters[i - 1].ch)
                    let iot = iotated.contains(L.ch) && (prevVowel || afterSign) || (L.ch == "и" && afterSign)
                    if iot { sounds.append(ExpectedSound(ipa: "j", letters: L.range, word: w, syllable: syl, isVowel: false)) }
                    let last = sounds.last.flatMap { $0.word == w && !$0.isVowel ? $0 : nil }
                    let afterSoft = iot || last.map { $0.ipa.hasSuffix("ʲ") || ["j", "tɕ", "ɕː"].contains($0.ipa) } ?? false
                    let afterHardSibilant = last.map { ["ʐ", "ʂ", "ts"].contains($0.ipa) } ?? false
                    let mySyl = syllableOf[i]
                    let rel: VowelPosition
                    if isClitic {
                        rel = .pretonic1
                    } else if let ss = stressSyl {
                        rel = mySyl == ss ? .stressed : (mySyl == ss - 1 ? .pretonic1 : (mySyl < ss ? .pretonicFar : .posttonic))
                    } else {
                        rel = .unknown
                    }
                    var v = vowel(L.ch, position: rel, afterSoft: afterSoft, afterHardSibilant: afterHardSibilant,
                                  wordInitial: i == 0, wordFinal: i == letters.count - 1)
                    if isClitic, let u = v.underlying { v.alsoFine.append(u == "e" ? "e" : u) }
                    v.letters = L.range; v.word = w; v.syllable = syl; v.stressKnown = stressKnown || isClitic
                    if v.ipa == "i" && !v.stressed { v.alsoFine.append("e") }
                    sounds.append(v)
                    prevVowel = true
                } else if let base = consonantPhone[L.ch] {
                    var ipa = base
                    var range = L.range
                    // A doubled consonant is one (long) sound: касса, ванна.
                    var j = i
                    while j + 1 < letters.count && letters[j + 1].ch == L.ch && L.ch != "й" { j += 1; range = range.lowerBound..<letters[j].range.upperBound }
                    let after: Character? = j + 1 < letters.count ? letters[j + 1].ch : nil
                    if !alwaysHard.contains(L.ch) && !alwaysSoft.contains(L.ch), let a = after, softening.contains(a) {
                        ipa = base + "ʲ"
                        if a == "ь" { range = range.lowerBound..<letters[j + 1].range.upperBound }
                    }
                    if (L.ch == "ч" || L.ch == "щ" || alwaysHard.contains(L.ch)), after == "ь" {
                        range = range.lowerBound..<letters[j + 1].range.upperBound
                    }
                    sounds.append(ExpectedSound(ipa: ipa, letters: range, word: w, syllable: syl, isVowel: false))
                    prevVowel = false
                    i = j
                } else {
                    // ь ъ: no sound of their own (softness/iotation handled above).
                }
                i += 1
            }
        }
        assimilate(&sounds, words: words)
        softBeforeSoft(&sounds)
        mergeAndMarkOptional(&sounds)
        return Transcription(sounds: sounds, syllables: syllables)
    }

    enum VowelPosition { case stressed, pretonic1, pretonicFar, posttonic, unknown }

    static func vowel(_ ch: Character, position: VowelPosition, afterSoft: Bool, afterHardSibilant: Bool,
                      wordInitial: Bool, wordFinal: Bool) -> ExpectedSound {
        // Underlying quality from the letter.
        let u: String
        switch ch {
        case "а", "я": u = "a"
        case "о", "ё": u = "o"
        case "у", "ю": u = "u"
        case "ы": u = "ɨ"
        case "и": u = afterHardSibilant ? "ɨ" : "i"   // жить [ʐɨtʲ]
        case "э": u = "e"
        default: u = "e"   // е
        }
        var s = ExpectedSound(ipa: u, letters: 0..<0, word: 0, syllable: nil, isVowel: true)
        switch position {
        case .stressed:
            s.stressed = true
            if u == "e" { s.ipa = afterSoft ? "e" : "ɛ"; s.alsoFine = ["ɛ", "e"] }
        case .unknown:
            // Stress not marked: accept both the full and the reduced vowel.
            switch u {
            case "a", "o": s.alsoFine = afterSoft ? ["ɪ", "i", "a", "o"] : ["ɐ", "ə", "a", "o"]
            case "e": s.alsoFine = ["ɪ", "i", "ɛ", "e", "ɨ"]
            default: break
            }
        case .pretonic1, .pretonicFar, .posttonic:
            let near = position == .pretonic1 || wordInitial
            switch u {
            case "a", "o":
                if afterSoft {
                    // часы́ [tɕɪˈsɨ], пятно́ [pʲɪtˈno]; a final unstressed я is [ə]: неде́ля.
                    s.ipa = wordFinal && position == .posttonic && u == "a" ? "ə" : "ɪ"
                } else {
                    // First syllable before the stress, or the very start of the word: [ɐ]; else [ə].
                    s.ipa = near ? "ɐ" : "ə"
                }
                s.underlying = u
            case "e":
                s.ipa = afterHardSibilant ? "ɨ" : "ɪ"   // жена́ [ʐɨˈna], река́ [rʲɪˈka]
                s.underlying = "e"
            default:
                break
            }
        }
        return s
    }

    /// Words of the target as letters with Character offsets; punctuation and spaces dropped,
    /// a hyphen joins (кто-то is one word).
    static func split(_ text: String) -> [[Letter]] {
        var words: [[Letter]] = []
        var cur: [Letter] = []
        for (k, c) in text.enumerated() {
            let marked = c.unicodeScalars.contains(acute)
            let base = Character(String(String(c).lowercased().unicodeScalars.filter { $0 != acute && $0 != "\u{300}" }))
            if base == "ё" || base.isLetter && ("а"..."я").contains(String(base)) {
                cur.append(Letter(ch: base, range: k..<(k + 1), marked: marked))
            } else if base == "-" && !cur.isEmpty {
                continue
            } else if !cur.isEmpty {
                words.append(cur); cur = []
            }
        }
        if !cur.isEmpty { words.append(cur) }
        return words
    }

    /// Applies whole-word respellings and the regular cluster simplifications, keeping each
    /// output letter's source range and stress mark.
    static func respelled(_ word: [Letter]) -> [Letter] {
        let plain = String(word.map(\.ch))
        var w = word
        if let r = respell[plain] { w = mapRespelling(word, to: r) }
        else if plain.count > 3, plain.hasSuffix("ого") || plain.hasSuffix("его"), !ogoExceptions.contains(plain) {
            // -ого/-его endings of adjectives and pronouns: г is в.
            w[w.count - 2].ch = "в"
        }
        // Reflexive -тся/-ться: one ц.
        let s = String(w.map(\.ch))
        for suf in ["ться", "тся"] where s.hasSuffix(suf) {
            let n = suf.count
            let start = w.count - n
            let ts = Letter(ch: "ц", range: w[start].range.lowerBound..<w[w.count - 2].range.upperBound, marked: false)
            let a = Letter(ch: "а", range: w[w.count - 1].range, marked: w[w.count - 1].marked)
            w = Array(w[..<start]) + [ts, a]
            break
        }
        // Cluster simplifications: сч/зч/жч → щ; здн, стн, стл, лнц, рдц, вств; гк/гч → хк/хч.
        var out: [Letter] = []
        var i = 0
        func at(_ k: Int) -> Character? { k < w.count ? w[k].ch : nil }
        while i < w.count {
            let c = w[i].ch
            if ["с", "з", "ж"].contains(c), at(i + 1) == "ч" {
                out.append(Letter(ch: "щ", range: w[i].range.lowerBound..<w[i + 1].range.upperBound, marked: false)); i += 2; continue
            }
            if (c == "з" && at(i + 1) == "д" && at(i + 2) == "н") || (c == "с" && at(i + 1) == "т" && (at(i + 2) == "н" || at(i + 2) == "л")) {
                out.append(Letter(ch: c, range: w[i].range.lowerBound..<w[i + 1].range.upperBound, marked: false)); i += 2; continue
            }
            if (c == "л" && at(i + 1) == "н" && at(i + 2) == "ц") || (c == "р" && at(i + 1) == "д" && at(i + 2) == "ц") {
                // солнце → сонце is a respelling; рдц → рц (сердце)
                if c == "р" { out.append(w[i]); out.append(Letter(ch: "ц", range: w[i + 1].range.lowerBound..<w[i + 2].range.upperBound, marked: false)); i += 3; continue }
            }
            if c == "в", at(i + 1) == "с", at(i + 2) == "т", at(i + 3) == "в" {
                out.append(Letter(ch: "в", range: w[i].range, marked: false))
                out.append(Letter(ch: "с", range: w[i + 1].range.lowerBound..<w[i + 2].range.upperBound, marked: false))
                i += 3; continue
            }
            if c == "д" || c == "т", at(i + 1) == "ц" {
                // два́дцать: дц is one ц.
                out.append(Letter(ch: "ц", range: w[i].range.lowerBound..<w[i + 1].range.upperBound, marked: false)); i += 2; continue
            }
            if c == "г", at(i + 1) == "к" || at(i + 1) == "ч" {
                out.append(Letter(ch: "х", range: w[i].range, marked: w[i].marked)); i += 1; continue
            }
            out.append(w[i]); i += 1
        }
        return out
    }

    /// Maps a respelling onto the original letters by edit alignment (a dropped letter joins
    /// the previous output letter's range, an added one borrows its neighbour's), carrying
    /// stress marks by vowel order.
    static func mapRespelling(_ word: [Letter], to r: String) -> [Letter] {
        let rs = Array(r), ws = word.map(\.ch)
        let n = ws.count, m = rs.count
        var d = [[Int]](repeating: [Int](repeating: 0, count: m + 1), count: n + 1)
        for i in 0...n { d[i][0] = i }
        for j in 0...m { d[0][j] = j }
        if n > 0 && m > 0 {
            for i in 1...n { for j in 1...m { d[i][j] = min(d[i - 1][j - 1] + (ws[i - 1] == rs[j - 1] ? 0 : 1), d[i - 1][j] + 1, d[i][j - 1] + 1) } }
        }
        var source = [Range<Int>?](repeating: nil, count: m)
        var i = n, j = m
        var pendingDrop: Range<Int>? = nil
        while i > 0 || j > 0 {
            if i > 0 && j > 0 && d[i][j] == d[i - 1][j - 1] + (ws[i - 1] == rs[j - 1] ? 0 : 1) {
                var r = word[i - 1].range
                if let p = pendingDrop { r = r.lowerBound..<p.upperBound; pendingDrop = nil }
                source[j - 1] = r; i -= 1; j -= 1
            } else if i > 0 && d[i][j] == d[i - 1][j] + 1 {
                pendingDrop = word[i - 1].range.lowerBound..<(pendingDrop?.upperBound ?? word[i - 1].range.upperBound); i -= 1
            } else {
                j -= 1
            }
        }
        for k in 0..<m where source[k] == nil { source[k] = (k > 0 ? source[k - 1] : nil) ?? word[0].range }
        let markedVowel = word.filter { vowelLetters.contains($0.ch) }.firstIndex(where: \.marked)
        var vowelCount = 0
        return rs.enumerated().map { k, ch in
            var marked = false
            if vowelLetters.contains(ch) { marked = markedVowel == vowelCount; vowelCount += 1 }
            return Letter(ch: ch, range: source[k]!, marked: marked)
        }
    }

    static let voicedToVoiceless: [String: String] = [
        "b": "p", "v": "f", "ɡ": "k", "d": "t", "z": "s", "ʐ": "ʂ",
        "bʲ": "pʲ", "vʲ": "fʲ", "ɡʲ": "kʲ", "dʲ": "tʲ", "zʲ": "sʲ",
    ]
    static var voicelessToVoiced: [String: String] {
        var d = Dictionary(uniqueKeysWithValues: voicedToVoiceless.map { ($1, $0) })
        d["x"] = "ɣ"; d["ts"] = "dz"; d["tɕ"] = "dʑ"
        return d
    }
    static func isObstruent(_ p: String) -> Bool {
        voicedToVoiceless[p] != nil || voicelessToVoiced[p] != nil || p == "ɕː"
    }
    static func isVoiced(_ p: String) -> Bool { voicedToVoiceless[p] != nil || ["ɣ", "dz", "dʑ"].contains(p) }

    /// Final devoicing and regressive voicing assimilation. A proclitic (в, с, из…) is one
    /// word with what follows; elsewhere a word boundary is a pause for devoicing, with the
    /// voiced form also accepted before a voiced obstruent (как дела́ may be [kaɡ dʲɪˈla]).
    static func assimilate(_ s: inout [ExpectedSound], words: [[Letter]]) {
        let isProclitic = words.map { proclitics.contains(String($0.map(\.ch))) }
        var i = s.count - 1
        while i >= 0 {
            defer { i -= 1 }
            guard !s[i].isVowel, isObstruent(s[i].ipa) else { continue }
            let p = s[i].ipa
            // Next sound and whether a word boundary (not a proclitic one) lies between.
            let nextIdx = i + 1 < s.count ? i + 1 : nil
            let boundary = nextIdx.map { s[$0].word != s[i].word } ?? true
            let joined = boundary && nextIdx != nil && isProclitic[s[i].word]
            let n = nextIdx.map { s[$0] }
            if boundary && !joined {
                if let v = voicedToVoiceless[p] {
                    s[i].ipa = v; s[i].underlying = p
                    if let n, !n.isVowel, isObstruent(n.ipa), isVoiced(n.ipa), !n.ipa.hasPrefix("v") { s[i].alsoFine.append(p) }
                }
                continue
            }
            guard let n, !n.isVowel, isObstruent(n.ipa) else { continue }
            if isVoiced(n.ipa) && !n.ipa.hasPrefix("v") {
                if let v = voicelessToVoiced[p] { s[i].ipa = v; s[i].underlying = p }
            } else if !isVoiced(n.ipa) {
                if let v = voicedToVoiceless[p] { s[i].ipa = v; s[i].underlying = p }
            }
        }
    }

    /// Two identical consonants in a row (французский: з+с → s s) are one sound; a final
    /// unstressed -ый/-ий й and a word-initial в before a voiceless consonant (всем) are optional.
    static func mergeAndMarkOptional(_ s: inout [ExpectedSound]) {
        var i = 1
        while i < s.count {
            if !s[i].isVowel, s[i].ipa == s[i - 1].ipa, s[i].word == s[i - 1].word, s[i].ipa != "j" {
                s[i - 1].letters = s[i - 1].letters.lowerBound..<s[i].letters.upperBound
                s.remove(at: i)
            } else { i += 1 }
        }
        for k in s.indices {
            let lastInWord = k == s.count - 1 || s[k + 1].word != s[k].word
            if s[k].ipa == "j", lastInWord, k > 0, s[k - 1].isVowel, !s[k - 1].stressed { s[k].optional = true }
            let firstInWord = k == 0 || s[k - 1].word != s[k].word
            if s[k].ipa == "f", s[k].underlying == "v", firstInWord { s[k].optional = true }
            // A final soft labial is only lightly soft in ordinary speech (во́семь, семь, вновь).
            if lastInWord, ["mʲ", "pʲ", "bʲ", "fʲ", "vʲ"].contains(s[k].ipa) { s[k].alsoFine.append(String(s[k].ipa.dropLast())) }
        }
    }

    /// A dental before a soft dental is often softened too (есть [jesʲtʲ], сне́г [sʲnʲek]):
    /// accept either.
    static func softBeforeSoft(_ s: inout [ExpectedSound]) {
        let dentals: Set<String> = ["s", "z", "t", "d", "n"]
        for i in s.indices.dropLast() where !s[i].isVowel && dentals.contains(s[i].ipa) {
            let n = s[i + 1]
            if !n.isVowel, n.ipa.hasSuffix("ʲ"), dentals.contains(String(n.ipa.dropLast())) || n.ipa == "lʲ" {
                s[i].alsoFine.append(s[i].ipa + "ʲ")
            }
        }
    }
}
