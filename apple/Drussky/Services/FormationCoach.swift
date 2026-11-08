import CoreGraphics
import PencilKit

/// Compares a learner's handwritten letter with its hand-printed stroke model and finds the one
/// thing most worth saying about how it was formed. Advice only: it never decides completion.
enum FormationCoach {
    enum Issue: Equatable, Sendable {
        /// Began with model stroke `found` instead of the first one.
        case firstStroke(found: Int)
        /// Model stroke `stroke` was drawn the other way round.
        case reversed(stroke: Int)
        case descender, tooBig, tooSmall, floating, sinking, extraStrokes
    }

    struct Assessment: Sendable {
        var glyph: String
        var issues: [Issue]
        /// Close to the model stroke for stroke: worth a word of praise when nothing is off.
        var wellMatched: Bool
    }

    // MARK: Ink

    /// The drawing's strokes as polylines in strip design coordinates, in the order written.
    static func polylines(_ drawing: PKDrawing) -> [[CGPoint]] {
        drawing.strokes.compactMap { stroke in
            let pts = stroke.path.interpolatedPoints(by: .distance(2)).map { $0.location.applying(stroke.transform) }
            return pts.isEmpty ? nil : pts
        }
    }

    /// Groups strokes into letters by horizontal gaps. Strokes that overlap in x belong together;
    /// separate groups join when the gap is no wider than a letter's own (the halves of ы) and the
    /// result is no wider than a letter. Each group lists stroke indices in writing order.
    static func segment(_ strokes: [[CGPoint]], joinGap: CGFloat, maxWidth: CGFloat) -> [[Int]] {
        let spans = strokes.map { s in (s.map(\.x).min() ?? 0, s.map(\.x).max() ?? 0) }
        var groups: [(lo: CGFloat, hi: CGFloat, members: [Int])] = []
        for i in strokes.indices.sorted(by: { spans[$0].0 < spans[$1].0 }) {
            if let last = groups.last, spans[i].0 <= last.hi + 1.5 {
                groups[groups.count - 1].hi = max(last.hi, spans[i].1)
                groups[groups.count - 1].members.append(i)
            } else {
                groups.append((spans[i].0, spans[i].1, [i]))
            }
        }
        var merged: [(lo: CGFloat, hi: CGFloat, members: [Int])] = []
        for g in groups {
            if let last = merged.last, g.lo - last.hi <= joinGap, max(g.hi, last.hi) - last.lo <= maxWidth {
                merged[merged.count - 1].hi = max(last.hi, g.hi)
                merged[merged.count - 1].members += g.members
            } else {
                merged.append(g)
            }
        }
        return merged.map { $0.members.sorted() }
    }

    // MARK: Comparison

    /// Assesses one written letter against the candidate models (capital and small). Returns nil
    /// when the ink does not clearly look like any of them; then nothing should be said.
    /// `directionMatters` false (a left-handed learner): a stroke drawn the other way round is not an issue.
    static func assess(_ ink: [[CGPoint]], candidates: [StrokeModels.Letter], lines: StrokeModels.Lines, directionMatters: Bool = true) -> Assessment? {
        let ink = ink.filter { !$0.isEmpty }
        guard !ink.isEmpty, let box = bounds(ink.flatMap { $0 }), max(box.width, box.height) > 6 else { return nil }
        let inkCloud = cloud(ink)
        var best: (letter: StrokeModels.Letter, cost: CGFloat, shape: CGFloat)?
        for model in candidates {
            let shape = cloudDistance(inkCloud, cloud(model.strokes.map(\.points)))
            let size = abs(log(max(box.height, 1) / max(model.bounds.height, 1))) * 0.12
            if best == nil || shape + size < best!.cost { best = (model, shape + size, shape) }
        }
        guard let (model, _, shape) = best, shape < 0.1 else { return nil }
        let mb = model.bounds
        guard box.width < mb.width * 2.2 + 20 else { return nil }

        var issues: [Issue] = []
        // Strokes: map the ink into the model's frame and pair strokes up.
        // Each axis on its own (handwriting is often wider or narrower than the model), but a
        // thin letter is not stretched sideways.
        let (bw, bh) = span(box), (mw, mh) = span(mb)
        let frame = { (p: CGPoint) in CGPoint(x: (p.x - box.midX) * mw / bw + mb.midX, y: (p.y - box.midY) * mh / bh + mb.midY) }
        let mine = ink.map { resample($0.map(frame), 16) }
        let theirs = model.strokes.map { resample($0.points, 16) }
        let norm = max(mb.width, mb.height)
        let costs = mine.map { l in theirs.enumerated().map { j, m in strokeCost(l, m, free: model.strokes[j].free, norm: norm) } }
        let pairing = pair(costs, modelCount: theirs.count)
        let meanCost = zip(mine.indices, pairing).map { costs[$0][$1].cost }.reduce(0, +) / CGFloat(mine.count)
        guard meanCost < 0.3 else { return nil }  // strokes that pair with nothing: not this letter
        if meanCost < 0.22 {
            if !model.anyOrder, let first = pairing.first, first != 0,
               costs[0][0].cost > costs[0][first].cost + 0.04 {
                issues.append(.firstStroke(found: first))
            }
            if directionMatters, let i = mine.indices.first(where: { costs[$0][pairing[$0]].reversed && costs[$0][pairing[$0]].cost < 0.2 }) {
                issues.append(.reversed(stroke: pairing[i]))
            }
            if ink.count >= model.strokes.count + 2 { issues.append(.extraStrokes) }
        }
        // Size and place against the lines.
        let capHeight = lines.baseline - lines.cap
        let ratio = box.height / max(mb.height, 1)
        let descends = mb.maxY > lines.baseline + 8
        if descends && box.maxY < lines.baseline + 4 && ratio < 1.4 { issues.append(.descender) }
        if ratio > 1.45 { issues.append(.tooBig) } else if ratio < 0.55 { issues.append(.tooSmall) }
        if !descends {
            if box.maxY < lines.baseline - capHeight * 0.18 { issues.append(.floating) }
            else if box.maxY > lines.baseline + capHeight * 0.18 { issues.append(.sinking) }
        }
        return Assessment(glyph: model.glyph, issues: issues.sorted { rank($0) < rank($1) }, wellMatched: shape < 0.06 && meanCost < 0.12)
    }

    private static func rank(_ i: Issue) -> Int {
        switch i {
        case .firstStroke: 0
        case .reversed: 1
        case .descender: 2
        case .tooBig: 3
        case .tooSmall: 4
        case .floating: 5
        case .sinking: 6
        case .extraStrokes: 7
        }
    }

    /// One hint for the letter written most recently in `ink`, or nil when there is nothing
    /// confident to say. `letter` is the capital; both cases are candidates.
    static func hint(for ink: [[CGPoint]], upper: String, lower: String, models: StrokeModels, directionMatters: Bool = true) -> String? {
        let candidates = [upper, lower].compactMap { models.letter($0) }
        guard !candidates.isEmpty, !ink.isEmpty else { return nil }
        let joinGap = max(4, candidates.map(\.internalGap).max()! + 5)
        let maxWidth = candidates.map(\.bounds.width).max()! * 1.4 + 6
        let groups = segment(ink, joinGap: joinGap, maxWidth: maxWidth)
        guard let latest = groups.max(by: { $0.max()! < $1.max()! }),
              let a = assess(latest.map { ink[$0] }, candidates: candidates, lines: models.lines, directionMatters: directionMatters),
              let model = models.letter(a.glyph) else { return nil }
        guard let issue = a.issues.first else { return a.wellMatched ? "That \(a.glyph) is formed like the model." : nil }
        return phrase(issue, model: model)
    }

    // MARK: Words

    static func phrase(_ issue: Issue, model: StrokeModels.Letter) -> String {
        let g = model.glyph
        let small = g == g.lowercased()
        let strokes = model.strokes
        switch issue {
        case .firstStroke:
            let s0 = strokes[0]
            let then = strokes.count > 1 ? ", then the \(strokes[1].name)" : ""
            return "Start \(g) at the \(place(s0.points[0], in: model.bounds)) with the \(s0.name)\(then)."
        case .reversed(let k):
            let s = strokes[k]
            let subject = strokes.count == 1 ? g : "the \(s.name) of \(g)"
            let a = s.points.first!, b = s.points.last!
            if isLoop(s.points) {
                let way = signedArea(s.points) < 0 ? "anticlockwise" : "clockwise"
                return "Go round \(subject) \(way), starting at the \(place(a, in: model.bounds))."
            }
            let dx = b.x - a.x, dy = b.y - a.y
            let straight = pathLength(s.points) < dist(a, b) * 1.08
            if straight, abs(dy) > abs(dx) * 3 { return "Draw \(subject) from the \(dy > 0 ? "top down" : "bottom up")." }
            if straight, abs(dx) > abs(dy) * 3 { return "Draw \(subject) from \(dx > 0 ? "left to right" : "right to left")." }
            return "Draw \(subject) starting at the \(place(a, in: model.bounds))."
        case .descender:
            switch g.lowercased() {
            case "д": return "The feet of \(g) go just below the baseline."
            case "ц", "щ": return "The tail of \(g) goes just below the baseline."
            case "у": return "The tail of у goes down below the baseline."
            default: return "The stem of \(g) goes down below the baseline."
            }
        case .tooBig:
            return small ? "Make \(g) smaller: a small letter is about two-thirds the height of a capital."
                         : "Write \(g) a little smaller: from the top line down to the baseline."
        case .tooSmall:
            return small ? "Let \(g) grow a little, like the model beside it."
                         : "Capital \(g) reaches all the way up to the top line."
        case .floating:
            return "Sit \(g) on the baseline."
        case .sinking:
            return "\(g) rests on the baseline; it doesn't hang below it."
        case .extraStrokes:
            let n = strokes.count
            return n == 1 ? "Try \(g) in one smooth stroke." : "\(g) takes \(n) strokes; try it with fewer pen lifts."
        }
    }

    /// "top-left", "middle", "bottom-right"… for a point within a letter.
    static func place(_ p: CGPoint, in b: CGRect) -> String {
        let fx = (p.x - b.minX) / max(b.width, 1), fy = (p.y - b.minY) / max(b.height, 1)
        let v = fy < 0.3 ? "top" : fy > 0.7 ? "bottom" : "middle"
        let h: String? = fx < 0.3 ? "left" : fx > 0.7 ? "right" : nil
        guard let h else { return v }
        return v == "middle" ? "\(h) side" : "\(v)-\(h)"
    }

    // MARK: Geometry

    struct StrokeMatch: Sendable { var cost: CGFloat; var reversed: Bool }

    static func strokeCost(_ l: [CGPoint], _ m: [CGPoint], free: Bool, norm: CGFloat) -> StrokeMatch {
        if isLoop(m) {
            // Closed rounds: any starting point; direction from the winding.
            var best = CGFloat.infinity
            for candidate in [l, Array(l.reversed())] {
                for k in 0..<candidate.count {
                    let shifted = Array(candidate[k...] + candidate[..<k])
                    best = min(best, meanDistance(shifted, m))
                }
            }
            let closed = dist(l.first!, l.last!) < pathLength(l) * 0.3
            let reversed = !free && closed && (signedArea(l) < 0) != (signedArea(m) < 0)
            return StrokeMatch(cost: best / norm, reversed: reversed)
        }
        let f = meanDistance(l, m), b = meanDistance(Array(l.reversed()), m)
        let reversed = !free && b < f * 0.6 && pathLength(m) > norm * 0.15
        return StrokeMatch(cost: min(f, b) / norm, reversed: reversed)
    }

    /// Learner stroke i → model stroke. Same count: the best one-to-one pairing; otherwise each
    /// learner stroke takes its nearest model stroke.
    static func pair(_ costs: [[StrokeMatch]], modelCount: Int) -> [Int] {
        let n = costs.count
        guard n == modelCount, n <= 6 else { return costs.map { row in row.indices.min { row[$0].cost < row[$1].cost }! } }
        var best: (sum: CGFloat, perm: [Int]) = (.infinity, Array(0..<n))
        func search(_ i: Int, _ used: Set<Int>, _ perm: [Int], _ sum: CGFloat) {
            if sum >= best.sum { return }
            if i == n { best = (sum, perm); return }
            for j in 0..<n where !used.contains(j) { search(i + 1, used.union([j]), perm + [j], sum + costs[i][j].cost) }
        }
        search(0, [], [], 0)
        return best.perm
    }

    static func isLoop(_ pts: [CGPoint]) -> Bool {
        guard let a = pts.first, let b = pts.last else { return false }
        return dist(a, b) < pathLength(pts) * 0.12
    }

    /// Shoelace area in screen coordinates: negative winds anticlockwise on screen.
    static func signedArea(_ pts: [CGPoint]) -> CGFloat {
        zip(pts, pts.dropFirst() + [pts[0]]).reduce(0) { $0 + ($1.0.x * $1.1.y - $1.1.x * $1.0.y) } / 2
    }

    static func resample(_ pts: [CGPoint], _ n: Int) -> [CGPoint] {
        guard pts.count > 1 else { return Array(repeating: pts.first ?? .zero, count: n) }
        let total = pathLength(pts)
        guard total > 0.01 else { return Array(repeating: pts[0], count: n) }
        let step = total / CGFloat(n - 1)
        var out = [pts[0]]
        var acc: CGFloat = 0
        var prev = pts[0]
        var i = 1
        while i < pts.count && out.count < n {
            let d = dist(prev, pts[i])
            if acc + d >= step && d > 0 {
                let t = (step - acc) / d
                let q = CGPoint(x: prev.x + t * (pts[i].x - prev.x), y: prev.y + t * (pts[i].y - prev.y))
                out.append(q)
                prev = q
                acc = 0
            } else {
                acc += d
                prev = pts[i]
                i += 1
            }
        }
        while out.count < n { out.append(pts.last!) }
        return out
    }

    /// All points, resampled per stroke by length, centred and scaled to unit size.
    static func cloud(_ strokes: [[CGPoint]]) -> [CGPoint] {
        let lengths = strokes.map(pathLength)
        let total = max(lengths.reduce(0, +), 0.01)
        var pts: [CGPoint] = []
        for (s, len) in zip(strokes, lengths) { pts += resample(s, max(2, Int((len / total * 48).rounded()))) }
        guard let b = bounds(pts) else { return [] }
        let (w, h) = span(b)
        return pts.map { CGPoint(x: ($0.x - b.midX) / w, y: ($0.y - b.midY) / h) }
    }

    /// Symmetric greedy nearest-point distance between two clouds (after the $P recognizer).
    static func cloudDistance(_ a: [CGPoint], _ b: [CGPoint]) -> CGFloat {
        func oneWay(_ a: [CGPoint], _ b: [CGPoint]) -> CGFloat {
            a.reduce(0) { sum, p in sum + (b.map { dist(p, $0) }.min() ?? 1) } / CGFloat(max(a.count, 1))
        }
        return (oneWay(a, b) + oneWay(b, a)) / 2
    }

    /// Width and height for normalising, neither below 30% of the other.
    static func span(_ b: CGRect) -> (CGFloat, CGFloat) {
        let size = max(b.width, b.height, 1)
        return (max(b.width, size * 0.3), max(b.height, size * 0.3))
    }

    static func meanDistance(_ a: [CGPoint], _ b: [CGPoint]) -> CGFloat {
        zip(a, b).reduce(0) { $0 + dist($1.0, $1.1) } / CGFloat(max(min(a.count, b.count), 1))
    }

    static func pathLength(_ pts: [CGPoint]) -> CGFloat { zip(pts, pts.dropFirst()).reduce(0) { $0 + dist($1.0, $1.1) } }
    static func dist(_ a: CGPoint, _ b: CGPoint) -> CGFloat { hypot(a.x - b.x, a.y - b.y) }

    static func bounds(_ pts: [CGPoint]) -> CGRect? {
        guard let x0 = pts.map(\.x).min(), let x1 = pts.map(\.x).max(), let y0 = pts.map(\.y).min(), let y1 = pts.map(\.y).max() else { return nil }
        return CGRect(x: x0, y: y0, width: x1 - x0, height: y1 - y0)
    }
}
