import CoreGraphics
import Foundation

/// Hand-printed stroke models for every letter (`strokes.json`, built by `scripts/build_strokes.py`).
/// Coordinates are the writing strip's 132-pt design space: y is absolute on the strip, x runs from
/// the letter's pen origin in the model font. Points run in writing direction.
struct StrokeModels: Sendable {
    struct Lines: Codable, Sendable {
        var cap: CGFloat
        var xHeight: CGFloat
        var baseline: CGFloat
        var descender: CGFloat
    }

    struct Stroke: Sendable {
        var name: String
        var points: [CGPoint]
        /// Either direction is fine (the diagonal of и).
        var free: Bool

        var length: CGFloat { zip(points, points.dropFirst()).reduce(0) { $0 + hypot($1.1.x - $1.0.x, $1.1.y - $1.0.y) } }
    }

    struct Letter: Sendable {
        var glyph: String
        var advance: CGFloat
        var strokes: [Stroke]
        /// Any stroke may come first (т, н, х, ф).
        var anyOrder: Bool

        var bounds: CGRect {
            let pts = strokes.flatMap(\.points)
            let xs = pts.map(\.x), ys = pts.map(\.y)
            guard let x0 = xs.min(), let x1 = xs.max(), let y0 = ys.min(), let y1 = ys.max() else { return .zero }
            return CGRect(x: x0, y: y0, width: x1 - x0, height: y1 - y0)
        }

        /// The widest horizontal gap between parts of the letter (the two halves of ы).
        var internalGap: CGFloat {
            let spans = strokes.map { s in (s.points.map(\.x).min() ?? 0, s.points.map(\.x).max() ?? 0) }.sorted { $0.0 < $1.0 }
            var reach = spans.first?.1 ?? 0
            var gap: CGFloat = 0
            for span in spans.dropFirst() {
                gap = max(gap, span.0 - reach)
                reach = max(reach, span.1)
            }
            return gap
        }
    }

    var lines: Lines
    var letters: [String: Letter]

    func letter(_ glyph: String) -> Letter? { letters[glyph] }

    static let shared: StrokeModels? = {
        guard let url = Bundle.main.url(forResource: "strokes", withExtension: "json"),
              let data = try? Data(contentsOf: url) else { return nil }
        return try? decode(data)
    }()

    static func decode(_ data: Data) throws -> StrokeModels {
        struct RawStroke: Decodable { var name: String; var points: [[CGFloat]]; var free: Bool? }
        struct RawLetter: Decodable { var advance: CGFloat; var strokes: [RawStroke]; var anyOrder: Bool? }
        struct Raw: Decodable { var lines: Lines; var letters: [String: RawLetter] }
        let raw = try JSONDecoder().decode(Raw.self, from: data)
        var letters: [String: Letter] = [:]
        for (glyph, l) in raw.letters {
            letters[glyph] = Letter(glyph: glyph, advance: l.advance,
                                    strokes: l.strokes.map { Stroke(name: $0.name, points: $0.points.map { CGPoint(x: $0[0], y: $0[1]) }, free: $0.free ?? false) },
                                    anyOrder: l.anyOrder ?? false)
        }
        return StrokeModels(lines: raw.lines, letters: letters)
    }
}
