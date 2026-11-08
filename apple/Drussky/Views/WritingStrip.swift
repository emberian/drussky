import PencilKit
import SwiftUI
import UIKit

/// Geometry of one ruled writing line, proportional to its height.
struct StripMetrics {
    /// Ink is stored in a strip of this height; taller or shorter strips scale the canvas so
    /// writing always sits on its own lines, whatever the device or orientation.
    static let designHeight: CGFloat = 132
    var height: CGFloat
    var capY: CGFloat { height * 0.17 }
    var baselineY: CGFloat { height * 0.70 }
    var descenderY: CGFloat { height * 0.90 }

    /// The dashed line: the model font's own x-height above the baseline, so small letters
    /// fill exactly the space between the two.
    func xHeightY(cursive: Bool) -> CGFloat { baselineY - font(cursive: cursive).xHeight }

    func font(cursive: Bool) -> UIFont {
        let probe = Theme.modelFont(size: 100, cursive: cursive)
        let capPerPoint = max(probe.capHeight, 1) / 100
        return Theme.modelFont(size: (baselineY - capY) / capPerPoint, cursive: cursive)
    }

    /// Height at which `text` (repeated `times`) fits `width` in the model font.
    static func fitting(_ text: String, times: Int = 1, width: CGFloat, cursive: Bool, preferred: CGFloat) -> CGFloat {
        let probe = StripMetrics(height: 100)
        let font = probe.font(cursive: cursive)
        let one = (RussianText.unstressed(text) as NSString).size(withAttributes: [.font: font]).width
        let gap = font.pointSize * 0.8
        let needed = one * CGFloat(max(times, 1)) + gap * CGFloat(max(times - 1, 0)) + 40
        let scale = (width - 32) / max(needed, 1)
        return max(72, min(preferred, 100 * scale))
    }
}

/// Owns a strip's PencilKit canvas so SwiftUI controls (Undo, Clear, Check) can reach it.
/// Every change gets a new revision and goes to the notebook's one `InkWriter`, which writes
/// in order and never lets an older drawing replace a newer one.
@MainActor
final class PadController: ObservableObject {
    @Published private(set) var isEmpty = true
    @Published private(set) var strokeCount = 0
    /// The revision of the ink now on the strip; a check compares it before trusting a reading.
    private(set) var revision = 0
    fileprivate weak var canvas: PKCanvasView?
    let fileURL: URL
    let writer: InkWriter
    var onChange: ((PKDrawing) -> Void)?

    init(fileURL: URL, writer: InkWriter) {
        self.fileURL = fileURL
        self.writer = writer
        Self.live.append(Weak(pad: self))
    }

    private struct Weak { weak var pad: PadController? }
    private static var live: [Weak] = []

    /// Hands every open strip's current ink to its writer and waits until each has it, so a
    /// following `flush()` writes everything (background, export, restore).
    static func submitAll() async {
        live.removeAll { $0.pad == nil }
        for pad in live.compactMap(\.pad) where pad.revision > 0 { await pad.submitCurrent(fresh: false) }
    }

    /// Submits what is on the canvas, and waits for the writer to take it. Under its own
    /// revision (it is the ink of that change) unless `fresh`, an explicit new version.
    func submitCurrent(fresh: Bool) async {
        guard let canvas else { return }
        if fresh || revision == 0 { revision = InkRevision.next() }
        let d = canvas.drawing
        await writer.submit(fileURL, revision: revision, data: d.strokes.isEmpty ? nil : d.dataRepresentation())
    }

    var drawing: PKDrawing { canvas?.drawing ?? storedDrawing() }

    func storedDrawing() -> PKDrawing {
        guard let data = try? Data(contentsOf: fileURL), let d = try? PKDrawing(data: data) else { return PKDrawing() }
        return d
    }

    /// Empties the strip. Undo brings the ink back.
    func clear() {
        guard let canvas, !canvas.drawing.strokes.isEmpty else { return }
        replace(with: PKDrawing(), undoable: true)
    }

    func undo() { canvas?.undoManager?.undo() }

    /// Replaces the ink (used by QA to inject synthetic writing).
    func setDrawing(_ drawing: PKDrawing) {
        replace(with: drawing, undoable: false)
    }

    private func replace(with drawing: PKDrawing, undoable: Bool) {
        guard let canvas else { changed(drawing); return }
        let old = canvas.drawing
        if undoable, let undo = canvas.undoManager {
            undo.registerUndo(withTarget: self) { pad in
                MainActor.assumeIsolated { pad.replace(with: old, undoable: true) }
            }
        }
        canvas.drawing = drawing
        changed(drawing)
    }

    /// Keeps the current ink as an earlier attempt at `destination` and starts a clean strip.
    func archive(to destination: URL) async throws {
        await submitCurrent(fresh: true)
        try await writer.archive(fileURL, to: destination)
        canvas?.drawing = PKDrawing()
        canvas?.undoManager?.removeAllActions()
        revision = InkRevision.next()
        isEmpty = true
        strokeCount = 0
        await writer.submit(fileURL, revision: revision, data: nil)
    }

    fileprivate func changed(_ drawing: PKDrawing) {
        isEmpty = drawing.strokes.isEmpty
        strokeCount = drawing.strokes.count
        revision = InkRevision.next()
        onChange?(drawing)
        let url = fileURL, writer = self.writer, revision = self.revision
        let data = drawing.strokes.isEmpty ? nil : drawing.dataRepresentation()
        Task { await writer.submit(url, revision: revision, data: data) }
    }

    fileprivate func attach(_ canvas: PKCanvasView) {
        self.canvas = canvas
        let d = storedDrawing()
        canvas.drawing = d
        isEmpty = d.strokes.isEmpty
        strokeCount = d.strokes.count
    }
}

/// A ruled line of exercise-book paper you write on with the Pencil.
struct WritingStrip: UIViewRepresentable {
    @ObservedObject var controller: PadController
    var height: CGFloat
    var ghost: String?
    var ghostTimes: Int = 1
    var cursive: Bool
    var fingerDrawing: Bool
    var locked = false

    func makeUIView(context: Context) -> PaperStripView {
        let view = PaperStripView()
        controller.attach(view.canvas)
        view.canvas.delegate = context.coordinator
        context.coordinator.controller = controller
        return view
    }

    func updateUIView(_ view: PaperStripView, context: Context) {
        context.coordinator.controller = controller
        view.configure(metrics: StripMetrics(height: height), ghost: ghost.map(RussianText.unstressed),
                       ghostTimes: ghostTimes, cursive: cursive)
        #if targetEnvironment(simulator)
        view.canvas.drawingPolicy = .anyInput
        #else
        view.canvas.drawingPolicy = fingerDrawing ? .anyInput : .default
        #endif
        view.canvas.isUserInteractionEnabled = !locked
    }

    func sizeThatFits(_ proposal: ProposedViewSize, uiView: PaperStripView, context: Context) -> CGSize? {
        CGSize(width: proposal.width ?? 600, height: height)
    }

    func makeCoordinator() -> Coordinator { Coordinator() }

    @MainActor
    final class Coordinator: NSObject, PKCanvasViewDelegate {
        var controller: PadController?
        func canvasViewDrawingDidChange(_ canvasView: PKCanvasView) {
            controller?.changed(canvasView.drawing)
        }
    }
}

/// Each strip keeps its own undo history; the window's shared one would let Undo on one
/// exercise remove strokes from another.
final class StripCanvas: PKCanvasView {
    private let ownUndo = UndoManager()
    override var undoManager: UndoManager? { ownUndo }
}

final class PaperStripView: UIView, UIPencilInteractionDelegate {
    let canvas = StripCanvas()
    private var metrics = StripMetrics(height: 120)
    private var ghost: String?
    private var ghostTimes = 1
    private var cursive = false
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
        // The canvas's own accessibility tree grows with every stroke and makes snapshots crawl;
        // the strip itself is the accessible element.
        canvas.accessibilityElementsHidden = true
        isAccessibilityElement = true
        accessibilityLabel = "Writing line"
        accessibilityTraits = [.allowsDirectInteraction]
        addSubview(canvas)
        let pencil = UIPencilInteraction()
        pencil.delegate = self
        addInteraction(pencil)
        registerForTraitChanges([UITraitUserInterfaceStyle.self]) { (view: PaperStripView, _) in view.setNeedsDisplay() }
    }

    required init?(coder: NSCoder) { fatalError() }

    func configure(metrics: StripMetrics, ghost: String?, ghostTimes: Int, cursive: Bool) {
        guard metrics.height != self.metrics.height || ghost != self.ghost || ghostTimes != self.ghostTimes
            || cursive != self.cursive else { return }
        self.metrics = metrics
        self.ghost = ghost
        self.ghostTimes = ghostTimes
        self.cursive = cursive
        setNeedsDisplay()
        setNeedsLayout()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        canvas.frame = bounds
        let zoom = max(bounds.height, 1) / StripMetrics.designHeight
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
        func line(_ y: CGFloat, width: CGFloat, dash: [CGFloat]? = nil, alpha: CGFloat = 1) {
            ctx.saveGState()
            ctx.setStrokeColor(rule.withAlphaComponent(rule.cgColor.alpha * alpha).cgColor)
            ctx.setLineWidth(width)
            if let dash { ctx.setLineDash(phase: 0, lengths: dash) }
            ctx.move(to: CGPoint(x: 0, y: y)); ctx.addLine(to: CGPoint(x: w, y: y))
            ctx.strokePath()
            ctx.restoreGState()
        }
        line(metrics.capY, width: 0.8, alpha: 0.55)
        line(metrics.xHeightY(cursive: cursive), width: 0.8, dash: [5, 5], alpha: 0.6)
        line(metrics.baselineY, width: 1.4)
        line(metrics.descenderY, width: 0.6, alpha: 0.35)

        guard let ghost, !ghost.isEmpty else { return }
        let font = metrics.font(cursive: cursive)
        let attrs: [NSAttributedString.Key: Any] = [
            .font: font, .foregroundColor: Theme.ghostUI.resolvedColor(with: traitCollection),
        ]
        let piece = NSAttributedString(string: ghost, attributes: attrs)
        let gap = font.pointSize * 0.8
        let inset = 16 * metrics.height / StripMetrics.designHeight
        var x: CGFloat = inset
        for _ in 0..<max(1, ghostTimes) {
            let size = piece.size()
            guard x + size.width < w || x == inset else { break }
            piece.draw(at: CGPoint(x: x, y: metrics.baselineY - font.ascender))
            x += size.width + gap
        }
    }

    func pencilInteraction(_ interaction: UIPencilInteraction, didReceiveTap tap: UIPencilInteraction.Tap) {
        erasing.toggle()
        canvas.tool = erasing ? PKEraserTool(.vector) : pen
    }
}


// MARK: - Margin notes

/// A pencil layer over the whole page: write anywhere, like the margins of a real notebook.
/// Ink lives in a 900-pt-wide coordinate space and scales with the page width.
struct MarginInk: UIViewRepresentable {
    static let designWidth: CGFloat = 900
    @ObservedObject var controller: PadController
    var enabled: Bool
    var fingerDrawing: Bool

    func makeUIView(context: Context) -> MarginInkView {
        let view = MarginInkView()
        controller.attach(view.canvas)
        view.canvas.delegate = context.coordinator
        context.coordinator.controller = controller
        return view
    }

    func updateUIView(_ view: MarginInkView, context: Context) {
        context.coordinator.controller = controller
        view.isUserInteractionEnabled = enabled
        view.canvas.isUserInteractionEnabled = enabled
        #if targetEnvironment(simulator)
        view.canvas.drawingPolicy = .anyInput
        #else
        view.canvas.drawingPolicy = fingerDrawing ? .anyInput : .default
        #endif
    }

    func makeCoordinator() -> WritingStrip.Coordinator { WritingStrip.Coordinator() }
}

final class MarginInkView: UIView, UIPencilInteractionDelegate {
    let canvas = StripCanvas()
    private let pencil = PKInkingTool(.pencil, color: UIColor(hex: 0x3A4A66), width: 3)
    private var erasing = false

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = .clear
        canvas.backgroundColor = .clear
        canvas.isOpaque = false
        canvas.isScrollEnabled = false
        canvas.tool = pencil
        canvas.accessibilityElementsHidden = true
        addSubview(canvas)
        let interaction = UIPencilInteraction()
        interaction.delegate = self
        addInteraction(interaction)
    }

    required init?(coder: NSCoder) { fatalError() }

    override func layoutSubviews() {
        super.layoutSubviews()
        canvas.frame = bounds
        let zoom = max(bounds.width, 1) / MarginInk.designWidth
        canvas.minimumZoomScale = zoom
        canvas.maximumZoomScale = zoom
        canvas.zoomScale = zoom
        canvas.contentSize = bounds.size
        canvas.contentOffset = .zero
    }

    func pencilInteraction(_ interaction: UIPencilInteraction, didReceiveTap tap: UIPencilInteraction.Tap) {
        erasing.toggle()
        canvas.tool = erasing ? PKEraserTool(.vector) : pencil
    }
}
