import Foundation

/// The analyzer the app uses until (and wherever) phone-level analysis isn't available: the
/// words the Russian recognizer heard, aligned letter by letter with the target, plus the
/// syllable-by-syllable comparison with the model (`ProsodyComparison`).
///
/// It can say a letter came out as another letter the recognizer wrote (был heard as бил), and
/// where the stress and the climb of the voice fell; it cannot vouch for each sound, so its
/// reports carry no heard phones (`isWordLevel`) and the page doesn't colour letters it
/// couldn't hear as right.
struct WordLevelAnalyzer: PronunciationAnalyzer {
    /// Words heard in a take, given the target as a hint. nil when nothing was recognised.
    var transcribe: @Sendable (URL, String) async -> String?

    init(transcribe: @escaping @Sendable (URL, String) async -> String? = { url, hint in await SpeechCheck.transcribe(url, hint: hint) }) {
        self.transcribe = transcribe
    }

    struct Unreadable: Error {}

    func analyze(take: URL, target: String, model: URL?) async throws -> PronunciationReport {
        let hint = RussianText.unstressed(target)
        async let heard = transcribe(take, hint)
        let comparison = try await Task.detached(priority: .userInitiated) { () throws -> ProsodyComparison? in
            guard let x = Pitch.samples(take), !x.isEmpty else { throw Unreadable() }
            return ProsodyComparison.compute(target: target, take: x, model: model.flatMap(Pitch.samples))
        }.value
        return Self.report(target: target, transcript: await heard, comparison: comparison)
    }

    static func report(target: String, transcript: String?, comparison: ProsodyComparison?) -> PronunciationReport {
        let sounds = transcript.map { letters(target: target, heard: $0) } ?? []
        let syllables: [SyllableJudgement] = comparison.map { c in
            c.syllables.indices.map { i in
                SyllableJudgement(text: c.syllables[i].text, isStressed: c.syllables[i].isStressed,
                                  start: c.take[i].start, end: c.take[i].end, prominence: c.take[i].prominence)
            }
        } ?? Syllables.split(target).map { SyllableJudgement(text: $0.text, isStressed: $0.isStressed) }
        let judged = sounds.filter { $0.verdict != .extra }
        let good = judged.filter { $0.verdict == .good || $0.verdict == .close }.count
        return PronunciationReport(
            target: target, transcript: transcript, expected: judged.compactMap(\.expected), heard: [],
            sounds: sounds, syllables: syllables, stressedSyllable: comparison?.takeStress,
            accuracy: judged.isEmpty ? 0 : Double(good) / Double(judged.count),
            tips: tips(target: target, sounds: sounds, comparison: comparison),
            speechStart: comparison?.takeSpeech?.lowerBound, speechEnd: comparison?.takeSpeech?.upperBound, provenance: .words)
    }

    // MARK: Letters

    /// Target letters against the recognised words. Empty when the words are too far apart to
    /// be the same thing said differently (another word altogether says nothing per letter).
    static func letters(target: String, heard: String) -> [PhoneJudgement] {
        let chars = Array(target)
        let expected: [(offset: Int, letter: Character)] = chars.indices.compactMap { i in
            Syllables.isLetter(chars[i]) ? (i, fold(Syllables.base(chars[i]))) : nil
        }
        let got: [Character] = RussianText.key(heard).filter { Syllables.isLetter($0) }.map { fold($0) }
        guard !expected.isEmpty, !got.isEmpty else { return [] }
        let a = expected.map(\.letter)
        let n = a.count, m = got.count
        var d = Array(repeating: Array(repeating: 0, count: m + 1), count: n + 1)
        for i in 0...n { d[i][0] = i }
        for j in 0...m { d[0][j] = j }
        for i in 1...n { for j in 1...m {
            d[i][j] = min(d[i - 1][j] + 1, d[i][j - 1] + 1, d[i - 1][j - 1] + (a[i - 1] == got[j - 1] ? 0 : 1))
        } }
        guard d[n][m] <= max(1, n / 2) else { return [] }
        var out: [PhoneJudgement] = []
        var i = n, j = m
        while i > 0 || j > 0 {
            if i > 0 && j > 0 && d[i][j] == d[i - 1][j - 1] + (a[i - 1] == got[j - 1] ? 0 : 1) {
                let e = expected[i - 1]
                let same = a[i - 1] == got[j - 1]
                out.append(PhoneJudgement(expected: LetterSounds.phone(e.letter), heard: LetterSounds.phone(got[j - 1]),
                                          verdict: same ? .good : .substituted, letterRange: e.offset..<(e.offset + 1)))
                i -= 1; j -= 1
            } else if i > 0 && d[i][j] == d[i - 1][j] + 1 {
                let e = expected[i - 1]
                out.append(PhoneJudgement(expected: LetterSounds.phone(e.letter), heard: nil, verdict: .missing,
                                          letterRange: e.offset..<(e.offset + 1)))
                i -= 1
            } else {
                out.append(PhoneJudgement(expected: nil, heard: LetterSounds.phone(got[j - 1]), verdict: .extra, letterRange: nil))
                j -= 1
            }
        }
        return out.reversed()
    }

    /// ё is written е in most text the recognizer returns.
    static func fold(_ c: Character) -> Character {
        let l = Character(String(c).lowercased())
        return l == "ё" ? "е" : l
    }

    /// The letter a judgement is about, and the one heard instead, from the target text.
    static func letter(of j: PhoneJudgement, in target: String) -> Character? {
        guard let r = j.letterRange, r.lowerBound < target.count else { return nil }
        return fold(Syllables.base(Array(target)[r.lowerBound]))
    }

    static func heardLetter(_ j: PhoneJudgement) -> Character? {
        guard let ipa = j.heard?.ipa else { return nil }
        return LetterSounds.ipa.first { $0.value == ipa }?.key
    }

    // MARK: Tips

    /// What the recognizer's words suggest. They are a hypothesis: a recognizer that writes «бил»
    /// for «был» may have misheard a good ы, so each tip carries a modest explicit confidence
    /// (`SpeakingFeedback.wordHypothesisConfidence`, below `SpeakingFeedback.confidenceFloor`)
    /// so it stays hidden from actionable corrections by design.
    static func tips(target: String, sounds: [PhoneJudgement], comparison: ProsodyComparison?) -> [PronunciationTip] {
        var tips: [PronunciationTip] = []
        let hypothesisConfidence = SpeakingFeedback.wordHypothesisConfidence
        // A letter heard as another, contrast first (those have a page to practise).
        let subs = sounds.filter { $0.verdict == .substituted }
        let ranked = subs.sorted { a, b in
            let ca = letter(of: a, in: target).map { SoundContrasts.id(expected: $0, heard: heardLetter(a)) } ?? nil
            let cb = letter(of: b, in: target).map { SoundContrasts.id(expected: $0, heard: heardLetter(b)) } ?? nil
            return (ca != nil ? 0 : 1) < (cb != nil ? 0 : 1)
        }
        if let s = ranked.first, let want = letter(of: s, in: target) {
            let got = heardLetter(s)
            let contrast = SoundContrasts.id(expected: want, heard: got)
            var message = got.map { "The recognizer heard «\($0)» where «\(want)» should be — it may have misheard, or the «\(want)» may want attention." }
                ?? "The recognizer didn't make out the «\(want)»."
            if let c = contrast, let advice = SoundContrasts.advice(c) { message += " If so: " + advice }
            tips.append(PronunciationTip(message: message, contrast: contrast, letterRange: s.letterRange, confidence: hypothesisConfidence))
        } else if let missing = sounds.first(where: { $0.verdict == .missing }), let want = letter(of: missing, in: target) {
            tips.append(PronunciationTip(message: "The recognizer didn't catch the «\(want)» — it may have been quiet or misheard. Listen to the model once more and say it again.",
                                         contrast: nil, letterRange: missing.letterRange, confidence: hypothesisConfidence))
        }
        if let c = comparison {
            if let miss = c.stressMisses.first {
                let want = c.syllables[miss.expected], got = c.syllables[miss.heard]
                let word = c.syllables.filter { $0.word == miss.word }.map(\.text).joined()
                let lead = c.syllables.filter({ $0.word == miss.word }).count == c.syllables.count ? "" : "In «\(word)», "
                let body = "the stress is on «\(want.text)» — make it longer and louder than «\(got.text)»."
                tips.append(PronunciationTip(message: lead.isEmpty ? body.prefix(1).uppercased() + body.dropFirst() : lead + body,
                                             contrast: "stress", letterRange: want.letters, confidence: hypothesisConfidence))
            } else if let k = c.missedRise {
                tips.append(PronunciationTip(message: "The model's voice climbs on «\(c.syllables[k].text)»; let yours climb there too.",
                                             contrast: nil, letterRange: c.syllables[k].letters, confidence: hypothesisConfidence))
            }
        }
        return Array(tips.prefix(2))
    }
}

extension PronunciationReport {
    /// A report from words heard rather than sounds heard: its letter verdicts are the
    /// recognizer's hypothesis (shown as uncertain, never evidence) and vouch for nothing.
    /// Reports saved before `provenance` existed are read by whether they carry heard phones.
    var isWordLevel: Bool { provenance.map { $0 == .words } ?? heard.isEmpty }
}
