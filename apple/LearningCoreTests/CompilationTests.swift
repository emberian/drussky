import LearningCore
import XCTest

/// Response latency as evidence: the field, the running average beside FSRS, and "compiled"
/// (right and quick) as distinct from known.
final class CompilationTests: XCTestCase {
    let a = ComponentID("sym", "a"), b = ComponentID("sym", "b"), c = ComponentID("sym", "c"), ab = ComponentID("word", "ab")
    let say: Modality = "say", write: Modality = "write"
    let t0 = Date(timeIntervalSince1970: 1_800_000_000)

    func outcome(_ comps: [ComponentID], _ m: Modality, _ g: Grade, at t: Date, ms: Int?) -> LearningEvent {
        LearningEvent(t: t, type: .outcome, item: UUID().uuidString, modality: m, components: comps, grade: g, latencyMs: ms)
    }

    func testOldLinesDecodeWithoutLatencyAndNewOnesWithIt() throws {
        let old = #"{"v":1,"id":"x","t":"2026-11-07T10:31:02Z","type":"outcome","source":"live","item":"p#1","components":["sym:a"],"grade":3}"#
        let e = try EventLog.decoder().decode(LearningEvent.self, from: Data(old.utf8))
        XCTAssertNil(e.latencyMs)
        let line = try EventLog.encoder().encode(outcome([a], write, .good, at: t0, ms: 640))
        XCTAssertTrue(String(decoding: line, as: UTF8.self).contains(#""latencyMs":640"#))
        XCTAssertEqual(try EventLog.decoder().decode(LearningEvent.self, from: line).latencyMs, 640)
        var model = KnowledgeModel()
        model.apply(e)
        XCTAssertNil(model.traces(of: a).values.first?.latencyEMA, "no latency, no average")
    }

    func testRightAnswersFeedTheAverageAndWrongOnesDoNot() {
        var model = KnowledgeModel()
        model.apply(outcome([a], write, .good, at: t0, ms: 1000))
        model.apply(outcome([a], write, .good, at: t0.addingTimeInterval(60), ms: 500))
        model.apply(outcome([a], write, .again, at: t0.addingTimeInterval(120), ms: 9000))
        model.apply(outcome([a], write, .hard, at: t0.addingTimeInterval(180), ms: 7000))
        model.apply(outcome([a], write, .good, at: t0.addingTimeInterval(240), ms: nil))
        let trace = model.trace(a, write)!
        XCTAssertEqual(trace.latencySamples, 2)
        XCTAssertEqual(trace.latencyEMA ?? 0, 1000 + 0.3 * (500 - 1000), accuracy: 1e-9, "alpha 0.3 over right answers only")
        XCTAssertEqual(trace.reviews.compactMap(\.latencyMs), [1000, 500, 9000, 7000], "every measured answer is on the review")
        XCTAssertEqual(trace.reviews.count, 5)
    }

    func testCompiledNeedsSamplesSpeedAndAccuracy() {
        let policy = CompilationPolicy(floorsMs: [write: 2000, say: 800])
        var model = KnowledgeModel()
        for i in 0..<2 { model.apply(outcome([a], write, .good, at: t0.addingTimeInterval(Double(i) * 60), ms: 600)) }
        XCTAssertFalse(model.isCompiled(a, write, policy: policy), "two timed answers are not enough")
        XCTAssertFalse(model.trace(a, write)!.hasLatencyEvidence(policy))
        model.apply(outcome([a], write, .good, at: t0.addingTimeInterval(120), ms: 600))
        XCTAssertTrue(model.isCompiled(a, write, policy: policy))
        XCTAssertFalse(model.isCompiled(a, say, policy: policy), "per modality")
        XCTAssertEqual(model.compiledModalities(of: a, policy: policy), [write])
        // One stumble inside the window: known still, compiled no longer.
        model.apply(outcome([a], write, .again, at: t0.addingTimeInterval(180), ms: nil))
        XCTAssertFalse(model.isCompiled(a, write, policy: policy))
        XCTAssertGreaterThan(model.strength(a, at: t0.addingTimeInterval(200)) ?? 0, 0.8, "FSRS still calls it known")
        // Fast but under a stricter floor: not compiled in that modality.
        for i in 0..<3 { model.apply(outcome([b], say, .good, at: t0.addingTimeInterval(Double(i) * 60), ms: 900)) }
        XCTAssertFalse(model.isCompiled(b, say, policy: policy))
        XCTAssertTrue(model.isCompiled(b, say, policy: CompilationPolicy(floorsMs: [say: 1000])))
    }

    /// Known but slow prerequisites are named; never-timed ones are not (no measurement, no claim).
    func testUncompiledPrerequisitesOfAPage() {
        let graph = KnowledgeGraph(components: [Component(id: a, label: "a", order: 1), Component(id: b, label: "b", order: 2),
                                                Component(id: c, label: "c", order: 3), Component(id: ab, label: "ab", order: 4)],
                                   edges: [Edge(ab, a, .uses), Edge(ab, b, .uses), Edge(ab, c, .uses)])
        let policy = CompilationPolicy(floorsMs: [write: 2000])
        var model = KnowledgeModel()
        let t = t0.addingTimeInterval(600)
        for i in 0..<3 {
            model.apply(outcome([a], write, .good, at: t0.addingTimeInterval(Double(i) * 60), ms: 500))   // quick
            model.apply(outcome([b], write, .good, at: t0.addingTimeInterval(Double(i) * 60), ms: 6000))  // right, slow
            model.apply(outcome([c], write, .good, at: t0.addingTimeInterval(Double(i) * 60), ms: nil))   // never timed
        }
        XCTAssertTrue(model.isKnownButUncompiled(b, at: t, ready: 0.8, policy: policy))
        XCTAssertFalse(model.isKnownButUncompiled(a, at: t, ready: 0.8, policy: policy))
        XCTAssertFalse(model.isKnownButUncompiled(c, at: t, ready: 0.8, policy: policy), "untimed: not flagged")
        XCTAssertEqual(model.uncompiledPrerequisites(of: [ab], in: graph, at: t, policy: policy), [b])
        XCTAssertTrue(model.uncompiledPrerequisites(of: [ab], in: graph, at: t0.addingTimeInterval(400 * 86_400), policy: policy).isEmpty,
                      "forgotten is not known-but-slow")
    }
}
