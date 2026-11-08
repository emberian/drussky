import CoreGraphics
import Foundation

/// Formation hints against the cursive models: the comparer is `FormationCoach`'s, the words
/// are cursive's (hooks, humps, loops, one stroke without lifting the pen).
enum CursiveCoach {
    /// One hint for the letter written most recently in `ink`, or nil when there is nothing
    /// confident to say. `upper` is the capital; both cases are candidates.
    static func hint(for ink: [[CGPoint]], upper: String, lower: String, models: CursiveModels, directionMatters: Bool = true) -> String? {
        let candidates = [upper, lower].compactMap { models.letter($0) }
        guard !candidates.isEmpty, !ink.isEmpty else { return nil }
        let shapes = candidates.map(\.model)
        let joinGap = max(4, shapes.map(\.internalGap).max()! + 5)
        let maxWidth = shapes.map(\.bounds.width).max()! * 1.4 + 6
        let groups = FormationCoach.segment(ink, joinGap: joinGap, maxWidth: maxWidth)
        guard let latest = groups.max(by: { $0.max()! < $1.max()! }) else { return nil }
        let strokes = latest.map { ink[$0] }
        if let a = FormationCoach.assess(strokes, candidates: shapes, lines: models.lines, directionMatters: directionMatters), let letter = models.letter(a.glyph) {
            if let issue = a.issues.first { return phrase(issue, letter: letter) }
            if startsWithoutHook(strokes, letter: letter) { return letter.hint }
            return a.wellMatched ? "That \(a.glyph) is written like the model." : nil
        }
        // Not the model stroke for stroke. The two cursive slips a point-by-point comparison
        // cannot see: a letter that should start with a hook written without it (compare with
        // the model minus its hook), and one stroke written in several pen lifts (compare the
        // lifts joined up with the model).
        let hookless = candidates.filter { $0.entryKind == "base" }.map { l -> StrokeModels.Letter in
            var m = l.model
            let waist = (models.lines.xHeight + models.lines.baseline) / 2
            m.strokes[0].points = Array(m.strokes[0].points.drop { $0.y > waist })
            return m
        }
        if !hookless.isEmpty, let a = FormationCoach.assess(strokes, candidates: hookless, lines: models.lines, directionMatters: directionMatters),
           let letter = models.letter(a.glyph), startsWithoutHook(strokes, letter: letter) {
            return letter.hint
        }
        if strokes.count > 1 {
            let joined = [strokes.flatMap { $0 }]
            if let a = FormationCoach.assess(joined, candidates: shapes, lines: models.lines, directionMatters: directionMatters), let letter = models.letter(a.glyph),
               strokes.count > letter.model.strokes.count {
                return phrase(.extraStrokes, letter: letter)
            }
        }
        return nil
    }

    /// л, м, я (and their capitals) begin with a hook on the baseline (the bottom fifth of the
    /// letter); ink that starts anywhere in the upper three fifths skipped it.
    static func startsWithoutHook(_ strokes: [[CGPoint]], letter: CursiveModels.Letter) -> Bool {
        guard letter.entryKind == "base", let first = strokes.first?.first,
              let box = FormationCoach.bounds(strokes.flatMap { $0 }), box.height > 6 else { return false }
        return (first.y - box.minY) / box.height < 0.6
    }

    static func phrase(_ issue: FormationCoach.Issue, letter: CursiveModels.Letter) -> String {
        let g = letter.glyph
        switch issue {
        case .firstStroke, .reversed:
            return letter.hint
        case .extraStrokes:
            let n = letter.model.strokes.count
            return n == 1 ? "Cursive \(g) is one stroke, without lifting the pen: \(lowered(letter.hint))"
                          : "\(g) takes \(n) strokes here, not more: \(lowered(letter.hint))"
        case .descender:
            switch g.lowercased() {
            case "д", "у", "з": return "The loop of \(g) drops below the line."
            case "ц", "щ": return "The tail of \(g) dips just below the line."
            default: return "The stem of \(g) goes down below the line."
            }
        default:
            return FormationCoach.phrase(issue, model: letter.model)
        }
    }

    /// "м starts with…" → "м starts with…" (a hint already beginning with the glyph keeps its
    /// case; one beginning with a capitalised word is lowered to follow a colon).
    private static func lowered(_ hint: String) -> String {
        guard let first = hint.first, first.isUppercase, hint.count > 1,
              !"АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ".contains(first) else { return hint }
        return first.lowercased() + hint.dropFirst()
    }
}
