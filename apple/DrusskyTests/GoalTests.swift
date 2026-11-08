import LearningCore
import XCTest
@testable import Drussky

/// "Your book": the goal text, its distance from the knowledge model, the pages toward it.
@MainActor
final class GoalTests: XCTestCase {
    private func course() throws -> Course {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        return try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    private func tempStore() -> BookStore {
        BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
    }

    private func libraryText(_ id: String) throws -> Library.Text {
        try XCTUnwrap(Library.load().texts.first { $0.id == id })
    }

    private struct Sample: Decodable { var tokens: [[String]] }

    private func sample() throws -> [(token: String, lemmas: Set<String>)] {
        let url = try XCTUnwrap(Bundle(for: Self.self).url(forResource: "goal-lemmas", withExtension: "json"))
        let s = try JSONDecoder().decode(Sample.self, from: Data(contentsOf: url))
        return s.tokens.map { ($0[0], Set($0[1].split(separator: "|").map(String.init))) }
    }

    // MARK: Lemmatizer

    /// The rule-based lemmatizer on 200 labelled running tokens: exact lemma accuracy, and
    /// grouping accuracy (tokens of one word land on one output), which is what the ladder needs.
    func testRuleBasedLemmatizerAccuracy() throws {
        let c = try course()
        let lemmas = Lemmas.load()
        let domain = RussianDomain(course: c, lemmas: lemmas, pairs: SoundPairs.load())
        let vocabulary = Set(lemmas.forms.values.compactMap(\.first)).union(domain.words)
        let lemmatizer = RussianLemmatizer(vocabulary: vocabulary)
        let gold = try sample()
        XCTAssertEqual(gold.count, 200)
        var exact = 0
        var misses: [String] = []
        var outputByGold: [String: Set<String>] = [:]
        for (token, lemmasOK) in gold {
            let got = lemmas.lemma(token) ?? lemmatizer.lemma(token)
            let g = got.replacingOccurrences(of: "ё", with: "е")
            if lemmasOK.contains(g) { exact += 1 } else { misses.append("\(token)→\(g) (\(lemmasOK.sorted().joined(separator: "/")))") }
            outputByGold[lemmasOK.sorted().first!, default: []].insert(g)
        }
        let grouped = outputByGold.values.filter { $0.count == 1 }.count
        let exactPct = 100 * Double(exact) / Double(gold.count)
        let groupPct = 100 * Double(grouped) / Double(outputByGold.count)
        print(String(format: "LEMMATIZER rule-based: exact %.1f%% (%d/%d), words collapsing to one form %.1f%% (%d/%d)",
                     exactPct, exact, gold.count, groupPct, grouped, outputByGold.count))
        print("LEMMATIZER misses:", misses.joined(separator: "; "))
        XCTAssertGreaterThanOrEqual(exactPct, 85, "misses: \(misses)")
        XCTAssertGreaterThanOrEqual(groupPct, 95)
    }

    func testLemmatizerRules() {
        let l = RussianLemmatizer(vocabulary: ["существовать", "планета", "стих"])
        XCTAssertEqual(l.lemma("читала"), "читать")
        XCTAssertEqual(l.lemma("существует"), "существовать")
        XCTAssertEqual(l.lemma("существую"), "существовать")
        XCTAssertEqual(l.lemma("обитающие"), "обитать")
        XCTAssertEqual(l.lemma("напечатано"), "напечатать")
        XCTAssertEqual(l.lemma("покорили"), "покорить")
        XCTAssertEqual(l.lemma("засмеялся"), "засмеяться")
        XCTAssertEqual(l.lemma("героические"), "героический")
        XCTAssertEqual(l.lemma("государственной"), "государственный")
        XCTAssertEqual(l.lemma("веревкой"), "веревка")
        XCTAssertEqual(l.lemma("женой"), "жена")
        XCTAssertEqual(l.lemma("государства"), "государство")
        XCTAssertEqual(l.lemma("власти"), "власть")
        XCTAssertEqual(l.lemma("иных"), "иной")
        XCTAssertEqual(l.lemma("дикую"), "дикий")
        XCTAssertEqual(l.lemma("планете"), "планета", "the vocabulary decides -е")
        XCTAssertEqual(l.lemma("стихи"), "стих")
        XCTAssertEqual(l.lemma("книги"), "книга")
        XCTAssertEqual(l.lemma("уравнении"), "уравнение")
        XCTAssertEqual(l.lemma("станции"), "станция")
        XCTAssertEqual(l.lemma("ее"), "она")
        XCTAssertEqual(l.lemma("шел"), "идти")
        XCTAssertEqual(l.lemma("котенка"), "котенок")
        XCTAssertEqual(l.lemma("последние"), "последний")
        XCTAssertEqual(l.lemma("знание"), "знание")
        XCTAssertEqual(l.lemma("да"), "да")
    }

    // MARK: Analysis and distance

    func testGoalAnalysisSplitsSentencesAndTokens() {
        let s = GoalAnalysis.splitSentences("Лев спал. Мышь пробежала ему по телу! «Если ты меня пустишь…» Он проснулся? Да.")
        XCTAssertEqual(s, ["Лев спал.", "Мышь пробежала ему по телу!", "«Если ты меня пустишь…»", "Он проснулся?", "Да."])
        XCTAssertEqual(GoalAnalysis.splitSentences("Т. е. так. Потом."), ["Т. е. так.", "Потом."])
        XCTAssertEqual(GoalAnalysis.words(in: "Я, Д-503, строитель «Интеграла», — кто-нибудь."), ["Я", "Д", "строитель", "Интеграла", "кто", "нибудь"])
    }

    func testDistanceMovesWithKnowledge() throws {
        let book = Book(course: try course(), store: tempStore())
        let goal = Goal(title: "t", origin: .pasted, text: "Я читаю книгу. Она читает книгу. Кот спит.", created: book.now())
        book.addGoal(goal)
        let before = book.distance(for: goal)
        XCTAssertEqual(before.analysis.sentences.count, 3)
        XCTAssertEqual(before.coverage, 0)
        XCTAssertEqual(Set(before.ladder.prefix(2).map(\.lemma)), ["читать", "книга"], "two occurrences each, both on the first page")
        XCTAssertEqual(before.ladder.first?.count, 2)
        XCTAssertEqual(before.ladder.first?.value, 6, "1 per occurrence + the first-page bonus")
        XCTAssertTrue(before.ladder.first!.onFirstPage)
        XCTAssertEqual(before.ladder.map(\.lemma).suffix(2), ["кот", "спать"], "the stemmer's guess, until the tutor's table")
        XCTAssertEqual(before.readableSentences, 0)
        XCTAssertFalse(before.firstPageReady)
        XCTAssertTrue(before.analysis.patterns["acc-u"] ?? 0 >= 2, "\(before.analysis.patterns)")
        // Learn the words of the first two sentences (writing them right).
        var ex: [Exercise] = []
        for w in ["я", "чита́ю", "кни́гу", "она́", "чита́ет"] { var d = Exercise(kind: .dictation); d.text = w; ex.append(d) }
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: ex, newWords: [])
        for i in ex.indices { book.record(page: book.currentPage, exercise: i) { $0.attempts += 1; $0.status = .correct } }
        let after = book.distance(for: goal)
        XCTAssertEqual(after.sentenceCoverage[0], 1, accuracy: 1e-9)
        XCTAssertEqual(after.sentenceCoverage[1], 1, accuracy: 1e-9)
        XCTAssertEqual(after.sentenceCoverage[2], 0)
        XCTAssertEqual(after.readableSentences, 2)
        XCTAssertEqual(after.passageWindow(), 0..<2, "the readable stretch")
        XCTAssertEqual(after.ladder.map(\.lemma), ["кот", "спать"])
        XCTAssertEqual(after.unknownLemmas, ["кот", "спать"])
        XCTAssertFalse(after.firstPageReady, "the first page is all three sentences; one is unknown")
        // Knowing the rest opens the first page. спит is outside the course: the page that teaches it
        // lists it as a new word (as the tutor's pages do), and the goal's table knows its lemma.
        var g = goal; g.lemmaCache = ["спит": .init(lemma: "спать", en: "sleeps")]
        book.update(goal: g)
        XCTAssertEqual(book.distance(for: g).ladder.map(\.lemma), ["кот", "спать"])
        var more: [Exercise] = []
        for w in ["кот", "спит"] { var d = Exercise(kind: .dictation); d.text = w; more.append(d) }
        book.insertWrittenPage(title: "t2", intro: nil, unitTitle: "t", exercises: more, newWords: [.init(ru: "спит", en: "sleeps")])
        let gUpdated = book.currentGoal!
        XCTAssertEqual(book.goalYield(of: book.page!, toward: gUpdated), 2, "a page teaching спит counts toward the lemma спать")
        for i in more.indices { book.record(page: book.currentPage, exercise: i) { $0.attempts += 1; $0.status = .correct } }
        XCTAssertEqual(book.goalYield(of: book.page!, toward: gUpdated), 0, "once known, it adds nothing")
        let ready = book.distance(for: gUpdated)
        XCTAssertTrue(ready.firstPageReady)
        XCTAssertEqual(ready.coverage, 1, accuracy: 1e-9)
        let pagesBefore = book.pages.count
        XCTAssertTrue(book.openFirstPageIfReady())
        XCTAssertEqual(book.pages.count, pagesBefore + 1)
        XCTAssertTrue(book.page!.isGoalFirstPage)
        XCTAssertEqual(book.page?.exercises.first?.kind, .passage)
        XCTAssertEqual(book.page?.exercises.first?.text, "Я читаю книгу. Она читает книгу. Кот спит.")
        XCTAssertNotNil(book.currentGoal?.firstPageOpened)
        XCTAssertFalse(book.openFirstPageIfReady(), "opens once")
    }

    func testGoalYieldAndPullPolicy() throws {
        let book = Book(course: try course(), store: tempStore())
        let goal = Goal(title: "t", origin: .pasted, text: "Кот спит. Мама читает.", created: book.now())
        book.addGoal(goal)
        var teaches = Exercise(kind: .dictation); teaches.text = "кот"
        var elsewhere = Exercise(kind: .dictation); elsewhere.text = "дом"
        let page = BookPage(id: "x", number: 1, sourceID: "u1", unitTitle: "u", title: "t", intro: nil, isReview: false,
                            exercises: [teaches, elsewhere], results: [ExerciseResult(), ExerciseResult()], created: Date())
        XCTAssertEqual(book.goalYield(of: page, toward: goal), 1)
        var other = page; other.exercises = [elsewhere]; other.results = [ExerciseResult()]
        XCTAssertEqual(book.goalYield(of: other, toward: goal), 0)
        // Nothing readable yet, and no key in the test runner: never due.
        XCTAssertFalse(book.goalPullDue(reaching: book.page!))
    }

    // MARK: Pages toward the goal

    func testGoalPageRejectsInventedSentences() throws {
        let book = Book(course: try course(), store: tempStore())
        let goal = Goal(title: "t", origin: .pasted, text: "Лев спал. Мышь пробежала ему по телу. Он проснулся и поймал её.", created: book.now())
        let d = book.distance(for: goal)
        let brief = GoalPull.Brief(goal: goal, distance: d, patterns: [])
        var real = Exercise(kind: .dictation); real.text = "Мышь пробежа́ла ему́ по те́лу."
        XCTAssertNotNil(GoalPull.verify(real, in: brief))
        var invented = Exercise(kind: .dictation); invented.text = "Мышь спала́ на столе́."
        XCTAssertNil(GoalPull.verify(invented, in: brief))
        var fill = Exercise(kind: .fill); fill.before = "Лев "; fill.answerText = "спал"; fill.after = "."; fill.hint = "The lion slept."
        XCTAssertNotNil(GoalPull.verify(fill, in: brief))
        var altered = fill; altered.before = "Кот "
        XCTAssertNil(GoalPull.verify(altered, in: brief))
        var passage = Exercise(kind: .passage); passage.text = "Лев спал.\nМышь пробежала ему по телу."
        XCTAssertNotNil(GoalPull.verify(passage, in: brief))
        passage.text = "Лев спал. Потом он ушёл."
        XCTAssertNil(GoalPull.verify(passage, in: brief))
        var pick = Exercise(kind: .listenPick); pick.say = "Лев спал."; pick.options = ["Лев спал.", "Лев ел.", "Кот"]
        let kept = GoalPull.verify(pick, in: brief)
        XCTAssertEqual(kept?.options, ["Лев спал.", "Кот"], "multi-word distractors must be from the text; single words may be invented")
        var explain = Exercise(kind: .explain); explain.text = "Past tense."; explain.examples = [.init(ru: "Лев спал", en: "x"), .init(ru: "Кот спал", en: "y")]
        XCTAssertEqual(GoalPull.verify(explain, in: brief)?.examples?.count, 1)
    }

    func testOfflineGoalPageFromTheTextAlone() throws {
        let book = Book(course: try course(), store: tempStore())
        var goal = Goal(title: "Лев и мышь", author: "Толстой", origin: .library, source: "x",
                        text: try libraryText("tolstoy-lev-i-mysh").text, created: book.now())
        goal.lemmaCache = ["спал": .init(lemma: "спать", en: "slept"), "мышь": .init(lemma: "мышь", en: "mouse"),
                           "пробежала": .init(lemma: "пробежать", en: "ran past"), "телу": .init(lemma: "тело", en: "body")]
        book.addGoal(goal)
        let d = book.distance(for: goal)
        XCTAssertEqual(d.ladder.first { $0.lemma == "мышь" }?.gloss, "mouse")
        let brief = GoalPull.Brief(goal: goal, distance: d, patterns: [])
        XCTAssertNil(brief.passage, "nothing readable to a fresh learner")
        let all = Set(book.course.letters.map(\.upper))
        let page = GoalPull.localPage(brief, met: all, allMet: true)
        XCTAssertGreaterThanOrEqual(page.exercises.count, 3)
        for ex in page.exercises { XCTAssertNotNil(GoalPull.verify(ex, in: brief), "\(ex)") }
        book.insertGoalPage(PageWriter.Written(title: page.title, intro: page.intro, exercises: page.exercises, new_words: nil),
                            exercises: page.exercises, goal: goal, sentences: brief.sentences)
        XCTAssertTrue(book.page!.isGoalPage)
        XCTAssertEqual(book.currentGoal?.pulledPages, [book.page!.id])
    }

    // MARK: Persistence

    func testGoalsPersistAndOldNotebooksDecode() throws {
        let store = tempStore()
        let book = Book(course: try course(), store: store)
        var goal = Goal(title: "Мы", author: "Замятин", origin: .library, source: "zamyatin-my-zapis-1",
                        text: try libraryText("zamyatin-my-zapis-1").text, created: book.now())
        goal.lemmaCache = ["списываю": .init(lemma: "списывать", en: "copy out")]
        book.addGoal(goal)
        book.addGoal(Goal(title: "second", origin: .pasted, text: "Кот спит.", created: book.now()))
        book.currentGoalID = goal.id
        book.saveNow()
        let reopened = Book(course: try course(), store: store)
        XCTAssertEqual(reopened.goals.map(\.id), book.goals.map(\.id))
        XCTAssertEqual(reopened.currentGoal?.id, goal.id)
        XCTAssertEqual(reopened.currentGoal?.lemmaCache?["списываю"]?.lemma, "списывать")
        XCTAssertEqual(reopened.currentGoal?.text, Goal.clean(goal.text))
        // A notebook from before goals existed decodes with none.
        let data = try Data(contentsOf: store.stateURL)
        var object = try XCTUnwrap(try JSONSerialization.jsonObject(with: data) as? [String: Any])
        object.removeValue(forKey: "goals")
        object.removeValue(forKey: "currentGoalID")
        let old = try JSONSerialization.data(withJSONObject: object)
        guard case .success(let state) = BookStore.decode(old) else { return XCTFail("old notebook should decode") }
        XCTAssertNil(state.goals)
        XCTAssertEqual(state.pages.count, book.pages.count)
    }

    // MARK: Fetching a text

    func testWikisourceParagraphsFromParseAPI() throws {
        let url = try XCTUnwrap(Bundle(for: Self.self).url(forResource: "wikisource-my", withExtension: "json"))
        let object = try XCTUnwrap(try JSONSerialization.jsonObject(with: Data(contentsOf: url)) as? [String: Any])
        let parse = try XCTUnwrap(object["parse"] as? [String: Any])
        let text = Wikisource.paragraphs(inHTML: try XCTUnwrap(parse["text"] as? String))
        let lines = text.split(separator: "\n")
        XCTAssertTrue(lines.first?.hasPrefix("Я просто списываю") ?? false, "\(lines.first ?? "")")
        XCTAssertTrue(lines.last?.hasSuffix("Я готов.") ?? false, "\(lines.last ?? "")")
        XCTAssertFalse(text.contains("Википедия"), "the header template is not part of the text")
        XCTAssertFalse(text.contains("<"))
        XCTAssertEqual(lines.count, 9)
        let (title, author) = Wikisource.split(title: try XCTUnwrap(parse["title"] as? String))
        XCTAssertEqual(title, "Мы: Запись 1-ая")
        XCTAssertEqual(author, "Замятин")
        XCTAssertEqual(Wikisource.pageTitle(of: URL(string: "https://ru.wikisource.org/wiki/%D0%9C%D1%8B_(%D0%97%D0%B0%D0%BC%D1%8F%D1%82%D0%B8%D0%BD)/%D0%97%D0%B0%D0%BF%D0%B8%D1%81%D1%8C_1-%D0%B0%D1%8F")!),
                       "Мы (Замятин)/Запись 1-ая")
        XCTAssertEqual(Wikisource.decode("a&nbsp;&amp;&#1052;&#x44b;"), "a\u{00A0}&Мы")
    }

    // MARK: Live

    /// The tutor's lemma table on the labelled sample, and a real page toward the goal (live API;
    /// runs only with TEST_RUNNER_DRUSSKY_TUTOR_KEY).
    func testGoalLemmaTableAndPageLive() async throws {
        let key = ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"] ?? ""
        try XCTSkipIf(key.isEmpty, "no tutor key supplied")
        let previous = Keychain.apiKey
        Keychain.apiKey = key
        defer { Keychain.apiKey = previous }
        let gold = try sample()
        let table = try await GoalLemmaTable.lemmatize(Array(Set(gold.map(\.token))))
        var exact = 0, answered = 0
        var misses: [String] = []
        for (token, ok) in gold {
            guard let e = table[token] else { continue }
            answered += 1
            let got = e.lemma.replacingOccurrences(of: "ё", with: "е")
            if ok.contains(got) { exact += 1 } else { misses.append("\(token)→\(got) (\(ok.sorted().joined(separator: "/")))") }
        }
        print(String(format: "LEMMATIZER tutor table: exact %.1f%% (%d/%d answered of %d)", 100 * Double(exact) / Double(max(1, answered)), exact, answered, gold.count))
        print("LEMMATIZER tutor misses:", misses.joined(separator: "; "))
        XCTAssertGreaterThanOrEqual(answered, 190)
        XCTAssertGreaterThanOrEqual(Double(exact) / Double(max(1, answered)), 0.9)

        let book = Book(course: try course(), store: tempStore())
        for p in 0..<8 { QA.complete(book, page: p, upTo: .max); book.turnForward() }
        var goal = Goal(library: try libraryText("tolstoy-lev-i-mysh"), created: book.now())
        goal = try await GoalLemmaTable.fill(goal, analysis: book.analysis(for: goal))
        book.addGoal(goal)
        XCTAssertFalse(goal.lemmaCache?.isEmpty ?? true)
        let d = book.distance(for: goal)
        let brief = GoalPull.Brief(goal: goal, distance: d, patterns: d.patterns(domain: book.knowledge.domain, model: book.knowledge.model, at: book.now()))
        let (written, exercises) = try await PageWriter.write(.goal(brief), book: book) { _ in }
        print("GOAL PAGE:", written.title, "|", exercises.count, "of", written.exercises.count, "kept |", exercises.map(\.kind.rawValue),
              "| new words:", written.new_words?.map(\.ru) ?? [])
        for ex in written.exercises where !exercises.contains(ex) {
            print("GOAL PAGE dropped:", ex.kind.rawValue, ex.text ?? ex.answerText ?? ex.say ?? "", ex.before ?? "", ex.after ?? "", ex.options ?? [])
        }
        XCTAssertGreaterThanOrEqual(exercises.count, 4)
        for ex in exercises { XCTAssertNotNil(GoalPull.verify(ex, in: brief)) }
        book.insertGoalPage(written, exercises: exercises, goal: goal, sentences: brief.sentences)
        XCTAssertTrue(book.page!.isGoalPage)
    }
}
