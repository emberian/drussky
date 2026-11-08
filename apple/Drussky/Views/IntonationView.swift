import SwiftUI

/// The melody of the model and of the learner on one plot, same time scale as the loudness
/// rows above it. Each line is relative to that voice's own middle pitch.
struct IntonationRow: View {
    let modelURL: URL?
    let mineURL: URL
    let version: Int
    @State private var model: Contour?
    @State private var mine: Contour?

    static let range: Float = 9  // semitones either side of the speaker's median

    var body: some View {
        VStack(spacing: 0) {
            if let mine {
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 10) {
                        Text("Pitch").font(Theme.label(11)).foregroundStyle(Theme.muted).frame(width: 44, alignment: .leading)
                        Canvas { ctx, size in draw(ctx, size: size, mine: mine) }
                            .frame(height: 64)
                            .accessibilityLabel(accessibility(mine))
                    }
                    if let line = hint(mine) {
                        Text(line).font(.system(size: 13)).foregroundStyle(Theme.muted)
                    }
                }
            }
        }
        .task(id: version) {
            if let modelURL { model = await Contour.compute(modelURL) }
            mine = await Contour.compute(mineURL)
        }
    }

    private func draw(_ ctx: GraphicsContext, size: CGSize, mine: Contour) {
        let longest = max(model?.semitones.count ?? 0, mine.semitones.count, 1)
        let w = size.width / CGFloat(longest)
        let mid = size.height / 2
        let scale = (size.height / 2 - 3) / CGFloat(Self.range)
        // Median line and ±1 octave-ish guides, faint like the paper's rules.
        var guide = Path()
        guide.move(to: CGPoint(x: 0, y: mid)); guide.addLine(to: CGPoint(x: size.width, y: mid))
        ctx.stroke(guide, with: .color(Theme.rule.opacity(0.35)), style: StrokeStyle(lineWidth: 1, dash: [3, 4]))
        func line(_ c: Contour, color: Color, width: CGFloat) {
            var path = Path()
            var open = false
            for (i, v) in c.semitones.enumerated() {
                guard let v else { open = false; continue }
                let clamped = max(-Self.range, min(Self.range, v))
                let p = CGPoint(x: (CGFloat(i) + 0.5) * w, y: mid - CGFloat(clamped) * scale)
                if open { path.addLine(to: p) } else { path.move(to: p); open = true }
            }
            ctx.stroke(path, with: .color(color), style: StrokeStyle(lineWidth: width, lineCap: .round, lineJoin: .round))
        }
        if let model { line(model, color: Theme.accent.opacity(0.75), width: 3) }
        line(mine, color: Theme.ink, width: 2)
    }

    /// What the plot shows, in measurements only. The pitch tracker finds the biggest climb but
    /// not which syllable carries it or what kind of question it is, so it doesn't say whether
    /// either voice "is a question" or tell the learner what to change. Listening decides.
    private func hint(_ mine: Contour) -> String? {
        guard let model else { return nil }
        return "Biggest climb: model \(Self.describe(model)); yours \(Self.describe(mine)). Play both and compare by ear."
    }

    static func describe(_ c: Contour) -> String {
        let r = c.rise
        guard r >= 1.5 else { return "about level" }
        return "about \(Int(r.rounded())) semitones, peaking \(c.risePeakPlace)"
    }

    private func accessibility(_ mine: Contour) -> String {
        (model.map { "Model pitch: biggest climb \(Self.describe($0)). " } ?? "") + "Your pitch: biggest climb \(Self.describe(mine))."
    }
}

extension Contour {
    /// Where the top of the biggest climb falls in the utterance: "early", "in the middle", "near the end".
    var risePeakPlace: String {
        var low = Float.infinity, best: Float = 0, peak = 0
        for (i, v) in semitones.enumerated() {
            guard let v else { continue }
            low = min(low, v)
            if v - low > best { best = v - low; peak = i }
        }
        let f = Float(peak) / Float(max(1, semitones.count - 1))
        return f < 0.34 ? "early" : f < 0.67 ? "in the middle" : "near the end"
    }
}
