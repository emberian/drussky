import LearningCore
import XCTest
@testable import Drussky

/// Memory hooks: parsing the tutor's JSON, keeping them in the notebook, fading them out.
@MainActor
final class HookTests: XCTestCase {
    func course() throws -> Course {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        return try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    func tempStore() -> BookStore {
        BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
    }

    let t0 = Date(timeIntervalSince1970: 1_800_000_000)

    func testParseOneHookTolerantly() {
        let h = HookTutor.parse("Here you go:\n```json\n{\"hook\": \"MEAL-o — wash your hands before the meal\", \"emoji\": \"🧼 soap\"}\n```", at: t0)
        XCTAssertEqual(h?.text, "MEAL-o — wash your hands before the meal")
        XCTAssertEqual(h?.emoji, "🧼", "one emoji, the first")
        XCTAssertEqual(h?.source, .tutor)
        XCTAssertEqual(h?.created, t0)
        XCTAssertEqual(h?.model, Tutor.model)
        XCTAssertEqual(HookTutor.parse("{\"text\": \"COT — a cat on a cot\", \"emoji\": \"\"}", at: t0)?.text, "COT — a cat on a cot")
        XCTAssertNil(HookTutor.parse("{\"text\": \"COT\", \"emoji\": \"\"}", at: t0)?.emoji)
        XCTAssertNil(HookTutor.parse("Sorry, no.", at: t0))
        XCTAssertNil(HookTutor.parse("{\"hook\": \"\"}", at: t0))
        // A single-item batch shape also reads.
        XCTAssertEqual(HookTutor.parse("{\"hooks\": [{\"ru\": \"кот\", \"hook\": \"COT — a cat on a cot\"}]}", at: t0)?.text, "COT — a cat on a cot")
    }

    func testParseBatchByWordKey() {
        let reply = """
        {"hooks": [
          {"ru": "мы́ло", "hook": "MEAL-o — wash your hands before the meal", "emoji": "🧼"},
          {"word": "Кот", "mnemonic": "COT — a cat curled up on a cot", "emoji": "🐈"},
          {"ru": "дом", "hook": ""},
          {"hook": "orphan without a word"}
        ]}
        """
        let hooks = HookTutor.parseBatch(reply, at: t0)
        XCTAssertEqual(Set(hooks.keys), ["мыло", "кот"], "stress and case fold into the word key; empty and orphan hooks are dropped")
        XCTAssertEqual(hooks["кот"]?.emoji, "🐈")
        XCTAssertTrue(HookTutor.parseBatch("nothing here", at: t0).isEmpty)
        XCTAssertTrue(HookTutor.batchPrompt(for: [Course.Word(ru: "кот", en: "cat", emoji: nil, note: nil)]).contains("\"ru\": \"кот\""))
    }

    func testFadingSchedule() {
        XCTAssertEqual(HookFading.opacity(recalls: 0), 0.8, "faint from the start")
        XCTAssertEqual(HookFading.opacity(recalls: 2), 0.5)
        XCTAssertEqual(HookFading.opacity(recalls: 4), 0.2)
        XCTAssertEqual(HookFading.opacity(recalls: 5), 0, "gone after a handful of recalls")
        XCTAssertEqual(HookFading.opacity(recalls: 40), 0)
        let steps = (0..<6).map { HookFading.opacity(recalls: $0) }
        XCTAssertEqual(steps, steps.sorted(by: >), "never brighter again")
    }

    func testHooksLiveWithTheWordAndFadeWithItsRecalls() throws {
        let store = tempStore()
        let book = Book(course: try course(), store: store)
        let targetPageIdx = try XCTUnwrap(book.course.pageSequence.firstIndex { $0.page.exercises.contains { $0.kind == .translate && !RussianText.key($0.answerText ?? "").isEmpty } })
        for _ in 0..<targetPageIdx {
            QA.complete(book, page: book.currentPage, upTo: .max)
            book.turnForward()
        }
        let page = book.currentPage
        let i = try XCTUnwrap(book.pages[page].exercises.firstIndex { $0.kind == .translate && !RussianText.key($0.answerText ?? "").isEmpty })
        let wordKey = RussianText.key(book.pages[page].exercises[i].answerText ?? "")
        XCTAssertNil(book.hook(for: wordKey))
        XCTAssertEqual(book.hookOpacity(for: wordKey), 0, "no hook, nothing shown")
        book.setHook(MemoryHook(text: "COT — a cat curled up asleep on a cot", emoji: "🐈", source: .tutor, created: book.now()), for: wordKey.uppercased())
        XCTAssertEqual(book.hook(for: wordKey)?.text, "COT — a cat curled up asleep on a cot", "keyed by the word, stress and case aside")
        XCTAssertEqual(book.hookOpacity(for: wordKey), 0.8)

        // Each successful recall after the hook fades it one step; a miss does not.
        book.advanceClock(byDays: 1)
        book.record(page: page, exercise: i) { r in r.attempts += 1; r.judged = "right"; r.readerAgreed = true; r.status = .correct }
        XCTAssertEqual(book.recallsSinceHook(for: wordKey), 1)
        XCTAssertEqual(book.hookOpacity(for: wordKey), 0.65)
        book.reopen(page: page, exercise: i)
        book.record(page: page, exercise: i) { r in r.attempts += 1; r.status = .missed; r.revealed = true }
        XCTAssertEqual(book.recallsSinceHook(for: wordKey), 1)
        // Rewriting after being shown is practice, not a recall.
        book.reopen(page: page, exercise: i)
        book.record(page: page, exercise: i) { r in r.attempts += 1; r.status = .correct }
        XCTAssertEqual(book.recallsSinceHook(for: wordKey), 1)

        // Persistence: the shelf rides in book.json; a notebook without it opens with none.
        book.saveNow()
        let again = Book(course: try course(), store: store)
        XCTAssertEqual(again.hook(for: wordKey)?.emoji, "🐈")
        XCTAssertEqual(again.hook(for: wordKey)?.source, .tutor)
        let json = try String(contentsOf: store.stateURL, encoding: .utf8)
        XCTAssertTrue(json.contains("\"hooks\""))
        let state = try JSONDecoder().decode(BookState.self, from: Data("{\"version\":2,\"pages\":[],\"memories\":[],\"courseCursor\":0,\"currentPage\":0,\"settings\":{}}".utf8))
        XCTAssertNil(state.hooks)
        // An unknown source from a later build reads as the tutor's.
        let future = try JSONDecoder().decode(MemoryHook.self, from: Data("{\"text\":\"x\",\"source\":\"oracle\"}".utf8))
        XCTAssertEqual(future.source, .tutor)
        XCTAssertEqual(future.created, .distantPast)

        // Removing.
        again.setHook(nil, for: wordKey)
        XCTAssertNil(again.hook(for: wordKey))
        XCTAssertTrue(again.hooks.isEmpty)
    }

    func testWordsWantingHooksAreTheMostMissed() throws {
        let book = Book(course: try course(), store: tempStore())
        for p in 0..<4 { QA.complete(book, page: p, upTo: .max); book.turnForward() }
        // QA.complete misses some recalls (i % 5 == 3): those words have been graded again.
        var wanting = book.wordsWantingHooks(limit: 60)
        XCTAssertFalse(wanting.isEmpty)
        for w in wanting {
            let id = try XCTUnwrap(book.knowledge.domain.utteranceID(w.ru))
            XCTAssertTrue(book.knowledge.model.traces(of: id).values.contains { $0.reviews.contains { $0.grade == .again } }, w.ru)
        }
        // A word missed repeatedly comes first.
        let page = try XCTUnwrap(book.pages.indices.first { p in book.pages[p].exercises.contains { $0.kind == .translate && !RussianText.key($0.answerText ?? "").isEmpty } })
        let i = try XCTUnwrap(book.pages[page].exercises.firstIndex { $0.kind == .translate && !RussianText.key($0.answerText ?? "").isEmpty })
        let wordKey = RussianText.key(book.pages[page].exercises[i].answerText ?? "")
        for _ in 0..<3 {
            book.advanceClock(byDays: 1)
            book.reopen(page: page, exercise: i)
            book.record(page: page, exercise: i) { r in r.attempts += 1; r.status = .missed; r.revealed = true }
        }
        wanting = book.wordsWantingHooks(limit: 60)
        XCTAssertEqual(RussianText.key(wanting[0].ru), wordKey)
        XCTAssertEqual(book.wordsWantingHooks(limit: 1).count, 1)
        // Words with a hook drop out.
        let first = try XCTUnwrap(wanting.first)
        book.setHook(MemoryHook(text: "x", created: book.now()), for: first.ru)
        XCTAssertFalse(book.wordsWantingHooks(limit: 60).contains(first))
        XCTAssertFalse(HookBatch.isDue(book), "no key, no batch")
        XCTAssertTrue(book.hooksMarkdown().contains(RussianText.unstressed(first.ru)))
        XCTAssertNotNil(book.exportHooks())
    }

    func testEditorSource() {
        let tutor = MemoryHook(text: "COT — a cat", source: .tutor, created: t0)
        XCTAssertEqual(HookCard.source(of: "COT — a cat", proposed: "COT — a cat", existing: nil), .tutor)
        XCTAssertEqual(HookCard.source(of: "COT — my cat", proposed: "COT — a cat", existing: nil), .edited)
        XCTAssertEqual(HookCard.source(of: "COT — my cat", proposed: nil, existing: tutor), .edited)
        XCTAssertEqual(HookCard.source(of: "COT — a cat", proposed: nil, existing: tutor), .tutor)
        XCTAssertEqual(HookCard.source(of: "anything", proposed: nil, existing: nil), .mine)
        XCTAssertEqual(HookCard.source(of: "anything", proposed: nil, existing: MemoryHook(text: "z", source: .mine, created: t0)), .mine)
    }

    /// One live hook and one small batch through the app's own client (runs only with TEST_RUNNER_DRUSSKY_TUTOR_KEY).
    func testHookTutorLive() async throws {
        let key = ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"] ?? ""
        try XCTSkipIf(key.isEmpty, "no tutor key supplied")
        let previous = Keychain.apiKey
        Keychain.apiKey = key
        defer { Keychain.apiKey = previous }
        let soap = Course.Word(ru: "мы́ло", en: "soap", emoji: "🧼", note: nil)
        let h = try await HookTutor.suggest(for: soap, at: t0)
        print("HOOK:", h.emoji ?? "", h.text)
        XCTAssertFalse(h.text.isEmpty)
        XCTAssertLessThanOrEqual(h.text.split(separator: " ").count, 24)
        XCTAssertTrue(h.text.contains(where: \.isUppercase), "the sound-alike is in capitals")
        let batch = try await HookTutor.suggestBatch(for: [soap, Course.Word(ru: "кот", en: "cat", emoji: nil, note: nil),
                                                          Course.Word(ru: "хлеб", en: "bread", emoji: nil, note: nil)], at: t0)
        print("HOOKS:", batch.map { "\($0.key): \($0.value.emoji ?? "") \($0.value.text)" }.sorted())
        XCTAssertGreaterThanOrEqual(batch.count, 2)
        XCTAssertNotNil(batch["кот"] ?? batch["мыло"])
    }
}
