import LearningCore
import XCTest
@testable import Drussky

/// The structure layer (Structure/): the generated file decodes and lines up with the library, the
/// word rule matches the script's, a passage finds its sentences, the structure exercise is built
/// and graded as reading evidence on the construction, and the notebook still decodes.
@MainActor
final class StructureTests: XCTestCase {
    var course: Course {
        let url = Bundle.main.url(forResource: "course", withExtension: "json")!
        return try! JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    func testFileDecodesAndMatchesTheLibrary() throws {
        let structure = Structure.shared
        XCTAssertFalse(structure.isEmpty, "library-structure.json is bundled (scripts/library_structure.py)")
        XCTAssertEqual(structure.file.version, 1)
        for (id, role) in structure.roles {
            XCTAssertTrue(["member", "clause"].contains(role.tier), id)
            XCTAssertFalse(role.ru.isEmpty, id)
        }
        let library = Library.load()
        var covered = 0, total = 0
        for t in library.texts where t.isPrivate != true {
            let sentences = structure.sentences(of: t.id)
            // The app's splitter finds the same sentences the script stored, in order, word for word.
            let mine = t.text.split(separator: "\n", omittingEmptySubsequences: true).flatMap { GoalAnalysis.splitSentences(String($0)) }
            total += mine.count
            for s in sentences {
                XCTAssertTrue(mine.indices.contains(s.index), "\(t.id) S\(s.index) exists")
                guard mine.indices.contains(s.index) else { continue }
                XCTAssertEqual(mine[s.index], s.text, "\(t.id) S\(s.index): the stored sentence is the text's")
                XCTAssertEqual(Structure.words(in: s.text).map { [$0.lowerBound, $0.upperBound] }, s.words,
                               "\(t.id) S\(s.index): the app's word rule is the script's")
                XCTAssertNotNil(structure.sentence(matching: s.text), "\(t.id) S\(s.index) is found by its text")
                let n = s.wordCount
                for c in s.chunks {
                    XCTAssertTrue(c.words.count == 2 && c.first >= 0 && c.first <= c.last && c.last < n, "\(t.id) S\(s.index) chunk \(c.words)")
                    XCTAssertNotNil(structure.role(c.role), "\(t.id) S\(s.index): role \(c.role) is in the table")
                }
                for r in s.referents where r.headSentence == s.index {
                    XCTAssertTrue((0..<n).contains(r.word) && (0..<n).contains(r.head) && r.word != r.head, "\(t.id) S\(s.index) referent \(r)")
                }
                for x in s.roots {
                    XCTAssertEqual(RussianText.unstressed(s.word(x.word)).lowercased(), x.prefix + x.root + x.suffix + x.inflection,
                                   "\(t.id) S\(s.index): x-ray spells its word")
                }
                covered += 1
            }
        }
        XCTAssertGreaterThan(covered, 0)
        // Full coverage is the validator's job (scripts/validate_structure.py); the app only needs what is there to be sound.
        print("structure: \(covered)/\(total) library sentences")
    }

    func testWordRule() {
        XCTAssertEqual(Structure.wordStrings(in: "Мышь пробежала ему по телу."), ["Мышь", "пробежала", "ему", "по", "телу"])
        XCTAssertEqual(Structure.wordStrings(in: "«Кто-то — 138 арш.?» f(x), по-ру́сски…"), ["Кто-то", "138", "арш", "f", "x", "по-ру́сски"])
        XCTAssertEqual(Structure.words(in: "Лев спал.").map { [$0.lowerBound, $0.upperBound] }, [[0, 3], [4, 8]])
        XCTAssertEqual(Structure.wordStrings(in: " — "), [])
    }

    func testPassageFindsItsSentencesAndTokens() throws {
        let s = try XCTUnwrap(Structure.shared.sentences(of: "tolstoy-lev-i-mysh").first, "the first fable has structure")
        let text = "Лев спал. Мышь пробежала ему по телу.\nПотом охотники поймали льва."
        let p = try XCTUnwrap(PassageStructure(text: text))
        XCTAssertEqual(p.sentences.first, s)
        XCTAssertEqual(p.lines.count, 2)
        XCTAssertEqual(p.lines[0].count, 7)
        XCTAssertEqual(p.lines[0][0], PassageStructure.Token(sentence: 0, words: 0..<1))
        XCTAssertEqual(p.lines[0][1], PassageStructure.Token(sentence: 0, words: 1..<2), "«спал.» carries word 1 of sentence 0")
        XCTAssertEqual(p.lines[0][2].sentence, p.sentences.count > 1 ? 1 : nil)
        // Stress marks added by the tutor do not break the match.
        let stressed = try XCTUnwrap(PassageStructure(text: "Лев спал. Мышь пробежа́ла ему́ по те́лу."))
        XCTAssertEqual(stressed.sentences.count, p.sentences.count)
        XCTAssertNil(PassageStructure(text: "Это предложение не из библиотеки."), "a text with no structured sentence has no layer")
        // Marks: the subject is underlined once, the predicate twice, a pronoun wears a ring.
        let layer = StructureLayer()
        let roles = Structure.shared.roles
        let m0 = layer.mark(for: p.lines[0][0], previous: nil, next: p.lines[0][1], in: p, roles: roles)
        let m1 = layer.mark(for: p.lines[0][1], previous: p.lines[0][0], next: p.lines[0][2], in: p, roles: roles)
        XCTAssertEqual(m0.underline, .single)
        XCTAssertEqual(m1.underline, .double)
        XCTAssertFalse(m0.joinRight, "two different members are not joined")
        if p.sentences.count > 1 {
            let him = layer.mark(for: p.lines[0][4], previous: p.lines[0][3], next: p.lines[0][5], in: p, roles: roles)
            XCTAssertTrue(him.isReferent, "«ему» points back to the lion in the sentence before, which is on the page")
            XCTAssertEqual(p.head(of: p.sentences[1].referent(at: 2)!)?.sentence, 0)
            XCTAssertNil(PassageStructure(text: "Мышь пробежала ему по телу.")?.head(of: p.sentences[1].referent(at: 2)!),
                         "alone on a page, the pronoun has nothing to point at")
            let pp = layer.mark(for: p.lines[0][5], previous: p.lines[0][4], next: p.lines[0][6], in: p, roles: roles)
            XCTAssertTrue(pp.joinRight, "по телу is one prepositional phrase")
        }
    }

    func testStructureExerciseIsBuiltAndGraded() throws {
        let exercises = Structure.shared.exercises(for: "tolstoy-lev-i-mysh", limit: 2)
        XCTAssertFalse(exercises.isEmpty, "the fable has a pronoun with its head in the same sentence")
        let ex = exercises[0]
        XCTAssertEqual(ex.kind, .structure)
        let probe = try XCTUnwrap(Structure.probe(of: ex))
        XCTAssertEqual(ex.word, probe.probe)
        XCTAssertNotEqual(probe.probe, probe.head)
        XCTAssertEqual(ex.skills?.count, 1)
        // Round trip, with the new optional field.
        let back = try JSONDecoder().decode(Exercise.self, from: JSONEncoder().encode(ex))
        XCTAssertEqual(back, ex)
        XCTAssertEqual(back.probe, ex.probe)
        let old = try JSONDecoder().decode(Exercise.self, from: Data(#"{"kind":"passage","text":"Лев спал."}"#.utf8))
        XCTAssertNil(old.probe, "exercises without the field still decode")
        // Evidence: a reading outcome on the construction (and the pronoun), never a memory item of its own.
        let d = RussianDomain(course: course, lemmas: Lemmas.load(), pairs: SoundPairs.load())
        XCTAssertEqual(RussianDomain.modality(for: .structure), .readPick)
        let components = d.components(for: ex)
        XCTAssertTrue(components.contains(ComponentID(.construction, ex.skills![0])), "\(components)")
        var r = ExerciseResult(); r.status = .correct; r.attempts = 1; r.chosen = ex.answerIndex
        XCTAssertEqual(RussianDomain.grade(ex, r), .good)
        var again = ExerciseResult(); again.status = .done; again.attempts = 2
        XCTAssertEqual(RussianDomain.grade(ex, again), .again, "right after a wrong tap is not recall")
        let page = BookPage(id: "p", number: 1, sourceID: nil, unitTitle: "u", title: "t", intro: nil, isReview: false,
                            exercises: [ex], results: [r], created: .now, finished: nil)
        let event = try XCTUnwrap(d.evidence(for: RussianAttempt(page: page, index: 0, before: ExerciseResult(), after: r, at: .now, latency: nil)))
        XCTAssertEqual(event.modality, .readPick)
        XCTAssertEqual(event.expected, probe.head)
        XCTAssertEqual(event.grade, .good)
        let store = BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
        let book = Book(course: course, store: store)
        XCTAssertNil(book.memoryKey(for: ex), "not a recall item")
        // The tutor is told what was asked.
        let text = Library.load().texts.first { $0.id == "tolstoy-lev-i-mysh" }!
        let made = Structure.shared.page(for: text)
        XCTAssertEqual(made.exercises.first?.kind, .passage)
        XCTAssertEqual(made.exercises.dropFirst().map(\.kind), exercises.map(\.kind))
    }

    func testSettingsKeepDecodingWithoutTheToggle() throws {
        let s = try JSONDecoder().decode(Settings.self, from: Data(#"{"fingerDrawing":true}"#.utf8))
        XCTAssertNil(s.showStructure)
        var on = Settings(); on.showStructure = true
        XCTAssertEqual(try JSONDecoder().decode(Settings.self, from: JSONEncoder().encode(on)).showStructure, true)
    }
}
