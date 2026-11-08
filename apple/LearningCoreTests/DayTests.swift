import LearningCore
import XCTest

/// The shape of a day, the frontier, probes and fatigue, on toy symbols.
final class DayTests: XCTestCase {
    let a = ComponentID("sym", "a"), b = ComponentID("sym", "b"), c = ComponentID("sym", "c")
    let ab = ComponentID("word", "ab"), bc = ComponentID("word", "bc"), abc = ComponentID("word", "abc")
    let say: Modality = "say", write: Modality = "write"
    let t0 = Date(timeIntervalSince1970: 1_800_000_000)   // 2027-01-15 08:00 UTC
    func day(_ n: Double) -> Date { t0.addingTimeInterval(n * 86_400) }
    var utc: Calendar { var c = Calendar(identifier: .gregorian); c.timeZone = TimeZone(identifier: "UTC")!; return c }

    func outcome(_ c: [ComponentID], _ m: Modality, _ g: Grade, at t: Date, item: String = UUID().uuidString, latency: Double? = nil) -> LearningEvent {
        LearningEvent(t: t, type: .outcome, item: item, modality: m, components: c, grade: g, latency: latency)
    }

    func due(_ id: String, r: Double, relearning: Bool = false) -> DueReview {
        DueReview(id: id, modality: write, priority: (relearning ? 1 : 0) + (1 - r), relearning: relearning)
    }

    // MARK: Day shape

    func testDayBoundaryAndAbsence() {
        let cal = utc
        XCTAssertEqual(DayShape.shape(lastActivity: nil, now: t0, calendar: cal), .ordinary, "no history: an ordinary first page")
        XCTAssertEqual(DayShape.shape(lastActivity: t0, now: t0.addingTimeInterval(3 * 3600), calendar: cal), .ordinary, "same day")
        XCTAssertEqual(DayShape.shape(lastActivity: t0, now: day(1), calendar: cal), .morning, "first page of the next day")
        // 23:30 then 00:30: a new calendar day even though only an hour passed.
        let late = cal.date(bySettingHour: 23, minute: 30, second: 0, of: t0)!
        XCTAssertEqual(DayShape.shape(lastActivity: late, now: late.addingTimeInterval(3600), calendar: cal), .morning)
        XCTAssertEqual(DayShape.shape(lastActivity: t0, now: day(6.9), calendar: cal), .morning)
        XCTAssertEqual(DayShape.shape(lastActivity: t0, now: day(10), calendar: cal), .welcomeBack(days: 10))
        XCTAssertEqual(DayShape.shape(lastActivity: day(1), now: t0, calendar: cal), .ordinary, "a clock that went backwards changes nothing")
    }

    func testMorningCompositionCapAndOverflowOrder() {
        // Twenty due items, recall spread 0.1…0.86; two relearning.
        var items = (0..<20).map { due("i\($0)", r: 0.1 + 0.04 * Double($0)) }
        items[3] = due("i3", r: 0.5, relearning: true)
        items[7] = due("i7", r: 0.7, relearning: true)
        let policy = DayPolicy(reviewCap: 25, morningReviews: 10, warmUp: 3)
        let m = DayComposition.morning(due: items.shuffled(), allowance: 25, policy: policy)
        XCTAssertEqual(m.warmUp.map(\.id), ["i19", "i18", "i17"], "the easiest first: highest recall, not relearning")
        XCTAssertEqual(m.reviews.count, 7)
        XCTAssertEqual(m.reviews.prefix(2).map(\.id), ["i3", "i7"], "relearning items are the most urgent")
        XCTAssertEqual(m.reviews[2].id, "i0", "then the most forgotten")
        XCTAssertEqual(m.deferred.count, 10)
        XCTAssertEqual(m.reviews.map(\.id), ["i3", "i7", "i0", "i1", "i2", "i4", "i5"])
        XCTAssertEqual(m.deferred.first?.id, "i6", "the overflow keeps the urgency order so the next page takes it in turn")
        XCTAssertTrue(zip(m.deferred, m.deferred.dropFirst()).allSatisfy { $0.priority >= $1.priority })
        XCTAssertEqual(Set(m.placed.map(\.id)).count, 10, "no item twice")

        // The day's remaining allowance caps the page too.
        let tight = DayComposition.morning(due: items, allowance: 4, policy: policy)
        XCTAssertEqual(tight.placed.count, 4)
        XCTAssertEqual(tight.warmUp.count, 3)
        XCTAssertEqual(tight.reviews.map(\.id), ["i3"])
        let none = DayComposition.morning(due: items, allowance: 0, policy: policy)
        XCTAssertTrue(none.placed.isEmpty)
        XCTAssertEqual(none.deferred.count, 20)
        // Fewer due than the warm-up: no warm-up, just the reviews.
        let few = DayComposition.morning(due: Array(items.prefix(2)), allowance: 25, policy: policy)
        XCTAssertTrue(few.warmUp.isEmpty)
        XCTAssertEqual(few.reviews.count, 2)
        // An ordinary page during the day weaves a few, within the allowance.
        XCTAssertEqual(DayComposition.weave(due: items, allowance: 25, policy: policy).reviews.count, 3)
        XCTAssertEqual(DayComposition.weave(due: items, allowance: 1, policy: policy).reviews.map(\.id), ["i3"])
    }

    func testWelcomeBackSamplesAcrossUrgency() {
        let items = (0..<40).map { due("i\($0)", r: Double($0) / 40) }
        let w = DayComposition.welcomeBack(due: items, allowance: 25, policy: DayPolicy(welcomeBackSample: 8))
        XCTAssertEqual(w.reviews.count, 8)
        XCTAssertEqual(w.reviews.first?.id, "i0", "the most urgent is always in the sample")
        XCTAssertEqual(w.reviews.last?.id, "i39", "and so is the easiest: the page measures the whole range")
        XCTAssertEqual(w.deferred.count, 32)
        XCTAssertEqual(Set(w.reviews.map(\.id)).intersection(w.deferred.map(\.id)).count, 0)
        let small = DayComposition.welcomeBack(due: Array(items.prefix(5)), allowance: 25)
        XCTAssertEqual(small.reviews.count, 5, "fewer due than the sample: all of them")
        // The day's cap holds on a return too: a smaller sample, still spanning the range.
        let capped = DayComposition.welcomeBack(due: items, allowance: 5, policy: DayPolicy(welcomeBackSample: 8))
        XCTAssertEqual(capped.reviews.count, 5)
        XCTAssertEqual(capped.reviews.first?.id, "i0")
        XCTAssertEqual(capped.reviews.last?.id, "i39")
        XCTAssertEqual(capped.deferred.count, 35)
        XCTAssertTrue(DayComposition.welcomeBack(due: items, allowance: 0).reviews.isEmpty, "nothing left today: nothing placed")
    }

    // MARK: Frontier

    /// a, b, c are symbols; ab and bc are words using them; abc uses a, b, c and needs ab and bc first.
    var graph: KnowledgeGraph {
        KnowledgeGraph(components: [a, b, c].enumerated().map { Component(id: $1, label: $1.key, order: $0) }
                       + [Component(id: ab, label: "ab", order: 3), Component(id: bc, label: "bc", order: 4), Component(id: abc, label: "abc", order: 5)],
                       edges: [Edge(ab, a, .uses), Edge(ab, b, .uses), Edge(bc, b, .uses), Edge(bc, c, .uses),
                               Edge(abc, a, .uses), Edge(abc, b, .uses), Edge(abc, c, .uses),
                               Edge(ab, abc, .prerequisite), Edge(bc, abc, .prerequisite)])
    }

    func testFrontierPrefersTheCoursePageWhenItsPrerequisitesAreStrong() {
        let g = graph
        // Everything the next page (abc) needs was learned yesterday and is strong.
        let m = KnowledgeModel(events: [a, b, c, ab, bc].map { outcome([$0], write, .good, at: t0) }
                               + [a, b, c, ab, bc].map { outcome([$0], write, .good, at: day(2)) })
        XCTAssertEqual(m.frontierDecision(pageComponents: [abc], in: g, dueCount: 3, at: day(2.5)), .proceed, "few due: nothing to decide")
        XCTAssertEqual(m.frontierDecision(pageComponents: [abc], in: g, dueCount: 14, at: day(2.5)), .preferCourse,
                       "many due, but the new material is ready: take the course page")
        XCTAssertEqual(m.frontierDecision(pageComponents: [abc, bc], in: g, dueCount: 14, at: day(2.5)), .preferCourse,
                       "already-known components on the page don't count against it")
        XCTAssertEqual(m.frontierDecision(pageComponents: [abc], in: g, dueCount: 14, at: day(60)), .proceed,
                       "prerequisites that have faded no longer pull the course forward")
        XCTAssertEqual(m.frontierDecision(pageComponents: [], in: g, dueCount: 14, at: day(2.5)), .proceed)
    }

    func testFrontierRemediatesWeakPrerequisites() {
        let g = graph
        // a, b, c met and then lapsed; ab, bc fine.
        var events = [a, b, c, ab, bc].map { outcome([$0], write, .good, at: t0) }
        events += [a, b, c].map { outcome([$0], write, .again, at: day(1)) }
        events += [ab, bc].map { outcome([$0], write, .good, at: day(1)) }
        let m = KnowledgeModel(events: events)
        let weak = m.weakPrerequisites(of: [abc], in: g, at: day(1.01))
        XCTAssertEqual(Set(weak), [a, b, c])
        XCTAssertEqual(m.frontierDecision(pageComponents: [abc], in: g, dueCount: 0, at: day(1.01)), .remediate(weak))
        XCTAssertEqual(weak.count, 3)
        // Only two weak: below the threshold, the course proceeds.
        let two = KnowledgeModel(events: events + [outcome([c], write, .good, at: day(1.02)), outcome([c], write, .good, at: day(3))])
        XCTAssertEqual(two.weakPrerequisites(of: [abc], in: g, at: day(3.1)).count, 2)
        XCTAssertEqual(two.frontierDecision(pageComponents: [abc], in: g, dueCount: 0, at: day(3.1)), .proceed)
        // A prerequisite never met isn't weak: the course will teach it.
        XCTAssertEqual(KnowledgeModel().weakPrerequisites(of: [abc], in: g, at: t0), [])
        // Remediation is capped and weakest first.
        let capped = m.frontierDecision(pageComponents: [abc], in: g, dueCount: 0, at: day(1.01), policy: FrontierPolicy(maxRemediation: 2))
        XCTAssertEqual(capped, .remediate(Array(weak.prefix(2))))
    }

    // MARK: Probes

    func testProbesAreDrawnStratifiedAndDeterministic() {
        // Twelve words at spread recall: learned on different days.
        let words = (0..<12).map { ComponentID("word", "w\($0)") }
        let events = words.enumerated().map { i, w in outcome([w], write, .good, at: day(-Double(i) * 5)) }
        let m = KnowledgeModel(events: events)
        let candidates = words.map { ReviewCandidate(id: $0.key, components: [$0], modalities: [write, say], introduced: day(-30)) }
        let policy = ProbePolicy(sampleSize: 6)
        let probes = m.drawProbes(candidates, at: day(0.5), policy: policy, seed: 7)
        XCTAssertEqual(probes.count, 18, "six items × three delays")
        func shape(_ ps: [Probe]) -> [String] { ps.map { "\($0.candidate)/\($0.modality)/\($0.due.timeIntervalSince1970)" } }
        XCTAssertEqual(shape(probes), shape(m.drawProbes(candidates, at: day(0.5), policy: policy, seed: 7)), "the same seed draws the same set")
        XCTAssertNotEqual(shape(probes), shape(m.drawProbes(candidates, at: day(0.5), policy: policy, seed: 8)))
        XCTAssertTrue(probes.allSatisfy { $0.modality == write }, "only a modality the item was practised in")
        let ids = Set(probes.map(\.candidate))
        XCTAssertEqual(ids.count, 6)
        let rs = ids.map { m.retrievability(ComponentID("word", $0), write, at: day(0.5))! }.sorted()
        XCTAssertLessThan(rs.first!, 0.5, "some weak")
        XCTAssertGreaterThan(rs.last!, 0.85, "some strong")
        XCTAssertEqual(Set(probes.map { $0.due.timeIntervalSince(day(0.5)) }), [86_400, 7 * 86_400, 30 * 86_400])

        var ledger = ProbeLedger()
        XCTAssertTrue(ledger.isDrawDue(at: day(0.5), policy: policy))
        ledger.add(probes, at: day(0.5))
        XCTAssertFalse(ledger.isDrawDue(at: day(3), policy: policy))
        XCTAssertTrue(ledger.isDrawDue(at: day(8), policy: policy))
        XCTAssertFalse(ledger.isDrawDue(at: day(8), policy: ProbePolicy(enabled: false)))
        XCTAssertTrue(ledger.pending(at: day(1.4)).isEmpty)
        XCTAssertEqual(ledger.pending(at: day(1.6)).count, 6)
        let first = ledger.pending(at: day(1.6))[0]
        ledger.place(first.id, on: "p1#4")
        XCTAssertEqual(ledger.pending(at: day(1.6)).count, 5)
        XCTAssertNil(ledger.observe(item: "p9#0", predicted: 0.5, observed: true, at: day(1.7)), "nothing placed there")
        XCTAssertNotNil(ledger.observe(item: "p1#4", predicted: 0.8, observed: false, at: day(1.7)))
        XCTAssertEqual(ledger.observations, [Calibration.Observation(modality: write, predicted: 0.8, observed: false)])
        XCTAssertNil(ledger.observe(item: "p1#4", predicted: 0.8, observed: true, at: day(1.8)), "answered once")
    }

    func testCalibrationMathAndNudge() {
        // Perfectly calibrated: at predicted p, a share p recalled.
        var obs: [Calibration.Observation] = []
        for (p, n) in [(0.3, 10), (0.7, 10), (0.9, 10)] {
            for i in 0..<n { obs.append(.init(modality: write, predicted: p, observed: Double(i) < p * Double(n) - 1e-9)) }
        }
        let good = Calibration(observations: obs, binCount: 5)
        let all = good.overall!
        XCTAssertEqual(all.count, 30)
        XCTAssertEqual(all.predicted, (0.3 + 0.7 + 0.9) / 3, accuracy: 1e-9)
        XCTAssertEqual(all.observed, 19.0 / 30, accuracy: 1e-9)
        XCTAssertEqual(all.bins.count, 3)
        XCTAssertEqual(all.bins[0].lower, 0.2, accuracy: 1e-9)
        XCTAssertEqual(all.bins[0].observed, 0.3, accuracy: 1e-9)
        XCTAssertEqual(all.error, 0, accuracy: 1e-9, "every bin's outcome matches its prediction")
        XCTAssertEqual(all.brier, (10 * (0.3 * 0.49 + 0.7 * 0.09) + 10 * (0.7 * 0.09 + 0.3 * 0.49) + 10 * (0.9 * 0.01 + 0.1 * 0.81)) / 30, accuracy: 1e-9)
        XCTAssertEqual(good.series.count, 1)
        XCTAssertEqual(good.series[0].modality, write)
        XCTAssertNil(Calibration(observations: []).overall)
        XCTAssertEqual(ParameterNudge.scale(current: 1, calibration: good, minObservations: 12), 1, "calibrated: no nudge")

        // Overconfident: predicted 0.9, recalled 0.5.
        let over = Calibration(observations: (0..<20).map { .init(modality: say, predicted: 0.9, observed: $0 < 10) })
        XCTAssertEqual(over.overall!.overconfidence, 0.4, accuracy: 1e-9)
        XCTAssertEqual(ParameterNudge.scale(current: 1, calibration: over, minObservations: 12), 0.98, accuracy: 1e-9, "one small step down")
        XCTAssertEqual(ParameterNudge.scale(current: 1, calibration: over, minObservations: 30), 1, "too few observations: no step")
        XCTAssertEqual(ParameterNudge.scale(current: 0.705, calibration: over, minObservations: 12), 0.7, accuracy: 1e-9, "bounded")
        let under = Calibration(observations: (0..<20).map { .init(modality: say, predicted: 0.5, observed: $0 < 18) })
        XCTAssertEqual(ParameterNudge.scale(current: 1.29, calibration: under, minObservations: 12), 1.3, accuracy: 1e-9)

        // The scaled scheduler: shorter stabilities, so an item comes due sooner; the same difficulty.
        let f = FSRS(), g = FSRS().scalingStability(by: 0.8)
        XCTAssertEqual(g.initialStability(.good), 0.8 * f.initialStability(.good), accuracy: 1e-9)
        let s0 = f.next(nil, elapsedDays: 0, grade: .good), s0g = g.next(nil, elapsedDays: 0, grade: .good)
        XCTAssertEqual(s0g.difficulty, s0.difficulty)
        let s1 = f.next(s0, elapsedDays: 3, grade: .good), s1g = g.next(s0, elapsedDays: 3, grade: .good)
        XCTAssertEqual(s1g.stability - s0.stability, 0.8 * (s1.stability - s0.stability), accuracy: 1e-6, "growth scales too")
        XCTAssertEqual(FSRS().scalingStability(by: 1).w, FSRS.defaultWeights)
        // Export carries it.
        let m = KnowledgeModel(events: [outcome([ab], write, .good, at: t0)])
        var doc = GraphExport(domain: "toy", modalities: [write], graph: graph, model: m, at: day(1))
        doc.calibration = over
        let json = try! JSONSerialization.jsonObject(with: doc.json()) as! [String: Any]
        XCTAssertNotNil((json["calibration"] as! [String: Any])["overall"])
    }

    // MARK: Fatigue

    /// `n` outcomes a minute apart from `start`, with the given latencies and grades.
    func session(start: Date, latencies: [Double], misses: Set<Int> = [], attemptsOn: Set<Int> = []) -> [LearningEvent] {
        var out: [LearningEvent] = []
        for (i, l) in latencies.enumerated() {
            let t = start.addingTimeInterval(Double(i) * 60)
            let item = "p#\(i)"
            if attemptsOn.contains(i) {
                out.append(LearningEvent(t: t.addingTimeInterval(-20), type: .attempt, item: item, modality: write, components: [a], correct: false))
                out.append(LearningEvent(t: t.addingTimeInterval(-10), type: .attempt, item: item, modality: write, components: [a], correct: false))
            }
            out.append(outcome([a], write, misses.contains(i) ? .again : .good, at: t, item: item, latency: l))
        }
        return out
    }

    func testFatigueFromSyntheticSessions() {
        let start = t0
        // Steady: 12 quick right answers, 12 minutes.
        let steady = session(start: start, latencies: Array(repeating: 6, count: 12))
        let e1 = Fatigue.estimate(events: steady, now: start.addingTimeInterval(11 * 60 + 10), pageStarted: start)
        XCTAssertEqual(e1.level, .sharp, "\(e1)")
        XCTAssertEqual(e1.signals.items, 12)
        XCTAssertEqual(e1.signals.latencyRatio ?? 0, 1, accuracy: 1e-9)
        XCTAssertTrue(e1.reasons.isEmpty)

        // Tired: answers slow to twice the time, 3 of the last 8 missed, retries, 40 minutes in.
        let slow = (0..<12).map { 6.0 + Double($0) } + [16, 16, 16, 16]
        let tired = session(start: start, latencies: slow, misses: [12, 14, 15], attemptsOn: [12, 13, 14])
        let e2 = Fatigue.estimate(events: tired, now: start.addingTimeInterval(40 * 60), pageStarted: start)
        XCTAssertEqual(e2.level, .tired, "\(e2)")
        XCTAssertGreaterThan(e2.signals.latencyRatio ?? 0, 1.8)
        XCTAssertEqual(e2.signals.missRate ?? 0, 3.0 / 8, accuracy: 1e-9)
        XCTAssertGreaterThan(e2.signals.triesPerItem ?? 0, 1.5)
        XCTAssertTrue(e2.reasons.contains { $0.contains("minutes") }, "\(e2.reasons)")
        XCTAssertTrue(e2.reasons.contains { $0.contains("missed") }, "\(e2.reasons)")
        XCTAssertTrue(e2.reasons.contains { $0.contains("as long") }, "\(e2.reasons)")

        // Only the time signal counts before enough answers; a short page is steady.
        let three = session(start: start, latencies: [6, 6, 6])
        XCTAssertEqual(Fatigue.estimate(events: three, now: start.addingTimeInterval(3 * 60), pageStarted: start).level, .steady)
        XCTAssertNil(Fatigue.estimate(events: three, now: start.addingTimeInterval(3 * 60)).signals.missRate)

        // A long pause starts a new session: yesterday's misses don't tire today.
        let yesterday = session(start: start, latencies: Array(repeating: 12, count: 10), misses: [5, 6, 7, 8, 9])
        let today = session(start: day(1), latencies: Array(repeating: 6, count: 8))
        let e3 = Fatigue.estimate(events: yesterday + today, now: day(1).addingTimeInterval(8 * 60))
        XCTAssertEqual(e3.signals.items, 8)
        XCTAssertEqual(e3.signals.missRate ?? 1, 0)
        XCTAssertEqual(Fatigue.sessionEvents(yesterday + today, now: day(1).addingTimeInterval(8 * 60)).count, 8)
        XCTAssertTrue(Fatigue.sessionEvents(yesterday, now: day(1)).isEmpty, "the last event is long past: no session")
    }

    /// Component-level annotations are not more answers or more tries: six corrections on one line
    /// of free writing are one completion, and a take logged twice (the attempt and its
    /// pronunciation annotation) is one try.
    func testAnnotationsDoNotInflateFatigue() {
        let start = t0
        var events = session(start: start, latencies: Array(repeating: 6, count: 6))
        let t = start.addingTimeInterval(7 * 60)
        for i in 0..<6 { events.append(LearningEvent(id: "edit\(i)", t: t, type: .outcome, item: "p#d", modality: write, components: [a], grade: .again)) }
        events.append(outcome([a], write, .good, at: t, item: "p#d", latency: 6))
        let u = start.addingTimeInterval(8 * 60)
        events.append(LearningEvent(t: u, type: .attempt, item: "p#s", modality: write, components: [a], correct: false, attempt: 1))
        events.append(LearningEvent(t: u, type: .attempt, item: "p#s", modality: write, components: [a], correct: false, attempt: 1))
        events.append(outcome([a], write, .good, at: u.addingTimeInterval(5), item: "p#s", latency: 6))
        let e = Fatigue.estimate(events: events, now: u.addingTimeInterval(10), pageStarted: start)
        XCTAssertEqual(e.signals.items, 8, "\(e)")
        XCTAssertEqual(e.signals.missRate ?? 1, 0)
        XCTAssertEqual(e.signals.triesPerItem ?? 0, 9.0 / 8, accuracy: 1e-9)
        XCTAssertNotEqual(e.level, .tired)
        XCTAssertEqual(Fatigue.completions(in: Fatigue.sessionEvents(events, now: u.addingTimeInterval(10))).map(\.item).filter { $0 == "p#d" }.count, 1)
    }

    func testHourlyPreference() {
        let cal = utc
        let morning = cal.date(bySettingHour: 8, minute: 0, second: 0, of: t0)!
        let evening = cal.date(bySettingHour: 20, minute: 0, second: 0, of: t0)!
        var events: [LearningEvent] = []
        // Saying goes well in the morning (9/10) and badly in the evening (3/10); writing is even.
        for i in 0..<10 {
            events.append(outcome([a], say, i < 9 ? .good : .again, at: morning.addingTimeInterval(Double(i))))
            events.append(outcome([a], say, i < 3 ? .good : .again, at: evening.addingTimeInterval(Double(i))))
            events.append(outcome([a], write, i < 6 ? .good : .again, at: morning.addingTimeInterval(Double(i) + 100)))
            events.append(outcome([a], write, i < 6 ? .good : .again, at: evening.addingTimeInterval(Double(i) + 100)))
        }
        let pref = HourlyPreference(events: events, calendar: cal)
        XCTAssertEqual(pref.accuracy(say, hour: 9).trials, 10)
        XCTAssertEqual(pref.accuracy(say, hour: 9).rate, 10.0 / 12, accuracy: 1e-9, "smoothed")
        XCTAssertEqual(pref.order([write, say], hour: 9), [say, write])
        XCTAssertEqual(pref.order([write, say], hour: 21), [write, say])
        XCTAssertEqual(pref.order([write, say], hour: 14), [write, say], "no evidence for the afternoon: the given order")
        XCTAssertEqual(pref.standout([write, say], hour: 9), say)
        XCTAssertNil(pref.standout([write, say], hour: 14))
        XCTAssertEqual(HourlyPreference.band(ofHour: 23), 3)
    }
}

extension DayTests {
    func testProbeProvenanceAndLegacyDecoding() throws {
        var ledger = ProbeLedger(probes: [Probe(id: "p", candidate: "x", modality: write, drawn: day(0), due: day(7))])
        ledger.place("p", on: "page#0")
        ledger.observe(item: "page#0", predicted: 0.8, observed: false, at: day(9), lastExposure: day(8),
                       firstAttempt: true, assistance: [], context: "new context")
        let p = try XCTUnwrap(ledger.probes.first)
        XCTAssertEqual(p.lastExposure, day(8), "scheduled seven days does not mean seven days without practice")
        XCTAssertEqual(p.context, "new context")
        XCTAssertEqual(p.firstAttempt, true)
        XCTAssertEqual(try JSONDecoder().decode(Probe.self, from: JSONEncoder().encode(p)), p)
        let old = Probe(id: "old", candidate: "x", modality: write, drawn: day(0), due: day(7))
        let decoded = try JSONDecoder().decode(Probe.self, from: JSONEncoder().encode(old))
        XCTAssertNil(decoded.lastExposure)
        XCTAssertNil(decoded.firstAttempt)
    }
}
