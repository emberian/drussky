import LearningCore
import XCTest
@testable import Drussky

final class ReadingEvidenceTests: XCTestCase {
    let date = Date(timeIntervalSince1970: 1_800_000_000)
    let book = ComponentID(.lexeme, "книга")
    let form = ComponentID(.form, "книгу")

    func event(_ modality: Modality, grade: Grade = .good, expected: String = "кни́га", components: [ComponentID]? = nil) -> LearningEvent {
        LearningEvent(t: date, type: .outcome, item: UUID().uuidString, modality: modality,
                      components: components ?? [book], grade: grade, expected: expected)
    }

    func testCopyAndRecognitionNeverCountAsKnown() {
        for m in [Modality.copy, .readPick, .hearPick, .build] {
            XCTAssertFalse(ReadingEvidence.knows(book, model: KnowledgeModel(events: [event(m)]), at: date))
        }
        for m in [Modality.hearWrite, .meaningWrite, .say] {
            XCTAssertTrue(ReadingEvidence.knows(book, model: KnowledgeModel(events: [event(m)]), at: date))
        }
    }

    func testFailureRetryAndDecayDoNotClaimKnowledge() {
        for grade in [Grade.again, .hard] {
            XCTAssertFalse(ReadingEvidence.knows(book, model: KnowledgeModel(events: [event(.hearWrite, grade: grade)]), at: date))
        }
        let model = KnowledgeModel(events: [event(.hearWrite)])
        XCTAssertFalse(ReadingEvidence.knows(book, model: model, at: date.addingTimeInterval(365 * 86_400)))
    }

    func testInflectedProductionDoesNotProveTheLemmaForm() {
        let model = KnowledgeModel(events: [event(.hearWrite, expected: "кни́гу", components: [book, form])])
        XCTAssertTrue(ReadingEvidence.knows(form, model: model, at: date))
        XCTAssertFalse(ReadingEvidence.knows(book, model: model, at: date))
    }

    func testKnownLemmaDoesNotCoverUnseenFormInEitherReadingRoute() {
        let domain = RussianDomain(course: Course(version: 1, letters: [], lexicon: [], units: []),
                                   lemmas: Lemmas(version: 1, constructions: [], forms: ["книгу": ["книга", "acc-u"]]),
                                   pairs: SoundPairs(version: 1, contrasts: [], pairs: []))
        let model = KnowledgeModel(events: [event(.meaningWrite)])
        let goal = Goal(title: "test", origin: .pasted, text: "Кни́га. Кни́гу.", created: date)
        let distance = GoalAnalysis(goal: goal, domain: domain).distance(model: model, at: date)
        XCTAssertEqual(distance.sentenceCoverage, [1, 0])
        XCTAssertEqual(ReadingEvidence.coverage(goal.text, domain: domain, model: model, at: date), 0.5)
        XCTAssertEqual(Goal.readableThreshold, 0.95)
        XCTAssertEqual(Goal.firstPageThreshold, 0.98)
        XCTAssertEqual(ReadingEvidence.label(for: 0.8), "study with support")
    }
}
