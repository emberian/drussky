import CoreText
import PencilKit
import UIKit

/// Launch-argument hooks for simulator QA and screenshots. Inert without the arguments.
///   -qa-reset          start from an empty notebook
///   -qa-complete N     work through the first N pages (with synthetic handwriting)
///   -qa-partial K      also complete the first K exercises of the page after that
///   -qa-age D          pretend D days have passed since everything was learned
@MainActor
enum QA {
    static func value(_ flag: String) -> Int? {
        let args = ProcessInfo.processInfo.arguments
        guard let i = args.firstIndex(of: flag), i + 1 < args.count else { return nil }
        return Int(args[i + 1])
    }
    static func has(_ flag: String) -> Bool { ProcessInfo.processInfo.arguments.contains(flag) }

    static func prepare(store: BookStore) {
        #if DEBUG
        if let key = ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"], !key.isEmpty { Keychain.apiKey = key }
        #endif
        guard has("-qa-reset") else { return }
        try? FileManager.default.removeItem(at: store.root)
        _ = BookStore(root: store.root)
    }

    static func apply(to book: Book) {
        guard has("-qa-reset") else { return }
        let pages = value("-qa-complete") ?? 0
        for _ in 0..<pages {
            complete(book, page: book.currentPage, upTo: Int.max)
            #if DEBUG
            if let days = value("-qa-age") { book.shiftMemories(byDays: Double(days)) }
            #endif
            book.turnForward()
        }
        if let i = ProcessInfo.processInfo.arguments.firstIndex(of: "-qa-until"), i + 1 < ProcessInfo.processInfo.arguments.count {
            let target = ProcessInfo.processInfo.arguments[i + 1]
            var guardCount = 0
            while book.page?.sourceID != target && guardCount < 300 {
                complete(book, page: book.currentPage, upTo: Int.max)
                book.turnForward()
                guardCount += 1
            }
        }
        if let partial = value("-qa-partial") { complete(book, page: book.currentPage, upTo: partial) }
        book.saveNow()
    }

    static func complete(_ book: Book, page pageIndex: Int, upTo limit: Int) {
        let page = book.pages[pageIndex]
        for (i, ex) in page.exercises.enumerated() where i < limit {
            if ex.kind.isWriting, let target = ex.kind == .meet ? ex.letter.map({ l in
                book.course.letter(l).map { "\($0.upper) \($0.lower)" } ?? l }) : ex.target {
                let drawing = SyntheticInk.drawing(for: RussianText.unstressed(target), height: 132, wobble: Double(i))
                try? drawing.dataRepresentation().write(to: book.inkURL(page: page, exercise: i))
            }
            book.record(page: pageIndex, exercise: i) { r in
                r.attempts = 1
                switch ex.kind {
                case .listenPick: r.chosen = ex.options?.firstIndex(of: ex.say ?? ""); r.status = .correct
                case .readMeaning: r.chosen = ex.answerIndex; r.status = .correct
                case .readAloud: r.recognized = ex.text.map(RussianText.unstressed); r.status = .correct
                case .dictation, .translate, .fill: r.status = i % 5 == 3 ? .missed : .correct
                    if r.status == .missed { r.revealed = true }
                default: r.status = .done
                }
            }
        }
    }
}

/// Handwriting-like ink generated from font outlines: each glyph contour becomes a stroke.
enum SyntheticInk {
    static func drawing(for text: String, height: CGFloat, wobble: Double = 0, cursive: Bool = false) -> PKDrawing {
        let metrics = StripMetrics(height: height)
        let font = metrics.font(cursive: cursive) as CTFont
        let attr = NSAttributedString(string: text, attributes: [.font: font])
        let line = CTLineCreateWithAttributedString(attr)
        var strokes: [PKStroke] = []
        let ink = PKInk(.pen, color: Theme.inkUI.resolvedColor(with: .init(userInterfaceStyle: .light)))
        let runs = CTLineGetGlyphRuns(line) as! [CTRun]
        for run in runs {
            let count = CTRunGetGlyphCount(run)
            var glyphs = [CGGlyph](repeating: 0, count: count)
            var positions = [CGPoint](repeating: .zero, count: count)
            CTRunGetGlyphs(run, CFRange(location: 0, length: count), &glyphs)
            CTRunGetPositions(run, CFRange(location: 0, length: count), &positions)
            let runFont = (CTRunGetAttributes(run) as NSDictionary)[kCTFontAttributeName] as! CTFont
            for (g, p) in zip(glyphs, positions) {
                guard let path = CTFontCreatePathForGlyph(runFont, g, nil) else { continue }
                // Glyph space is y-up; flip onto the strip's baseline.
                var t = CGAffineTransform(translationX: 16 + p.x, y: metrics.baselineY).scaledBy(x: 1, y: -1)
                guard let placed = path.copy(using: &t) else { continue }
                for contour in contours(of: placed) where contour.count > 1 {
                    let points = contour.enumerated().map { j, pt in
                        PKStrokePoint(location: CGPoint(x: pt.x + sin(Double(j) * 0.7 + wobble) * 0.6, y: pt.y),
                                      timeOffset: Double(j) * 0.01, size: CGSize(width: 3.5, height: 3.5),
                                      opacity: 1, force: 1, azimuth: 0, altitude: .pi / 2)
                    }
                    strokes.append(PKStroke(ink: ink, path: PKStrokePath(controlPoints: points, creationDate: Date())))
                }
            }
        }
        return PKDrawing(strokes: strokes)
    }

    /// Flattens a path into polylines, one per closed contour.
    static func contours(of path: CGPath) -> [[CGPoint]] {
        var result: [[CGPoint]] = []
        var current: [CGPoint] = []
        var last = CGPoint.zero
        path.applyWithBlock { element in
            let e = element.pointee
            switch e.type {
            case .moveToPoint:
                if !current.isEmpty { result.append(current) }
                last = e.points[0]; current = [last]
            case .addLineToPoint:
                last = e.points[0]; current.append(last)
            case .addQuadCurveToPoint:
                let c = e.points[0], end = e.points[1]
                for s in 1...6 {
                    let t = CGFloat(s) / 6
                    current.append(CGPoint(x: (1-t)*(1-t)*last.x + 2*(1-t)*t*c.x + t*t*end.x,
                                           y: (1-t)*(1-t)*last.y + 2*(1-t)*t*c.y + t*t*end.y))
                }
                last = end
            case .addCurveToPoint:
                let c1 = e.points[0], c2 = e.points[1], end = e.points[2]
                for s in 1...8 {
                    let t = CGFloat(s) / 8, u = 1 - t
                    current.append(CGPoint(x: u*u*u*last.x + 3*u*u*t*c1.x + 3*u*t*t*c2.x + t*t*t*end.x,
                                           y: u*u*u*last.y + 3*u*u*t*c1.y + 3*u*t*t*c2.y + t*t*t*end.y))
                }
                last = end
            case .closeSubpath:
                if let first = current.first { current.append(first) }
                result.append(current); current = []
            @unknown default: break
            }
        }
        if !current.isEmpty { result.append(current) }
        return result
    }
}
