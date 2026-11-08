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
            speechStart: comparison?.takeSpeech?.lowerBound, speechEnd: comparison?.takeSpeech?.upperBound)
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

    static func tips(target: String, sounds: [PhoneJudgement], comparison: ProsodyComparison?) -> [PronunciationTip] {
        // A word recognizer's spelling is not a measurement of individual sounds.
        // Retain the transcript and alignment; offer playback rather than accent corrections.
        return []
    }
}

extension PronunciationReport {
    /// A report from words heard rather than sounds heard: its letter verdicts only flag
    /// letters that came out as others, never vouch for the rest.
    var isWordLevel: Bool { heard.isEmpty }
}
