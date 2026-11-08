import CoreGraphics
import XCTest
@testable import Drussky

/// The letter-formation comparer on synthetic ink made from the stroke models themselves.
final class FormationCoachTests: XCTestCase {
    private var models: StrokeModels!

    override func setUpWithError() throws {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "strokes", withExtension: "json"))
        models = try StrokeModels.decode(Data(contentsOf: url))
    }

    /// The model's strokes as a learner might write them: shifted along the line, a little
    /// wobble, optionally scaled about the baseline.
    private func ink(_ glyph: String, at x: CGFloat = 20, scale: CGFloat = 1, seed: Double = 0) -> [[CGPoint]] {
        let base = models.lines.baseline
        return models.letter(glyph)!.strokes.enumerated().map { j, s in
            s.points.enumerated().map { i, p in
                CGPoint(x: x + p.x * scale + sin(Double(i) * 0.9 + seed + Double(j)) * 0.9,
                        y: base + (p.y - base) * scale + cos(Double(i) * 0.7 + seed) * 0.9)
            }
        }
    }

    private func hint(_ ink: [[CGPoint]], _ upper: String) -> String? {
        let lower = upper.lowercased()
        return FormationCoach.hint(for: ink, upper: upper, lower: lower, models: models)
    }

    func testEveryLetterHasUpperAndLowerModels() {
        XCTAssertEqual(models.letters.count, 66)
        for ch in "АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ" {
            let up = String(ch), low = up.lowercased()
            for g in [up, low] {
                let m = models.letter(g)
                XCTAssertNotNil(m, g)
                XCTAssertFalse(m?.strokes.isEmpty ?? true, g)
                XCTAssertTrue(m?.strokes.allSatisfy { $0.points.count >= 2 } ?? false, g)
            }
        }
    }

    /// The dashed guide is the model font's x-height, and the stroke models sit on it.
    func testGuideLinesMatchTheModels() {
        let m = StripMetrics(height: StripMetrics.designHeight)
        XCTAssertEqual(m.xHeightY(cursive: false), models.lines.xHeight, accuracy: 0.6)
        XCTAssertEqual(m.baselineY, models.lines.baseline, accuracy: 0.1)
        XCTAssertEqual(m.capY, models.lines.cap, accuracy: 0.1)
        let o = models.letter("о")!.bounds
        XCTAssertEqual(o.minY, m.xHeightY(cursive: false), accuracy: 1.5)
        let cursive = m.xHeightY(cursive: true)
        XCTAssertTrue(cursive > m.capY + 5 && cursive < m.baselineY - 20, "cursive x-height \(cursive)")
    }

    func testEveryModelWrittenAsModelledIsPraised() {
        for (glyph, _) in models.letters {
            let upper = glyph.uppercased()
            let h = hint(ink(glyph), upper)
            XCTAssertEqual(h, "That \(glyph) is formed like the model.", glyph)
        }
    }

    /// Slanted, smaller, shakier writing in the right order and direction draws no stroke hints.
    func testHandLikeVariationDrawsNoStrokeHints() {
        let base = models.lines.baseline
        for (glyph, model) in models.letters {
            let shaky = model.strokes.enumerated().map { j, s in
                s.points.enumerated().map { i, p in
                    CGPoint(x: 30 + p.x * 0.85 + (base - p.y) * 0.18 + sin(Double(i) * 1.3 + Double(j)) * 2.2,
                            y: base + (p.y - base) * 0.85 + cos(Double(i) * 0.8) * 2.2)
                }
            }
            let a = FormationCoach.assess(shaky, candidates: [glyph.uppercased(), glyph.lowercased()].compactMap { models.letter($0) }, lines: models.lines)
            for issue in a?.issues ?? [] {
                switch issue {
                case .firstStroke, .reversed, .extraStrokes: XCTFail("\(glyph): \(issue)")
                default: break
                }
            }
        }
    }

    func testReversedDirection() {
        var t = ink("Т")
        t[1].reverse()  // the upright drawn bottom-up
        XCTAssertEqual(hint(t, "Т"), "Draw the upright of Т from the top down.")

        var o = ink("о")
        o[0].reverse()  // clockwise
        XCTAssertEqual(hint(o, "О"), "Go round о anticlockwise, starting at the top.")
        // A left hand rounds о the other way; with direction left alone the clockwise о is simply well formed.
        XCTAssertEqual(FormationCoach.hint(for: o, upper: "О", lower: "о", models: models, directionMatters: false), "That о is formed like the model.")

        var h = ink("Н")
        h[2].reverse()
        XCTAssertEqual(hint(h, "Н"), "Draw the crossbar of Н from left to right.")

        // A wide, straight-lined А (as touch strokes make it) with the bar drawn right to left.
        let cap = models.lines.cap, base = models.lines.baseline
        let wide: [[CGPoint]] = [[CGPoint(x: 240, y: cap), CGPoint(x: 200, y: base)], [CGPoint(x: 240, y: cap), CGPoint(x: 280, y: base)],
                                 [CGPoint(x: 265, y: 70), CGPoint(x: 215, y: 70)]].map { FormationCoach.resample($0, 20) }
        XCTAssertEqual(hint(wide, "А"), "Draw the crossbar of А from left to right.")
        XCTAssertEqual(hint(wide.enumerated().map { $0 == 2 ? $1.reversed() : $1 }, "А"), "That А is formed like the model.")

        var i = ink("И")
        i[1].reverse()  // the diagonal may go either way
        XCTAssertEqual(hint(i, "И"), "That И is formed like the model.")
    }

    func testWrongOrder() {
        let b = ink("Б")
        XCTAssertEqual(hint([b[1], b[0], b[2]], "Б"), "Start Б at the top-left with the upright, then the roof.")
        let t = ink("т")
        XCTAssertEqual(hint([t[1], t[0]], "Т"), "That т is formed like the model.", "т may start either way")
    }

    func testProportionAndLines() {
        // Small б the size of a capital: still recognisably б, but far too tall.
        XCTAssertEqual(hint(ink("б", scale: 1.6), "Б"), FormationCoach.phrase(.tooBig, model: models.letter("б")!))
        // д with its feet cut off at the baseline.
        let d = ink("д").map { $0.map { CGPoint(x: $0.x, y: min($0.y, models.lines.baseline)) } }
        XCTAssertEqual(hint(d, "Д"), "The feet of д go just below the baseline.")
        // к floating well above the baseline.
        let k = ink("к").map { $0.map { CGPoint(x: $0.x, y: $0.y - 22) } }
        XCTAssertEqual(hint(k, "К"), "Sit к on the baseline.")
    }

    func testRepeatedAlongTheLineHintsTheLatest() {
        var line: [[CGPoint]] = []
        line += ink("Б", at: 16)
        line += ink("б", at: 100, seed: 1)
        var bad = ink("б", at: 175, seed: 2)
        bad = bad.map { $0.map { CGPoint(x: $0.x, y: $0.y - 24) } }
        line += bad
        XCTAssertEqual(hint(line, "Б"), "Sit б on the baseline.")
        // Then a good one after it.
        line += ink("б", at: 250, seed: 3)
        XCTAssertEqual(hint(line, "Б"), "That б is formed like the model.")
    }

    func testSegmentationKeepsYAndJoinsDots() {
        let models = self.models!
        var line: [[CGPoint]] = []
        for (n, x) in [CGFloat(16), 120, 220].enumerated() { line += ink(n == 1 ? "ы" : "Ы", at: x, seed: Double(n)) }
        line += ink("ё", at: 310)
        let groups = FormationCoach.segment(line, joinGap: models.letter("Ы")!.internalGap + 5, maxWidth: models.letter("Ы")!.bounds.width * 1.4 + 6)
        XCTAssertEqual(groups.map(\.count), [3, 3, 3, 3])
    }

    func testScribbleSaysNothing() {
        let zigzag = [(0..<30).map { CGPoint(x: 20 + CGFloat($0) * 3, y: $0 % 2 == 0 ? 30 : 90) }]
        XCTAssertNil(hint(zigzag, "Б"))
    }
}
