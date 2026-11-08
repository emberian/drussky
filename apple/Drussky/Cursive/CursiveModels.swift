import CoreGraphics
import Foundation
import SwiftUI

/// School-cursive stroke models for the 66 letter forms (`strokes-cursive.json`, built by
/// `scripts/build_strokes_cursive.py`): the same design space as `StrokeModels`, plus where a
/// join from the letter before arrives (`entry`) and where the join to the next leaves (`exit`,
/// absent on letters the school hand does not join onward), so whole words can be written through.
struct CursiveModels: Sendable {
    /// How two letters join, in a unit frame: (0,0) at the exit, (1,1) at the entry.
    struct Join: Sendable {
        var points: [CGPoint]
        /// An arch along the way, in design points.
        var lift: CGFloat
    }

    struct Letter: Sendable {
        var model: StrokeModels.Letter
        var entry: CGPoint
        var entryKind: String
        var exit: CGPoint?
        var exitKind: String?
        /// The one thing to say about forming this letter in cursive.
        var hint: String
        var glyph: String { model.glyph }
        var advance: CGFloat { model.advance }
    }

    /// The forms as the coach compares them (glyph → strokes), for `FormationCoach`.
    var models: StrokeModels
    var letters: [String: Letter]
    /// By "<exitKind>-<entryKind>".
    var joins: [String: Join]
    var slant: CGFloat
    var space: CGFloat
    var lines: StrokeModels.Lines { models.lines }

    func letter(_ glyph: String) -> Letter? { letters[glyph] }

    func join(from a: Letter, to b: Letter) -> Join? {
        guard let k = a.exitKind else { return nil }
        return joins["\(k)-\(b.entryKind)"]
    }

    static let shared: CursiveModels? = {
        guard let url = Bundle.main.url(forResource: "strokes-cursive", withExtension: "json"),
              let data = try? Data(contentsOf: url) else { return nil }
        return try? decode(data)
    }()

    static func decode(_ data: Data) throws -> CursiveModels {
        struct RawStroke: Decodable { var name: String; var points: [[CGFloat]]; var free: Bool? }
        struct RawLetter: Decodable {
            var advance: CGFloat; var strokes: [RawStroke]; var anyOrder: Bool?
            var entry: [CGFloat]; var entryKind: String; var exit: [CGFloat]?; var exitKind: String?; var hint: String
        }
        struct RawJoin: Decodable { var points: [[CGFloat]]; var lift: CGFloat? }
        struct Raw: Decodable {
            var lines: StrokeModels.Lines; var slant: CGFloat; var space: CGFloat
            var letters: [String: RawLetter]; var connections: [String: RawJoin]
        }
        let raw = try JSONDecoder().decode(Raw.self, from: data)
        var letters: [String: Letter] = [:]
        var models: [String: StrokeModels.Letter] = [:]
        for (glyph, l) in raw.letters {
            let model = StrokeModels.Letter(glyph: glyph, advance: l.advance,
                                            strokes: l.strokes.map { StrokeModels.Stroke(name: $0.name, points: $0.points.map { CGPoint(x: $0[0], y: $0[1]) }, free: $0.free ?? false) },
                                            anyOrder: l.anyOrder ?? false)
            models[glyph] = model
            letters[glyph] = Letter(model: model, entry: CGPoint(x: l.entry[0], y: l.entry[1]), entryKind: l.entryKind,
                                    exit: l.exit.map { CGPoint(x: $0[0], y: $0[1]) }, exitKind: l.exitKind, hint: l.hint)
        }
        let joins = raw.connections.mapValues { Join(points: $0.points.map { CGPoint(x: $0[0], y: $0[1]) }, lift: $0.lift ?? 0) }
        return CursiveModels(models: StrokeModels(lines: raw.lines, letters: models), letters: letters, joins: joins, slant: raw.slant, space: raw.space)
    }
}

/// Places the letters of a text along a line and draws the joins between them. The same
/// algorithm as `layout()` in `scripts/build_strokes_cursive.py`, so the proof sheet and the
/// app agree.
enum CursiveLayout {
    enum Item {
        /// A letter's model at its origin x.
        case letter(CursiveModels.Letter, x: CGFloat)
        /// The join into the letter that follows, in strip coordinates.
        case join([CGPoint])
    }

    struct Placement {
        var items: [Item]
        /// Pen position after the last letter (from x0).
        var width: CGFloat
    }

    /// Width a letter the models don't have takes (digits, Latin, punctuation).
    static let unknownAdvance: CGFloat = 20

    static func place(_ text: String, in m: CursiveModels, x0: CGFloat = 0) -> Placement {
        var items: [Item] = []
        var x = x0
        var prev: (letter: CursiveModels.Letter, x: CGFloat)?
        for ch in RussianText.unstressed(text) {
            if ch == " " {
                x += m.space
                prev = nil
                continue
            }
            guard let l = m.letter(String(ch)) else {
                x += unknownAdvance
                prev = nil
                continue
            }
            if let prev, let exit = prev.letter.exit, let join = m.join(from: prev.letter, to: l) {
                let a = CGPoint(x: prev.x + exit.x, y: exit.y)
                let b = CGPoint(x: x + l.entry.x, y: l.entry.y)
                items.append(.join(join.points.map { p in
                    CGPoint(x: a.x + (b.x - a.x) * p.x, y: a.y + (b.y - a.y) * p.y - join.lift * sin(.pi * p.x))
                }))
            }
            items.append(.letter(l, x: x))
            x += l.advance
            prev = (l, x - l.advance)
        }
        return Placement(items: items, width: x - x0)
    }

    static func width(_ text: String, in m: CursiveModels) -> CGFloat { place(text, in: m).width }

    /// Every polyline of the written text in writing order: each letter's strokes, with the
    /// join into it before them.
    static func strokes(_ text: String, in m: CursiveModels, x0: CGFloat = 0) -> [[CGPoint]] {
        place(text, in: m, x0: x0).items.flatMap { item -> [[CGPoint]] in
            switch item {
            case .join(let pts): [pts]
            case .letter(let l, let x): l.model.strokes.map { $0.points.map { CGPoint(x: $0.x + x, y: $0.y) } }
            }
        }
    }

    /// The letters with their origins, as the stroke demo lays them out.
    static func placedLetters(_ text: String, in m: CursiveModels, x0: CGFloat) -> [(StrokeModels.Letter, CGFloat)] {
        place(text, in: m, x0: x0).items.compactMap {
            if case .letter(let l, let x) = $0 { return (l.model, x) }
            return nil
        }
    }

    /// The text as one path, scaled by `scale` about the strip's origin and offset by `origin`.
    static func path(_ text: String, in m: CursiveModels, origin: CGPoint = .zero, scale: CGFloat = 1) -> Path {
        var path = Path()
        for pts in strokes(text, in: m) where pts.count > 1 {
            path.move(to: CGPoint(x: origin.x + pts[0].x * scale, y: origin.y + pts[0].y * scale))
            for p in pts.dropFirst() { path.addLine(to: CGPoint(x: origin.x + p.x * scale, y: origin.y + p.y * scale)) }
        }
        return path
    }
}
