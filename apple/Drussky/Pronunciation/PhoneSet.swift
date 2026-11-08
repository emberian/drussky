import Foundation

/// What a sound can come out as, for scoring: the realisations that count as right, and the
/// specific mistakes worth naming (each tied to a practice contrast in pairs.json).
struct SoundOptions: Sendable {
    /// Canonical IPA strings that are correct here; the first is the textbook one.
    var good: [String]
    var errors: [Mistake]
}

struct Mistake: Sendable, Hashable {
    enum Kind: String, Sendable { case yi, softened, hardened, shch, sh, devoicing, voicing, reduction, fullVowel, rhotic, other }
    var ipa: String
    var kind: Kind
    /// pairs.json contrast id, when practice exists for it.
    var contrast: String? {
        switch kind {
        case .yi: "y-i"
        case .softened, .hardened: "hard-soft"
        case .shch, .sh: "sh-shch"
        case .devoicing, .voicing: "zh-sh"
        case .reduction, .fullVowel: "reduction"
        case .rhotic, .other: nil
        }
    }
}

/// The recognizer's symbol inventory (ZIPA's 127 IPA characters) and how canonical Russian
/// phones are spelled in it. ZIPA writes one Unicode scalar per token, so "tʲ" is [t, ʲ] and
/// "ɕː" is [ɕ, ː]; ɡ is written with ASCII g.
enum PhoneSet {
    static let paired: Set<String> = ["p", "b", "t", "d", "k", "ɡ", "f", "v", "s", "z", "x", "m", "n", "l", "r"]

    /// Ways the recognizer may spell a phone that are all the same sound to a Russian ear.
    static func spellings(_ ipa: String) -> [String] {
        switch ipa {
        case "l": return ["l", "l̴", "ɫ", "ɭ"]
        case "lʲ": return ["lʲ", "ʎ"]
        case "r": return ["r", "ɾ"]
        case "rʲ": return ["rʲ", "ɾʲ"]
        case "tʲ": return ["tʲ", "tsʲ", "ts", "tɕ"]
        case "dʲ": return ["dʲ", "dzʲ", "dz", "dʑ"]
        case "nʲ": return ["nʲ", "ɲ"]
        case "sʲ": return ["sʲ", "ɕ"]
        case "zʲ": return ["zʲ", "ʑ"]
        case "kʲ": return ["kʲ", "c"]
        case "ɡʲ": return ["ɡʲ", "ɟ"]
        case "xʲ": return ["xʲ", "ç"]
        case "ʂ": return ["ʂ", "ʃ"]
        case "ʐ": return ["ʐ", "ʒ"]
        case "ɕː": return ["ɕː", "ɕ", "ʃʲ", "ʃː", "ʃ"]
        case "tɕ": return ["tɕ", "tʃ", "tʂ", "ʈʂ"]
        case "ts": return ["ts"]
        case "j": return ["j"]
        case "x": return ["x", "χ"]
        case "ɣ": return ["ɣ", "ɡ"]
        case "a": return ["a", "ɑ", "ä"]
        case "o": return ["o", "ɔ"]
        case "u": return ["u", "ʊ"]
        case "e": return ["e", "ɛ"]
        case "ɛ": return ["ɛ", "e"]
        case "i": return ["i", "ɪ"]
        case "ɨ": return ["ɨ", "ɘ"]
        case "ɐ": return ["ɐ", "ʌ", "a", "ɑ"]
        case "ə": return ["ə", "ɐ", "ʌ", "a", "ɑ"]
        case "ɪ": return ["ɪ", "i", "e", "ɛ"]
        default: return [ipa]
        }
    }

    /// The options for one expected sound.
    static func options(for s: ExpectedSound) -> SoundOptions {
        var good = [s.ipa] + s.alsoFine
        var errors: [Mistake] = []
        let p = s.ipa
        func add(_ ipa: String, _ k: Mistake.Kind) { if !good.contains(ipa) { errors.append(Mistake(ipa: ipa, kind: k)) } }
        if s.isVowel {
            switch p {
            case "ɨ": add("i", .yi); add("u", .other)
            case "i": add("ɨ", .yi)
            case "ɐ", "ə":
                if s.underlying == "o" { add("o", .reduction) }
                if p == "ə" { good.append("ɪ") }
            case "ɪ":
                if s.underlying == "a" { add("a", .reduction) }
                if s.underlying == "e" { add("e", .fullVowel) }
                if s.underlying == "o" { add("o", .reduction) }
            case "o": add("a", .fullVowel)
            default: break
            }
            // Any other clear vowel, so a wrong vowel is named rather than read as missing.
            for v in ["a", "o", "u", "i", "ɨ", "e"] where !errors.contains(where: { $0.ipa == v }) { add(v, .other) }
            // Without a known stress the vowel's quality is open; only ы/и is still judged.
            if !s.stressKnown { errors.removeAll { $0.kind != .yi } }
        } else {
            if p.hasSuffix("ʲ"), paired.contains(String(p.dropLast())) {
                add(String(p.dropLast()), .hardened)
            } else if paired.contains(p) {
                if !["x"].contains(p) { add(p + "ʲ", .softened) }
                if p == "r" { add("ɹ", .rhotic); add("ʁ", .rhotic) }
            }
            switch p {
            case "ʂ": add("ɕː", .shch); if s.underlying == nil { add("ʐ", .voicing) }
            case "ɕː": add("ʂ", .sh)
            case "ʐ": add("ʂ", .devoicing)
            case "j": break
            default: break
            }
            // Voicing: a devoiced final/assimilated consonant said with the voice on (код as [kod]),
            // or a voiced one said voiceless.
            if let u = s.underlying, !s.alsoFine.contains(u) {
                add(u, RussianG2P.isVoiced(u) ? .voicing : .devoicing)
            }
        }
        // A good spelling must never be listed as a mistake.
        let goodSpellings = Set(good.flatMap(spellings))
        errors.removeAll { goodSpellings.contains($0.ipa) }
        return SoundOptions(good: good, errors: errors)
    }

    /// Phones the free decode is read back into, longest spelling first.
    static let inventory: [String] = {
        let base = ["p", "b", "t", "d", "k", "ɡ", "f", "v", "s", "z", "x", "m", "n", "l", "r"]
        return base + base.map { $0 + "ʲ" } + ["ʂ", "ʐ", "ɕː", "tɕ", "ts", "j", "ɣ", "a", "o", "u", "e", "ɛ", "i", "ɨ", "ɐ", "ə", "ɪ", "ɹ", "ʁ", "w"]
    }()

    /// Rough articulatory distance between two canonical phones, 0 (same) … 1, for aligning a
    /// free decode against the expected sounds.
    static func distance(_ a: String, _ b: String) -> Double {
        if a == b { return 0 }
        if Set(spellings(a)).intersection(spellings(b)).count > 0 { return 0.1 }
        let va = isVowel(a), vb = isVowel(b)
        if va != vb { return a == "j" || b == "j" ? 0.7 : 1 }
        if va {
            let fa = vowelFeatures[a] ?? (0.5, 0.5, false), fb = vowelFeatures[b] ?? (0.5, 0.5, false)
            return min(1, 0.15 + abs(fa.0 - fb.0) * 0.5 + abs(fa.1 - fb.1) * 0.5 + (fa.2 != fb.2 ? 0.15 : 0))
        }
        let sa = a.hasSuffix("ʲ"), sb = b.hasSuffix("ʲ")
        let ba = sa ? String(a.dropLast()) : a, bb = sb ? String(b.dropLast()) : b
        if ba == bb { return 0.35 }   // differ only in softness
        let pa = place[ba] ?? 0.5, pb = place[bb] ?? 0.5
        let ma = manner[ba] ?? 0.5, mb = manner[bb] ?? 0.5
        let voice = RussianG2P.isVoiced(ba) != RussianG2P.isVoiced(bb) ? 0.15 : 0
        return min(1, 0.2 + abs(pa - pb) * 0.4 + abs(ma - mb) * 0.4 + voice + (sa != sb ? 0.1 : 0))
    }

    static func isVowel(_ p: String) -> Bool { vowelFeatures[p] != nil }
    /// (height 0 low…1 high, backness 0 front…1 back, rounded)
    static let vowelFeatures: [String: (Double, Double, Bool)] = [
        "i": (1, 0, false), "ɪ": (0.85, 0.15, false), "ɨ": (1, 0.6, false), "u": (1, 1, true), "ʊ": (0.85, 0.9, true),
        "e": (0.65, 0.05, false), "ɛ": (0.5, 0.1, false), "ə": (0.5, 0.5, false), "o": (0.6, 1, true), "ɔ": (0.45, 1, true),
        "ɐ": (0.35, 0.5, false), "a": (0, 0.4, false), "ɑ": (0, 0.9, false), "ʌ": (0.35, 0.8, false), "y": (1, 0, true),
    ]
    static let place: [String: Double] = [
        "p": 0, "b": 0, "m": 0, "f": 0.1, "v": 0.1, "w": 0.05, "t": 0.3, "d": 0.3, "n": 0.3, "s": 0.32, "z": 0.32, "ts": 0.32,
        "l": 0.33, "r": 0.35, "ɹ": 0.4, "ʂ": 0.45, "ʐ": 0.45, "ɕː": 0.55, "tɕ": 0.55, "j": 0.65, "k": 0.8, "ɡ": 0.8,
        "x": 0.85, "ɣ": 0.85, "ʁ": 0.95,
    ]
    static let manner: [String: Double] = [
        "p": 0, "b": 0, "t": 0, "d": 0, "k": 0, "ɡ": 0, "ts": 0.25, "tɕ": 0.25, "f": 0.5, "v": 0.5, "s": 0.5, "z": 0.5,
        "ʂ": 0.5, "ʐ": 0.5, "ɕː": 0.5, "x": 0.5, "ɣ": 0.5, "m": 0.75, "n": 0.75, "l": 0.9, "r": 0.85, "ɹ": 0.95,
        "ʁ": 0.7, "j": 1, "w": 1,
    ]
}
