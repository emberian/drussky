import LearningCore
import SwiftUI

/// A small line over time: no axes, no numbers but the last one. Honest about its range (it
/// says the span in the accessibility label) and about having little data (two points draw a
/// line, one draws a dot).
struct Sparkline: View {
    let samples: [MetricSample]
    var color: Color = Theme.accent
    var format: (Double) -> String = { Fluency.round($0) }

    var body: some View {
        HStack(alignment: .center, spacing: 8) {
            Canvas { ctx, size in
                let values = samples.map(\.value)
                guard let lo = values.min(), let hi = values.max(), let first = samples.first?.t, let last = samples.last?.t else { return }
                let span = max(last.timeIntervalSince(first), 1)
                let range = max(hi - lo, max(abs(hi), 1) * 0.1)
                func point(_ s: MetricSample) -> CGPoint {
                    CGPoint(x: 3 + (size.width - 6) * s.t.timeIntervalSince(first) / span,
                            y: size.height - 3 - (size.height - 6) * (s.value - lo + range * 0.5 - (hi - lo) * 0.5) / range)
                }
                var path = Path()
                for (i, s) in samples.enumerated() {
                    if i == 0 { path.move(to: point(s)) } else { path.addLine(to: point(s)) }
                }
                ctx.stroke(path, with: .color(color.opacity(0.8)), style: StrokeStyle(lineWidth: 1.5, lineCap: .round, lineJoin: .round))
                if let s = samples.last {
                    let p = point(s)
                    ctx.fill(Path(ellipseIn: CGRect(x: p.x - 3, y: p.y - 3, width: 6, height: 6)), with: .color(color))
                }
            }
            .frame(width: 96, height: 26)
            if let last = samples.last {
                Text(format(last.value)).scaledFont(13, weight: .semibold, design: .rounded).monospacedDigit().foregroundStyle(Theme.text)
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(label)
    }

    private var label: String {
        guard let f = samples.first, let l = samples.last else { return "no measurements" }
        return "\(samples.count) measurements, from \(format(f.value)) on \(f.t.formatted(.dateTime.month().day())) to \(format(l.value)) on \(l.t.formatted(.dateTime.month().day()))"
    }
}

/// One labelled sparkline row.
struct SparkRow: View {
    let title: String
    let unit: String
    let samples: [MetricSample]
    var color: Color = Theme.accent
    var format: (Double) -> String = { Fluency.round($0) }

    var body: some View {
        HStack(spacing: 12) {
            Text(title).scaledFont(14, weight: .medium).foregroundStyle(Theme.text).frame(minWidth: 130, alignment: .leading)
            Sparkline(samples: samples, color: color, format: format)
            Text(unit).scaledFont(12).foregroundStyle(Theme.muted)
            Spacer(minLength: 0)
        }
    }
}

/// On a component's history: its fluency series, when it has any.
struct FluencySection: View {
    @Environment(Book.self) private var book
    let id: ComponentID

    var body: some View {
        let store = book.fluency.store
        let sps = store.samples(Fluency.strokesPerSecond, of: id)
        let legible = store.samples(Fluency.legibility, of: id)
        let say = store.samples(Fluency.sayWPM, of: id)
        if sps.count >= 2 || legible.count >= 2 || say.count >= 2 {
            VStack(alignment: .leading, spacing: 10) {
                Text("Fluency").font(.system(size: 17, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
                if sps.count >= 2 { SparkRow(title: "Writing speed", unit: "strokes/s", samples: sps) }
                if legible.count >= 2 { SparkRow(title: "Legibility", unit: "to the reader", samples: legible, color: GrowthChart.colors[2], format: Fluency.percent) }
                if say.count >= 2 { SparkRow(title: "Saying it", unit: "words/min", samples: say, color: GrowthChart.colors[1]) }
            }
            .accessibilityIdentifier("fluency-section")
        }
    }
}

/// Inside the cover: one line about this week, and the three small instruments under it.
struct FluencyNotes: View {
    @Environment(Book.self) private var book

    var body: some View {
        let store = book.fluency.store
        let now = book.now()
        let reading = store.samples(Fluency.readWPM, of: Fluency.reading)
        let writing = store.samples(Fluency.strokesPerSecond, of: Fluency.writing)
        let full = store.mean(Fluency.listening, of: Fluency.listeningSkill, window: 2 * Fluency.week, at: now, note: "1")
        let slow = store.mean(Fluency.listening, of: Fluency.listeningSkill, window: 2 * Fluency.week, at: now, note: "0.75")
        if reading.count >= 2 || writing.count >= 2 || full != nil || slow != nil {
            VStack(alignment: .leading, spacing: 10) {
                if let line = Fluency.coverLine(store, now: now, label: { book.knowledge.label($0) }) {
                    Text(line).scaledFont(16).foregroundStyle(Theme.text).fixedSize(horizontal: false, vertical: true)
                        .accessibilityIdentifier("fluency-line")
                }
                VStack(alignment: .leading, spacing: 8) {
                    if reading.count >= 2 { SparkRow(title: "Reading", unit: "words/min", samples: Array(reading.suffix(30))) }
                    if writing.count >= 2 { SparkRow(title: "Writing", unit: "strokes/s", samples: Array(writing.suffix(30)), color: GrowthChart.colors[1]) }
                    if full != nil || slow != nil {
                        HStack(spacing: 12) {
                            Text("Listening").scaledFont(14, weight: .medium).foregroundStyle(Theme.text).frame(minWidth: 130, alignment: .leading)
                            Text([full.map { "1× \(Fluency.percent($0.value))" }, slow.map { "¾× \(Fluency.percent($0.value))" }].compactMap { $0 }.joined(separator: " · "))
                                .scaledFont(13, weight: .semibold, design: .rounded).monospacedDigit().foregroundStyle(Theme.text)
                            Text("of the reader's sentences").scaledFont(12).foregroundStyle(Theme.muted)
                        }
                    }
                }
            }
        }
    }
}

// MARK: - Reading a passage with a finger

/// A timed pass over a passage: the learner follows the lines with a finger (reading silently or
/// aloud) and the time from the first touch to the last lift gives words a minute. Nothing ticks
/// on the screen while they read.
@MainActor
@Observable
final class ReadingPass {
    private(set) var active = false
    private(set) var startedAt: Date?
    private(set) var lastWPM: Double?
    private(set) var previousWPM: Double?

    func begin() { active = true; startedAt = nil }
    func cancel() { active = false; startedAt = nil }

    func touched(at t: Date) { if startedAt == nil { startedAt = t } }

    /// The finger lifted at `point` in a block of `size`; the pass ends when that is in the last
    /// quarter of the text (lifting between lines keeps the clock running).
    func lifted(at point: CGPoint, in size: CGSize, text: String, book: Book) {
        guard active, let start = startedAt, point.y >= size.height * 0.75 else { return }
        finish(text: text, seconds: book.now().timeIntervalSince(start), book: book)
    }

    func finish(text: String, seconds: Double, book: Book) {
        previousWPM = book.fluency.lastReads(of: text).last
        lastWPM = book.fluency.noteReadingPass(text: text, seconds: seconds)
        active = false
        startedAt = nil
    }
}

/// Sits over the passage text while a pass is on: takes the finger, nothing else.
struct ReadingPassOverlay: View {
    @Environment(Book.self) private var book
    let pass: ReadingPass
    let text: String

    var body: some View {
        if pass.active {
            GeometryReader { g in
                Color.clear.contentShape(Rectangle())
                    .gesture(DragGesture(minimumDistance: 0, coordinateSpace: .local)
                        .onChanged { _ in pass.touched(at: book.now()) }
                        .onEnded { v in pass.lifted(at: v.location, in: g.size, text: text, book: book) })
            }
            .accessibilityLabel("Reading pass: follow the lines with a finger; lift it at the end")
        }
    }
}

/// Under a passage: start a pass ("Read it again, a little faster" once there has been one), and
/// afterwards, calmly, the pace.
struct ReadingPassBar: View {
    @Environment(Book.self) private var book
    let pass: ReadingPass
    let text: String

    var body: some View {
        let reads = book.fluency.lastReads(of: text)
        HStack(spacing: 12) {
            if pass.active {
                Text(pass.startedAt == nil ? "Touch the first word and follow the lines as you read; lift at the end." : "Reading…")
                    .scaledFont(14).foregroundStyle(Theme.muted)
                PillButton(title: "Stop", systemImage: "xmark") { pass.cancel() }
            } else {
                PillButton(title: reads.last == nil ? "Read it with a finger" : "Read it again, a little faster", systemImage: "hand.point.up.left") { pass.begin() }
                    .accessibilityIdentifier("reading-pass")
                if let wpm = pass.lastWPM ?? reads.last {
                    Text(paceLine(wpm, previous: pass.lastWPM != nil ? pass.previousWPM : reads.previous))
                        .scaledFont(14).foregroundStyle(Theme.muted)
                }
            }
        }
    }

    private func paceLine(_ wpm: Double, previous: Double?) -> String {
        var s = "About \(Fluency.round(wpm)) words a minute"
        if let p = previous, p > 0 {
            let rel = (wpm - p) / p
            s += abs(rel) < 0.05 ? ", as before." : rel > 0 ? ", \(Fluency.percent(rel)) faster than last time." : ", a little slower than last time."
        } else { s += "." }
        return s
    }
}
