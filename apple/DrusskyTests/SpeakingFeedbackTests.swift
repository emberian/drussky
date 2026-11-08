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
        XCTAssertEqual(SpokenText.marked("был", report: report, showStress: false), SpokenText.marked("был", report: nil, showStress: false))
        // Omit the range to avoid depending on the standard library's range encoding.
        let tip = try JSONDecoder().decode(PronunciationTip.self, from: Data(#"{"message":"old","contrast":"stress"}"#.utf8))
        XCTAssertNil(tip.confidence)
    }
}
