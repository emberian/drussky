import Foundation

/// At most two pieces of advice, most useful first, each naming the sound, the word, what
/// happened and what to do, with the pairs.json contrast that practises it.
enum Tips {
    static func make(target: String, expected: Transcription, sounds: [PhoneJudgement],
                     mistakes: [(Mistake?, Float)], stress: [Prosody.Finding]) -> [PronunciationTip] {
        let chars = Array(target)
        let words = RussianG2P.split(target)
        func text(_ r: Range<Int>) -> String {
            String(String(chars[max(0, r.lowerBound)..<min(chars.count, r.upperBound)]).lowercased().unicodeScalars.filter { $0 != RussianG2P.acute })
        }
        func word(_ w: Int) -> String {
            guard w < words.count, let a = words[w].first?.range.lowerBound, let b = words[w].last?.range.upperBound else { return target }
            return String(chars[a..<b]).lowercased()
        }
        var candidates: [(Float, PronunciationTip)] = []
        // Stress first when it moved: it changes the word (за́мок / замо́к).
        for f in stress {
            let syl = expected.syllables
            let said = syl[f.heard].text, should = syl[f.expected].text
            let msg = "In «\(word(f.word))» the stress fell on «\(plain(said))». Put it on «\(should)»: make that vowel longer and louder, and let the others shrink."
            candidates.append((10 + f.strength, PronunciationTip(message: msg, contrast: "stress", letterRange: syl[f.expected].letters)))
        }
        var e = 0
        for j in sounds {
            guard let exp = j.expected else { continue }
            defer { e += 1 }
            guard e < expected.sounds.count, e < mistakes.count else { continue }
            let s = expected.sounds[e]
            let (mistake, margin) = mistakes[e]
            let l = text(s.letters), w = word(s.word)
            switch j.verdict {
            case .substituted:
                // Advice only for a clear call: a full nat past the verdict threshold.
                guard let m = mistake, margin < -1 else { continue }
                candidates.append((-margin + weight(m.kind) + (s.isVowel ? 0.5 : 0), PronunciationTip(message: message(m, letters: l, word: w, expected: exp.ipa, sound: s),
                                                                              contrast: m.contrast, letterRange: s.letters)))
            case .missing where margin < -6:
                candidates.append((-margin * 0.7, PronunciationTip(message: "I couldn’t hear the «\(l)» in «\(w)». Make sure it’s there, even if it’s short.",
                                                                   contrast: nil, letterRange: s.letters)))
            default:
                break
            }
        }
        for j in sounds where j.verdict == .extra {
            guard let h = j.heard else { continue }
            candidates.append((1, PronunciationTip(message: "There was an extra vowel ([\(h.ipa)]) in there. Run the consonants together without a vowel between them.",
                                                   contrast: nil, letterRange: j.letterRange)))
        }
        var out: [PronunciationTip] = []
        var seen = Set<String>()
        for (_, tip) in candidates.sorted(by: { $0.0 > $1.0 }) {
            let key = tip.contrast ?? tip.message
            if seen.contains(key) { continue }
            seen.insert(key)
            out.append(tip)
            if out.count == 2 { break }
        }
        return out
    }

    private static func plain(_ s: String) -> String { String(s.unicodeScalars.filter { $0 != RussianG2P.acute }) }

    /// Contrasts English speakers find hardest count for a little more.
    static func weight(_ k: Mistake.Kind) -> Float {
        switch k {
        case .yi, .hardened, .softened, .shch, .sh: 2
        case .reduction, .fullVowel: 1.5
        case .rhotic: 1
        case .devoicing, .voicing: 1
        case .other: 0
        }
    }

    static func message(_ m: Mistake, letters l: String, word w: String, expected: String, sound s: ExpectedSound) -> String {
        switch m.kind {
        case .yi where expected == "ɨ" || expected == "ɨ":
            return "Your «\(l)» in «\(w)» came out like и. Keep the lips spread, but pull the whole tongue back, as if starting to say у."
        case .yi:
            return "Your «\(l)» in «\(w)» came out like ы. Push the tongue forward and up, and soften the consonant before it."
        case .hardened:
            return "The «\(l)» in «\(w)» came out hard. Raise the middle of your tongue toward the roof of your mouth while you say it, as if a й were blended in."
        case .softened:
            return "The «\(l)» in «\(w)» came out soft. Keep the middle of the tongue low, so no й slips in after it."
        case .shch:
            return "Your «\(l)» in «\(w)» sounded soft, like щ. For ш keep the tongue low and cupped back."
        case .sh:
            return "Your «\(l)» in «\(w)» sounded hard, like ш. For щ raise the tongue forward and make it long and soft, like a drawn-out sh in “sheep”."
        // Named by what was heard: .voicing = voice on where it should be off, .devoicing = the reverse.
        case .voicing where s.underlying != nil:
            return "The «\(l)» in «\(w)» should sound like [\(expected)]: the voice switches off at the end of a word and before a voiceless consonant."
        case .voicing:
            return "Your «\(l)» in «\(w)» came out voiced, like ж. Keep the voice off: ш is only a hiss."
        case .devoicing where s.underlying != nil:
            return "The «\(l)» in «\(w)» takes on the voice of the consonant after it: say it as [\(expected)]."
        case .devoicing:
            return "Your «\(l)» in «\(w)» lost its voice. Hum through it: ж is ш with the voice on."
        case .reduction where s.underlying == "o":
            return "The unstressed о in «\(w)» came out as a full о. Only the stressed о sounds like о; this one is a short а."
        case .reduction:
            return "The unstressed «\(l)» in «\(w)» came out too full. Unstressed я and е shrink to a short, и-like sound."
        case .fullVowel where s.stressed:
            return "The stressed «\(l)» in «\(w)» came out reduced. Give it its full sound: it carries the word."
        case .fullVowel:
            return "The unstressed «\(l)» in «\(w)» came out too full. Let it shrink to a short и-like sound."
        case .rhotic:
            return "Your р in «\(w)» sounded like an English or French r. Tap the tip of the tongue once (or roll it) just behind the top teeth."
        case .other:
            return "The «\(l)» in «\(w)» came out like [\(m.ipa)] instead of [\(expected)]."
        }
    }
}
