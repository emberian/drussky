import CoreText
import SwiftUI
import UIKit

/// Plays a letter's stroke model over the ghost on its writing strip: numbered starts, the
/// line drawn at writing speed, an arrowhead showing which way it goes.
@MainActor @Observable
final class StrokeDemo {
    private(set) var startedAt: Date?
    func play() { startedAt = .now }
    func stop() { startedAt = nil }
}

/// What a writing strip needs to coach one letter: its models, the demo, and where they sit.
struct LetterCoach {
    var letter: Course.Letter
    var demo: StrokeDemo
    /// Cursive models and their joined-up layout instead of the print ones (Cursive/).
    var cursive = false

    /// The models laid out as the meet ghost draws them ("Б б"), in strip design coordinates.
    @MainActor
    func placed(ghost: String) -> [(StrokeModels.Letter, CGFloat)] {
        if cursive, let m = CursiveModels.shared { return CursiveLayout.placedLetters(ghost, in: m, x0: 16) }
        guard let models = StrokeModels.shared else { return [] }
        let font = StripMetrics(height: StripMetrics.designHeight).font(cursive: false)
        let line = CTLineCreateWithAttributedString(NSAttributedString(string: ghost, attributes: [.font: font]))
        var out: [(StrokeModels.Letter, CGFloat)] = []
        let ns = ghost as NSString
        for i in 0..<ns.length {
            guard let model = models.letter(ns.substring(with: NSRange(location: i, length: 1))) else { continue }
            out.append((model, 16 + CTLineGetOffsetForStringIndex(line, i, nil)))
        }
        return out
    }
}

struct StrokeDemoOverlay: View {
    let demo: StrokeDemo
    let placed: [(StrokeModels.Letter, CGFloat)]
    let height: CGFloat

    /// Design points per second: calm, about the pace of careful handwriting.
    static let speed: CGFloat = 55
    static let pause: Double = 0.45
    static let linger: Double = 2.5

    private struct Segment { var points: [CGPoint]; var number: Int; var start: Double; var duration: Double }

    private var segments: [Segment] {
        var t = 0.3
        var out: [Segment] = []
        for (model, x) in placed {
            for (i, s) in model.strokes.enumerated() {
                let d = max(0.25, Double(s.length / Self.speed))
                out.append(Segment(points: s.points.map { CGPoint(x: $0.x + x, y: $0.y) }, number: i + 1, start: t, duration: d))
                t += d + Self.pause
            }
            t += 0.4
        }
        return out
    }

    var body: some View {
        if let started = demo.startedAt {
            let segs = segments
            let total = (segs.last.map { $0.start + $0.duration } ?? 0) + Self.linger
            TimelineView(.animation) { timeline in
                let t = timeline.date.timeIntervalSince(started)
                Canvas { ctx, _ in
                    let k = height / StripMetrics.designHeight
                    let fade = t > total ? max(0, 1 - (t - total) / 0.6) : 1
                    ctx.opacity = fade
                    for seg in segs where t >= seg.start {
                        let f = min(1, (t - seg.start) / seg.duration)
                        draw(seg, fraction: f, scale: k, in: &ctx)
                    }
                }
            }
            .allowsHitTesting(false)
            .accessibilityHidden(true)
            .task(id: started) {
                try? await Task.sleep(for: .seconds(total + 0.7))
                if demo.startedAt == started { demo.stop() }
            }
        }
    }

    private func draw(_ seg: Segment, fraction: Double, scale k: CGFloat, in ctx: inout GraphicsContext) {
        let pts = seg.points.map { CGPoint(x: $0.x * k, y: $0.y * k) }
        let shown = prefix(pts, fraction: fraction)
        var path = Path()
        path.addLines(shown)
        ctx.stroke(path, with: .color(Theme.redPen.opacity(0.85)), style: StrokeStyle(lineWidth: 3.4 * k, lineCap: .round, lineJoin: .round))
        // Arrowhead at the moving tip.
        if shown.count >= 2 {
            let tip = shown[shown.count - 1]
            let back = shown[max(0, shown.count - 4)]
            let a = atan2(tip.y - back.y, tip.x - back.x)
            let len = 7 * k
            var head = Path()
            head.move(to: CGPoint(x: tip.x - len * cos(a - 0.5), y: tip.y - len * sin(a - 0.5)))
            head.addLine(to: tip)
            head.addLine(to: CGPoint(x: tip.x - len * cos(a + 0.5), y: tip.y - len * sin(a + 0.5)))
            ctx.stroke(head, with: .color(Theme.redPen), style: StrokeStyle(lineWidth: 2.4 * k, lineCap: .round, lineJoin: .round))
        }
        // Numbered start, set just off the line's beginning so it doesn't hide the stroke.
        let r = 6.5 * k
        var start = pts[0]
        if let earlier = earlierStart(seg, scale: k), FormationCoach.dist(earlier, start) < 2 * r {
            start.x += 2.2 * r  // two strokes from one point: set the badges side by side
        }
        let badge = CGRect(x: start.x - r, y: start.y - r, width: 2 * r, height: 2 * r)
        ctx.fill(Path(ellipseIn: badge), with: .color(Theme.redPen))
        ctx.draw(Text("\(seg.number)").font(.system(size: 9 * k, weight: .bold, design: .rounded)).foregroundStyle(.white),
                 at: start)
    }

    private func earlierStart(_ seg: Segment, scale k: CGFloat) -> CGPoint? {
        segments.last { $0.start < seg.start && $0.number < seg.number }.map { CGPoint(x: $0.points[0].x * k, y: $0.points[0].y * k) }
    }

    private func prefix(_ pts: [CGPoint], fraction: Double) -> [CGPoint] {
        guard fraction < 1, pts.count > 1 else { return pts }
        let total = FormationCoach.pathLength(pts)
        var remaining = total * fraction
        var out = [pts[0]]
        for (a, b) in zip(pts, pts.dropFirst()) {
            let d = FormationCoach.dist(a, b)
            if remaining >= d { out.append(b); remaining -= d; continue }
            let t = d > 0 ? remaining / d : 0
            out.append(CGPoint(x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t))
            break
        }
        return out
    }
}

/// The one formation hint, under the writing line. Recomputed a moment after the pen stops.
struct FormationHintLine: View {
    @Environment(Book.self) private var book
    @ObservedObject var pad: PadController
    let letter: Course.Letter
    /// Compare with the cursive models, in cursive's words (Cursive/CursiveCoach.swift).
    var cursive = false
    /// How far to rise into the strip's empty bottom margin, and room to leave on the right.
    var lift: CGFloat = 0
    var clearTrailing: CGFloat = 0
    @State private var hint: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {  // a real container, so .task runs with no hint showing
            if let hint {
                let good = hint.hasSuffix("like the model.")
                Label(hint, systemImage: good ? "checkmark.circle" : "pencil.tip")
                    .font(.system(size: 15, weight: .medium))
                    .foregroundStyle(good ? Theme.good : Theme.accent)
                    .transition(.opacity)
                    .accessibilityIdentifier("formation-hint")
                    .padding(.top, -lift)
                    .padding(.trailing, clearTrailing)
            }
        }
        .animation(.easeOut(duration: 0.25), value: hint)
        .task(id: pad.strokeCount) {
            if pad.strokeCount == 0 { hint = nil; return }
            try? await Task.sleep(for: .milliseconds(900))
            guard !Task.isCancelled else { return }
            let ink = FormationCoach.polylines(pad.drawing)
            if cursive, let models = CursiveModels.shared {
                hint = CursiveCoach.hint(for: ink, upper: letter.upper, lower: letter.lower, models: models, directionMatters: !(book.settings.leftHanded ?? false))
            } else if let models = StrokeModels.shared {
                hint = FormationCoach.hint(for: ink, upper: letter.upper, lower: letter.lower, models: models, directionMatters: !(book.settings.leftHanded ?? false))
            }
        }
    }
}

/// "Watch it printed" / "Watch it written": plays (or replays) the stroke demo on the letter's
/// line, from the print models or the cursive ones.
struct WatchStrokesButton: View {
    let demo: StrokeDemo
    var cursive = false

    var body: some View {
        let playing = demo.startedAt != nil
        Button { demo.play() } label: {
            Label(playing ? "Again" : cursive ? "Watch it written" : "Watch it printed", systemImage: playing ? "arrow.counterclockwise" : "pencil.and.scribble")
                .font(Theme.label(14))
                .padding(.horizontal, 12).padding(.vertical, 8)
                .foregroundStyle(Theme.redPen)
                .background(Capsule().fill(Theme.redPen.opacity(0.09)))
        }
        .buttonStyle(.plain)
        .fixedSize()
        .accessibilityIdentifier("watch-strokes")
        .accessibilityHint(cursive ? "Shows how the letter is written in cursive, stroke by stroke" : "Shows how the printed letter is formed, stroke by stroke")
    }
}
