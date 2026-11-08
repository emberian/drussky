import LearningCore
import SwiftUI
import UIKit

/// One rendering of a word or line in somebody's hand: a handwriting font or the school models
/// written through with a little jitter, slanted, sized and inked differently each time, on
/// ruled, squared or plain paper. Everything is decided by the seed, so a page looks the same
/// every time it is opened and a test can pin it down.
struct HandwritingSample: Equatable, Sendable {
    enum Style: Equatable, Sendable {
        case font(String)
        /// The stroke models, joined up.
        case strokes
    }
    enum Ink: Int, Sendable { case blue, black, pencil }
    enum Paper: Int, Sendable { case ruled, squared, plain }

    var text: String
    var style: Style
    /// Extra shear to the right (negative leans back).
    var slant: CGFloat
    /// Size relative to the nominal.
    var size: CGFloat
    /// Jitter phase and amplitude (design points) for the stroke models.
    var wobble: Double
    var jitter: CGFloat
    var ink: Ink
    var paper: Paper
    /// Baseline tilt in degrees.
    var tilt: CGFloat

    /// A seed for an exercise that has none: stable across launches (unlike `hashValue`).
    static func seed(for text: String, index: Int) -> UInt64 {
        SeededGenerator.seed("\(RussianText.key(text))#\(index)")
    }

    static func make(text: String, seed: UInt64, fonts: [String] = CursiveFonts.names, strokes: Bool = CursiveModels.shared != nil) -> HandwritingSample {
        var rng = SeededGenerator(seed: seed)
        func unit() -> CGFloat { CGFloat(rng.next() % 10_000) / 10_000 }
        // The school hand is one voice among the fonts, a little more often than any one of them.
        var styles: [Style] = fonts.map { .font($0) }
        if strokes { styles += [.strokes, .strokes] }
        if styles.isEmpty { styles = [.font("MarckScript-Regular")] }
        let style = styles[Int(rng.next() % UInt64(styles.count))]
        return HandwritingSample(
            text: RussianText.unstressed(text), style: style,
            slant: -0.08 + unit() * 0.4, size: 0.82 + unit() * 0.33,
            wobble: Double(unit()) * 6.28, jitter: 0.3 + unit() * 0.9,
            ink: Ink(rawValue: Int(rng.next() % 3))!, paper: Paper(rawValue: Int(rng.next() % 3))!,
            tilt: -2 + unit() * 4)
    }

    var inkColor: Color {
        switch ink {
        case .blue: Color(UIColor(hex: 0x1F3A8A))
        case .black: Color(UIColor(hex: 0x1C1C22))
        case .pencil: Color(UIColor(hex: 0x4A4F5A))
        }
    }
}

/// A scrap of paper with the sample written on it.
struct HandwritingSampleView: View {
    let sample: HandwritingSample
    var height: CGFloat = 112

    var body: some View {
        Canvas { ctx, size in
            drawPaper(&ctx, size)
            // The line tilts a little, about the middle of the scrap.
            ctx.translateBy(x: size.width / 2, y: size.height / 2)
            ctx.rotate(by: .degrees(Double(sample.tilt)))
            ctx.translateBy(x: -size.width / 2, y: -size.height / 2)
            switch sample.style {
            case .strokes: drawStrokes(&ctx, size)
            case .font(let name): drawFont(name, &ctx, size)
            }
        }
        .frame(height: height)
        .clipShape(RoundedRectangle(cornerRadius: 10))
        .overlay(RoundedRectangle(cornerRadius: 10).stroke(Theme.rule.opacity(0.35), lineWidth: 1))
        .accessibilityLabel("A handwritten line")
    }

    private var paperColor: Color {
        switch sample.paper {
        case .ruled: Theme.paper
        case .squared: Color(Theme.dynamic(0xFFFFFF, 0x13202E))
        case .plain: Color(Theme.dynamic(0xFFF6C8, 0x2A2612))
        }
    }

    private func drawPaper(_ ctx: inout GraphicsContext, _ size: CGSize) {
        ctx.fill(Path(CGRect(origin: .zero, size: size)), with: .color(paperColor))
        let rule = Theme.rule.opacity(0.35)
        switch sample.paper {
        case .ruled:
            var y = size.height * 0.72
            var p = Path()
            while y > 4 { p.move(to: CGPoint(x: 0, y: y)); p.addLine(to: CGPoint(x: size.width, y: y)); y -= size.height * 0.36 }
            ctx.stroke(p, with: .color(rule), lineWidth: 1)
        case .squared:
            var p = Path()
            var x: CGFloat = 0
            while x <= size.width { p.move(to: CGPoint(x: x, y: 0)); p.addLine(to: CGPoint(x: x, y: size.height)); x += 22 }
            var y: CGFloat = 0
            while y <= size.height { p.move(to: CGPoint(x: 0, y: y)); p.addLine(to: CGPoint(x: size.width, y: y)); y += 22 }
            ctx.stroke(p, with: .color(Theme.grid), lineWidth: 0.6)
        case .plain:
            break
        }
    }

    private func drawStrokes(_ ctx: inout GraphicsContext, _ size: CGSize) {
        guard let m = CursiveModels.shared else { return }
        var k = size.height / StripMetrics.designHeight * sample.size
        let design = CursiveLayout.width(sample.text, in: m)
        if design * k > size.width - 36 { k = (size.width - 36) / design }
        let baseline = size.height * 0.72
        let yShift = baseline - m.lines.baseline * k
        var path = Path()
        var n = 0
        for stroke in CursiveLayout.strokes(sample.text, in: m) where stroke.count > 1 {
            let pts = stroke.map { p -> CGPoint in
                n += 1
                // Slow drift along the stroke (a hand's sway), not per-point noise.
                let dx = CGFloat(sin(Double(n) * 0.16 + sample.wobble)) * sample.jitter
                let dy = CGFloat(cos(Double(n) * 0.11 + sample.wobble * 1.3)) * sample.jitter
                let x = p.x + dx + (m.lines.baseline - p.y) * sample.slant
                return CGPoint(x: 18 + x * k, y: yShift + (p.y + dy) * k)
            }
            path.move(to: pts[0])
            for p in pts.dropFirst() { path.addLine(to: p) }
        }
        let width = (sample.ink == .pencil ? 2.2 : 3.0) * max(k, 0.5)
        ctx.stroke(path, with: .color(sample.inkColor.opacity(sample.ink == .pencil ? 0.8 : 0.95)),
                   style: StrokeStyle(lineWidth: width, lineCap: .round, lineJoin: .round))
    }

    private func drawFont(_ name: String, _ ctx: inout GraphicsContext, _ size: CGSize) {
        var fontSize = size.height * 0.46 * sample.size
        var resolved = ctx.resolve(Text(sample.text).font(.custom(name, size: fontSize)).foregroundStyle(sample.inkColor))
        var measured = resolved.measure(in: CGSize(width: 10_000, height: 10_000))
        if measured.width > size.width - 36 {
            fontSize *= (size.width - 36) / measured.width
            resolved = ctx.resolve(Text(sample.text).font(.custom(name, size: fontSize)).foregroundStyle(sample.inkColor))
            measured = resolved.measure(in: CGSize(width: 10_000, height: 10_000))
        }
        let baseline = size.height * 0.72
        // Shear about the baseline: tops lean right by slant × height.
        ctx.transform = ctx.transform.concatenating(CGAffineTransform(a: 1, b: 0, c: -sample.slant, d: 1, tx: sample.slant * baseline, ty: 0))
        ctx.draw(resolved, at: CGPoint(x: 18, y: baseline - measured.height * 0.72), anchor: .topLeading)
    }
}
