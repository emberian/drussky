import LearningCore
import XCTest

/// The metric store on a toy subject: a speed measured over weeks.
final class MetricsTests: XCTestCase {
    let thing = ComponentID("thing", "x"), other = ComponentID("thing", "y")
    let t0 = Date(timeIntervalSince1970: 1_800_000_000)
    func day(_ n: Double) -> Date { t0.addingTimeInterval(n * 86_400) }

    func testAppendAndQuery() {
        var s = MetricStore()
        s.append(10, metric: "speed", component: thing, at: day(0))
        s.append(12, metric: "speed", component: thing, at: day(2), note: "fast")
        s.append(3, metric: "speed", component: other, at: day(1))
        s.append(7, metric: "size", component: thing, at: day(1))
        XCTAssertEqual(s.samples("speed", of: thing).map(\.value), [10, 12])
        XCTAssertEqual(s.latest("speed", of: thing)?.value, 12)
        XCTAssertEqual(s.latest("speed", of: thing)?.note, "fast")
        XCTAssertEqual(s.samples("speed", of: thing, note: "fast").count, 1)
        XCTAssertEqual(s.components(with: "speed"), [thing, other], "most recently measured first")
        XCTAssertEqual(s.metrics(of: thing), ["size", "speed"])
        XCTAssertNil(s.latest("speed", of: ComponentID("thing", "z")))
        // Out-of-order arrival still gives a sorted series.
        s.append(11, metric: "speed", component: thing, at: day(1))
        XCTAssertEqual(s.samples("speed", of: thing).map(\.value), [10, 11, 12])
        XCTAssertEqual(s.records.count, 5)
    }

    func testChangeComparesMediansOfTwoWindows() {
        var s = MetricStore()
        let week = 7.0 * 86_400
        // Last week 60, 62, 70; this week 80, 84, 90.
        for (d, v) in [(-13.0, 60.0), (-10, 70), (-8, 62), (-6, 80), (-3, 90), (-1, 84)] {
            s.append(v, metric: "speed", component: thing, at: day(d))
        }
        let c = s.change("speed", of: thing, window: week, at: day(0))
        XCTAssertEqual(c?.recent, 84)
        XCTAssertEqual(c?.earlier, 62)
        XCTAssertEqual(c?.recentCount, 3)
        XCTAssertEqual(c?.earlierCount, 3)
        XCTAssertEqual(c!.relative!, 22.0 / 62, accuracy: 1e-9)
        // One sample on a side is not a change.
        XCTAssertNil(s.change("speed", of: thing, window: week, at: day(0), minimum: 4))
        XCTAssertNil(s.change("speed", of: other, window: week, at: day(0)))
        // Windows are relative to `at`: a week later the recent window is empty.
        XCTAssertNil(s.change("speed", of: thing, window: week, at: day(8)))
        let m = s.mean("speed", of: thing, window: week, at: day(0))
        XCTAssertEqual(m?.value ?? 0, (80 + 90 + 84) / 3, accuracy: 1e-9)
        XCTAssertEqual(m?.count, 3)
        XCTAssertNil(s.mean("speed", of: thing, window: week, at: day(30)))
        XCTAssertEqual(MetricStore.median([3, 1, 2]), 2)
        XCTAssertEqual(MetricStore.median([4, 1, 2, 3]), 2.5)
        XCTAssertNil(MetricStore.median([]))
    }

    func testLogRoundTripAndTolerance() throws {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("metrics-\(UUID().uuidString).jsonl")
        let log = MetricLog(url: url)
        var s = MetricStore()
        let r1 = s.append(1.5, metric: "speed", component: thing, at: day(0), note: "a")
        let r2 = s.append(2.5, metric: "speed", component: thing, at: day(1))
        XCTAssertTrue(log.append([r1]))
        // A torn line in between must not take the rest down.
        let handle = try FileHandle(forWritingTo: url)
        try handle.seekToEnd()
        try handle.write(contentsOf: Data("{\"v\":1,\"component\":\"thing:x\",\"metric\":".utf8))
        try handle.close()
        XCTAssertTrue(log.append([r2]))
        let back = log.load()
        XCTAssertEqual(back, [r1, r2])
        let rebuilt = MetricStore(records: back)
        XCTAssertEqual(rebuilt.samples("speed", of: thing), s.samples("speed", of: thing))
        XCTAssertEqual(rebuilt, s)
    }
}
