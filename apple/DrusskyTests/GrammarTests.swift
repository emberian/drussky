import LearningCore
import XCTest
@testable import Drussky

/// Grammar as a system: the paradigm data, the three exercise kinds, cell evidence and its
/// aggregation, the transform checker, the row hint, and the page built from what is known.
@MainActor
final class GrammarTests: XCTestCase {
    var course: Course {
        let url = Bundle.main.url(forResource: "course", withExtension: "json")!
        return try! JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    func tempStore() -> BookStore {
        BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
    }

    func testParadigmsLoadAndCoverTheCourse() {
        let p = Paradigms.load()
        XCTAssertGreaterThanOrEqual(p.tables.count, 12)
        XCTAssertGreaterThanOrEqual(p.words.count, 150)
        XCTAssertGreaterThanOrEqual(p.data.components.count, 80)
        // The three noun tables, adjectives, pronouns, two conjugations, irregulars, past, imperative, uses.
        for id in ["decl1", "decl2", "decl3", "adj", "pron", "conj1", "conj2", "irregular", "past", "imp", "uses"] { XCTAssertNotNil(p.table(id), id) }
        let kniga = p.word("кни́га")!
        XCTAssertEqual(kniga.forms["gen-sg"], "кни́ги")
        XCTAssertEqual(kniga.forms["gen-pl"], "книг")
        XCTAssertEqual(p.word("окно́")?.forms["nom-pl"], "о́кна")
        XCTAssertEqual(p.word("кот")?.forms["acc-sg"], "кота́", "animate accusative")
        XCTAssertEqual(p.word("стол")?.forms["acc-sg"], "стол", "inanimate accusative")
        XCTAssertEqual(p.word("идти́")?.forms["past-m"], "шёл")
        XCTAssertEqual(p.word("хоте́ть")?.forms["1pl"], "хоти́м")
        // A form is indexed to every cell it fills; the tags narrow it.
        XCTAssertEqual(Set(p.cells(of: "книги").map(\.cell)), ["gen-sg", "nom-pl", "acc-pl"])
        XCTAssertEqual(p.cells(of: "книгу", lemma: "книга", tags: ["acc-u"]).map(\.cell), ["acc-sg"])
        XCTAssertEqual(p.cells(of: "книги", lemma: "книга", tags: ["plural"]).map(\.cell).sorted(), ["acc-pl", "nom-pl"])
        // Every inflected course token whose lemma has a table is a cell of it, or a listed exception.
        let lemmas = Lemmas.load()
        var missing: [String] = []
        for (_, page) in course.pageSequence {
            for ex in page.exercises {
                let strings = [ex.text, ex.answerText, ex.before, ex.after, ex.say, ex.word].compactMap { $0 } + (ex.examples ?? []).map(\.ru) + (ex.forms?.values.map { $0 } ?? [])
                for s in strings {
                    for t in RussianText.key(s).split(separator: " ").map(String.init) {
                        let lemma = lemmas.lemma(t) ?? t
                        guard let w = p.words[lemma], lemma != t else { continue }
                        if p.cells(of: t).contains(where: { $0.word.key == w.key }) || p.data.exceptions[t] != nil || p.data.exceptions[lemma] != nil { continue }
                        missing.append("\(t) (\(lemma))")
                    }
                }
            }
        }
        XCTAssertTrue(missing.isEmpty, "course forms outside every table: \(Set(missing).sorted())")
    }

    func testNewKindsDecodeAndRoundTripAndOldNotebooksStillDecode() throws {
        let json = """
        [{"kind":"paradigm_fill","word":"кни́га","paradigm":"decl1","forms":{"nom-sg":"кни́га","gen-sg":"кни́ги","acc-sg":"кни́гу"},"cells":["gen-sg","acc-sg"]},
         {"kind":"paradigm_fill","word":"чита́ть","paradigm":"conj1","forms":{"1sg":"чита́ю","2sg":"чита́ешь"},"cells":["2sg"],"options":["чита́ешь","чита́ет"]},
         {"kind":"transform","word":"стол","cue":"prep-sg","answer":"столе́","before":"Кни́га на ","after":".","hint":"The book is on the table."},
         {"kind":"transform","word":"я","cue":"gen","answer":"меня́"},
         {"kind":"cue_sort","paradigm":"uses","options":["у меня́ есть","в / на + where"],"items":[{"text":"У меня́ есть кот.","answer":0},{"text":"Кот в до́ме.","answer":1}]}]
        """
        let exs = try JSONDecoder().decode([Exercise].self, from: Data(json.utf8))
        XCTAssertEqual(exs[0].kind, .paradigmFill)
        XCTAssertEqual(exs[0].cells, ["gen-sg", "acc-sg"])
        XCTAssertEqual(exs[0].forms?["gen-sg"], "кни́ги")
        XCTAssertNil(exs[0].target, "a table has no single target")
        XCTAssertEqual(exs[1].options, ["чита́ешь", "чита́ет"])
        XCTAssertEqual(exs[2].kind, .transform)
        XCTAssertEqual(exs[2].target, "столе́")
        XCTAssertEqual(exs[2].spoken, "столе́")
        XCTAssertTrue(exs[2].kind.isWriting)
        XCTAssertEqual(exs[3].cue, "gen")
        XCTAssertEqual(exs[4].items?.count, 2)
        XCTAssertEqual(exs[4].items?[1].answer, 1)
        let again = try JSONDecoder().decode([Exercise].self, from: JSONEncoder().encode(exs))
        XCTAssertEqual(again, exs)
        // An exercise written before these fields existed decodes unchanged, and encodes without them.
        let old = try JSONDecoder().decode(Exercise.self, from: Data("{\"kind\":\"fill\",\"before\":\"Я чита́ю \",\"after\":\".\",\"hint\":\"I read a book.\",\"answer\":\"кни́гу\"}".utf8))
        XCTAssertNil(old.paradigm); XCTAssertNil(old.cells); XCTAssertNil(old.items)
        let encoded = String(decoding: try JSONEncoder().encode(old), as: UTF8.self)
        XCTAssertFalse(encoded.contains("paradigm") || encoded.contains("items") || encoded.contains("cells"))
        // The course carries all three kinds and they decode with the rest.
        let kinds = Set(course.pageSequence.flatMap { $0.page.exercises.map(\.kind) })
        XCTAssertTrue(kinds.isSuperset(of: [.paradigmFill, .transform, .cueSort]))
        let expected = course.units.flatMap(\.pages).filter { $0.id.hasPrefix("g") && $0.id.count == 2 }.count
        XCTAssertGreaterThanOrEqual(expected, 6)
        XCTAssertEqual(course.pageSequence.filter { $0.page.id.hasPrefix("g") && $0.page.id.count == 2 }.count, expected, "authored grammar pages")
    }

    func testTransformCheckerAcceptsStressAndYoVariants() {
        XCTAssertTrue(TransformCheck.matches("книги", answer: "кни́ги"))
        XCTAssertTrue(TransformCheck.matches("кни́ги", answer: "кни́ги"))
        XCTAssertTrue(TransformCheck.matches("Книги", answer: "кни́ги"))
        XCTAssertTrue(TransformCheck.matches("звезды", answer: "звёзды"), "ё may be written е")
        XCTAssertTrue(TransformCheck.matches("звёзды", answer: "звёзды"))
        XCTAssertTrue(TransformCheck.matches("идет", answer: "идёт"))
        XCTAssertFalse(TransformCheck.matches("книга", answer: "кни́ги"))
        XCTAssertFalse(TransformCheck.matches("", answer: "кни́ги"))
        XCTAssertTrue(TransformCheck.matches("нового", answer: "но́вый / но́вого"), "either half of an animate accusative")
        XCTAssertTrue(TransformCheck.matches("новый", answer: "но́вый / но́вого"))
        XCTAssertFalse(TransformCheck.matches("новая", answer: "но́вый / но́вого"))
    }

    func testCellsAreComponentsAndExercisesTagThem() {
        let d = RussianDomain(course: course, lemmas: Lemmas.load(), pairs: SoundPairs.load(), morphology: Morphology.load(), paradigms: Paradigms.load())
        let g = d.graph
        XCTAssertNotNil(g[ComponentID(.construction, "decl1-gen-sg")])
        XCTAssertEqual(g[ComponentID(.construction, "decl1-gen-sg")]?.unit, "Grammar")
        XCTAssertTrue(g.prerequisites(of: ComponentID(.construction, "decl1-gen-pl")).contains(ComponentID(.construction, "decl1-gen-sg")))
        XCTAssertTrue(g.prerequisites(of: ComponentID(.construction, "decl1-nom-sg")).contains(ComponentID(.construction, "gender")))
        // An inflected token in any exercise fills its cell and the older pattern with it.
        var dict = Exercise(kind: .dictation); dict.text = "Я чита́ю кни́гу."
        let c = d.components(for: dict)
        XCTAssertTrue(c.contains(ComponentID(.construction, "decl1-acc-sg")) && c.contains(ComponentID(.construction, "acc-u")), "\(c)")
        XCTAssertFalse(c.contains(ComponentID(.construction, "decl1-gen-sg")), "книгу is not a genitive")
        XCTAssertTrue(c.contains(ComponentID(.construction, "conj1-1sg")) && c.contains(ComponentID(.construction, "pres-1sg")))
        // A transform names its cell; its word and form are credited too.
        var tf = Exercise(kind: .transform); tf.word = "кни́га"; tf.cue = "gen-sg"; tf.answerText = "кни́ги"
        let t = d.components(for: tf)
        XCTAssertTrue(t.contains(ComponentID(.construction, "decl1-gen-sg")) && t.contains(ComponentID(.construction, "genitive")) && t.contains(ComponentID(.lexeme, "книга")), "\(t)")
        XCTAssertFalse(t.contains(ComponentID(.construction, "decl1-nom-pl")), "the cue disambiguates книги")
        XCTAssertEqual(RussianDomain.modality(for: .transform), .transform)
        XCTAssertEqual(RussianDomain.modality(for: .cueSort), .classify)
        // A table asks several cells at once.
        var pf = Exercise(kind: .paradigmFill); pf.word = "стол"; pf.cells = ["gen-sg", "prep-sg"]; pf.forms = ["nom-sg": "стол", "gen-sg": "стола́", "prep-sg": "столе́"]
        let f = d.components(for: pf)
        XCTAssertTrue(f.contains(ComponentID(.construction, "decl2-gen-sg")) && f.contains(ComponentID(.construction, "decl2-prep-sg")) && f.contains(ComponentID(.construction, "prep-e")))
        XCTAssertFalse(f.contains(ComponentID(.construction, "decl2-nom-sg")), "only the asked cells")
        // A sort credits the uses its buckets name, and a wrong bucket is a confusion between them.
        var cs = Exercise(kind: .cueSort); cs.options = ["у меня́ есть", "в / на + where"]; cs.items = [SortItem(text: "У меня́ есть кот.", answer: 0), SortItem(text: "Кот в до́ме.", answer: 1)]
        let s = d.components(for: cs)
        XCTAssertTrue(s.contains(ComponentID(.construction, "have")) && s.contains(ComponentID(.construction, "prep-e")), "\(s)")
        let conf = d.confusions(cs, response: "1:0")
        XCTAssertEqual(conf.first?.target, ComponentID(.construction, "prep-e"))
        XCTAssertEqual(conf.first?.produced, ComponentID(.construction, "have"))
    }

    func testCellStrengthAggregatesPerComponentAndModality() {
        let book = Book(course: course, store: tempStore())
        let p = book.knowledge.domain.paradigms
        let decl1 = p.table("decl1")!
        XCTAssertTrue(CellStrength.table(decl1, book.knowledge).values.allSatisfy { !$0.met }, "empty before any evidence")
        var tf = Exercise(kind: .transform); tf.word = "кни́га"; tf.cue = "gen-sg"; tf.answerText = "кни́ги"
        var rm = Exercise(kind: .readMeaning); rm.text = "на столе́"; rm.options = ["on the table", "in the house"]; rm.answerIndex = 0
        var pf = Exercise(kind: .paradigmFill); pf.word = "стол"; pf.cells = ["acc-sg"]; pf.forms = ["nom-sg": "стол", "acc-sg": "стол"]; pf.options = ["стол", "стола́"]
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [tf, rm, pf], newWords: [])
        let page = book.currentPage
        book.record(page: page, exercise: 0) { $0.attempts = 1; $0.recognized = "книги"; $0.status = .correct }
        book.record(page: page, exercise: 1) { $0.attempts = 1; $0.chosen = 0; $0.status = .correct }
        book.record(page: page, exercise: 2) { $0.attempts = 1; $0.recognized = "acc-sg=✓стол"; $0.status = .correct }
        let k = book.knowledge
        let strengths = CellStrength.table(decl1, k)
        let gen = strengths["gen-sg"]!
        XCTAssertTrue(gen.met && gen.produced, "a written transform fills its cell as produced")
        XCTAssertGreaterThan(gen.strength ?? 0, 0.5)
        XCTAssertFalse(strengths["dat-sg"]!.met, "an unasked cell stays empty")
        let decl2 = CellStrength.table(p.table("decl2")!, k)
        XCTAssertTrue(decl2["prep-sg"]!.met, "столе́ read for its meaning fills prepositional singular")
        XCTAssertFalse(decl2["prep-sg"]!.produced, "recognized, not produced")
        XCTAssertTrue(decl2["acc-sg"]!.met)
        XCTAssertFalse(decl2["acc-sg"]!.produced, "tiles are picked, not written")
        XCTAssertNotNil(k.model.trace(ComponentID(.construction, "decl2-acc-sg"), .classify), "a tile table reports the cue-pick modality")
        XCTAssertNotNil(k.model.trace(ComponentID(.construction, "decl1-gen-sg"), .transform))
        let summary = CellStrength.summary(decl1, k)
        XCTAssertEqual(summary.filled, 1)
        XCTAssertEqual(summary.total, 12)
    }

    func testRowHintShowsTheParadigmRowOfAMissedForm() {
        let d = RussianDomain(course: course, lemmas: Lemmas.load(), pairs: SoundPairs.load(), morphology: Morphology.load(), paradigms: Paradigms.load())
        let hint = d.paradigmRow(forAnswer: "кни́гу")!
        XCTAssertEqual(hint.table.id, "decl1")
        XCTAssertEqual(hint.cell, "acc-sg")
        XCTAssertEqual(hint.rowLabel, "accusative")
        XCTAssertEqual(hint.cells.map(\.form), ["кни́гу", "кни́ги"], "the row: singular and plural")
        XCTAssertEqual(hint.cells.map(\.cell.ending), ["-у / -ю / -ию", "= nom / gen"])
        // A form in two rows is pinned by its cue; a verb's past cell is drawn by the past table.
        XCTAssertEqual(d.paradigmRow(forAnswer: "кни́ги", word: "кни́га", cue: "nom-pl")?.rowLabel, "nominative")
        XCTAssertEqual(d.paradigmRow(forAnswer: "кни́ги", word: "кни́га", cue: "gen-sg")?.rowLabel, "genitive")
        let past = d.paradigmRow(forAnswer: "чита́ла", word: "чита́ть", cue: "past-f")!
        XCTAssertEqual(past.table.id, "past")
        XCTAssertEqual(past.cells.map(\.form), ["чита́ла"])
        XCTAssertNil(d.paradigmRow(forAnswer: "спаси́бо"), "a word in no table has no row")
        // A course fill whose answer is a form gets the same hint without naming its cell.
        var fill = Exercise(kind: .fill); fill.before = "Кни́га на "; fill.after = "."; fill.answerText = "столе́"
        XCTAssertEqual(d.paradigmRow(forAnswer: fill.target!)?.rowLabel, "prepositional")
    }

    /// The generated page uses only words the learner has met, in letters already met; its
    /// tables and cues agree with paradigms.json; cases are interleaved; the same seed gives the
    /// same page.
    func testGeneratedGrammarPageUsesKnownWordsAndMetLetters() {
        let book = Book(course: course, store: tempStore())
        let p = book.knowledge.domain.paradigms
        XCTAssertTrue(book.grammarExercises(from: p, seed: 1).exercises.isEmpty, "nothing known yet: no page")
        // Work until some letters and paradigm words are known while at least one letter is still ahead.
        let allLetters = Set(book.course.letters.map(\.upper))
        var guardCount = 0
        while guardCount < book.course.pageSequence.count {
            let metNow = Set(book.course.letters.filter { book.letterKnown($0.upper) }.map(\.upper))
            let exs = book.grammarExercises(from: p, seed: 7).exercises
            let casesNow = Set(exs.flatMap { e in
                (e.cells ?? [e.cue].compactMap { $0 }).map { "\(e.paradigm ?? "")/\($0)".split(separator: "-").first.map(String.init) ?? "" }
            })
            if !metNow.isEmpty && metNow.count < allLetters.count && book.knownParadigmWords(from: p).count >= 3 && exs.count >= 4 && casesNow.count >= 2 {
                break
            }
            QA.complete(book, page: book.currentPage, upTo: .max)
            book.turnForward()
            guardCount += 1
        }
        let met = Set(book.course.letters.filter { book.letterKnown($0.upper) }.map(\.upper))
        XCTAssertFalse(met.isEmpty)
        XCTAssertLessThan(met.count, allLetters.count, "at least one letter still unmet")
        let known = Set(book.knownParadigmWords(from: p).map(\.key))
        XCTAssertGreaterThanOrEqual(known.count, 3, "\(known)")
        XCTAssertTrue(known.allSatisfy { p.words[$0] != nil }, "\(known)")
        for w in book.knownParadigmWords(from: p) { for f in w.forms.values { XCTAssertTrue(RussianText.letters(in: f).isSubset(of: met), "\(f) uses unmet letters") } }
        let (exercises, tables, spoken) = book.grammarExercises(from: p, seed: 7)
        XCTAssertGreaterThanOrEqual(exercises.count, 4, "\(exercises.map(\.kind))")
        XCTAssertFalse(tables.isEmpty)
        XCTAssertFalse(spoken.isEmpty)
        var rows: [String] = []
        for e in exercises {
            let strings = [e.text, e.answerText, e.word, e.before, e.after].compactMap { $0 } + (e.forms?.values.map { $0 } ?? []) + (e.options ?? []).filter { _ in e.kind == .paradigmFill } + (e.items ?? []).map(\.text) + (e.examples ?? []).map(\.ru)
            for s in strings { XCTAssertTrue(RussianText.letters(in: s).isSubset(of: met), "\(s) uses unmet letters") }
            switch e.kind {
            case .paradigmFill:
                let w = p.word(e.word ?? "")!
                XCTAssertTrue(known.contains(w.key), "\(w.key) is not known")
                for (cell, form) in e.forms ?? [:] { XCTAssertEqual(w.forms[cell], form) }
                for c in e.cells ?? [] { XCTAssertNotNil(e.forms?[c]); rows.append("\(e.paradigm ?? "")/\(c)") }
                if let options = e.options { for c in e.cells ?? [] { XCTAssertTrue(options.contains(e.forms![c]!)) } }
            case .transform:
                let w = p.word(e.word ?? "")!
                XCTAssertTrue(known.contains(w.key), "\(w.key) is not known")
                XCTAssertTrue(w.answers(e.cue ?? "").contains(e.answerText ?? ""), "\(e.answerText ?? "") is not \(w.key) \(e.cue ?? "")")
                if e.before != nil { XCTAssertNotNil(e.hint) }
                rows.append("\(e.paradigm ?? "")/\(e.cue ?? "")")
            case .cueSort:
                XCTAssertGreaterThanOrEqual(e.options?.count ?? 0, 2)
                for it in e.items ?? [] { XCTAssertTrue((e.options?.indices.contains(it.answer)) ?? false) }
                XCTAssertGreaterThanOrEqual(Set((e.items ?? []).map(\.answer)).count, 2)
            default: break
            }
        }
        XCTAssertGreaterThanOrEqual(Set(rows.map { $0.split(separator: "-").first.map(String.init) ?? $0 }).count, 2, "cases are interleaved, not one drilled: \(rows)")
        XCTAssertEqual(book.grammarExercises(from: p, seed: 7).exercises, exercises, "the same seed gives the same page")
        XCTAssertEqual(exercises.first?.kind, .explain, "the card names the tables in play")
    }
}
