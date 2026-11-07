import PencilKit
import UIKit
@preconcurrency import Vision

/// Reads Russian handwriting from a PencilKit drawing with Vision's on-device recognizer.
enum InkReader {
    struct Reading: Sendable {
        var best: String
        var candidates: [String]
        var matches: Bool
    }

    /// Recognizes the drawing and compares it to `target`. Returns nil for an empty drawing.
    static func read(_ drawing: PKDrawing, target: String) async -> Reading? {
        guard !drawing.strokes.isEmpty else { return nil }
        let single = RussianText.comparable(target).count == 1
        // Recognizers return nothing for a lone glyph; tile it so it reads like a word.
        let source = single ? tiled(drawing, times: 3) : drawing
        let image = await MainActor.run { render(source) }
        guard let cg = image?.cgImage else { return nil }
        let candidates = await recognize(cg, hint: RussianText.unstressed(single ? String(repeating: target, count: 3) : target))
        let matched = candidates.first {
            single ? majority($0, of: target)
                : (RussianText.matches($0, target) || RussianText.matches(caseFolded($0), target))
        }
        let best = matched ?? candidates.first ?? ""
        return Reading(best: single ? String(best.prefix(1)) : best, candidates: candidates, matches: matched != nil)
    }

    /// For tiled single letters: most of what was read is the target letter.
    static func majority(_ heard: String, of target: String) -> Bool {
        let want = RussianText.comparable(target).lowercased()
        let got = RussianText.comparable(heard).lowercased().filter { !$0.isWhitespace }
        return !got.isEmpty && got.filter { String($0) == want }.count * 2 > got.count
    }

    static func tiled(_ drawing: PKDrawing, times: Int) -> PKDrawing {
        let step = drawing.bounds.width + max(drawing.bounds.height * 0.35, 12)
        var out = drawing
        for i in 1..<times {
            out.append(drawing.transformed(using: CGAffineTransform(translationX: step * CGFloat(i), y: 0)))
        }
        return out
    }

    /// Single letters written alone are hard for any recognizer; accept either case.
    private static func caseFolded(_ s: String) -> String { s.lowercased() }

    /// Dark ink on white, padded, at a size the recognizer likes.
    @MainActor
    static func render(_ drawing: PKDrawing) -> UIImage? {
        var bounds = drawing.bounds.insetBy(dx: -24, dy: -24)
        if bounds.width < bounds.height * 1.2 {  // give short words some horizontal room
            bounds = bounds.insetBy(dx: -(bounds.height * 1.2 - bounds.width) / 2, dy: 0)
        }
        let scale = max(1, min(3, 160 / max(bounds.height, 1)))
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

    private static func recognize(_ image: CGImage, hint: String) async -> [String] {
        await withCheckedContinuation { continuation in
            let request = VNRecognizeTextRequest { request, _ in
                let observations = (request.results as? [VNRecognizedTextObservation]) ?? []
                // Join lines left-to-right; keep alternatives from the top line.
                let sorted = observations.sorted { $0.boundingBox.minX < $1.boundingBox.minX }
                var lines: [String] = []
                let joined = sorted.compactMap { $0.topCandidates(1).first?.string }.joined(separator: " ")
                if !joined.isEmpty { lines.append(joined) }
                if sorted.count == 1 {
                    lines += sorted[0].topCandidates(5).map(\.string)
                }
                var seen = Set<String>()
                continuation.resume(returning: lines.filter { seen.insert($0).inserted })
            }
            request.recognitionLevel = .accurate
            request.recognitionLanguages = ["ru-RU"]
            request.usesLanguageCorrection = hint.count > 3
            request.customWords = [hint]
            request.minimumTextHeight = 0.1
            let handler = VNImageRequestHandler(cgImage: image)
            DispatchQueue.global(qos: .userInitiated).async {
                do { try handler.perform([request]) } catch { continuation.resume(returning: []) }
            }
        }
    }
}
