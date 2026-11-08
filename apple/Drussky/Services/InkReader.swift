import PencilKit
import UIKit
@preconcurrency import Vision

/// Reads Russian handwriting from a PencilKit drawing with Vision's on-device recognizer.
enum InkReader {
    struct Reading: Sendable {
        var best: String
        var candidates: [String]
        var matches: Bool
        /// The recognizer's own confidence (0–1) in the pass that produced `best`, when it gave one.
        var confidence: Double? = nil
    }

    /// One way of showing the ink to the recognizer.
    private struct Pass {
        var tiles = 1
        var height: CGFloat = 160
        var accurate = true
        var correction = false
        /// Shear the ink upright before rendering (a slanted cursive hand), by this much per point of height.
        var shear: CGFloat = 0
    }

    /// Adds an upright-sheared pass for slanted cursive. Measured on synthetic cursive by
    /// CursiveTests.testInkReaderOnCursiveHands; see the numbers there before turning it on.
    nonisolated(unsafe) static var cursivePass = false

    /// Recognizes the drawing and compares it to `target`. Returns nil for an empty drawing.
    /// Recognizers differ between devices and OS versions, so several presentations are tried
    /// in turn and the first that reads as the answer wins; if none does, the most common
    /// reading is reported.
    static func read(_ drawing: PKDrawing, target: String, subjectDrop: Bool = false) async -> Reading? {
        guard !drawing.strokes.isEmpty else { return nil }
        let single = RussianText.comparable(target).count == 1
        var passes: [Pass] = single
            ? [Pass(tiles: 3), Pass(tiles: 4, height: 260, correction: true), Pass(tiles: 1, height: 260),
               Pass(tiles: 3, accurate: false), Pass(tiles: 6, height: 200)]
            : [Pass(correction: RussianText.comparable(target).count > 3), Pass(height: 260, correction: true),
               Pass(height: 260), Pass(accurate: false)]
        if cursivePass { passes.append(single ? Pass(tiles: 3, shear: 0.3) : Pass(height: 260, correction: true, shear: 0.3)) }
        var all: [String] = []
        var firstConfidence: Double?
        for pass in passes {
            var source = pass.tiles > 1 ? tiled(drawing, times: pass.tiles) : drawing
            if pass.shear != 0 {  // x' = x - shear × (height below the top): a right-leaning hand stood upright
                let b = source.bounds
                source = source.transformed(using: CGAffineTransform(a: 1, b: 0, c: pass.shear, d: 1, tx: -pass.shear * b.maxY, ty: 0))
            }
            let image = await MainActor.run { render(source, height: pass.height) }
            guard let cg = image?.cgImage else { continue }
            let hint = RussianText.unstressed(pass.tiles > 1 ? String(repeating: target, count: pass.tiles) : target)
            let (candidates, confidence) = await recognize(cg, hint: hint, accurate: pass.accurate, correction: pass.correction)
            if firstConfidence == nil, !candidates.isEmpty { firstConfidence = confidence }
            all += candidates.filter { !all.contains($0) }
            if let hit = candidates.first(where: { single ? (pass.tiles > 1 ? majority($0, of: target) : RussianText.matches($0, target))
                                                          : RussianText.answerMatches($0, target, subjectDrop: subjectDrop) }) {
                return Reading(best: single ? String(hit.prefix(1)) : hit, candidates: all, matches: true, confidence: confidence)
            }
        }
        let best = all.first ?? ""
        return Reading(best: single ? String(best.prefix(1)) : best, candidates: all, matches: false, confidence: firstConfidence)
    }

    /// For tiled single letters: most of what was read is the target letter.
    static func majority(_ heard: String, of target: String) -> Bool {
        guard let want = RussianText.comparable(target).lowercased().first else { return false }
        let got = RussianText.key(RussianText.foldMultiGlyph(heard)).filter { !$0.isWhitespace }
        func isWant(_ c: Character) -> Bool {
            let mapped = RussianText.comparable(String(c)).first
            return mapped == want || (RussianText.lookalikes[Character(c.lowercased())]?.contains(want) ?? false)
        }
        return !got.isEmpty && got.filter(isWant).count * 2 > got.count
    }

    static func tiled(_ drawing: PKDrawing, times: Int) -> PKDrawing {
        let step = drawing.bounds.width + max(drawing.bounds.height * 0.35, 12)
        var out = drawing
        for i in 1..<times {
            out.append(drawing.transformed(using: CGAffineTransform(translationX: step * CGFloat(i), y: 0)))
        }
        return out
    }


    /// Dark ink on white, padded, at a size the recognizer likes.
    @MainActor
    static func render(_ drawing: PKDrawing, height: CGFloat = 160) -> UIImage? {
        var bounds = drawing.bounds.insetBy(dx: -24, dy: -24)
        if bounds.width < bounds.height * 1.2 {  // give short words some horizontal room
            bounds = bounds.insetBy(dx: -(bounds.height * 1.2 - bounds.width) / 2, dy: 0)
        }
        let scale = max(1, min(4, height / max(bounds.height, 1)))
        let traits = UITraitCollection(userInterfaceStyle: .light)
        var image: UIImage?
        traits.performAsCurrent { image = drawing.image(from: bounds, scale: scale) }
        guard let ink = image else { return nil }
        let renderer = UIGraphicsImageRenderer(size: ink.size, format: {
            let f = UIGraphicsImageRendererFormat(); f.scale = ink.scale; f.opaque = true; return f
        }())
        return renderer.image { ctx in
            UIColor.white.setFill()
            ctx.fill(CGRect(origin: .zero, size: ink.size))
            ink.draw(at: .zero)
        }
    }

    /// The candidate strings and the recognizer's mean confidence in its top reading (nil when none).
    private static func recognize(_ image: CGImage, hint: String, accurate: Bool, correction: Bool) async -> ([String], Double?) {
        await withCheckedContinuation { (continuation: CheckedContinuation<([String], Double?), Never>) in
            let once = Once()
            let request = VNRecognizeTextRequest { request, _ in
                guard once.claim() else { return }
                let observations = (request.results as? [VNRecognizedTextObservation]) ?? []
                // Join lines left-to-right; keep alternatives from the top line.
                let sorted = observations.sorted { $0.boundingBox.minX < $1.boundingBox.minX }
                var lines: [String] = []
                let tops = sorted.compactMap { $0.topCandidates(1).first }
                let joined = tops.map(\.string).joined(separator: " ")
                if !joined.isEmpty { lines.append(joined) }
                if sorted.count == 1 {
                    lines += sorted[0].topCandidates(5).map(\.string)
                }
                var seen = Set<String>()
                let confidence = tops.isEmpty ? nil : Double(tops.map(\.confidence).reduce(0, +)) / Double(tops.count)
                continuation.resume(returning: (lines.filter { seen.insert($0).inserted }, confidence))
            }
            request.recognitionLevel = accurate ? .accurate : .fast
            request.recognitionLanguages = ["ru-RU"]
            request.usesLanguageCorrection = correction
            request.customWords = [hint]
            request.minimumTextHeight = 0.1
            let handler = VNImageRequestHandler(cgImage: image)
            DispatchQueue.global(qos: .userInitiated).async {
                do { try handler.perform([request]) } catch { if once.claim() { continuation.resume(returning: ([], nil)) } }
            }
        }
    }
}

/// Lets exactly one of several callbacks resume a continuation.
final class Once: @unchecked Sendable {
    private let lock = NSLock()
    private var done = false
    func claim() -> Bool {
        lock.lock(); defer { lock.unlock() }
        if done { return false }
        done = true
        return true
    }
}
