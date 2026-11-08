import SwiftUI
import XCTest
@testable import Drussky

/// The stress layer for library Russian (Model/LibraryStress.swift, Goal/GoalStress.swift): the
/// generated file decodes and every entry passes the rule; a passage shows the stressed form with
/// «Show stress» on and the bare text with it off; the goal cache round-trips through the notebook;
/// audio, gloss and ink keys never see a mark.
@MainActor
final class StressTests: XCTestCase {
    var course: Course {
        let url = Bundle.main.url(forResource: "course", withExtension: "json")!
        return try! JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    func freshBook() -> Book {
        Book(course: course, store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
    }

    func testRule() {
        func ok(_ src: String, _ st: String, _ line: UInt = #line) { XCTAssertEqual(RussianStress.problems(source: src, stressed: st), [], st, line: line) }
        func bad(_ src: String, _ st: String, _ line: UInt = #line) { XCTAssertFalse(RussianStress.problems(source: src, stressed: st).isEmpty, st, line: line) }
        ok("Лев спал.", "Лев спал.")
        ok("Мышь пробежала ему по телу.", "Мышь пробежа́ла ему́ по те́лу.")
        ok("Всё ещё идёт.", "Всё ещё идёт.")
        ok("Кто-то пришёл по-русски.", "Кто́-то пришёл по-ру́сски.")
        ok("Кто-то пришёл по-русски.", "Кто-то пришёл по-ру́сски.")
        ok("f(x) равно 2.", "f(x) равно́ 2.")
        bad("Мышь пробежала ему по телу.", "Мышь пробе́жа́ла ему́ по те́лу.")
        bad("Мышь пробежала ему по телу.", "Мышь пробежала ему́ по те́лу.")
        bad("Мышь пробежала ему по телу.", "Мы́шь пробежа́ла ему́ по те́лу.")
        bad("Всё ещё идёт.", "Всё ещё идё́т.")
        bad("Он сказал: «Иди».", "Он сказа́л: \"Иди́\".")
        bad("Он сказал.", "Он с́казал.")
        // A source that already marks a word keeps that mark where it was.
        ok("Верти́т хвостом.", "Верти́т хвосто́м.")
        bad("Верти́т хвостом.", "Ве́ртит хвосто́м.")
        bad("Верти́т хвостом.", "Вертит хвосто́м.")
        XCTAssertTrue(RussianStress.lacksStress("Мышь пробежала."))
        XCTAssertFalse(RussianStress.lacksStress("Мышь пробежа́ла."))
        XCTAssertFalse(RussianStress.lacksStress("Лев спал. Всё идёт."))
        XCTAssertFalse(RussianStress.lacksStress("What is 2 + 2?"))
    }

    func testFileDecodesAndCoversTheLibrary() throws {
        let stress = LibraryStress.shared
        XCTAssertFalse(stress.isEmpty, "library-stress.json is bundled (scripts/library_stress.py)")
        XCTAssertEqual(stress.file.version, 1)
        let library = Library.load()
        var covered = 0, total = 0
        for t in library.loose where t.isPrivate != true {
            let sentences = t.text.split(separator: "\n", omittingEmptySubsequences: true).flatMap { GoalAnalysis.splitSentences(String($0)) }
            total += sentences.count
            let stored = stress.file.texts[t.id]?.sentences ?? []
            for s in stored {
                XCTAssertTrue(sentences.indices.contains(s.index), "\(t.id) S\(s.index) exists")
                guard sentences.indices.contains(s.index) else { continue }
                XCTAssertEqual(sentences[s.index], s.text, "\(t.id) S\(s.index): the stored sentence is the text's")
                XCTAssertEqual(RussianStress.problems(source: s.text, stressed: s.stressed), [], "\(t.id) S\(s.index)")
                XCTAssertEqual(stress.stressed(sentence: s.text), s.stressed)
                XCTAssertEqual(t.stressed(sentence: s.text), s.stressed)
                covered += 1
            }
        }
        XCTAssertGreaterThan(covered, 0)
        print("stress: \(covered)/\(total) library sentences, \(stress.sentenceCount) in the index")
        // The Tatoeba sentences are the pseudo-text "sentences", one entry each in file order.
        for (i, s) in (library.sentences ?? []).prefix(40).enumerated() {
            guard let e = stress.file.texts["sentences"]?.sentences.first(where: { $0.index == i }) else { continue }
            XCTAssertEqual(e.text, s.ru, "sentences S\(i)")
            XCTAssertEqual(RussianStress.problems(source: s.ru, stressed: e.stressed), [])
        }
        // Full coverage is the validator's job (scripts/validate_stress.py); the app only needs what is there to be sound.
    }

    func testPassageShowsStressOnlyWhenAsked() throws {
        let text = try XCTUnwrap(Library.load().texts.first { $0.id == "tolstoy-lev-i-mysh" })
        let book = freshBook()
        book.settings.showStress = true
        let shown = book.displayText(text.text)
        XCTAssertNotEqual(shown, text.text, "the fable has stored marks")
        XCTAssertTrue(shown.unicodeScalars.contains("\u{0301}"))
        XCTAssertEqual(RussianText.unstressed(shown), text.text, "nothing but marks was added")
        XCTAssertEqual(shown, text.stressedText)
        // Line and token counts match the bare text's, so structure tokens and narration timings still line up.
        let bareLines = text.text.split(separator: "\n", omittingEmptySubsequences: true).map { $0.split(separator: " ").count }
        let shownLines = shown.split(separator: "\n", omittingEmptySubsequences: true).map { $0.split(separator: " ").count }
        XCTAssertEqual(bareLines, shownLines)
        XCTAssertEqual(Narration.timedTokens(shown).map(\.word), Narration.timedTokens(text.text).map(\.word), "timed tokens are bare")
        // The rendering: marked vowels are underlined; nothing underlined with the toggle off, and the string is the bare text.
        let on = Theme.stressed(shown, show: true)
        XCTAssertTrue(on.runs.contains { $0.swiftUI.underlineStyle == .single })
        XCTAssertEqual(String(on.characters), text.text)
        let light = Theme.passageStressed(shown, show: true)
        XCTAssertTrue(light.runs.contains { $0.swiftUI.underlineStyle != nil }, "a passage still underlines (its colour is faded; LineStyle equality ignores colour)")
        XCTAssertEqual(String(light.characters), text.text)
        book.settings.showStress = false
        XCTAssertEqual(book.displayText(text.text), text.text)
        let off = Theme.stressed(book.displayText(text.text), show: false)
        XCTAssertFalse(off.runs.contains { $0.swiftUI.underlineStyle != nil })
        XCTAssertEqual(String(off.characters), text.text)
        // A passage already carrying the tutor's marks is re-marked from the file, not doubled.
        let tutorVersion = "Лев спа́л. Мы́шь пробежа́ла ему по телу."
        let fixed = book.stressed(passage: tutorVersion)
        XCTAssertEqual(RussianText.unstressed(fixed), RussianText.unstressed(tutorVersion))
        XCTAssertEqual(fixed, "Лев спал. Мышь пробежа́ла ему́ по те́лу.")
        // Unknown text stays exactly as written; blank lines and spacing survive.
        let foreign = "Это предложение не из библиотеки.\n\n— Да."
        XCTAssertEqual(book.stressed(passage: foreign), foreign)
        XCTAssertEqual(LibraryStress.apply(to: "Лев спал.  Мышь пробежала ему по телу.") { LibraryStress.shared.stressed(sentence: $0) },
                       "Лев спал.  Мышь пробежа́ла ему́ по те́лу.")
    }

    func testKeysIgnoreMarks() throws {
        let bare = "Мышь пробежала ему по телу."
        let marked = try XCTUnwrap(LibraryStress.shared.stressed(sentence: bare))
        XCTAssertNotEqual(bare, marked)
        XCTAssertEqual(RussianText.key(marked), RussianText.key(bare))
        XCTAssertEqual(RussianText.audioKey(marked), RussianText.audioKey(bare))
        XCTAssertEqual(RussianText.audioKey("Э́то чай?"), RussianText.audioKey("Это чай?"))
        XCTAssertEqual(RussianText.comparable(marked), RussianText.comparable(bare))
        // The tapped token keeps its mark for display; every key made from it strips it.
        let tapped = PassageExercise.bare("пробежа́ла.")
        XCTAssertEqual(tapped, "пробежа́ла")
        XCTAssertEqual(RussianText.key(tapped), "пробежала")
        XCTAssertEqual(RussianText.audioKey(tapped), RussianText.audioKey("пробежала"))
        XCTAssertEqual(Narration.bare(Substring("«те́лу».")), "телу")
        // A gloss keyed by the bare word is found from the marked token.
        let glosses = ["пробежала": "ran across"]
        XCTAssertEqual(glosses.first { RussianText.key($0.key) == RussianText.key(PassageExercise.bare("пробежа́ла,")) }?.value, "ran across")
        XCTAssertEqual(RussianText.unstressed(PassageExercise.bare("«те́лу».")), "телу", "the read-along accessibility label is bare")
        // A word label: the library's spelling for a word the course lacks, the course's for one it has.
        let book = freshBook()
        XCTAssertEqual(String(book.stressedLabel("пробежала").characters), "пробежала")
        XCTAssertEqual(LibraryStress.shared.stressed(word: "Пробежала"), "пробежа́ла")
        XCTAssertTrue(book.stressedLabel("пробежала").runs.contains { $0.swiftUI.underlineStyle == .single })
    }

    func testGoalStressCacheRoundTrips() throws {
        let book = freshBook()
        let goal = Goal(title: "Своя книга", origin: .pasted, text: "Собака бежала домой. Лев спал.\nНикто не знал.", created: .now)
        book.addGoal(goal)
        let a = book.analysis(for: goal)
        // Nothing cached: the library's fable sentence is known, the others wait for the tutor.
        XCTAssertEqual(GoalStress.missing(in: a, have: nil), ["Собака бежала домой.", "Никто не знал."])
        XCTAssertNil(book.goalStressTable(goal.id))
        // The tutor's reply: one sound form, one with the text altered, one with a wrong mark, one unasked key.
        let reply: [String: Any] = ["0": "Соба́ка бежа́ла домо́й.", "1": "Никто не зна́л!", "2": "Лев спа́л.", "x": "что́-то"]
        let entries = GoalStress.entries(in: reply, asked: ["Собака бежала домой.", "Никто не знал.", "Лев спал."])
        XCTAssertEqual(entries, ["Собака бежала домой.": "Соба́ка бежа́ла домо́й."])
        book.mergeGoalStress(goalID: goal.id, entries)
        XCTAssertEqual(GoalStress.missing(in: a, have: book.goalStressTable(goal.id)), ["Никто не знал."])
        book.settings.showStress = true
        XCTAssertEqual(book.displayText(goal.text), "Соба́ка бежа́ла домо́й. Лев спал.\nНикто не знал.")
        XCTAssertEqual(String(book.stressedLabel("собака").characters), "собака")
        XCTAssertTrue(book.stressedLabel("собака").runs.contains { $0.swiftUI.underlineStyle == .single }, "a goal word label takes the goal's mark")
        // Written to the notebook, read back by a fresh Book on the same folder; an edited bad line is dropped.
        book.saveNow()
        let url = GoalStress.url(goalID: goal.id, root: book.store.root)
        XCTAssertTrue(FileManager.default.fileExists(atPath: url.path))
        var table = try JSONDecoder().decode(GoalStress.Table.self, from: Data(contentsOf: url))
        XCTAssertEqual(table.sentences, entries)
        XCTAssertEqual(table.model, GoalStress.model)
        table.sentences["Никто не знал."] = "Ни́кто не́ знал."
        try GoalStress.save(table, goalID: goal.id, root: book.store.root)
        let again = Book(course: course, store: BookStore(root: book.store.root))
        XCTAssertEqual(again.goalStressTable(goal.id)?.sentences, entries, "a form that fails the rule never comes back in")
        XCTAssertEqual(again.stressed(passage: "Собака бежала домой."), "Соба́ка бежа́ла домо́й.")
        again.settings.showStress = false
        XCTAssertEqual(again.displayText(goal.text), goal.text)
        // Putting the book away removes its table.
        again.removeGoal(id: goal.id)
        XCTAssertFalse(FileManager.default.fileExists(atPath: url.path))
        XCTAssertNil(again.goalStressTable(goal.id))
    }
}
