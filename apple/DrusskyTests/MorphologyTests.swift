import LearningCore
import XCTest
@testable import Drussky

/// Roots and word families: the data, the two exercise kinds, root evidence, the build_word
/// checker, decoding help, and the pages built from what the learner knows.
@MainActor
final class MorphologyTests: XCTestCase {
    var course: Course {
        let url = Bundle.main.url(forResource: "course", withExtension: "json")!
        return try! JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    func tempStore() -> BookStore {
        BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
    }

    func testMorphologyLoadsAndCoversTheLexicon() {
        let m = Morphology.load()
        XCTAssertGreaterThanOrEqual(m.roots.count, 120)
        XCTAssertGreaterThanOrEqual(m.data.prefixes.count, 25)
        XCTAssertGreaterThanOrEqual(m.cognates.count, 100)
        let lemmas = Lemmas.load()
        var missing: [String] = []
        let cognates = Set(m.cognates.map { RussianText.key($0.ru) })
        for w in course.lexicon {
            for t in RussianText.key(w.ru).split(separator: " ").map(String.init) {
                let lemma = lemmas.lemma(t) ?? t
                if m.entry(word: lemma) == nil && m.entry(word: t) == nil && !cognates.contains(t) && !(m.data.names ?? []).contains(t) { missing.append(t) }
            }
        }
        XCTAssertTrue(missing.isEmpty, "lexicon words without a morphology entry: \(missing)")
        // Families share their root: every member has a morpheme that is one of the root's forms.
        for (key, e) in m.lemmas {
            guard let fam = e.family else { continue }
            let root = m.root(fam)
            XCTAssertNotNil(root, key)
            XCTAssertNotNil(m.rootIndex(of: e), "\(key) has no morpheme of root \(fam)")
        }
        let e = m.entry(word: "переписать")!
        XCTAssertEqual(e.roots, ["пис"])
        XCTAssertEqual(e.prefixes, ["пере"])
        XCTAssertEqual(m.tiles(for: e), ["пере-", "пис", "-а́ть"])
        XCTAssertEqual(m.entry(word: "книгу")?.key, "книга", "an inflected form maps to its lemma")
        XCTAssertTrue(m.members(of: "ход").contains("выход"))
    }

    func testNewKindsDecodeAndRoundTrip() throws {
        let json = """
        [{"kind":"build_word","en":"a writer","answer":"писа́тель","parts":["пис","-а́-","-тель"],"options":["-тель","пере-","пис","-а́-"],"alternatives":["чита́тель"]},
         {"kind":"family_guess","text":"Где вход в парк?","word":"вход","root":"ход","options":["exit","entrance","crossing"],"answer":1,"en":"Where is the entrance to the park?"}]
        """
        let exs = try JSONDecoder().decode([Exercise].self, from: Data(json.utf8))
        XCTAssertEqual(exs[0].kind, .buildWord)
        XCTAssertEqual(exs[0].parts, ["пис", "-а́-", "-тель"])
        XCTAssertEqual(exs[0].options?.count, 4)
        XCTAssertEqual(exs[0].alternatives, ["чита́тель"])
        XCTAssertEqual(exs[0].target, "писа́тель")
        XCTAssertEqual(exs[0].spoken, "писа́тель")
        XCTAssertEqual(exs[1].kind, .familyGuess)
        XCTAssertEqual(exs[1].word, "вход")
        XCTAssertEqual(exs[1].root, "ход")
        XCTAssertEqual(exs[1].answerIndex, 1)
        XCTAssertEqual(exs[1].spoken, "Где вход в парк?")
        let again = try JSONDecoder().decode([Exercise].self, from: JSONEncoder().encode(exs))
        XCTAssertEqual(again, exs)
        // The course tail carries both kinds and they decode with the rest.
        let kinds = Set(course.pageSequence.flatMap { $0.page.exercises.map(\.kind) })
        XCTAssertTrue(kinds.contains(.buildWord) && kinds.contains(.familyGuess))
    }

    func testBuildWordChecker() {
        XCTAssertTrue(BuildWordCheck.matches(["пере-", "пис", "-а́ть"], answer: "переписа́ть"))
        XCTAssertTrue(BuildWordCheck.matches(["пере-", "пис", "-ать"], answer: "переписа́ть"), "stress marks never count")
        XCTAssertFalse(BuildWordCheck.matches(["пис", "пере-", "-а́ть"], answer: "переписа́ть"), "order matters")
        XCTAssertFalse(BuildWordCheck.matches(["пере-", "пис"], answer: "переписа́ть"), "every part is needed")
        XCTAssertFalse(BuildWordCheck.matches([], answer: "переписа́ть"))
        XCTAssertTrue(BuildWordCheck.matches(["про-", "чит", "-а́ть"], answer: "дочита́ть", alternatives: ["прочита́ть"]), "an accepted alternative")
        XCTAssertTrue(BuildWordCheck.canSpell(["-тель", "пис", "-а́-"], answer: "писа́тель"))
        XCTAssertFalse(BuildWordCheck.canSpell(["-тель", "пис"], answer: "писа́тель"))
        XCTAssertEqual(BuildWordCheck.assemble(["фило-", "-со́фия"]), "философия")
    }

    func testRootsAndAffixesAreComponentsWithEdges() {
        let m = Morphology.load()
        let d = RussianDomain(course: course, lemmas: Lemmas.load(), pairs: SoundPairs.load(), morphology: m)
        let g = d.graph
        XCTAssertNotNil(g[ComponentID(.root, "пис")])
        XCTAssertEqual(g[ComponentID(.root, "пис")]?.gloss, "write")
        XCTAssertNotNil(g[ComponentID(.affix, "пере-")])
        XCTAssertNotNil(g[ComponentID(.affix, "-тель")])
        XCTAssertEqual(g.edges(from: ComponentID(.lexeme, "писать"), .containsRoot).map(\.to), [ComponentID(.root, "пис")])
        XCTAssertEqual(g.edges(from: ComponentID(.lexeme, "переписать"), .derivedFrom).map(\.to), [ComponentID(.lexeme, "писать")])
        XCTAssertTrue(g.edges(from: ComponentID(.lexeme, "переписать"), .containsAffix).map(\.to).contains(ComponentID(.affix, "пере-")))
        XCTAssertTrue(g.edges(to: ComponentID(.root, "ход"), .containsRoot).count >= 8, "the family of ход- hangs off its root")
        // Root edges don't make a word's prerequisites (a word stays ready to learn once its letters are).
        XCTAssertFalse(g.prerequisites(of: ComponentID(.lexeme, "переписать")).contains(ComponentID(.root, "пис")))
        // Producing a word tags its root; building a word tags its affixes; a family guess tags word and root.
        var dict = Exercise(kind: .dictation); dict.text = "Я чита́ю кни́гу."
        let c = d.components(for: dict)
        XCTAssertTrue(c.contains(ComponentID(.root, "чит")) && c.contains(ComponentID(.root, "книг")), "\(c)")
        var build = Exercise(kind: .buildWord); build.en = "to rewrite"; build.answerText = "переписа́ть"; build.parts = ["пере-", "пис", "-а́ть"]
        let b = d.components(for: build)
        XCTAssertTrue(b.contains(ComponentID(.lexeme, "переписать")) && b.contains(ComponentID(.root, "пис")) && b.contains(ComponentID(.affix, "пере-")), "\(b)")
        var guess = Exercise(kind: .familyGuess); guess.text = "Где вход в парк?"; guess.word = "вход"; guess.root = "ход"; guess.options = ["exit", "entrance"]; guess.answerIndex = 1
        let f = d.components(for: guess)
        XCTAssertEqual(f.filter { $0.kind == .lexeme }, [ComponentID(.lexeme, "вход")], "the sentence's other words are not credited")
        XCTAssertTrue(f.contains(ComponentID(.root, "ход")))
        XCTAssertEqual(RussianDomain.modality(for: .buildWord), .build)
        XCTAssertEqual(RussianDomain.modality(for: .familyGuess), .readPick)
    }

    func testRootEvidenceIsOneWay() {
        let book = Book(course: course, store: tempStore())
        var right = Exercise(kind: .dictation); right.text = "кни́га"
        var wrong = Exercise(kind: .dictation); wrong.text = "писа́ть"
        var build = Exercise(kind: .buildWord); build.en = "a writer"; build.answerText = "писа́тель"; build.parts = ["пис", "-а́-", "-тель"]; build.options = build.parts
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [right, wrong, build], newWords: [])
        let p = book.currentPage
        book.record(page: p, exercise: 0) { $0.attempts = 1; $0.recognized = "книга"; $0.status = .correct }
        book.record(page: p, exercise: 1) { $0.attempts = 1; $0.recognized = "песать" }
        book.record(page: p, exercise: 1) { $0.revealed = true; $0.status = .missed }
        book.record(page: p, exercise: 2) { $0.attempts = 1; $0.recognized = "писатель"; $0.status = .correct }
        let m = book.knowledge.model
        XCTAssertEqual(m.trace(ComponentID(.root, "книг"), .hearWrite)?.reviews.map(\.grade), [.good], "a right word credits its root")
        XCTAssertNil(m.trace(ComponentID(.root, "пис"), .hearWrite), "a missed word does not blame its root")
        XCTAssertEqual(m.trace(ComponentID(.lexeme, "писать"), .hearWrite)?.reviews.first?.grade, .again, "the word itself still lapses")
        XCTAssertEqual(m.trace(ComponentID(.root, "пис"), .build)?.reviews.map(\.grade), [.good])
        XCTAssertEqual(m.trace(ComponentID(.affix, "-тель"), .build)?.reviews.count, 1)
        XCTAssertNotNil(m.trace(ComponentID(.lexeme, "писатель"), .build))
        XCTAssertTrue(book.knowledge.graph.edges(from: ComponentID(.lexeme, "писатель"), .containsRoot).map(\.to).contains(ComponentID(.root, "пис")))
    }

    func testDecodingHelpNamesAKnownRoot() {
        let book = Book(course: course, store: tempStore())
        var ex = Exercise(kind: .dictation); ex.text = "Я зна́ю."
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [ex], newWords: [])
        XCTAssertNil(book.knowledge.rootHint(for: "знание"), "nothing known yet: no hint")
        book.record(page: book.currentPage, exercise: 0) { $0.attempts = 1; $0.recognized = "я знаю"; $0.status = .correct }
        let hint = book.knowledge.rootHint(for: "знание")
        XCTAssertNotNil(hint)
        XCTAssertTrue(hint?.contains("root зна-") == true && hint?.contains("know") == true, hint ?? "nil")
        XCTAssertNil(book.knowledge.rootHint(for: "знать"), "a word already known needs no decoding")
        XCTAssertNil(book.knowledge.rootHint(for: "стол"), "a word on an unknown root gets nothing")
    }

    /// Pages are built only from roots with two known members, in letters already met, and
    /// their tiles spell their answers.
    func testGeneratedPagesUseOnlyKnownLettersAndRoots() throws {
        let book = Book(course: course, store: tempStore())
        let m = book.knowledge.domain.morphology
        XCTAssertTrue(book.wordFamilyExercises(from: m).exercises.isEmpty, "nothing known yet: no page")
        XCTAssertTrue(book.cognateExercises(from: m).exercises.isEmpty, "no letters yet: no cognates")
        // Work until Ё is met (Ф, Ъ are still ahead), then learn a second word on кот- and on стол-.
        var guardCount = 0
        while !book.letterKnown("Ё") && guardCount < 60 { QA.complete(book, page: book.currentPage, upTo: .max); book.turnForward(); guardCount += 1 }
        XCTAssertTrue(book.letterKnown("Ё") && !book.letterKnown("Ф"))
        var cat = Exercise(kind: .dictation); cat.text = "кот"
        var kitten = Exercise(kind: .dictation); kitten.text = "котёнок"
        var table = Exercise(kind: .dictation); table.text = "стол"
        var joiner = Exercise(kind: .dictation); joiner.text = "столя́р"
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [cat, kitten, table, joiner],
                               newWords: [Course.Word(ru: "котёнок", en: "kitten"), Course.Word(ru: "столя́р", en: "joiner")])
        for i in 0..<4 { book.record(page: book.currentPage, exercise: i) { $0.attempts = 1; $0.status = .correct } }
        let roots = book.familyRoots(from: m)
        XCTAssertTrue(roots.contains("кот") && roots.contains("стол"), "\(roots)")
        let met = Set(book.course.letters.filter { book.letterKnown($0.upper) }.map(\.upper))
        XCTAssertLessThan(met.count, 33, "some letters are still unmet, so the page has something to avoid")
        let known = book.knownLemmas
        let (exercises, used, newWords) = book.wordFamilyExercises(from: m, seed: 7)
        XCTAssertFalse(exercises.isEmpty)
        for r in used { XCTAssertGreaterThanOrEqual(m.members(of: r).filter(known.contains).count, 2, "root \(r) has fewer than two known members") }
        for e in exercises {
            let strings = [e.text, e.answerText, e.word].compactMap { $0 } + (e.options ?? []).filter { _ in e.kind == .buildWord } + (e.examples ?? []).map(\.ru)
            for s in strings { XCTAssertTrue(RussianText.letters(in: s).isSubset(of: met), "\(s) uses unmet letters") }
            if e.kind == .buildWord {
                XCTAssertTrue(BuildWordCheck.matches(e.parts ?? [], answer: e.answerText ?? ""), "\(e.parts ?? []) must spell \(e.answerText ?? "")")
                XCTAssertTrue(Set(e.parts ?? []).isSubset(of: Set(e.options ?? [])))
                XCTAssertEqual(m.entry(word: e.answerText ?? "")?.family.map { used.contains($0) }, true)
            }
            if e.kind == .familyGuess {
                XCTAssertTrue(used.contains(e.root ?? ""))
                XCTAssertNotNil(e.answerIndex)
                XCTAssertEqual(e.options?[e.answerIndex ?? -1], m.entry(word: e.word ?? "")?.en)
                XCTAssertTrue(RussianText.key(e.text ?? "").split(separator: " ").map(String.init).contains(RussianText.key(e.word ?? "")))
            }
        }
        for w in newWords { XCTAssertNil(book.word(w.ru), "\(w.ru) is new to the learner") }
        // The same seed gives the same page; the cognates page also keeps to met letters.
        XCTAssertEqual(book.wordFamilyExercises(from: m, seed: 7).exercises, exercises)
        let (cog, cogWords) = book.cognateExercises(from: m, seed: 3)
        for e in cog {
            for s in [e.text, e.answerText].compactMap({ $0 }) + (e.options ?? []).filter({ _ in e.kind == .buildWord }) {
                XCTAssertTrue(RussianText.letters(in: s).isSubset(of: met), "\(s) uses unmet letters")
            }
            if e.kind == .buildWord { XCTAssertTrue(BuildWordCheck.matches(e.parts ?? [], answer: e.answerText ?? "")) }
        }
        XCTAssertEqual(cog.isEmpty, cogWords.isEmpty)

        // Making a page proposes its new words as vocabulary; it does not make them known. Only a
        // finished exercise on one does — and the families view draws by the same criterion.
        let knownBefore = book.knownLemmas
        XCTAssertNil(MorphologyPages.addFamilies(book))
        let familyPage = book.page!
        XCTAssertEqual(book.knownLemmas, knownBefore, "an unanswered page teaches nothing yet")
        for w in book.customWords.filter({ $0.note?.hasPrefix("from the family") == true }).compactMap({ m.entry(word: RussianText.key($0.ru))?.key }) {
            XCTAssertFalse(book.knownLemmas.contains(w), "\(w) counted known on being proposed")
        }
        XCTAssertFalse(familyPage.exercises.isEmpty)
        for hub in WordFamiliesSection.hubs(book) { for (w, _) in hub.known { XCTAssertTrue(knownBefore.contains(w), "\(w) shown known without evidence") } }
        // A page proposing a family member the learner hasn't met: vocabulary until an exercise on
        // it is finished, and then known to generation and view alike.
        let fresh = try XCTUnwrap(m.lemmas.values.sorted { $0.key < $1.key }
            .first { $0.family != nil && !$0.en.isEmpty && !knownBefore.contains($0.key) && RussianText.letters(in: $0.ru).isSubset(of: met) })
        var d = Exercise(kind: .dictation); d.text = fresh.ru
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [d], newWords: [Course.Word(ru: fresh.ru, en: fresh.en)])
        XCTAssertNotNil(book.word(fresh.ru), "proposed")
        XCTAssertFalse(book.knownLemmas.contains(fresh.key), "not known")
        book.record(page: book.currentPage, exercise: 0) { $0.attempts = 1; $0.recognized = fresh.ru; $0.status = .correct }
        XCTAssertTrue(book.knownLemmas.contains(fresh.key), "answered: now known")
        XCTAssertTrue(WordFamiliesSection.hubs(book).contains { $0.known.contains { $0.0 == fresh.key } })
    }
}
