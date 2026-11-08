import SwiftUI
import XCTest
@testable import Drussky

final class SpeakingFeedbackTests: XCTestCase {
    func testConfidenceFloorFailsClosed() {
        let values: [Double?] = [nil, .nan, -.infinity, .infinity, -0.1, 0, 0.94, 1.1]
        for confidence in values {
            XCTAssertFalse(SpeakingFeedback.permits(confidence))
        }
        XCTAssertTrue(SpeakingFeedback.permits(0.95))
        XCTAssertTrue(SpeakingFeedback.permits(1))
    }

    func testMissingAndSubstitutedSoundsNeedTheirOwnEvidence() {
        var sound = PhoneJudgement(expected: Phone(ipa: "ɨ"), heard: Phone(ipa: "i"), verdict: .substituted, letterRange: 1..<2)
        XCTAssertFalse(sound.supportsCorrection)
        sound.heard?.confidence = 0.94
        XCTAssertFalse(sound.supportsCorrection)
        sound.heard?.confidence = 0.99
        XCTAssertTrue(sound.supportsCorrection)
        sound.verdict = .missing; sound.heard = nil
        XCTAssertFalse(sound.supportsCorrection)
        sound.expected?.confidence = 0.01
        XCTAssertTrue(sound.supportsCorrection)
    }

    func testUncertainOldReportsDoNotMakeRedMarksOrTips() throws {
        var report = WordLevelAnalyzer.report(target: "был", transcript: "бил", comparison: nil)
        report.tips = [PronunciationTip(message: "Old unsupported claim", contrast: "y-i", letterRange: 1..<2)]
        XCTAssertTrue(report.actionableTips.isEmpty)
        // A word-level report draws its hypothesis in amber, never in red pen.
        let hypothesis = SpokenText.marked("был", report: report, showStress: false)
        XCTAssertFalse(hypothesis.runs.contains { $0.foregroundColor == Theme.redPen })
        XCTAssertTrue(hypothesis.runs.contains { $0.foregroundColor == SpokenText.close })
        // A phone-level report whose verdicts sit below the floor marks nothing at all.
        var unsure = report
        unsure.provenance = .phones
        XCTAssertEqual(SpokenText.marked("был", report: unsure, showStress: false), SpokenText.marked("был", report: nil, showStress: false))
        // Omit the range to avoid depending on the standard library's range encoding.
        let tip = try JSONDecoder().decode(PronunciationTip.self, from: Data(#"{"message":"old","contrast":"stress"}"#.utf8))
        XCTAssertNil(tip.confidence)
    }

    func testPhoneStressFindingBecomesActionableWhileWordHypothesisStaysHidden() {
        let target = "за́мок"
        let expected = RussianG2P.transcribe(target)
        let sounds = expected.sounds.map {
            PhoneJudgement(expected: Phone(ipa: $0.ipa, confidence: 0.99),
                           heard: Phone(ipa: $0.ipa, confidence: 0.99),
                           verdict: .good, letterRange: $0.letters)
        }
        let mistakes: [(Mistake?, Float)] = expected.sounds.map { _ in (nil, 3) }

        // A high-confidence phone-level stress finding becomes an actionable tip.
        let highStress = Prosody.Finding(word: 0, expected: 0, heard: 1, strength: 4.0)
        let tips = Tips.make(target: target, expected: expected, sounds: sounds, mistakes: mistakes, stress: [highStress])
        XCTAssertEqual(tips.first?.contrast, "stress")
        XCTAssertTrue(SpeakingFeedback.permits(tips.first?.confidence))

        var phoneReport = WordLevelAnalyzer.report(target: target, transcript: target, comparison: nil)
        phoneReport.provenance = .phones
        phoneReport.tips = tips
        XCTAssertEqual(phoneReport.actionableTips.map(\.contrast), ["stress"])

        // A low-confidence phone-level stress finding stays out.
        let lowStress = Prosody.Finding(word: 0, expected: 0, heard: 1, strength: 1.6)
        XCTAssertTrue(Tips.make(target: target, expected: expected, sounds: sounds, mistakes: mistakes, stress: [lowStress]).isEmpty)

        // Word-level hypotheses carry an explicit confidence below the floor and stay hidden.
        let wordReport = WordLevelAnalyzer.report(target: "был", transcript: "бил", comparison: nil)
        XCTAssertFalse(wordReport.tips.isEmpty)
        XCTAssertEqual(wordReport.tips.first?.confidence, SpeakingFeedback.wordHypothesisConfidence)
        XCTAssertTrue(wordReport.actionableTips.isEmpty)
    }
}
