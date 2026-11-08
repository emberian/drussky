import PencilKit
import UIKit

/// The тетрадь on paper: every page of the notebook as A4 or Letter sheets in the workbook's
/// own style — squared paper, the red margin, the dated header, numbered exercises with the
/// learner's ink, red ticks and corrections, the diary's red pen, the mark.
///
/// A page is laid out as blocks (header, one per exercise, the questions, the footer) that are
/// measured first and then placed on sheets so no exercise is cut in two (`paginate`).
@MainActor
enum NotebookPDF {
    struct Paper: Sendable {
        var size: CGSize
        var margin: CGFloat
        static let a4 = Paper(size: CGSize(width: 595.2, height: 841.8), margin: 44)
        static let letter = Paper(size: CGSize(width: 612, height: 792), margin: 44)
        /// Letter where the US customary system is in use, A4 everywhere else.
        static var local: Paper { Locale.current.measurementSystem == .us ? .letter : .a4 }
    }

    /// One piece of a page that is drawn whole.
    struct Block {
        enum Kind: Equatable { case header, exercise(Int), notes, footer }
        var kind: Kind
        var height: CGFloat
        /// The learner's ink as it will be printed (nil when the exercise has none).
        var ink: UIImage?
        var draw: (CGRect) -> Void
    }

    static let gap: CGFloat = 10
    /// Width of the margin column (exercise numbers, ticks) left of the red rule.
    static let marginColumn: CGFloat = 38

    /// Distributes blocks onto sheets in order, starting a new sheet when the next block doesn't
    /// fit. A block taller than a sheet gets a sheet of its own (and is clipped); nothing is split.
    nonisolated static func paginate(_ heights: [CGFloat], usable: CGFloat, gap: CGFloat = 10) -> [[Int]] {
        var sheets: [[Int]] = []
        var current: [Int] = []
        var used: CGFloat = 0
        for (i, h) in heights.enumerated() {
            let needed = current.isEmpty ? h : used + gap + h
            if !current.isEmpty && needed > usable {
                sheets.append(current)
                current = []
                used = 0
            }
            used = current.isEmpty ? h : used + gap + h
            current.append(i)
        }
        if !current.isEmpty { sheets.append(current) }
        return sheets
    }

    // MARK: Rendering

    /// The whole notebook (or the given pages) as PDF data.
    static func render(book: Book, pages: [BookPage]? = nil, paper: Paper = .local) -> Data {
        Theme.registerFonts()
        let format = UIGraphicsPDFRendererFormat()
        format.documentInfo = [kCGPDFContextTitle as String: "Drussky — моя тетрадь", kCGPDFContextCreator as String: "Drussky"]
        let renderer = UIGraphicsPDFRenderer(bounds: CGRect(origin: .zero, size: paper.size), format: format)
        let contentX = paper.margin + marginColumn + 14
        let width = paper.size.width - contentX - paper.margin
        let usable = paper.size.height - 2 * paper.margin - 16
        var sheetNumber = 0
        return renderer.pdfData { ctx in
            for page in pages ?? book.pages {
                let blocks = self.blocks(for: page, book: book, width: width, originX: contentX)
                for sheet in paginate(blocks.map(\.height), usable: usable) {
                    ctx.beginPage()
                    sheetNumber += 1
                    drawPaper(ctx.cgContext, paper: paper, ruleX: paper.margin + marginColumn)
                    var y = paper.margin
                    for i in sheet {
                        let b = blocks[i]
                        b.draw(CGRect(x: paper.margin, y: y, width: paper.size.width - 2 * paper.margin, height: b.height))
                        y += b.height + gap
                    }
                    let foot = text("стр. \(page.number) · лист \(sheetNumber)", font: rounded(8.5, .semibold), color: .muted)
                    foot.draw(at: CGPoint(x: paper.size.width - paper.margin - foot.size().width, y: paper.size.height - paper.margin + 4))
                }
            }
        }
    }

    /// Writes the PDF next to the other exports and returns it for the share sheet.
    static func export(book: Book, pages: [BookPage]? = nil, paper: Paper = .local) async -> URL? {
        await book.flush()
        let data = render(book: book, pages: pages, paper: paper)
        let name = pages.map { $0.count == 1 ? "Drussky page \($0[0].number).pdf" : "Drussky pages.pdf" } ?? "Drussky notebook.pdf"
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(name)
        do { try data.write(to: url, options: .atomic) } catch { book.report("pdf: \((error as NSError).localizedDescription)"); return nil }
        return url
    }

    // MARK: Blocks

    static func blocks(for page: BookPage, book: Book, width: CGFloat, originX: CGFloat) -> [Block] {
        var out: [Block] = [header(page, book: book, width: width, x: originX)]
        for i in page.exercises.indices {
            out.append(exercise(page, index: i, book: book, width: width, x: originX))
        }
        if let notes = page.notes, !notes.isEmpty { out.append(self.notes(notes, width: width, x: originX)) }
        out.append(footer(page, width: width, x: originX))
        return out
    }

    private static func header(_ page: BookPage, book: Book, width: CGFloat, x: CGFloat) -> Block {
        let date = text(PageView.russianDate(page.created), font: cursive(22), color: .ink)
        let kind = text(page.isReview ? "Повторение" : "Классная работа", font: cursive(18), color: .ink)
        let unit = text(page.unitTitle.uppercased(), font: rounded(8, .semibold), color: .accent, kern: 1.4)
        let title = text(page.title, font: serif(20, .bold), color: .text)
        let intro = page.intro.map { text($0, font: serif(10.5), color: .text, alpha: 0.82) }
        let heights = [max(height(date, width), height(kind, width)) + 4, height(unit, width) + 2, height(title, width) + 4, intro.map { height($0, width) + 4 } ?? 0]
        let h = heights.reduce(0, +) + 6
        return Block(kind: .header, height: h, ink: nil) { rect in
            var y = rect.minY
            date.draw(at: CGPoint(x: x, y: y))
            kind.draw(at: CGPoint(x: x + width - kind.size().width, y: y + 4))
            y += heights[0]
            unit.draw(in: CGRect(x: x, y: y, width: width, height: heights[1]))
            y += heights[1]
            title.draw(in: CGRect(x: x, y: y, width: width, height: heights[2]))
            y += heights[2]
            if let intro { intro.draw(in: CGRect(x: x, y: y, width: width, height: heights[3])) }
        }
    }

    /// One exercise: its margin mark, the prompt, the learner's ink on ruled lines or their
    /// choice, the red correction, and a word on how it went.
    private static func exercise(_ page: BookPage, index i: Int, book: Book, width: CGFloat, x: CGFloat) -> Block {
        let ex = page.exercises[i], r = page.results[i]
        var lines: [(NSAttributedString, CGFloat)] = []
        func add(_ a: NSAttributedString, after: CGFloat = 2) { lines.append((a, height(a, width) + after)) }

        add(text(prompt(ex), font: rounded(9.5, .semibold), color: .muted))
        switch ex.kind {
        case .explain:
            add(text(ex.text ?? "", font: serif(10.5), color: .text), after: 4)
        case .passage, .familyGuess:
            add(text(ex.text ?? "", font: serif(12.5), color: .text), after: 4)
            if ex.kind == .familyGuess { add(choices(ex.options ?? [], chosen: r.chosen, correct: ex.answerIndex, russian: false)) }
        case .translate:
            add(text(ex.en ?? "", font: serif(15, .semibold), color: .text))
        case .fill:
            add(text("\(ex.before ?? "") ____ \(ex.after ?? "")", font: serif(14, .medium), color: .text))
            if let hint = ex.hint { add(text(hint, font: serif(10, .regular, italic: true), color: .muted)) }
        case .copy, .trace:
            add(text(ex.text ?? "", font: serif(15, .medium), color: .text))
        case .listenPick, .soundPair:
            let options = ex.options ?? ex.examples?.map(\.ru) ?? []
            if !options.isEmpty {
                let right = ex.say.flatMap { s in options.firstIndex { RussianText.key($0) == RussianText.key(s) } } ?? ex.answerIndex
                add(choices(options, chosen: r.chosen, correct: right, russian: true))
            }
            if ex.kind == .soundPair, ex.say == nil, let note = ex.text { add(text(note, font: serif(10), color: .text)) }
        case .readMeaning:
            add(text(ex.text ?? "", font: serif(15, .medium), color: .text))
            add(choices(ex.options ?? [], chosen: r.chosen, correct: ex.answerIndex, russian: false))
        case .readAloud:
            add(text(ex.text ?? "", font: serif(15, .medium), color: .text))
            if let heard = r.recognized, !heard.isEmpty { add(text("heard: «\(heard)»", font: rounded(9), color: .muted)) }
        case .buildWord:
            add(text((ex.options ?? ex.parts ?? []).joined(separator: " · "), font: serif(13), color: .text))
            if let built = r.recognized { add(text(built, font: cursive(18), color: .ink)) }
        case .diary:
            if let prompt = ex.prompt { add(text(prompt, font: serif(12, .semibold), color: .text), after: 4) }
        case .talk:
            add(text(TalkConversation.describe(ex, r), font: serif(11), color: .text), after: 4)
        case .paradigmFill:
            add(text("«\(ex.word ?? "")» · \((ex.cells ?? []).joined(separator: ", "))", font: serif(13), color: .text))
        case .transform:
            add(text("\(ex.before ?? "")____\(ex.after ?? "")  ·  «\(ex.word ?? "")» → \(ex.cue ?? "")", font: serif(14, .medium), color: .text))
            if let hint = ex.hint { add(text(hint, font: serif(10, .regular, italic: true), color: .muted)) }
        case .cueSort:
            add(text((ex.items ?? []).map { "«\($0.text)»" }.joined(separator: "  "), font: serif(12.5), color: .text))
            add(text((ex.options ?? []).joined(separator: " / "), font: serif(10), color: .muted))
        case .readHandwriting:  // the hand it was shown in is not on paper; the word is, in cursive
            add(text(RussianText.unstressed(ex.text ?? ""), font: cursive(18), color: .ink))
            if let options = ex.options {
                add(choices(options, chosen: r.chosen, correct: options.firstIndex { RussianText.key($0) == RussianText.key(ex.text ?? "") }, russian: true))
            }
        case .meet, .dictation:
            break
        }

        // The ink, on the strip's own ruled lines, scaled so everything written is on the sheet.
        let drawing = inkDrawing(book.inkURL(page: page, exercise: i))
        var printedInk: UIImage?
        var strip: CGRect?
        let stripHeight: CGFloat = ex.kind == .diary ? 110 : 60
        if ex.kind.isWriting || ex.kind == .diary {
            if let drawing {
                printedInk = inkImage(drawing, width: width, height: stripHeight, ruled: ex.kind != .diary)
            }
            strip = CGRect(x: 0, y: 0, width: width, height: stripHeight)
        }
        if ex.kind == .diary, let d = r.diary {
            add(text(d.text, font: serif(12), color: .text), after: 3)
            if let c = d.correction {
                if !c.edits.isEmpty { add(text(c.corrected, font: cursive(17), color: .redPen)) }
                for e in c.edits.prefix(6) { add(text("\(e.span) → \(RussianText.unstressed(e.replacement))  (\(e.category.title))", font: rounded(9), color: .redPen)) }
                add(text(c.praise, font: cursive(14), color: .redPen), after: 4)
            }
        }
        if let typed = r.recognized, r.typed == true, ex.kind.isWriting { add(text("typed: \(typed)", font: rounded(9.5), color: .text)) }
        if let line = outcome(ex, r) { add(text(line.0, font: rounded(9), color: line.1)) }

        // The diary's prompt sits above its lines; what was written and the red pen come after.
        let beforeCount = ex.kind == .diary ? (ex.prompt == nil ? 1 : 2) : lines.count
        let before = Array(lines.prefix(beforeCount))
        let after = Array(lines.dropFirst(beforeCount))
        let stripHeightUsed: CGFloat = strip == nil ? 0 : stripHeight + 6
        let textHeight: CGFloat = lines.map(\.1).reduce(0, +)
        let h: CGFloat = textHeight + stripHeightUsed + 6
        var correction: NSAttributedString?
        if r.revealed || r.status == .missed, let target = ex.target, ex.kind.isWriting {
            correction = text(RussianText.unstressed(target), font: cursive(19), color: .redPen)
        }
        let status = r.status
        return Block(kind: .exercise(i), height: h, ink: printedInk) { rect in
            drawMark(number: i + 1, status: status, isReview: ex.reviewKey != nil, at: CGPoint(x: rect.minX + 6, y: rect.minY + 1))
            var y = rect.minY
            for (a, lh) in before { a.draw(in: CGRect(x: x, y: y, width: width, height: lh)); y += lh }
            if let strip {
                let frame = CGRect(x: x, y: y + 2, width: strip.width, height: strip.height)
                if ex.kind != .diary { drawRules(frame) } else { drawDiaryLines(frame) }
                printedInk?.draw(in: frame)
                if let correction {
                    let s = correction.size()
                    correction.draw(at: CGPoint(x: frame.maxX - s.width - 8, y: frame.minY - s.height * 0.35))
                }
                y += strip.height + 6
            }
            for (a, lh) in after { a.draw(in: CGRect(x: x, y: y, width: width, height: lh)); y += lh }
        }
    }

    private static func notes(_ notes: [PageNote], width: CGFloat, x: CGFloat) -> Block {
        var lines: [(NSAttributedString, CGFloat)] = [(text("Вопросы", font: cursive(16), color: .ink), 22)]
        for note in notes {
            for turn in note.turns {
                let body = String(turn.text.prefix(700))
                let a = text((turn.role == "user" ? "— " : "   ") + body, font: serif(9.5, turn.role == "user" ? .semibold : .regular), color: .text)
                lines.append((a, height(a, width) + 3))
            }
        }
        return Block(kind: .notes, height: lines.map(\.1).reduce(0, +) + 4, ink: nil) { rect in
            var y = rect.minY
            for (a, h) in lines { a.draw(in: CGRect(x: x, y: y, width: width, height: h)); y += h }
        }
    }

    private static func footer(_ page: BookPage, width: CGFloat, x: CGFloat) -> Block {
        guard page.isFinished else {
            let a = text("\(page.completedCount) of \(page.exercises.count) done", font: rounded(9), color: .muted)
            return Block(kind: .footer, height: 16, ink: nil) { rect in a.draw(at: CGPoint(x: x, y: rect.minY)) }
        }
        let grade = text(GradeStamp.grade(for: page).map { "\($0)" } ?? "", font: cursive(54), color: .redPen)
        let praise = text(GradeStamp.praise(for: page), font: cursive(22), color: .redPen)
        var parts = ["\(page.exercises.count) exercises"]
        let firstTry = page.results.filter(\.isRightFirstTime).count
        let missed = page.results.filter { $0.status == .missed }.count
        if firstTry > 0 { parts.append("\(firstTry) right first time") }
        if missed > 0 { parts.append("\(missed) to bring back soon") }
        let summary = text(parts.joined(separator: " · "), font: rounded(9), color: .muted)
        return Block(kind: .footer, height: 70, ink: nil) { rect in
            let ctx = UIGraphicsGetCurrentContext()
            ctx?.saveGState()
            ctx?.translateBy(x: x + 24, y: rect.minY + 34)
            ctx?.rotate(by: -10 * .pi / 180)
            grade.draw(at: CGPoint(x: -grade.size().width / 2, y: -grade.size().height / 2))
            ctx?.restoreGState()
            praise.draw(at: CGPoint(x: x + 62, y: rect.minY + 10))
            summary.draw(at: CGPoint(x: x + 62, y: rect.minY + 40))
        }
    }

    // MARK: Ink

    private static func inkDrawing(_ url: URL) -> PKDrawing? {
        guard let data = try? Data(contentsOf: url), let d = try? PKDrawing(data: data), !d.strokes.isEmpty else { return nil }
        return d
    }

    /// Ink is stored in the strip's design space (132 pt tall, as wide as the strip was); it is
    /// zoomed to the printed strip's height, or less when what was written runs past its width.
    static func inkImage(_ drawing: PKDrawing, width: CGFloat, height: CGFloat, ruled: Bool) -> UIImage {
        let bounds = drawing.bounds
        if ruled {
            let zoom = min(height / StripMetrics.designHeight, width / max(bounds.maxX + 16, 1))
            let visible = CGRect(x: 0, y: 0, width: width / zoom, height: height / zoom)
            return drawing.image(from: visible, scale: 2.5)
        }
        // Free lines (the diary): the ink's own bounds, fitted.
        let box = bounds.insetBy(dx: -10, dy: -10)
        let scale = min(width / max(box.width, 1), height / max(box.height, 1), 1)
        let fitted = CGRect(x: box.minX, y: box.minY, width: width / scale, height: height / scale)
        return drawing.image(from: fitted, scale: 2.5)
    }

    // MARK: Paper and marks

    private static func drawPaper(_ ctx: CGContext, paper: Paper, ruleX: CGFloat) {
        ctx.setFillColor(UIColor(hex: 0xFBF8F0).cgColor)
        ctx.fill(CGRect(origin: .zero, size: paper.size))
        ctx.setStrokeColor(UIColor(hex: 0x6F93C8, alpha: 0.22).cgColor)
        ctx.setLineWidth(0.4)
        let step: CGFloat = 17.6
        var x: CGFloat = 0
        while x <= paper.size.width { ctx.move(to: CGPoint(x: x, y: 0)); ctx.addLine(to: CGPoint(x: x, y: paper.size.height)); x += step }
        var y: CGFloat = 0
        while y <= paper.size.height { ctx.move(to: CGPoint(x: 0, y: y)); ctx.addLine(to: CGPoint(x: paper.size.width, y: y)); y += step }
        ctx.strokePath()
        ctx.setStrokeColor(UIColor(hex: 0xD8524A, alpha: 0.6).cgColor)
        ctx.setLineWidth(1)
        ctx.move(to: CGPoint(x: ruleX, y: 0)); ctx.addLine(to: CGPoint(x: ruleX, y: paper.size.height))
        ctx.strokePath()
    }

    private static func drawRules(_ frame: CGRect) {
        guard let ctx = UIGraphicsGetCurrentContext() else { return }
        let m = StripMetrics(height: frame.height)
        func line(_ y: CGFloat, width: CGFloat, alpha: CGFloat, dash: [CGFloat]? = nil) {
            ctx.saveGState()
            ctx.setStrokeColor(UIColor(hex: 0x3D5F94, alpha: 0.45 * alpha).cgColor)
            ctx.setLineWidth(width)
            if let dash { ctx.setLineDash(phase: 0, lengths: dash) }
            ctx.move(to: CGPoint(x: frame.minX, y: frame.minY + y)); ctx.addLine(to: CGPoint(x: frame.maxX, y: frame.minY + y))
            ctx.strokePath()
            ctx.restoreGState()
        }
        line(m.capY, width: 0.5, alpha: 0.55)
        line(m.xHeightY(cursive: false), width: 0.5, alpha: 0.6, dash: [3, 3])
        line(m.baselineY, width: 0.9, alpha: 1)
        line(m.descenderY, width: 0.4, alpha: 0.35)
    }

    private static func drawDiaryLines(_ frame: CGRect) {
        guard let ctx = UIGraphicsGetCurrentContext() else { return }
        ctx.saveGState()
        ctx.setStrokeColor(UIColor(hex: 0x3D5F94, alpha: 0.4).cgColor)
        ctx.setLineWidth(0.7)
        for k in 1...3 {
            let y = frame.minY + frame.height * CGFloat(k) / 3 - 4
            ctx.move(to: CGPoint(x: frame.minX, y: y)); ctx.addLine(to: CGPoint(x: frame.maxX, y: y))
        }
        ctx.strokePath()
        ctx.restoreGState()
    }

    private static func drawMark(number: Int, status: ExerciseResult.Status, isReview: Bool, at p: CGPoint) {
        let a: NSAttributedString
        switch status {
        case .correct, .done: a = text("✓", font: rounded(15, .heavy), color: .redPen)
        case .missed: a = text("\(number)", font: rounded(11, .bold), color: .redPen)
        case .skipped: a = text("–", font: rounded(13, .bold), color: .muted)
        case .open: a = text("\(number)", font: rounded(11, .bold), color: .muted)
        }
        a.draw(at: p)
        if status == .missed, let ctx = UIGraphicsGetCurrentContext() {
            ctx.setStrokeColor(Ink.redPen.cgColor); ctx.setLineWidth(1)
            ctx.strokeEllipse(in: CGRect(x: p.x - 5, y: p.y - 2, width: a.size().width + 10, height: a.size().height + 4))
        }
        if isReview { text("↺", font: rounded(8), color: .muted).draw(at: CGPoint(x: p.x + 14, y: p.y + 3)) }
    }

    // MARK: Words

    /// The instruction a page printed later still needs, in one line.
    static func prompt(_ ex: Exercise) -> String {
        switch ex.kind {
        case .meet: "Meet the letter \(ex.letter ?? ""): trace it, then write it"
        case .explain: "A note"
        case .trace: "Trace"
        case .copy: (ex.times ?? 1) > 1 ? "Copy it \(ex.times!) times" : "Copy"
        case .listenPick: "Listen and pick · «\(ex.say ?? "")»"
        case .dictation: "Dictation · «\(ex.text ?? "")»"
        case .readAloud: "Read aloud"
        case .translate: "Write it in Russian"
        case .fill: "Write the missing word"
        case .readMeaning: "What does it mean?"
        case .passage: "Read"
        case .soundPair: ex.say != nil ? "Sound pair · «\(ex.say ?? "")»" : "Sound pair"
        case .diary: "Дневни́к"
        case .buildWord: "Build «\(ex.answerText ?? "")» (\(ex.en ?? ""))"
        case .familyGuess: "Guess «\(ex.word ?? "")» from its root \(ex.root ?? "")"
        case .talk: "Разгово́р · Talk"
        case .paradigmFill: "Fill the table"
        case .transform: "Write the form"
        case .cueSort: "Sort by pattern"
        case .readHandwriting: ex.options != nil ? "Read the handwriting and pick the word" : "Read the handwriting and print it"
        }
    }

    private static func outcome(_ ex: Exercise, _ r: ExerciseResult) -> (String, Ink)? {
        switch r.status {
        case .correct: return ([.dictation, .translate, .fill].contains(ex.kind) ? (r.isRightFirstTime ? ("Right first time.", .good) : ("Got it.", .good)) : nil)
        case .missed: return ("Shown the answer; it comes back soon.", .redPen)
        case .skipped: return ("Skipped for now.", .muted)
        case .done where r.selfAssessed == true: return ("Marked right by you.", .muted)
        default: return nil
        }
    }

    /// Choices as one line: the picked one marked ✓ or ✗, the right one marked when it wasn't picked.
    private static func choices(_ options: [String], chosen: Int?, correct: Int?, russian: Bool) -> NSAttributedString {
        let out = NSMutableAttributedString()
        for (i, o) in options.enumerated() {
            if i > 0 { out.append(text("    ", font: serif(11), color: .muted)) }
            let mark = chosen == i ? (correct == i ? "✓ " : "✗ ") : (correct == i && chosen != nil ? "○ " : "")
            let color: Ink = chosen == i ? (correct == i ? .good : .redPen) : (correct == i && chosen != nil ? .good : .text)
            out.append(text(mark + o, font: russian ? serif(12.5, .medium) : serif(11), color: color))
        }
        return out
    }

    // MARK: Type

    enum Ink { case ink, text, muted, accent, redPen, good
        var ui: UIColor {
            switch self {
            case .ink: UIColor(hex: 0x1B2B4B)
            case .text: UIColor(hex: 0x222831)
            case .muted: UIColor(hex: 0x6A6F78)
            case .accent: UIColor(hex: 0x2B4C7E)
            case .redPen: UIColor(hex: 0xC8372D)
            case .good: UIColor(hex: 0x2E7D4F)
            }
        }
        var cgColor: CGColor { ui.cgColor }
    }

    static func serif(_ size: CGFloat, _ weight: UIFont.Weight = .regular, italic: Bool = false) -> UIFont {
        let base = UIFont.systemFont(ofSize: size, weight: weight)
        var d = base.fontDescriptor.withDesign(.serif) ?? base.fontDescriptor
        if italic { d = d.withSymbolicTraits(.traitItalic) ?? d }
        return UIFont(descriptor: d, size: size)
    }
    static func rounded(_ size: CGFloat, _ weight: UIFont.Weight = .regular) -> UIFont {
        let base = UIFont.systemFont(ofSize: size, weight: weight)
        return base.fontDescriptor.withDesign(.rounded).map { UIFont(descriptor: $0, size: size) } ?? base
    }
    static func cursive(_ size: CGFloat) -> UIFont {
        UIFont(name: "MarckScript-Regular", size: size) ?? serif(size, .regular, italic: true)
    }

    private static func text(_ s: String, font: UIFont, color: Ink, alpha: CGFloat = 1, kern: CGFloat = 0) -> NSAttributedString {
        let p = NSMutableParagraphStyle()
        p.lineBreakMode = .byWordWrapping
        p.lineSpacing = 1.5
        var attrs: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: color.ui.withAlphaComponent(alpha), .paragraphStyle: p]
        if kern != 0 { attrs[.kern] = kern }
        return NSAttributedString(string: s, attributes: attrs)
    }

    private static func height(_ a: NSAttributedString, _ width: CGFloat) -> CGFloat {
        ceil(a.boundingRect(with: CGSize(width: width, height: .greatestFiniteMagnitude), options: [.usesLineFragmentOrigin, .usesFontLeading], context: nil).height)
    }
}
