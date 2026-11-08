import CoreGraphics
import UIKit

/// The writing strip's cursive face: the ghost a learner traces is the stroke model written
/// through, not a font, and the dashed guide sits on the models' own x-line.
enum CursiveGhost {
    /// Gap between repetitions of the model, in design points.
    static let gap: CGFloat = 30

    /// How far above the baseline the small letters' top is, for a strip of `height`.
    static func xHeight(height: CGFloat) -> CGFloat? {
        CursiveModels.shared.map { height * ($0.lines.baseline - $0.lines.xHeight) / StripMetrics.designHeight }
    }

    /// The strip height at which `text` written `times` fits `width` (nil without models).
    static func fitting(_ text: String, times: Int, width: CGFloat, preferred: CGFloat) -> CGFloat? {
        guard let m = CursiveModels.shared else { return nil }
        let one = CursiveLayout.width(RussianText.unstressed(text), in: m)
        let needed = one * CGFloat(max(times, 1)) + gap * CGFloat(max(times - 1, 0)) + 40
        return max(72, min(preferred, StripMetrics.designHeight * (width - 32) / max(needed, 1)))
    }

    /// Draws the written model `times` along the strip, as many as fit.
    static func draw(_ text: String, times: Int, in ctx: CGContext, metrics: StripMetrics, width: CGFloat, color: UIColor) {
        guard let m = CursiveModels.shared else { return }
        let k = metrics.height / StripMetrics.designHeight
        let inset = 16 * k
        let one = CursiveLayout.width(text, in: m) * k
        var x = inset
        ctx.saveGState()
        ctx.setStrokeColor(color.cgColor)
        ctx.setLineWidth(3.2 * k)
        ctx.setLineCap(.round)
        ctx.setLineJoin(.round)
        for _ in 0..<max(1, times) {
            guard x + one < width || x == inset else { break }
            ctx.addPath(CursiveLayout.path(text, in: m, origin: CGPoint(x: x, y: 0), scale: k).cgPath)
            ctx.strokePath()
            x += one + gap * k
        }
        ctx.restoreGState()
    }
}
