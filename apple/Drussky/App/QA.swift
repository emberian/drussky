import CoreText
import PencilKit
import UIKit

/// Launch-argument hooks for simulator QA and screenshots. Inert without the arguments.
///   -qa-reset          start from an empty notebook
///   -qa-complete N     work through the first N pages (with synthetic handwriting)
///   -qa-partial K      also complete the first K exercises of the page after that
///   -qa-age D          pretend D days have passed since everything was learned
///   -qa-history D      spread the completed pages over the last D days (knowledge-map history)
///   -qa-misses         choice exercises are missed twice (the correction under the choices)
///   -qa-corrupt        an undecodable book.json in the QA folder (the recovery screen)
///   -qa-day N / -qa-away D / -qa-probes / -qa-tired   the day's shape (QA+Day.swift)
@MainActor
enum QA {
    static func value(_ flag: String) -> Int? {
        let args = ProcessInfo.processInfo.arguments
        guard let i = args.firstIndex(of: flag), i + 1 < args.count else { return nil }
        return Int(args[i + 1])
    }
    static func has(_ flag: String) -> Bool { ProcessInfo.processInfo.arguments.contains(flag) }

    /// Any QA launch works in its own notebook folder, so tests can never touch the learner's.
    static var active: Bool { ProcessInfo.processInfo.arguments.contains { $0.hasPrefix("-qa-") } }

    static func prepare(store: BookStore) {
        #if DEBUG
        if let key = ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"], !key.isEmpty { Keychain.apiKey = key }
        // -qa-corrupt: a notebook that exists but can't be decoded (the recovery screen).
        if has("-qa-corrupt") { try? Data("{ \"version\": 1, \"pages\": [".utf8).write(to: store.stateURL) }
        #endif
        guard has("-qa-reset") else { return }
        try? FileManager.default.removeItem(at: store.root)
        _ = BookStore(root: store.root)
    }

    static func apply(to book: Book) {
        if has("-qa-finger") { book.settings.fingerDrawing = true }  // XCUITest can only touch, not Pencil
        if has("-qa-cursive") { book.settings.cursiveModels = true }  // cursive ghosts, demo and hints (Cursive/)
        guard has("-qa-reset") else { return }
        let pages = value("-qa-complete") ?? 0
        let history = value("-qa-history").map { Double($0) * 86_400 }
        // -qa-age D: the pages were done D days ago, so their reviews are due now.
        if let days = value("-qa-age") ?? dayShift { book.now = { Date().addingTimeInterval(-Double(days) * 86_400) } }
        for n in 0..<pages {
            if let history {
                var tick = 0.0
                let start = Date().addingTimeInterval(-history * (1 - Double(n) / Double(max(1, pages))))
                book.now = { tick += 40; return start.addingTimeInterval(tick) }
            }
            complete(book, page: book.currentPage, upTo: Int.max)
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
        book.now = Date.init
        openDay(book)
        if let partial = value("-qa-partial") { complete(book, page: book.currentPage, upTo: partial) }
        applyGoal(to: book)
        applyInstruments(to: book)  // -qa-judge, -qa-hooks, -qa-fluency (Fluency/QA+Instruments.swift)
        // -qa-narration ID: the read-along page of that library text, right after the current page
        // (-qa-narration-at MS shows it paused at that moment of the reading).
        if let id = string("-qa-narration"), let text = Library.load().texts.first(where: { $0.id == id }) {
            let page = Narration.page(for: text)
            book.insertWrittenPage(title: page.title, intro: page.intro, unitTitle: "Library · \(text.level) · read aloud",
                                   exercises: page.exercises, newWords: [])
        }
        // -qa-series N / -qa-math N: episode N of the serial / rung N of the mathematics shelf, as a page (Model/Shelves.swift).
        if let n = value("-qa-series"), let serial = Shelves.serials.first, let ep = serial.episodes.first(where: { $0.number == n }) {
            book.open(Shelves.item(ep, in: serial))
        }
        if let n = value("-qa-math"), let shelf = Shelves.library.shelves?.first,
           let text = Shelves.library.texts(onShelf: shelf.id).first(where: { $0.rung == n }) {
            book.open(Shelves.item(text, on: shelf))
        }
        // -qa-baseline: the weekly sample page (Baseline/); -qa-construction: "Which ending fits?"
        // (Constructions/), whatever the notebook has met so far.
        if has("-qa-baseline") { _ = book.openBaseline() }
        if has("-qa-construction") { _ = book.openConstructionPractice(force: true) }
        book.saveNow()
    }

    static func string(_ flag: String) -> String? {
        let args = ProcessInfo.processInfo.arguments
        guard let i = args.firstIndex(of: flag), i + 1 < args.count else { return nil }
        return args[i + 1]
    }

    static func complete(_ book: Book, page pageIndex: Int, upTo limit: Int) {
        let page = book.pages[pageIndex]
        for (i, ex) in page.exercises.enumerated() where i < limit {
            if ex.kind.isWriting || (ex.kind == .readHandwriting && ex.options == nil), let target = ex.kind == .meet ? ex.letter.map({ l in
                book.course.letter(l).map { "\($0.upper) \($0.lower)" } ?? l }) : ex.target {
                let drawing = SyntheticInk.drawing(for: RussianText.unstressed(target), height: 132, wobble: Double(i))
                try? drawing.dataRepresentation().write(to: book.inkURL(page: page, exercise: i))
            }
            // Now and then a wrong pick first, as a learner would.
            // -qa-misses: choices missed twice (the correction screen).
            if has("-qa-misses"), [.listenPick, .readMeaning].contains(ex.kind), let options = ex.options,
               let wrong = options.indices.first(where: { ex.kind == .listenPick ? options[$0] != ex.say : $0 != ex.answerIndex }) {
                book.record(page: pageIndex, exercise: i) { r in r.chosen = wrong; r.attempts += 1 }
                book.record(page: pageIndex, exercise: i) { r in r.chosen = wrong; r.attempts += 1; r.status = .missed }
                continue
            }
            if ex.kind == .listenPick, i % 4 == 1, let options = ex.options, let wrong = options.firstIndex(where: { $0 != ex.say }) {
                book.record(page: pageIndex, exercise: i) { r in r.chosen = wrong; r.attempts += 1 }
            }
            book.record(page: pageIndex, exercise: i) { r in
                r.attempts += 1
                switch ex.kind {
                case .listenPick: r.chosen = ex.options?.firstIndex(of: ex.say ?? ""); r.status = .correct
                case .readMeaning, .familyGuess, .constructionChoice: r.chosen = ex.answerIndex; r.status = .correct
                case .readHandwriting:
                    r.chosen = ex.options?.firstIndex { RussianText.key($0) == RussianText.key(ex.text ?? "") }
                    r.recognized = ex.text.map(RussianText.unstressed); r.status = .correct
                case .buildWord: r.recognized = BuildWordCheck.assemble(ex.parts ?? []); r.status = .correct
                case .paradigmFill: r.recognized = (ex.cells ?? []).map { "\($0)=✓\(ex.forms?[$0] ?? "")" }.joined(separator: ";"); r.status = .correct
                case .cueSort: r.recognized = (ex.items ?? []).indices.map { "\($0):\(ex.items![$0].answer)" }.joined(separator: ","); r.status = .correct
                case .readAloud: r.recognized = ex.text.map(RussianText.unstressed); r.status = .correct
                case .dictation, .translate, .fill, .transform: r.status = i % 5 == 3 ? .missed : .correct
                    if r.status == .missed { r.revealed = true; r.recognized = misreading(ex.target ?? "") }
                default: r.status = .done
                }
            }
        }
    }
}

extension QA {
    /// A plausible misreading: one letter swapped for one learners mix up.
    static func misreading(_ target: String) -> String {
        let swaps: [Character: Character] = ["о": "а", "е": "и", "ы": "и", "ш": "щ", "ж": "ш", "т": "д", "у": "ю", "л": "п"]
        var chars = Array(RussianText.unstressed(target).lowercased())
        if let i = chars.indices.first(where: { swaps[chars[$0]] != nil }) { chars[i] = swaps[chars[i]]! }
        return String(chars)
    }
}

/// Handwriting-like ink generated from font outlines: each glyph contour becomes a stroke.
enum SyntheticInk {
    static func drawing(for text: String, height: CGFloat, wobble: Double = 0, cursive: Bool = false, font named: String? = nil) -> PKDrawing {
        let metrics = StripMetrics(height: height)
        // A named hand (one of the bundled handwriting fonts) at the cursive model's size, else the model font.
        let font = named.flatMap { UIFont(name: $0, size: metrics.font(cursive: true).pointSize) as CTFont? } ?? (metrics.font(cursive: cursive) as CTFont)
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
