import Foundation

/// Without the phone recognizer: compare what the word recogniser heard against the target,
/// sound by sound through the G2P of both, so the report keeps the same shape (sounds,
/// syllables, tips) with coarser evidence and no timing or prosody.
enum WordFallback {
    static func report(target: String, expected: Transcription, transcript: String,
                       speech: (start: Int, end: Int)) -> PronunciationReport {
        let heardT = RussianG2P.transcribe(stressLike(transcript, target))
        let a = expected.sounds.map(\.ipa), b = heardT.sounds.map(\.ipa)
        var sounds: [PhoneJudgement] = []
        var mistakes: [(Mistake?, Float)] = []
        for step in Align.run(a, b) {
            switch step {
            case .match(let i, let j), .substitute(let i, let j):
                let s = expected.sounds[i], h = b[j]
                let opts = PhoneSet.options(for: s)
                let ok = opts.good.contains { Set(PhoneSet.spellings($0)).contains(h) } || h == s.ipa
                let m = opts.errors.first { Set(PhoneSet.spellings($0.ipa)).contains(h) }
                let d = PhoneSet.distance(s.ipa, h)
                let verdict: PhoneJudgement.Verdict = ok || d <= 0.1 ? .good : (m == nil && d < 0.4 ? .close : .substituted)
                // Word-level G2P alignment is a hypothesis, not a phone measurement: give it a modest
                // explicit confidence below SpeakingFeedback.confidenceFloor so it stays hidden by design.
                let c = SpeakingFeedback.wordHypothesisConfidence
                sounds.append(PhoneJudgement(expected: Phone(ipa: s.ipa, confidence: c), heard: Phone(ipa: h, confidence: c), verdict: verdict, letterRange: s.letters))
                mistakes.append((verdict == .substituted ? (m ?? Mistake(ipa: h, kind: .other)) : nil, verdict == .substituted ? -3 : 3))
            case .delete(let i):
                let s = expected.sounds[i]
                sounds.append(PhoneJudgement(expected: Phone(ipa: s.ipa, confidence: SpeakingFeedback.wordHypothesisConfidence), heard: nil, verdict: .missing, letterRange: s.letters))
                mistakes.append((nil, -3))
            case .insert(let j):
                sounds.append(PhoneJudgement(expected: nil, heard: Phone(ipa: b[j], confidence: SpeakingFeedback.wordHypothesisConfidence), verdict: .extra, letterRange: nil))
            }
        }
        let expectedCount = max(1, sounds.filter { $0.expected != nil }.count)
        let good = sounds.filter { $0.expected != nil && ($0.verdict == .good || $0.verdict == .close) }.count
        // Tips need the judgements of expected sounds only, in order.
        let tips = Tips.make(target: target, expected: expected, sounds: sounds.filter { $0.expected != nil }, mistakes: mistakes, stress: [])
        return PronunciationReport(
            target: target, transcript: transcript,
            expected: expected.sounds.map { Phone(ipa: $0.ipa) }, heard: heardT.sounds.map { Phone(ipa: $0.ipa) },
            sounds: sounds,
            syllables: expected.syllables.map { SyllableJudgement(text: $0.text, isStressed: $0.stressed, start: nil, end: nil, prominence: nil) },
            stressedSyllable: nil, accuracy: Double(good) / Double(expectedCount), tips: tips,
            speechStart: Double(speech.start) / 16_000, speechEnd: Double(speech.end) / 16_000, provenance: .words)
    }

    /// The recogniser writes no stress marks: give each heard word the target word's stress
    /// when it is the same word, so a correct word is not judged against unreduced vowels.
    static func stressLike(_ heard: String, _ target: String) -> String {
        let marked = target.split(whereSeparator: { $0.isWhitespace }).map(String.init)
        let plainTarget = marked.map { RussianText.unstressed($0).lowercased().trimmingCharacters(in: .punctuationCharacters) }
        return heard.split(whereSeparator: { $0.isWhitespace }).map { w -> String in
            let p = w.lowercased().trimmingCharacters(in: .punctuationCharacters)
            if let k = plainTarget.firstIndex(of: p) { return marked[k] }
            return String(w)
        }.joined(separator: " ")
    }
}
