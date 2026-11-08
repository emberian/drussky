import CoreGraphics
import LearningCore
import PencilKit
import XCTest
@testable import Drussky

/// Russian cursive: the stroke data and its joins, the written-through layout, the cursive coach,
/// the read_handwriting exercise (decoding, rendering, look-alikes, the page from known words),
/// and the bundled handwriting fonts and their licences (recognition on synthetic cursive:
/// DrusskySlowTests/InkReaderSlowTests).
@MainActor
final class CursiveTests: XCTestCase {
    private var m: CursiveModels!

    override func setUp() async throws {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "strokes-cursive", withExtension: "json"))
        m = try CursiveModels.decode(Data(contentsOf: url))
    }

    private func course() throws -> Course {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        return try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    // MARK: Data

    func testEveryFormPresentInsideTheBoxWithEntryAndExit() {
        XCTAssertEqual(m.letters.count, 66)
        XCTAssertEqual(m.models.letters.count, 66)
        let kinds: Set<String> = ["top", "mid", "base"]
        for ch in "АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ" {
            for g in [String(ch), String(ch).lowercased()] {
                guard let l = m.letter(g) else { XCTFail("no cursive model for \(g)"); continue }
                XCTAssertFalse(l.model.strokes.isEmpty, g)
                XCTAssertTrue(l.model.strokes.allSatisfy { $0.points.count >= 2 }, g)
                XCTAssertFalse(l.hint.isEmpty, g)
                XCTAssertTrue(kinds.contains(l.entryKind), "\(g) entry kind \(l.entryKind)")
                XCTAssertEqual(l.exit == nil, l.exitKind == nil, g)
                if let k = l.exitKind { XCTAssertTrue(kinds.contains(k), g) }
                for p in l.model.strokes.flatMap(\.points) + [l.entry] + (l.exit.map { [$0] } ?? []) {
                    XCTAssertTrue((0...StripMetrics.designHeight).contains(p.y), "\(g) y \(p.y)")
                    XCTAssertTrue((-30...(l.advance + 45)).contains(p.x), "\(g) x \(p.x) advance \(l.advance)")
                }
                XCTAssertGreaterThan(l.advance, 10, g)
            }
        }
        for a in kinds { for b in kinds { XCTAssertNotNil(m.joins["\(a)-\(b)"], "\(a)-\(b)") } }
        // The famous distinctions are in the data: л/м start low (the hook), и/ш/т start at the top; т has humps (rounded at the top), ш cups (rounded at the bottom).
        for g in ["л", "м", "я"] { XCTAssertEqual(m.letter(g)!.entryKind, "base", g) }
        for g in ["и", "ш", "т", "п", "н"] { XCTAssertEqual(m.letter(g)!.entryKind, "top", g) }
        let t = m.letter("т")!.model.strokes[0].points, sh = m.letter("ш")!.model.strokes[0].points
        XCTAssertEqual(t.count(where: { $0.y < m.lines.xHeight + 6 }) > sh.count(where: { $0.y < m.lines.xHeight + 6 }), true, "т spends more of its stroke along the top than ш")
        XCTAssertGreaterThan(m.letter("д")!.model.bounds.maxY, m.lines.baseline + 10, "д loops below the line")
        XCTAssertLessThan(m.letter("б")!.model.bounds.minY, m.lines.xHeight - 15, "б rises above the small letters")
        XCTAssertEqual(m.letter("т")!.model.strokes.count, 1, "т is one stroke")
    }

    func testJoinsCoverEveryPairInTheCursiveUnit() throws {
        let course = try course()
        let unit = try XCTUnwrap(course.units.first { $0.id == "u31" })
        XCTAssertTrue((6...8).contains(unit.pages.count), "\(unit.pages.count) pages")
        var texts: [String] = []
        for p in unit.pages {
            for e in p.exercises {
                if e.kind == .readHandwriting || e.cursive == true { texts.append(try XCTUnwrap(e.text, "\(p.id)")) }
            }
        }
        XCTAssertGreaterThan(texts.count, 20)
        var pairs = 0, joins = 0
        for text in texts {
            for item in CursiveLayout.place(text, in: m).items { if case .join = item { joins += 1 } }
            // Every word of the unit is joined through: a join model for every pair of adjacent
            // letters whose first joins onward, and the layout draws exactly those.
            let chars = Array(RussianText.unstressed(text))
            for (a, b) in zip(chars, chars.dropFirst()) {
                guard let la = m.letter(String(a)), let lb = m.letter(String(b)), la.exit != nil else { continue }
                pairs += 1
                XCTAssertNotNil(m.join(from: la, to: lb), "\(a)\(b) in \(text)")
            }
        }
        XCTAssertEqual(pairs, joins)
        XCTAssertGreaterThan(joins, 40)
    }

    func testLayoutAdvancesAndJoinsMeetTheLetters() {
        let placement = CursiveLayout.place("молоко", in: m, x0: 16)
        var lastX: CGFloat = -1
        var pendingJoin: [CGPoint]?
        var prevExit: CGPoint?
        var letters = 0
        for item in placement.items {
            switch item {
            case .join(let pts):
                XCTAssertGreaterThanOrEqual(pts.count, 2)
                XCTAssertEqual(pts[0], prevExit!)
                pendingJoin = pts
            case .letter(let l, let x):
                letters += 1
                XCTAssertGreaterThan(x, lastX)
                lastX = x
                if let j = pendingJoin {
                    XCTAssertEqual(j.last!.x, x + l.entry.x, accuracy: 0.01)
                    XCTAssertEqual(j.last!.y, l.entry.y, accuracy: 0.01)
                }
                pendingJoin = nil
                prevExit = l.exit.map { CGPoint(x: x + $0.x, y: $0.y) }
            }
        }
        XCTAssertEqual(letters, 6)
        XCTAssertEqual(placement.width, "молоко".map { m.letter(String($0))!.advance }.reduce(0, +))
        // A space breaks the join; a capital that does not join onward leaves a gap.
        let two = CursiveLayout.place("Том и", in: m).items
        XCTAssertEqual(two.filter { if case .join = $0 { return true }; return false }.count, 1, "Т does not join onward, о→м does, the space breaks")
        XCTAssertEqual(CursiveLayout.strokes("э", in: m).count, 2)
        XCTAssertFalse(CursiveLayout.path("мама", in: m, origin: CGPoint(x: 10, y: 0), scale: 0.5).isEmpty)
    }

    func testGuideLinesAndGhostFitFollowTheCursiveModels() throws {
        try XCTSkipIf(CursiveModels.shared == nil, "cursive models not in the app bundle")
        let s = StripMetrics(height: StripMetrics.designHeight)
        XCTAssertEqual(s.xHeightY(cursive: true), m.lines.xHeight, accuracy: 0.6)
        XCTAssertEqual(s.xHeightY(cursive: false), StrokeModels.shared!.lines.xHeight, accuracy: 0.6, "print unchanged")
        XCTAssertEqual(m.letter("о")!.model.bounds.minY, m.lines.xHeight, accuracy: 2)
        let wide = StripMetrics.fitting("мама", times: 3, width: 1000, cursive: true, preferred: 112)
        let narrow = StripMetrics.fitting("мама", times: 3, width: 300, cursive: true, preferred: 112)
        XCTAssertEqual(wide, 112)
        XCTAssertTrue(narrow >= 72 && narrow < wide, "\(narrow)")
    }

    // MARK: Demo and coach

    func testCursiveDemoPlacesTheMeetGhost() throws {
        try XCTSkipIf(CursiveModels.shared == nil, "cursive models not in the app bundle")
        let letter = try XCTUnwrap(course().letters.first { $0.upper == "М" })
        let placed = LetterCoach(letter: letter, demo: StrokeDemo(), cursive: true).placed(ghost: "М м")
        XCTAssertEqual(placed.map(\.0.glyph), ["М", "м"])
        XCTAssertEqual(placed[0].1, 16)
        XCTAssertEqual(placed[1].1, 16 + m.letter("М")!.advance + m.space)
        let print = LetterCoach(letter: letter, demo: StrokeDemo()).placed(ghost: "М м")
        XCTAssertEqual(print.count, 2, "print layout untouched")
    }

    /// The model's strokes as a learner might write them: shifted, a little wobble.
    private func ink(_ glyph: String, at x: CGFloat = 30, seed: Double = 0) -> [[CGPoint]] {
        m.letter(glyph)!.model.strokes.enumerated().map { j, s in
            s.points.enumerated().map { i, p in
                CGPoint(x: x + p.x + sin(Double(i) * 0.9 + seed + Double(j)) * 0.8, y: p.y + cos(Double(i) * 0.7 + seed) * 0.8)
            }
        }
    }

    func testCoachPraisesTheModelAndSpeaksCursive() {
        for (glyph, _) in m.letters {
            let h = CursiveCoach.hint(for: ink(glyph), upper: glyph.uppercased(), lower: glyph.lowercased(), models: m)
            XCTAssertEqual(h, "That \(glyph) is written like the model.", glyph)
        }
        // м without its hook: the ink starts at the top of the rise.
        var noHook = ink("м")
        let xmid = (m.lines.xHeight + m.lines.baseline) / 2
        noHook[0] = Array(noHook[0].drop { $0.y > xmid })
        let hookHint = CursiveCoach.hint(for: noHook, upper: "М", lower: "м", models: m)
        XCTAssertEqual(hookHint, m.letter("м")!.hint)
        XCTAssertTrue(hookHint?.contains("hook") == true, hookHint ?? "nil")
        // т in three pen lifts instead of one stroke.
        let t = ink("т")[0]
        let third = t.count / 3
        let lifted = [Array(t[..<third]), Array(t[third..<(2 * third)]), Array(t[(2 * third)...])]
        let liftHint = CursiveCoach.hint(for: lifted, upper: "Т", lower: "т", models: m)
        XCTAssertTrue(liftHint?.hasPrefix("Cursive т is one stroke") == true, liftHint ?? "nil")
        // д with its loop cut off at the baseline.
        let d = ink("д").map { $0.map { CGPoint(x: $0.x, y: min($0.y, m.lines.baseline)) } }
        XCTAssertEqual(CursiveCoach.hint(for: d, upper: "Д", lower: "д", models: m), "The loop of д drops below the line.")
        // Floating letters get the shared hint; scribble gets nothing.
        let floating = ink("к").map { $0.map { CGPoint(x: $0.x, y: $0.y - 22) } }
        XCTAssertEqual(CursiveCoach.hint(for: floating, upper: "К", lower: "к", models: m), "Sit к on the baseline.")
        XCTAssertNil(CursiveCoach.hint(for: [(0..<30).map { CGPoint(x: 20 + CGFloat($0) * 3, y: $0 % 2 == 0 ? 30 : 90) }], upper: "Б", lower: "б", models: m))
    }

    // MARK: read_handwriting

    func testReadHandwritingDecodesRoundTripsAndIsItsOwnModality() throws {
        let json = """
        [{"kind":"read_handwriting","text":"ма́ма","options":["ла́ма","ма́ма","ма́ша"],"seed":12345},
         {"kind":"read_handwriting","text":"Кот до́ма."},
         {"kind":"trace","text":"сок","cursive":true},
         {"kind":"copy","text":"мир","times":2}]
        """
        let exs = try JSONDecoder().decode([Exercise].self, from: Data(json.utf8))
        XCTAssertEqual(exs[0].kind, .readHandwriting)
        XCTAssertEqual(exs[0].options?.count, 3)
        XCTAssertEqual(exs[0].seed, 12345)
        XCTAssertEqual(exs[0].target, "ма́ма")
        XCTAssertEqual(exs[0].spoken, "ма́ма")
        XCTAssertNil(exs[1].options)
        XCTAssertNil(exs[1].seed)
        XCTAssertEqual(exs[2].cursive, true)
        XCTAssertNil(exs[3].cursive)
        XCTAssertFalse(ExerciseKind.readHandwriting.isWriting)
        let round = try JSONDecoder().decode([Exercise].self, from: JSONEncoder().encode(exs))
        XCTAssertEqual(round, exs)
        XCTAssertEqual(RussianDomain.modality(for: .readHandwriting), .readCursive)
        XCTAssertEqual(Modality.readCursive.title, "read handwriting")
        var r = ExerciseResult(); r.status = .correct; r.attempts = 1
        XCTAssertEqual(RussianDomain.grade(exs[0], r), .good)
        r.attempts = 2; r.status = .done
        XCTAssertEqual(RussianDomain.grade(exs[0], r), .again)
        let book = Book(course: try course(), store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        XCTAssertTrue(book.knowledge.domain.modalities.contains(.readCursive))
        XCTAssertEqual(book.knowledge.domain.components(for: exs[0]).first, ComponentID(.lexeme, "мама"))
        let confusions = book.knowledge.domain.confusions(exs[0], response: "ма́ша")
        XCTAssertTrue(confusions.isEmpty, "an invented look-alike is no known word")
        let real = book.knowledge.domain.confusions(exs[1], response: "мама дома")
        XCTAssertEqual(real.count, 0, "a phrase that is not a lexicon entry has no single component to confuse")
        var w = Exercise(kind: .readHandwriting); w.text = "мир"
        XCTAssertEqual(book.knowledge.domain.confusions(w, response: "мы").map(\.channel), [.shape])
    }

    func testSampleRenderingIsDeterministicAndVaried() {
        let fonts = ["MarckScript-Regular", "Caveat-Regular", "BadScript-Regular"]
        let a = HandwritingSample.make(text: "молоко́", seed: 7, fonts: fonts, strokes: true)
        let b = HandwritingSample.make(text: "молоко́", seed: 7, fonts: fonts, strokes: true)
        XCTAssertEqual(a, b)
        XCTAssertEqual(a.text, "молоко", "stress marks are not written by hand")
        var styles = Set<String>(), papers = Set<Int>(), inks = Set<Int>()
        for seed in 0..<80 {
            let s = HandwritingSample.make(text: "окно", seed: UInt64(seed), fonts: fonts, strokes: true)
            switch s.style { case .font(let n): styles.insert(n); case .strokes: styles.insert("strokes") }
            papers.insert(s.paper.rawValue); inks.insert(s.ink.rawValue)
            XCTAssertTrue((0.8...1.16).contains(s.size) && (-0.1...0.33).contains(s.slant), "\(s)")
        }
        XCTAssertEqual(styles, Set(fonts + ["strokes"]), "every hand appears")
        XCTAssertEqual(papers.count, 3); XCTAssertEqual(inks.count, 3)
        XCTAssertEqual(HandwritingSample.seed(for: "ма́ма", index: 2), HandwritingSample.seed(for: "мама", index: 2), "stable across launches and stress marks")
        XCTAssertNotEqual(HandwritingSample.seed(for: "мама", index: 2), HandwritingSample.seed(for: "мама", index: 3))
        let signed = Exercise(kind: .readHandwriting)
        XCTAssertNil(signed.seed)
    }

    func testLookalikeDistractors() {
        XCTAssertTrue(CursiveTraps.lookalike("лишишься", "лишиться"), "ш→т, the лишишься problem")
        XCTAssertFalse(CursiveTraps.lookalike("кот", "код"), "т→д is not a look-alike swap")
        XCTAssertTrue(CursiveTraps.lookalike("мама", "лама"))
        XCTAssertTrue(CursiveTraps.lookalike("тут", "шут"))
        XCTAssertFalse(CursiveTraps.lookalike("кот", "кофе"))
        XCTAssertFalse(CursiveTraps.lookalike("мама", "мама"))
        var rng = SeededGenerator(seed: 3)
        let pool = ["ма́ма", "ла́ма", "па́па", "мир", "дом", "том"]
        let d = CursiveTraps.distractors(for: "ма́ма", pool: pool, count: 2, rng: &rng)
        XCTAssertEqual(d.count, 2)
        XCTAssertEqual(d.first, "ла́ма", "a real look-alike from the pool comes first")
        XCTAssertEqual(Set(d.map(RussianText.key)).count, 2)
        for x in d { XCTAssertNotEqual(RussianText.key(x), "мама"); XCTAssertTrue(CursiveTraps.lookalike(x, "мама"), x) }
        var rng2 = SeededGenerator(seed: 3)
        XCTAssertEqual(CursiveTraps.distractors(for: "ма́ма", pool: pool, count: 2, rng: &rng2), d, "deterministic")
        var rng3 = SeededGenerator(seed: 9)
        let invented = CursiveTraps.distractors(for: "окно́", pool: [], count: 2, rng: &rng3)
        XCTAssertEqual(invented.count, 2)
        for x in invented { XCTAssertTrue(CursiveTraps.lookalike(x, "окно"), x); XCTAssertEqual(x.count, "окно́".count, "the stress mark stays on a vowel: \(x)") }
        var rng4 = SeededGenerator(seed: 1)
        XCTAssertTrue(CursiveTraps.distractors(for: "Я", pool: [], count: 2, rng: &rng4).allSatisfy { $0.first!.isUppercase })
    }

    func testHandwritingPageIsBuiltFromKnownWords() throws {
        let book = Book(course: try course(), store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        XCTAssertNotNil(CursivePages.addHandwriting(book), "nothing known yet: a notice, no page")
        for p in 0..<14 { QA.complete(book, page: p, upTo: .max); book.turnForward() }
        let pages = book.pages.count
        let exs = book.readHandwritingExercises(seed: 5)
        XCTAssertGreaterThanOrEqual(exs.count, 7)
        XCTAssertEqual(exs[0].kind, .explain)
        let hw = exs.dropFirst().filter { $0.kind == .readHandwriting }
        XCTAssertEqual(hw.count, exs.count - 1)
        XCTAssertTrue(hw.contains { $0.options != nil } && hw.contains { $0.options == nil }, "both picking and printing")
        let known = Set(book.handwritingWords().map(RussianText.key))
        for e in hw {
            let text = try XCTUnwrap(e.text)
            XCTAssertNotNil(e.seed, text)
            if text.contains(" ") { XCTAssertNotNil(book.targets["word:\(RussianText.key(text))"], text) } else { XCTAssertTrue(known.contains(RussianText.key(text)), text) }
            if let options = e.options {
                XCTAssertEqual(options.count, 3)
                XCTAssertTrue(options.contains { RussianText.key($0) == RussianText.key(text) })
            }
        }
        XCTAssertEqual(book.readHandwritingExercises(seed: 5).map(\.text), exs.map(\.text), "seeded: the same page")
        XCTAssertNil(CursivePages.addHandwriting(book))
        XCTAssertEqual(book.pages.count, pages + 1)
        XCTAssertEqual(book.page?.title, "Read handwriting")
        QA.complete(book, page: book.currentPage, upTo: .max)
        XCTAssertTrue(book.page!.isFinished, "QA completes a handwriting page")
    }

    // MARK: Reviews

    /// Read-cursive reviews: none before the cursive unit's first page is finished; then never-read
    /// words come after everything due, two at a time, picked among look-alikes until read twice,
    /// printed back after; once answered, FSRS schedules them like any modality.
    func testReadCursiveReviewsEnterTheWeaveAfterTheUnitOpens() throws {
        let book = Book(course: try course(), store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        XCTAssertFalse(book.cursiveUnlocked)
        var guardCount = 0
        while book.page?.sourceID != "\(Book.cursiveUnitID)-p1" && guardCount < 300 {
            QA.complete(book, page: book.currentPage, upTo: .max); book.turnForward(); guardCount += 1
        }
        XCTAssertEqual(book.page?.sourceID, "u31-p1")
        XCTAssertFalse(book.cursiveUnlocked, "the first cursive page is open, not finished")
        XCTAssertTrue(book.dueReviews().allSatisfy { $0.modality != .readCursive })
        XCTAssertFalse(book.reviewModalities(for: "word:мама").contains(.readCursive))

        // The cursive pages' traces and copies are there to show the hand: fading never touches them.
        var ctr = Exercise(kind: .trace); ctr.text = "сок"; ctr.cursive = true
        var ccp = Exercise(kind: .copy); ccp.text = "сестра́"; ccp.times = 1; ccp.cursive = true
        var plain = Exercise(kind: .trace); plain.text = "сок"
        XCTAssertEqual(book.faded(ctr), ctr)
        XCTAssertEqual(book.faded(ccp), ccp)
        XCTAssertEqual(book.faded(plain).kind, .copy, "a secure word's print trace still fades")

        QA.complete(book, page: book.currentPage, upTo: .max)
        XCTAssertTrue(book.cursiveUnlocked)
        XCTAssertTrue(book.reviewModalities(for: "word:мама").contains(.readCursive))
        XCTAssertFalse(book.reviewModalities(for: "word:у меня есть").contains(.readCursive), "single words only")
        let due = book.dueReviews()
        let seeds = due.filter { $0.modality == .readCursive }
        XCTAssertEqual(seeds.count, Book.cursiveSeedsPerWeave)
        XCTAssertEqual(due.suffix(seeds.count).map(\.id), seeds.map(\.id), "after everything that is due")
        XCTAssertTrue(seeds.allSatisfy { s in !due.contains { d in d.id == s.id && d.modality != .readCursive } }, "not a word already due another way")
        let first = seeds[0]
        var ex = book.reviewExercise(for: first.id, modality: .readCursive, variant: 3)
        XCTAssertEqual(ex.kind, .readHandwriting)
        XCTAssertEqual(ex.reviewKey, first.id)
        XCTAssertEqual(ex.options?.count, 3, "picked among look-alikes first")
        XCTAssertNotNil(ex.seed)
        XCTAssertEqual(book.reviewExercise(for: first.id, modality: .readCursive, variant: 3), ex, "a page re-opens unchanged")

        // Two readings later it is printed back, and the word is no longer a seed: FSRS has it.
        book.turnForward()
        let page = book.currentPage
        let woven = try XCTUnwrap(book.pages[page].exercises.firstIndex { $0.reviewKey == first.id && $0.kind == .readHandwriting }, "seed woven into the next page")
        book.record(page: page, exercise: woven) { $0.chosen = book.pages[page].exercises[woven].options?.firstIndex { RussianText.key($0) == RussianText.key(ex.text!) }; $0.attempts = 1; $0.status = .correct }
        XCTAssertEqual(book.cursiveReadings(of: first.id), 1)
        XCTAssertFalse(book.dueReviews().contains { $0.id == first.id && $0.modality == .readCursive && $0.priority < 0.2 }, "no longer seeded")
        let second = book.reviewExercise(for: first.id, modality: .readCursive, variant: 4)
        XCTAssertNotNil(second.options, "one reading: still picking")
        book.insertWrittenPage(title: "again", intro: nil, unitTitle: "test", exercises: [second], newWords: [])
        book.record(page: book.currentPage, exercise: 0) { $0.chosen = second.options?.firstIndex { RussianText.key($0) == RussianText.key(second.text!) }; $0.attempts = 1; $0.status = .correct }
        XCTAssertEqual(book.cursiveReadings(of: first.id), 2)
        ex = book.reviewExercise(for: first.id, modality: .readCursive, variant: 5)
        XCTAssertNil(ex.options, "read twice: now print it back")
        XCTAssertEqual(ex.kind, .readHandwriting)
    }

    // MARK: Fonts

    func testFontManifestAndLicences() throws {
        CursiveFonts.register()
        let manifest = CursiveFonts.manifest()
        XCTAssertEqual(manifest.count, 5)
        XCTAssertEqual(manifest.first?.postscript, "MarckScript-Regular")
        for e in manifest {
            XCTAssertEqual(e.licence, "OFL-1.1", e.file)
            let name = (e.file as NSString).deletingPathExtension, ext = (e.file as NSString).pathExtension
            XCTAssertNotNil(Bundle.main.url(forResource: name, withExtension: ext), "\(e.file) in the bundle")
            let lic = try XCTUnwrap(Bundle.main.url(forResource: (e.licenceFile as NSString).deletingPathExtension, withExtension: "txt"), e.licenceFile)
            XCTAssertTrue(try String(contentsOf: lic, encoding: .utf8).contains("SIL Open Font License"), e.licenceFile)
            XCTAssertNotNil(UIFont(name: e.postscript, size: 20), "\(e.postscript) loads")
            XCTAssertTrue(CursiveFonts.hasCyrillic(e.postscript), "\(e.family) has Cyrillic")
        }
        XCTAssertEqual(CursiveFonts.names.count, 5)
    }
}
