import CoreGraphics
import PencilKit
import XCTest
@testable import Drussky

/// Vision over hundreds of synthetic ink samples. On the simulator one accurate recognition costs
/// 0.15–0.5 s and Vision runs them one at a time whatever the concurrency (measured 2026-11-08:
/// ten in a task group took exactly as long as ten in a row), so these take as long as they have
/// samples × passes; they live apart from the fast loop (`scripts/test.sh slow`) and run when the
/// ink or recognition code changes.
@MainActor
final class InkReaderSlowTests: XCTestCase {
    private var m: CursiveModels!

    override func setUp() async throws {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "strokes-cursive", withExtension: "json"))
        m = try CursiveModels.decode(Data(contentsOf: url))
    }

    /// 156 single-line pen-stroke samples (52 targets × three hands with slant and wobble) through
    /// the whole pipeline; at least three in four read as their target.
    func testInkReaderReadsPenStrokes() async throws {
        let url = try XCTUnwrap(Bundle(for: Self.self).url(forResource: "hershey-ink", withExtension: "json"))
        let fixture = try JSONDecoder().decode([String: [[[[Double]]]]].self, from: Data(contentsOf: url))
        let ink = PKInk(.pen, color: .black)
        var samples: [InkReader.Sample] = []
        for (target, variants) in fixture.sorted(by: { $0.key < $1.key }) {
            for strokes in variants {
                let pk = strokes.map { pts in
                    PKStroke(ink: ink, path: PKStrokePath(controlPoints: pts.enumerated().map { i, p in
                        PKStrokePoint(location: CGPoint(x: p[0], y: p[1]), timeOffset: Double(i) * 0.01,
                                      size: CGSize(width: 3.5, height: 3.5), opacity: 1, force: 1, azimuth: 0, altitude: .pi / 2)
                    }, creationDate: Date()))
                }
                samples.append(.init(drawing: PKDrawing(strokes: pk), target: target))
            }
        }
        let readings = await InkReader.readAll(samples)
        var hits = 0
        var misses: [String] = []
        for (sample, reading) in zip(samples, readings) {
            if reading?.matches == true { hits += 1 } else { misses.append("\(sample.target)→\(reading?.best ?? "nil")") }
        }
        let total = samples.count
        print("PEN hits: \(hits)/\(total) misses:", misses)
        XCTAssertGreaterThanOrEqual(Double(hits) / Double(total), 0.75, "misses: \(misses)")
    }

    /// Synthetic cursive from each bundled hand and from the stroke models, read by Vision with
    /// the normal passes and with the upright-sheared pass added. A measurement first: the
    /// assertion is only that the pipeline reads some cursive at all.
    func testInkReaderOnCursiveHands() async throws {
        CursiveFonts.register()
        let words = ["кот", "мама", "окно", "она", "книга", "спасибо", "молоко", "где", "хорошо", "звезда"]
        // Every hand × every word, with and without the sheared pass, read in one batch (the pass
        // is a per-sample choice, so neither half waits for the other).
        let hands = CursiveFonts.names.map { Optional($0) } + [nil]  // nil: the stroke models written through
        var samples: [InkReader.Sample] = [], labels: [(hand: String?, pass: Bool)] = []
        for pass in [false, true] {
            for hand in hands {
                for (i, w) in words.enumerated() {
                    let d = hand.map { SyntheticInk.drawing(for: w, height: 132, wobble: 1, font: $0) }
                        ?? strokeInk(w, slant: CGFloat(i % 3) * 0.12, jitter: 0.6)
                    samples.append(.init(drawing: d, target: w, cursivePass: pass))
                    labels.append((hand, pass))
                }
            }
        }
        let readings = await InkReader.readAll(samples)
        var report: [String] = []
        var strokesHits = (off: 0, on: 0), fontHits = (off: 0, on: 0), total = 0
        for pass in [false, true] {
            for hand in hands {
                let hits = zip(labels, readings).filter { $0.0.hand == hand && $0.0.pass == pass && $0.1?.matches == true }.count
                report.append("\(hand ?? "stroke models") pass=\(pass): \(hits)/\(words.count)")
                switch (hand, pass) {
                case (.some, false): fontHits.off += hits; total += words.count
                case (.some, true): fontHits.on += hits
                case (.none, false): strokesHits.off = hits
                case (.none, true): strokesHits.on = hits
                }
            }
        }
        print("CURSIVE-HANDS", report.joined(separator: " | "), "fonts total \(total)")
        XCTAssertGreaterThan(fontHits.off + strokesHits.off, 0)
    }

    /// The models written through as pen strokes, slanted a little more and jittered.
    private func strokeInk(_ text: String, slant: CGFloat, jitter: CGFloat) -> PKDrawing {
        let ink = PKInk(.pen, color: .black)
        var n = 0
        let strokes = CursiveLayout.strokes(text, in: m, x0: 16).map { pts in
            PKStroke(ink: ink, path: PKStrokePath(controlPoints: pts.enumerated().map { i, p in
                n += 1
                let x = p.x + (m.lines.baseline - p.y) * slant + sin(Double(n) * 0.8) * jitter
                return PKStrokePoint(location: CGPoint(x: x, y: p.y + cos(Double(n) * 0.6) * jitter), timeOffset: Double(i) * 0.01,
                                     size: CGSize(width: 3.5, height: 3.5), opacity: 1, force: 1, azimuth: 0, altitude: .pi / 2)
            }, creationDate: Date()))
        }
        return PKDrawing(strokes: strokes)
    }
}
