import CoreText
import PencilKit
import UIKit

/// Launch-argument hooks for simulator QA and screenshots. Inert without the arguments.
/// See `QA.knownFlags` below for the full catalog of supported `-qa-*` launch arguments.
@MainActor
enum QA {
    /// Canonical catalog of all `-qa-*` launch arguments recognized across the app and QA extensions.
    static let knownFlags: [(flag: String, summary: String)] = [
        // Notebook & progression (QA.swift)
        ("-qa-reset", "start from an empty QA notebook"),
        ("-qa-complete", "work through the first N pages with synthetic handwriting"),
        ("-qa-partial", "complete the first K exercises of the current page"),
        ("-qa-until", "complete pages forward until reaching the given source page ID"),
        ("-qa-age", "pretend D days have passed since everything was learned"),
        ("-qa-history", "spread completed pages over the last D days"),
        ("-qa-misses", "miss choice exercises twice to show the correction state"),
        ("-qa-corrupt", "write an undecodable book.json in the QA folder to show the recovery screen"),
        ("-qa-nokey", "hide DRUSSKY_TUTOR_KEY for this Debug launch without changing Keychain"),
        ("-qa-finger", "enable finger drawing in settings"),
        ("-qa-cursive", "enable cursive writing models in settings"),
        // Shelves, serials, listening & baseline (QA.swift, ReadAlongView.swift)
        ("-qa-series", "open episode N of «Ста́нция» as a page"),
        ("-qa-kitchen", "open dialogue N of «Ку́хня» as a page"),
        ("-qa-math", "open rung N of the mathematics shelf as a page"),
        ("-qa-listening", "open set N of the listening strand as a page"),
        ("-qa-listening-week", "open the weekly baseline page with reserved listening set N"),
        ("-qa-narration", "insert the read-along page for the given library text ID"),
        ("-qa-narration-at", "show the read-along page paused at MS milliseconds"),
        ("-qa-baseline", "open the weekly baseline sample page"),
        ("-qa-construction", "open 'Which ending fits?' construction practice"),
        // Sheets & navigation (BookView.swift, ContentsView.swift, KnowledgeMapView.swift, GrammarMapView.swift, WordFamiliesView.swift)
        ("-qa-contents", "open the Contents sheet on launch"),
        ("-qa-library", "open Contents on the Library tab"),
        ("-qa-shelves", "open Contents on the Shelves tab"),
        ("-qa-map", "open the Knowledge Map sheet"),
        ("-qa-map-select", "select the given component key in the Knowledge Map"),
        ("-qa-grammar", "open the Grammar Map sheet"),
        ("-qa-grammar-word", "select the given lemma in the Grammar Map"),
        ("-qa-families", "open the Word Families sheet"),
        ("-qa-family-root", "select the given root in Word Families"),
        ("-qa-my-russian", "open the «Мой ру́сский» learner portrait sheet"),
        ("-qa-settings", "open the Settings sheet"),
        ("-qa-sources", "open Settings on «Исто́чники · Sources»"),
        ("-qa-pronunciation", "open the Sound Pairs sheet"),
        ("-qa-drill", "open the Sound Pairs sheet on the given contrast ID"),
        // Typing (PageView.swift, TypedAnswer.swift)
        ("-qa-type", "enable keyboard typing mode on the current page"),
        ("-qa-type-text", "prefill the typed answer field with the given text"),
        ("-qa-type-submit", "prefill and submit the given typed answer"),
        ("-qa-typing-drill", "open the Cyrillic keyboard typing drill"),
        // Day shape & return after break (QA+Day.swift)
        ("-qa-day", "simulate N days of practice"),
        ("-qa-away", "simulate returning after D days away"),
        ("-qa-probes", "seed return-after-break diagnostic probes"),
        ("-qa-tired", "simulate a tired session with misses"),
        // Speaking & intonation (QA+Speaking.swift, QA+Intonation.swift)
        ("-qa-speak", "open a speaking exercise page for the given word"),
        ("-qa-speak-attempt", "seed a speaking attempt state (good, close, tryagain, words, silent)"),
        ("-qa-speak-compare", "seed a take-vs-model prosody comparison"),
        ("-qa-intonation", "open an intonation practice page for the given phrase"),
        // Fluency, self-judgement & hooks (QA+Instruments.swift)
        ("-qa-judge", "seed self-judgement state on writing exercises"),
        ("-qa-hooks", "seed memory hooks"),
        ("-qa-fluency", "seed reading, writing, speaking, and listening fluency series"),
        // Diary, structure, goal & talk (QA+Diary.swift, QA+Structure.swift, QA+Goal.swift, QA+Talk.swift)
        ("-qa-diary", "open a diary page and seed an entry or red-pen correction"),
        ("-qa-structure", "open a sentence-structure exercise page for the given item ID"),
        ("-qa-goal", "seed a reading goal fixture"),
        ("-qa-goal-sheet", "open the Reading Goal sheet"),
        ("-qa-goal-pull", "pull a local reading goal page"),
        ("-qa-talk", "open a conversation scenario page for the given scenario ID"),
        ("-qa-talk-done", "seed a completed conversation scenario with tutor marking"),
    ]
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
        if !has("-qa-nokey"), let key = ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"], !key.isEmpty { Keychain.apiKey = key }
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
        // -qa-series N / -qa-kitchen N / -qa-math N: episode N of «Ста́нция» / dialogue N of «Ку́хня» / rung N of
        // the mathematics shelf, as a page (Model/Shelves.swift); -qa-partial then applies to it.
        for (flag, serialID) in [("-qa-series", "stantsiya"), ("-qa-kitchen", "kukhnya")] {
            if let n = value(flag), let serial = Shelves.serials.first(where: { $0.id == serialID }),
               let ep = serial.episodes.first(where: { $0.number == n }) {
                book.open(Shelves.item(ep, in: serial))
            }
        }
        if let n = value("-qa-math"), let shelf = Shelves.library.shelves?.first,
           let text = Shelves.library.texts(onShelf: shelf.id).first(where: { $0.rung == n }) {
            book.open(Shelves.item(text, on: shelf))
        }
        // -qa-listening N: set N of the listening strand as a page (Model/Listening.swift); -qa-partial
        // then applies to it. -qa-listening-week N: the weekly sample page with reserved set N's messages.
        if let n = value("-qa-listening"), let set = Listening.shared.set(number: n) { book.open(Shelves.item(set, number: n)) }
        if let n = value("-qa-listening-week"), let set = Listening.shared.weeklySet(number: n) { _ = book.openBaseline(listening: set) }
        if let partial = value("-qa-partial") { complete(book, page: book.currentPage, upTo: partial) }
        applyGoal(to: book)
        applyInstruments(to: book)  // -qa-judge, -qa-hooks, -qa-fluency (Fluency/QA+Instruments.swift)
        applyStructure(to: book)  // -qa-structure ID (Structure/QA+Structure.swift)
        // -qa-narration ID: the read-along page of that library text, right after the current page
        // (-qa-narration-at MS shows it paused at that moment of the reading).
        if let id = string("-qa-narration"), let text = Library.load().texts.first(where: { $0.id == id }) {
            let page = Narration.page(for: text)
            book.insertWrittenPage(title: page.title, intro: page.intro, unitTitle: "Library · \(text.level) · read aloud",
                                   exercises: page.exercises, newWords: [])
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
        // One notebook write for the page (Book.batchingSaves), not one per answer.
        book.batchingSaves {
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
                    case .readMeaning, .familyGuess, .constructionChoice, .structure: r.chosen = ex.answerIndex; r.status = .correct
                    case .readHandwriting:
                        r.chosen = ex.options?.firstIndex { RussianText.key($0) == RussianText.key(ex.text ?? "") }
                        r.recognized = ex.text.map(RussianText.unstressed); r.status = .correct
                    case .buildWord: r.recognized = BuildWordCheck.assemble(ex.parts ?? []); r.status = .correct
                    case .paradigmFill: r.recognized = (ex.cells ?? []).map { "\($0)=✓\(ex.forms?[$0] ?? "")" }.joined(separator: ";"); r.status = .correct
                    case .cueSort: r.recognized = (ex.items ?? []).indices.map { "\($0):\(ex.items![$0].answer)" }.joined(separator: ","); r.status = .correct
                    case .readAloud: r.recognized = ex.text.map(RussianText.unstressed); r.status = .correct
                    case .listening:  // every decision right, as the card records them (Model/Listening.swift)
                        if let item = ex.listening {
                            if item.kind == .message { r.recognized = (0..<item.decisionCount).map { String(item.answer($0) ?? 0) }.joined(separator: ",") }
                            else { r.chosen = item.answer }
                        }
                        r.status = .correct
                    case .dictation, .translate, .fill, .transform: r.status = i % 5 == 3 ? .missed : .correct
                        if r.status == .missed { r.revealed = true; r.recognized = misreading(ex.target ?? "") }
                    default: r.status = .done
                    }
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
