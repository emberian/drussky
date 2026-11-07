import LearningCore
import XCTest

/// The core on a toy subject (no Drussky, no Russian): two symbols and a word made of them.
final class LearningCoreTests: XCTestCase {
    let a = ComponentID("sym", "a"), b = ComponentID("sym", "b"), ab = ComponentID("word", "ab"), ba = ComponentID("word", "ba")
    let say: Modality = "say", write: Modality = "write"
    let t0 = Date(timeIntervalSince1970: 1_800_000_000)
    func day(_ n: Double) -> Date { t0.addingTimeInterval(n * 86_400) }

    var graph: KnowledgeGraph {
        KnowledgeGraph(components: [Component(id: a, label: "a", order: 1), Component(id: b, label: "b", order: 2),
                                    Component(id: ab, label: "ab", order: 3), Component(id: ba, label: "ba", order: 4)],
                       edges: [Edge(ab, a, .uses), Edge(ab, b, .uses), Edge(ba, a, .uses), Edge(ba, b, .uses)])
    }

    func outcome(_ c: [ComponentID], _ m: Modality, _ g: Grade, at t: Date, item: String = UUID().uuidString,
                 confusions: [Confusion]? = nil) -> LearningEvent {
        LearningEvent(t: t, type: .outcome, item: item, modality: m, components: c, grade: g, confusions: confusions)
    }

    // MARK: FSRS

    func testFSRSFormulas() {
        let f = FSRS()
        XCTAssertEqual(FSRS.retrievability(elapsedDays: 7, stability: 7), 0.9, accuracy: 1e-12, "R(S,S) = 0.9 by construction")
        XCTAssertEqual(FSRS.interval(stability: 12), 12, accuracy: 1e-9)
        XCTAssertEqual(f.initialStability(.good), 3.173)
        XCTAssertEqual(f.initialDifficulty(.good), 5.282434, accuracy: 1e-5)
        let s0 = f.next(nil, elapsedDays: 0, grade: .good)
        // A Good review exactly when due (R = 0.9): values computed by hand from the FSRS-5 formulas.
        let good = f.next(s0, elapsedDays: s0.stability, grade: .good)
        XCTAssertEqual(good.stability, 11.131260, accuracy: 1e-4)
        XCTAssertEqual(good.difficulty, 5.272968, accuracy: 1e-4)
        let again = f.next(s0, elapsedDays: s0.stability, grade: .again)
        XCTAssertEqual(again.stability, 1.066878, accuracy: 1e-4)
        XCTAssertEqual(again.difficulty, 6.796933, accuracy: 1e-4)
        let sameDay = f.next(s0, elapsedDays: 0.1, grade: .good)
        XCTAssertEqual(sameDay.stability, 4.466858, accuracy: 1e-4)
        XCTAssertLessThan(f.next(s0, elapsedDays: 3.173, grade: .hard).stability, good.stability)
        XCTAssertGreaterThan(f.next(s0, elapsedDays: 3.173, grade: .easy).stability, good.stability)
    }

    // MARK: Model

    func testMemoryIsPerModality() {
        let m = KnowledgeModel(events: [outcome([ab], say, .good, at: t0), outcome([ab], say, .good, at: day(3))])
        XCTAssertNotNil(m.retrievability(ab, say, at: day(4)))
        XCTAssertNil(m.retrievability(ab, write, at: day(4)), "saying it is no evidence of writing it")
        XCTAssertEqual(m.trace(ab, say)?.reviews.count, 2)
        XCTAssertGreaterThan(m.retrievability(ab, say, at: day(4))!, m.retrievability(ab, say, at: day(30))!)
    }

    func testEventsReplayInTimeOrderAndDedupe() {
        let e1 = outcome([a], say, .good, at: day(1)), e2 = outcome([a], say, .again, at: day(5))
        let m1 = KnowledgeModel(events: [e2, e1, e1])
        let m2 = KnowledgeModel(events: [e1, e2])
        XCTAssertEqual(m1.trace(a, say), m2.trace(a, say))
        XCTAssertEqual(m1.events.count, 2)
    }

    func testConfusionEdgesFromEvidence() {
        let x = Confusion(target: ab, produced: ba, channel: "writing")
        let m = KnowledgeModel(events: [
            LearningEvent(t: t0, type: .attempt, item: "i1", modality: write, components: [ab], correct: false, confusions: [x]),
            outcome([ab], write, .again, at: day(0.01), item: "i1", confusions: [x]),
            outcome([ba], write, .good, at: day(1)),
        ])
        let edges = m.confusionEdges(at: day(1))
        XCTAssertEqual(edges.count, 1)
        XCTAssertEqual(edges[0].from, ab)
        XCTAssertEqual(edges[0].to, ba)
        XCTAssertEqual(edges[0].kind, .confusedWith)
        XCTAssertEqual(edges[0].weight, 2 * pow(0.5, 1.0 / 30), accuracy: 0.01)
        XCTAssertLessThan(m.confusionEdges(at: day(61), minWeight: 0)[0].weight, 0.6, "confusions fade with a 30-day half-life")
        XCTAssertTrue(m.history(of: ba).contains { $0.item == "i1" }, "the produced component sees the event too")
        // Confusion-aware order: ab is due and pulls its partner ba in right behind it.
        let order = m.reviewOrder(at: day(10))
        XCTAssertEqual(order.prefix(2).map(\.component), [ab, ba])
    }

    func testHelpCapsTheNextOutcome() {
        let help = LearningEvent(t: t0, type: .help, item: "i", modality: say, components: [a], help: ["tutor"])
        let m = KnowledgeModel(events: [help, outcome([a], say, .good, at: day(0.1), item: "i")])
        XCTAssertEqual(m.trace(a, say)?.reviews.first?.grade, .hard)
    }

    func testTransferAndEstimates() {
        // ab learned by saying, then first written: primed. ba first written cold.
        let m = KnowledgeModel(events: [
            outcome([ab], say, .good, at: t0), outcome([ab], say, .good, at: day(3)),
            outcome([ab], write, .good, at: day(4)),
            outcome([ba], write, .again, at: day(5)),
        ])
        let s = m.transfer.first { $0.from == say && $0.to == write }!
        XCTAssertEqual(s.primedTrials, 1)
        XCTAssertEqual(s.primedSuccess, 1)
        XCTAssertEqual(s.coldTrials, 1)
        XCTAssertGreaterThan(s.lift, 0)
        let m2 = KnowledgeModel(events: [outcome([a], say, .good, at: t0)])
        let est = m2.estimatedRetrievability(a, write, at: day(1))!
        XCTAssertTrue(est.estimated)
        XCTAssertLessThan(est.value, m2.retrievability(a, say, at: day(1))!)
    }

    func testFrontierNeedsStrongPrerequisites() {
        let g = graph
        XCTAssertEqual(KnowledgeModel().frontier(g, at: t0), [], "nothing is ready while its letters are unknown")
        let m = KnowledgeModel(events: [outcome([a, b], say, .good, at: t0)])
        XCTAssertEqual(m.frontier(g, at: day(0.5)), [ab, ba])
        XCTAssertEqual(m.frontier(g, at: day(60)), [], "prerequisites that faded no longer unlock")
        XCTAssertEqual(g.prerequisites(of: ab), [a, b])
        XCTAssertEqual(g.dependents(of: a), [ab, ba])
    }

    func testGrowthAndCoverage() {
        let m = KnowledgeModel(events: [outcome([ab], say, .good, at: day(1)), outcome([ba], write, .good, at: day(2))])
        let g = m.growth(of: [ab, ba], groups: [[say], [write], [say, write]], at: [t0, day(1), day(2)])
        XCTAssertEqual(g[0][0], 0)
        XCTAssertEqual(g[0][1], 1, accuracy: 1e-9)
        XCTAssertEqual(g[1][1], 0)
        XCTAssertEqual(g[2][2], g[0][2] + g[1][2], accuracy: 1e-9)
        XCTAssertEqual(m.coverage([[ab], [ab], [ComponentID("word", "zz")], [ba]], at: day(2)), 0.75)
    }

    // MARK: Contracts

    func testEventLogAppendsAndSurvivesABadLine() throws {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString).appendingPathComponent("events.jsonl")
        let log = EventLog(url: url)
        log.append([outcome([a], say, .good, at: t0)])
        let handle = try FileHandle(forWritingTo: url)
        try handle.seekToEnd()
        try handle.write(contentsOf: Data("{\"broken\": tr".utf8))
        try handle.close()
        log.append([outcome([b], say, .again, at: day(1))])
        let events = log.load()
        XCTAssertEqual(events.map(\.components), [[a], [b]])
        let line = String(decoding: try EventLog.encoder().encode(events[0]), as: UTF8.self)
        XCTAssertTrue(line.contains("\"components\":[\"sym:a\"]"), line)
        XCTAssertTrue(line.contains("\"grade\":3"), line)
        XCTAssertTrue(line.contains("\"v\":1"), line)
    }

    func testComponentIDAndExport() throws {
        XCTAssertEqual(ComponentID("lex:у меня есть")?.key, "у меня есть")
        XCTAssertEqual(ComponentID("form:a:b")?.key, "a:b", "only the first colon separates")
        XCTAssertNil(ComponentID("nokind"))
        let m = KnowledgeModel(events: [outcome([ab], say, .good, at: t0,
                                                confusions: [Confusion(target: ab, produced: ba, channel: "choice")])])
        let doc = GraphExport(domain: "toy", modalities: [say, write], graph: graph, model: m, at: day(1))
        let json = try JSONSerialization.jsonObject(with: doc.json()) as! [String: Any]
        XCTAssertEqual(json["format"] as? String, "learningcore.graph")
        let edges = json["edges"] as! [[String: Any]]
        XCTAssertTrue(edges.contains { $0["kind"] as? String == "confused-with" && $0["from"] as? String == "word:ab" })
        let node = (json["nodes"] as! [[String: Any]]).first { $0["id"] as? String == "word:ab" }!
        XCTAssertNotNil((node["modalities"] as! [String: Any])["say"])
    }
}
