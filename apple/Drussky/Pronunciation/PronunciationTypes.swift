import Foundation

// The contract between the speaking UI and the pronunciation engine. The UI records a take
// and asks an analyzer for a report; the engine (phone recognition + Russian G2P + alignment
// + prosody) produces it. Keep this file small and stable: both sides build against it.

/// One phone in IPA (espeak-style symbols), with timing when known.
struct Phone: Codable, Sendable, Hashable {
    var ipa: String
    var start: Double? = nil     // seconds into the trimmed take
    var end: Double? = nil
    var confidence: Double? = nil
}

/// How one expected sound came out.
struct PhoneJudgement: Codable, Sendable, Hashable {
    enum Verdict: String, Codable, Sendable { case good, close, substituted, missing, extra }
    var expected: Phone?          // nil for an extra sound
    var heard: Phone?             // nil for a missing sound
    var verdict: Verdict
    /// Index of the letter(s) in the target text this sound belongs to, for highlighting.
    var letterRange: Range<Int>?
}

/// A syllable of the target, with its prosody in the learner's take.
struct SyllableJudgement: Codable, Sendable, Hashable {
    var text: String              // the letters, e.g. "ма́"
    var isStressed: Bool          // expected stress
    var start: Double?
    var end: Double?
    var prominence: Double?       // 0…1 relative loudness × length × pitch, within the take
}

/// One thing to try next, most useful first. `contrast` names a sound-pair contrast
/// ("ы/и", "hard/soft", "stress", …) so practice can follow.
struct PronunciationTip: Codable, Sendable, Hashable {
    var message: String
    var contrast: String?
    var letterRange: Range<Int>?
    /// Nil in older reports and in analyses without sound-level confidence.
    var confidence: Double? = nil
}

/// This floor is a conservative display policy, not a calibrated probability of correctness.
/// Raw reports remain available for evaluation; uncertain claims never become corrections.
enum SpeakingFeedback {
    static let confidenceFloor = 0.95
    static func permits(_ confidence: Double?) -> Bool {
        guard let confidence, confidence.isFinite else { return false }
        return confidence >= confidenceFloor && confidence <= 1
    }
}

extension PhoneJudgement {
    var correctionConfidence: Double? {
        switch verdict {
        case .substituted, .extra: heard?.confidence
        case .missing: expected?.confidence.map { 1 - $0 }
        default: nil
        }
    }
    var supportsCorrection: Bool { SpeakingFeedback.permits(correctionConfidence) }
}

extension PronunciationReport {
    var actionableTips: [PronunciationTip] { tips.filter { SpeakingFeedback.permits($0.confidence) } }
}

struct PronunciationReport: Codable, Sendable {
    var target: String
    var transcript: String?       // words recognised, if any
    var expected: [Phone]
    var heard: [Phone]
    var sounds: [PhoneJudgement]
    var syllables: [SyllableJudgement]
    /// Index into `syllables` the learner stressed most, if determinable.
    var stressedSyllable: Int?
    /// 0…1, share of expected sounds judged good or close.
    var accuracy: Double
    var tips: [PronunciationTip]
    /// Where the speech begins/ends in the original recording (seconds), after trimming.
    var speechStart: Double?
    var speechEnd: Double?
}

protocol PronunciationAnalyzer: Sendable {
    /// Analyse a recorded take of `target` (Russian, stress-marked). `model` is the reference
    /// recording, when there is one, for timing/prosody comparison.
    func analyze(take: URL, target: String, model: URL?) async throws -> PronunciationReport
}
