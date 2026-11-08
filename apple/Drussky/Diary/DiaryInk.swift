import PencilKit
import SwiftUI
import UIKit
@preconcurrency import Vision

/// Several ruled lines of exercise-book paper for free writing. Ink is stored with
/// `StripMetrics.designHeight` per line and scaled to the line height shown, like a strip.
struct DiaryStrip: UIViewRepresentable {
    @ObservedObject var controller: PadController
    var lines: Int
    var lineHeight: CGFloat
    var fingerDrawing: Bool
    var locked = false

    var height: CGFloat { CGFloat(lines) * lineHeight }

    func makeUIView(context: Context) -> DiaryPaperView {
        let view = DiaryPaperView()
        controller.attach(view.canvas)
        view.canvas.delegate = context.coordinator
        context.coordinator.controller = controller
        return view
    }

    func updateUIView(_ view: DiaryPaperView, context: Context) {
        context.coordinator.controller = controller
        view.configure(lines: lines, lineHeight: lineHeight)
        #if targetEnvironment(simulator)
        view.canvas.drawingPolicy = .anyInput
        #else
        view.canvas.drawingPolicy = fingerDrawing ? .anyInput : .default
        #endif
        view.canvas.isUserInteractionEnabled = !locked
    }

    func sizeThatFits(_ proposal: ProposedViewSize, uiView: DiaryPaperView, context: Context) -> CGSize? {
        CGSize(width: proposal.width ?? 600, height: height)
    }

    func makeCoordinator() -> WritingStrip.Coordinator { WritingStrip.Coordinator() }
}

final class DiaryPaperView: UIView, UIPencilInteractionDelegate {
    let canvas = StripCanvas()
    private var lines = 2
    private var lineHeight: CGFloat = 96
    private let pen = PKInkingTool(.pen, color: Theme.inkUI.resolvedColor(with: .init(userInterfaceStyle: .light)), width: 4.5)
    private var erasing = false

    override init(frame: CGRect) {
        super.init(frame: frame)
        isOpaque = false
        backgroundColor = .clear
        contentMode = .redraw
        canvas.backgroundColor = .clear
        canvas.isOpaque = false
        canvas.isScrollEnabled = false
        canvas.tool = pen
        canvas.overrideUserInterfaceStyle = .unspecified
        canvas.accessibilityElementsHidden = true
        isAccessibilityElement = true
        accessibilityLabel = "Diary lines"
        accessibilityTraits = [.allowsDirectInteraction]
        addSubview(canvas)
        let pencil = UIPencilInteraction()
        pencil.delegate = self
        addInteraction(pencil)
        registerForTraitChanges([UITraitUserInterfaceStyle.self]) { (view: DiaryPaperView, _) in view.setNeedsDisplay() }
    }

    required init?(coder: NSCoder) { fatalError() }

    func configure(lines: Int, lineHeight: CGFloat) {
        guard lines != self.lines || lineHeight != self.lineHeight else { return }
        self.lines = lines
        self.lineHeight = lineHeight
        setNeedsDisplay()
        setNeedsLayout()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        canvas.frame = bounds
        let zoom = max(lineHeight, 1) / StripMetrics.designHeight
        canvas.minimumZoomScale = zoom
        canvas.maximumZoomScale = zoom
        canvas.zoomScale = zoom
        canvas.contentSize = bounds.size
        canvas.contentOffset = .zero
    }

    override func draw(_ rect: CGRect) {
        guard let ctx = UIGraphicsGetCurrentContext() else { return }
        let w = bounds.width
        let rule = Theme.ruleUI.resolvedColor(with: traitCollection)
        let metrics = StripMetrics(height: lineHeight)
        for k in 0..<lines {
            let top = CGFloat(k) * lineHeight
            func line(_ y: CGFloat, width: CGFloat, dash: [CGFloat]? = nil, alpha: CGFloat = 1) {
                ctx.saveGState()
                ctx.setStrokeColor(rule.withAlphaComponent(rule.cgColor.alpha * alpha).cgColor)
                ctx.setLineWidth(width)
                if let dash { ctx.setLineDash(phase: 0, lengths: dash) }
                ctx.move(to: CGPoint(x: 0, y: top + y)); ctx.addLine(to: CGPoint(x: w, y: top + y))
                ctx.strokePath()
                ctx.restoreGState()
            }
            line(metrics.capY, width: 0.8, alpha: 0.45)
            line(metrics.xHeightY(cursive: false), width: 0.8, dash: [5, 5], alpha: 0.5)
            line(metrics.baselineY, width: 1.4)
        }
    }

    func pencilInteraction(_ interaction: UIPencilInteraction, didReceiveTap tap: UIPencilInteraction.Tap) {
        erasing.toggle()
        canvas.tool = erasing ? PKEraserTool(.vector) : pen
    }
}

// MARK: - Reading free handwriting

extension InkReader {
    /// Transcribes free Russian handwriting (no answer to compare with): lines top to bottom,
    /// words left to right, joined with spaces. `vocabulary` biases the recognizer toward the
    /// learner's words. nil for an empty drawing, "" when nothing could be read.
    static func transcribe(_ drawing: PKDrawing, vocabulary: [String] = []) async -> String? {
        guard !drawing.strokes.isEmpty else { return nil }
        let rows = max(1, Int((drawing.bounds.height / (StripMetrics.designHeight * 0.6)).rounded(.up)))
        let image = await MainActor.run { render(drawing, height: CGFloat(rows) * 200) }
        guard let cg = image?.cgImage else { return "" }
        let words = Array(Set(vocabulary.map { RussianText.unstressed($0) })).prefix(300)
        return await recognizeLines(cg, vocabulary: Array(words))
    }

    private static func recognizeLines(_ image: CGImage, vocabulary: [String]) async -> String {
        await withCheckedContinuation { (continuation: CheckedContinuation<String, Never>) in
            let once = Once()
            let request = VNRecognizeTextRequest { request, _ in
                guard once.claim() else { return }
                let observations = (request.results as? [VNRecognizedTextObservation]) ?? []
                // Group boxes into rows (Vision's y axis points up), then read each row left to right.
                var rows: [[VNRecognizedTextObservation]] = []
                for o in observations.sorted(by: { $0.boundingBox.midY > $1.boundingBox.midY }) {
                    if let last = rows.last?.last, abs(last.boundingBox.midY - o.boundingBox.midY) < max(last.boundingBox.height, o.boundingBox.height) * 0.6 {
                        rows[rows.count - 1].append(o)
                    } else {
                        rows.append([o])
                    }
                }
                let text = rows.map { row in
                    row.sorted { $0.boundingBox.minX < $1.boundingBox.minX }.compactMap { $0.topCandidates(1).first?.string }.joined(separator: " ")
                }.filter { !$0.isEmpty }.joined(separator: " ")
                continuation.resume(returning: text.trimmingCharacters(in: .whitespacesAndNewlines))
            }
            request.recognitionLevel = .accurate
            request.recognitionLanguages = ["ru-RU"]
            request.usesLanguageCorrection = true
            request.customWords = vocabulary
            request.minimumTextHeight = 0.05
            let handler = VNImageRequestHandler(cgImage: image)
            DispatchQueue.global(qos: .userInitiated).async {
                do { try handler.perform([request]) } catch { if once.claim() { continuation.resume(returning: "") } }
            }
        }
    }
}
