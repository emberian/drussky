import LearningCore
import XCTest
@testable import Drussky

/// The listening strand (Model/Listening.swift): listening.json decodes by the authoring contract,
/// a set becomes a page of `listening` cards offered once by the course, answers become hear-pick
/// evidence (a miss blaming the crucial span), every string it can play has a clip, and the
/// reserved weekly messages reach the weekly sample ungraded.
@MainActor
final class ListeningTests: XCTestCase {
    private func course() throws -> Course {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        return try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    private func freshBook() throws -> Book {
        Book(course: try course(), store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
    }

    private func item(_ kind: ListeningItem.Kind) throws -> ListeningItem {
        try XCTUnwrap(Listening.shared.sets.flatMap(\.items).first { $0.kind == kind }, "\(kind)")
    }

    func testListeningDecodesByTheContract() throws {
        let l = Listening.shared
        XCTAssertEqual(l.version, 1)
        XCTAssertEqual(l.sets.count, 12)
        XCTAssertEqual(l.weekly_samples.count, 6)
        XCTAssertEqual(l.sets.flatMap(\.items).count, 94)
        let order = Dictionary(uniqueKeysWithValues: try course().pageSequence.enumerated().map { ($1.page.id, $0) })
        var ids = Set<String>()
        for set in l.sets + l.weekly_samples {
            XCTAssertNotNil(order[set.after_page], "\(set.id): \(set.after_page)")
            for it in set.items {
                XCTAssertTrue(ids.insert(it.id).inserted, it.id)
                XCTAssertTrue(["any", "unfamiliar"].contains(it.voice_hint), it.id)
                XCTAssertFalse(it.heard.isEmpty, "\(it.id) has nothing to play")
                XCTAssertTrue(it.heard.contains { $0.contains(it.crucial_span) } || it.kind == .message || it.kind == .questionOrStatement, it.id)
                switch it.kind {
                case .pickObject, .correction:
                    XCTAssertNotNil(it.ru); XCTAssertNotNil(it.question); XCTAssertNil(it.lines); XCTAssertNil(it.forms); XCTAssertNil(it.questions)
                    XCTAssertTrue((it.options ?? []).indices.contains(it.answer ?? -1), it.id)
                case .whoNeeds, .disagreement:
                    XCTAssertNil(it.ru)
                    let lines = try XCTUnwrap(it.lines, it.id)
                    XCTAssertTrue((2...4).contains(lines.count), it.id)
                    XCTAssertEqual(Set(lines.map(\.speaker)), ["A", "B"], it.id)
                    XCTAssertTrue((it.options ?? []).indices.contains(it.answer ?? -1), it.id)
                case .follow:
                    let scene = try XCTUnwrap(it.scene, it.id), outcomes = try XCTUnwrap(it.outcomes, it.id)
                    XCTAssertEqual(outcomes.count, it.options?.count, it.id)
                    XCTAssertTrue(scene.contains { $0.role == "object" } && scene.contains { $0.role == "place" }, it.id)
                    for o in outcomes {
                        XCTAssertTrue(scene.contains { $0.id == o.object && $0.role == "object" }, it.id)
                        if let p = o.place { XCTAssertTrue(scene.contains { $0.id == p && $0.role == "place" }, it.id) }
                    }
                case .questionOrStatement:
                    let forms = try XCTUnwrap(it.forms, it.id)
                    XCTAssertEqual(forms.count, 2); XCTAssertEqual(it.played, it.answer, it.id)
                    XCTAssertEqual(it.options, ["Statement", "Question"], it.id)
                    XCTAssertEqual(it.heard, [forms[it.played ?? 0].ru], it.id)
                    XCTAssertEqual(it.en, forms[it.played ?? 0].en, it.id)
                    XCTAssertNotEqual(RussianText.audioKey(forms[0].ru), RussianText.audioKey(forms[1].ru), "\(it.id): the question is its own clip")
                case .message:
                    XCTAssertNotNil(it.ru); XCTAssertNil(it.question); XCTAssertNil(it.options); XCTAssertNil(it.answer)
                    let qs = try XCTUnwrap(it.questions, it.id)
                    XCTAssertTrue((2...3).contains(qs.count), it.id)
                    XCTAssertEqual(it.decisionCount, qs.count)
                    for q in qs {
                        XCTAssertTrue(it.ru?.contains(q.crucial_span) == true, "\(it.id): \(q.crucial_span)")
                        if q.response_mode == "english_text" { XCTAssertNil(q.options); XCTAssertNotNil(q.answer_en) }
                        else { XCTAssertTrue((q.options ?? []).indices.contains(q.answer ?? -1), it.id) }
                    }
                }
            }
        }
        // An explicit null place («keep it where it is») decodes as nil, and the bank has such outcomes.
        let unchanged = l.sets.flatMap(\.items).filter { $0.kind == .follow }.flatMap { $0.outcomes ?? [] }.filter { $0.place == nil }
        XCTAssertFalse(unchanged.isEmpty)
        // Weekly messages are messages answered in English, never offered as practice.
        for set in l.weekly_samples {
            XCTAssertEqual(set.items.count, 5)
            XCTAssertTrue(set.items.allSatisfy { $0.kind == .message && $0.questions?.allSatisfy { $0.response_mode == "english_text" } == true })
        }
        XCTAssertFalse(Shelves.items.contains { $0.sourceID.hasPrefix("listening:listen-week") })
        // An exercise carrying an item round-trips through the notebook's own encoding.
        var ex = Exercise(kind: .listening)
        ex.listening = try item(.follow)
        let data = try JSONEncoder().encode(ex)
        let back = try JSONDecoder().decode(Exercise.self, from: data)
        XCTAssertEqual(back, ex)
        XCTAssertNil(back.spoken); XCTAssertNil(back.target)
        // A notebook from before the strand still opens.
        XCTAssertNil(try JSONDecoder().decode(Exercise.self, from: Data(#"{"kind":"dictation","text":"кот"}"#.utf8)).listening)
    }

    func testVoicesKeepTwoSpeakersApart() throws {
        let any = try XCTUnwrap(Listening.shared.sets.flatMap(\.items).first { $0.kind == .whoNeeds && $0.voice_hint == "any" })
        XCTAssertEqual(any.voice(forLine: 0), "xenia"); XCTAssertEqual(any.voice(forLine: 1), "aidar")
        let unfamiliar = try XCTUnwrap(Listening.shared.sets.flatMap(\.items).first { $0.kind.isExchange && $0.voice_hint == "unfamiliar" })
        let a = unfamiliar.lines?.firstIndex { $0.speaker == "A" } ?? 0, b = unfamiliar.lines?.firstIndex { $0.speaker == "B" } ?? 1
        XCTAssertEqual(unfamiliar.voice(forLine: a), "aidar"); XCTAssertEqual(unfamiliar.voice(forLine: b), "baya")
        let single = try XCTUnwrap(Listening.shared.sets.flatMap(\.items).first { !$0.kind.isExchange && $0.voice_hint == "unfamiliar" })
        XCTAssertEqual(single.voice(), "aidar")
        XCTAssertEqual(try item(.pickObject).voice(), "xenia")
    }

    func testEveryListeningStringHasAClipInItsVoice() throws {
        let voice = Voice()
        let strings = Listening.shared.spokenStrings
        // The inventory file is the source of truth for the count; the app's list must match it line for line.
        let inventory = try String(contentsOf: URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("research/artifacts/listening-spoken.txt"), encoding: .utf8)
            .split(separator: "\n").map(String.init).filter { !$0.isEmpty }
        XCTAssertEqual(strings.count, inventory.count, "the spoken inventory (research/artifacts/listening-spoken.txt)")
        var missing: [String] = []
        for s in strings where !voice.hasRecording(s) { missing.append(s) }
        XCTAssertTrue(missing.isEmpty, "no clip: \(missing)")
        // Each heard line has a clip in the voice assigned to it, so A and B stay two people and a
        // replay keeps its speaker; a course clip stands in only where the course voice was asked for.
        var wrongVoice: [String] = []
        for it in (Listening.shared.sets + Listening.shared.weekly_samples).flatMap(\.items) {
            for (i, text) in it.heard.enumerated() {
                let name = it.voice(forLine: i)
                if voice.recordingURL(for: text, voice: name) == nil && name != "xenia" { wrongVoice.append("\(it.id): \(text) (\(name))") }
            }
        }
        XCTAssertTrue(wrongVoice.isEmpty, "no clip in the assigned voice: \(wrongVoice)")
    }

    func testASetBecomesAPageOfCardsOfferedOnce() throws {
        let book = try freshBook()
        let set = try XCTUnwrap(Listening.shared.sets.first)
        let item = Shelves.item(set, number: 1)
        XCTAssertEqual(item.place, "На слух · 1")
        XCTAssertEqual(item.sourceID, "listening:listen-01")
        XCTAssertEqual(item.kind, .listening)
        let page = Listening.page(for: set, number: 1)
        XCTAssertEqual(page.exercises.count, set.items.count)
        XCTAssertTrue(page.exercises.allSatisfy { $0.kind == .listening && $0.listening != nil })
        XCTAssertEqual(Set(page.exercises.compactMap { $0.listening?.id }), Set(set.items.map(\.id)))
        let played = page.exercises.compactMap { $0.listening?.played }
        XCTAssertNotEqual(played, set.items.compactMap(\.played), "the authored alternation is not the page's order")
        XCTAssertEqual(Listening.ordered(set.items).map(\.id), Listening.ordered(set.items).map(\.id), "the order is stable")
        XCTAssertNil(book.memoryKey(for: page.exercises[0]), "a heard card is not a thing to recall")
        // Offered once its page is done, written onto the next composed page's footer, then opened.
        XCTAssertFalse(book.isReady(item))
        var ex = Exercise(kind: .copy); ex.text = "кот"
        book.appendPage(sourceID: set.after_page, unitTitle: "u", title: "t", intro: nil, isReview: false, exercises: [ex])
        book.record(page: book.pages.count - 1, exercise: 0) { $0.status = .done }
        XCTAssertTrue(book.isReady(item))
        XCTAssertTrue(book.shelfOffers().contains(item.sourceID))
        book.appendPage(sourceID: nil, unitTitle: "u", title: "next", intro: nil, isReview: false, exercises: [ex])
        XCTAssertTrue(book.pages.last?.offers?.contains(item.sourceID) == true)
        XCTAssertEqual(book.nextListeningSet()?.sourceID, item.sourceID)
        book.open(item)
        XCTAssertEqual(book.page?.sourceID, item.sourceID)
        XCTAssertEqual(book.page?.unitTitle, "На слух · 1")
        XCTAssertEqual(book.page?.exercises.count, 6)
        XCTAssertEqual(book.nextListeningSet()?.sourceID, "listening:listen-02")
        XCTAssertFalse(book.shelfOffers().contains(item.sourceID))
        // The frozen page survives a save and load with its items.
        book.saveNow()
        let again = Book(course: book.course, store: BookStore(root: book.store.root))
        XCTAssertEqual(again.page(id: book.page!.id)?.exercises.first?.listening?.kind, .questionOrStatement)
    }

    func testARightAnswerIsHearPickEvidenceOnTheHeardWords() throws {
        let book = try freshBook()
        let pick = try item(.pickObject)  // «Дай сыр.» → сыр
        var ex = Exercise(kind: .listening); ex.listening = pick
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [ex], newWords: [])
        let p = book.currentPage, pageID = book.page!.id
        let d = book.knowledge.domain
        let comps = d.components(for: ex)
        XCTAssertTrue(comps.contains(d.utteranceID("сыр")!), "\(comps)")
        XCTAssertFalse(comps.contains { $0.kind == .letter }, "hearing a word is not evidence of its letters")
        book.record(page: p, exercise: 0) { $0.chosen = pick.answer; $0.attempts += 1; $0.status = .correct; $0.responseMs = 900 }
        let event = try XCTUnwrap(book.knowledge.model.events.last { $0.item == "\(pageID)#0" })
        XCTAssertEqual(event.type, .outcome)
        XCTAssertEqual(event.modality, .hearPick)
        XCTAssertEqual(event.grade, .good)
        XCTAssertEqual(event.activity, "listening")
        XCTAssertEqual(event.expected, pick.heard.joined(separator: " "), "the heard line as written in listening.json")
        XCTAssertEqual(event.response, "сыр")
        XCTAssertEqual(event.latencyMs, 900)
        XCTAssertEqual(book.knowledge.model.trace(d.utteranceID("сыр")!, .hearPick)?.reviews.last?.grade, .good)
    }

    func testAMissIsAgainOnTheCrucialSpanWithTheConfusion() throws {
        let book = try freshBook()
        let pick = try item(.pickObject)
        let answer = try XCTUnwrap(pick.answer), options = try XCTUnwrap(pick.options)
        let wrong = try XCTUnwrap(options.indices.first { $0 != answer })
        var ex = Exercise(kind: .listening); ex.listening = pick
        // A correction: the span is a part of the turn.
        let corr = try item(.correction)
        var ex2 = Exercise(kind: .listening); ex2.listening = corr
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [ex, ex2], newWords: [])
        let p = book.currentPage, pageID = book.page!.id
        let d = book.knowledge.domain
        book.record(page: p, exercise: 0) { $0.chosen = wrong; $0.attempts += 1; $0.status = .missed }
        let event = try XCTUnwrap(book.knowledge.model.events.last { $0.item == "\(pageID)#0" })
        XCTAssertEqual(event.grade, .again)
        XCTAssertEqual(event.modality, .hearPick)
        XCTAssertEqual(event.response, options[wrong])
        XCTAssertEqual(event.confusions?.first?.target, d.utteranceID(options[answer]))
        XCTAssertEqual(event.confusions?.first?.produced, d.utteranceID(options[wrong]))
        XCTAssertEqual(event.confusions?.first?.channel, .choice)
        let corrWrong = try XCTUnwrap(corr.options?.indices.first { $0 != corr.answer })
        book.record(page: p, exercise: 1) { $0.chosen = corrWrong; $0.attempts += 1; $0.status = .missed }
        let miss = try XCTUnwrap(book.knowledge.model.events.last { $0.item == "\(pageID)#1" })
        let spanWords = Set(d.listeningComponents(for: ex2, result: book.result(ExerciseRef(pageID: pageID, index: 1))!))
        XCTAssertEqual(Set(miss.components), spanWords)
        XCTAssertTrue(spanWords.isSubset(of: Set(d.components(for: ex2))), "the span's words are among the turn's")
        XCTAssertLessThan(spanWords.count, d.components(for: ex2).count, "a miss blames the span, not the whole turn: \(corr.crucial_span)")
        XCTAssertNil(miss.confusions, "English choices name no Russian confusion")
    }

    func testAMessageIsAnsweredAsOneAndIntonationIsItsOwnSkill() throws {
        let book = try freshBook()
        let msg = try item(.message)
        var ex = Exercise(kind: .listening); ex.listening = msg
        let qs = try item(.questionOrStatement)
        var ex2 = Exercise(kind: .listening); ex2.listening = qs
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [ex, ex2], newWords: [])
        let p = book.currentPage, pageID = book.page!.id
        let answers = (0..<msg.decisionCount).map { msg.answer($0)! }
        var picks = answers; picks[0] = (answers[0] + 1) % msg.options(0).count  // the gist missed, the detail right
        book.record(page: p, exercise: 0) { $0.recognized = ListeningItem.encode(picks: picks); $0.attempts += 1; $0.status = .missed }
        let r = try XCTUnwrap(book.result(ExerciseRef(pageID: pageID, index: 0)))
        XCTAssertEqual(ListeningItem.picks(r).compactMap { $0 }, picks)
        let event = try XCTUnwrap(book.knowledge.model.events.last { $0.item == "\(pageID)#0" })
        XCTAssertEqual(event.grade, .again)
        XCTAssertEqual(event.response, picks.enumerated().map { msg.options($0.offset)[$0.element] }.joined(separator: " / "))
        let d = book.knowledge.domain
        var probe = Exercise(kind: .dictation); probe.text = msg.questions![0].crucial_span
        XCTAssertEqual(Set(event.components), Set(d.components(for: probe).filter { $0.kind != .letter && $0.kind != .root }), "the missed question's span, not the item's fallback (a miss never blames a root)")
        XCTAssertTrue(ListeningItem.describe(ex, r).hasPrefix("Hearing a voice message"))
        // Asking or telling: one skill, whatever the words.
        XCTAssertEqual(d.components(for: ex2), [RussianDomain.intonationComponent])
        XCTAssertNotNil(d.graph[RussianDomain.intonationComponent])
        book.record(page: p, exercise: 1) { $0.chosen = qs.answer; $0.attempts += 1; $0.status = .correct }
        XCTAssertEqual(book.knowledge.model.trace(RussianDomain.intonationComponent, .hearPick)?.reviews.last?.grade, .good)
    }

    func testWeeklyMessagesReachTheSampleUngradedOnceTheirPageIsDone() throws {
        let book = try freshBook()
        let week = try XCTUnwrap(Listening.shared.weekly_samples.first)
        // Nothing reached: the bank of human recordings, as before.
        XCTAssertTrue(book.openBaseline())
        XCTAssertTrue(book.page!.exercises.compactMap(\.baseline).filter { $0.kind == .listening }.allSatisfy { $0.audio != nil && $0.text == nil })
        for i in book.page!.exercises.indices { _ = book.sealBaseline(.init(pageID: book.page!.id, index: i), response: .init(started: book.now(), text: "x")) }
        // The first reserved set's page finished: its five messages are the sample's listening part.
        var ex = Exercise(kind: .copy); ex.text = "кот"
        book.appendPage(sourceID: week.after_page, unitTitle: "u", title: "t", intro: nil, isReview: false, exercises: [ex])
        book.record(page: book.pages.count - 1, exercise: 0) { $0.status = .done }
        XCTAssertTrue(book.openBaseline())
        let tasks = book.page!.exercises.compactMap(\.baseline).filter { $0.kind == .listening }
        XCTAssertEqual(tasks.map(\.id), week.items.map(\.id))
        XCTAssertEqual(tasks.map(\.text), week.items.map(\.ru))
        XCTAssertEqual(Set(tasks.compactMap(\.speaker)), ["aidar"], "unfamiliar: not the course voice")
        XCTAssertEqual(tasks[0].questions, week.items[0].questions?.map(\.prompt))
        XCTAssertTrue(tasks[0].reference.contains(week.items[0].ru!))
        XCTAssertTrue(tasks[0].reference.contains { $0.hasSuffix(week.items[0].questions![0].answer_en!) })
        XCTAssertTrue(book.page!.exercises.allSatisfy { $0.spoken == nil && $0.target == nil && $0.listening == nil })
        let before = book.knowledge.model.events.count
        let ref = ExerciseRef(pageID: book.page!.id, index: 1)
        XCTAssertTrue(book.sealBaseline(ref, response: .init(started: book.now(), text: "The soup is ready; they want bread.")))
        XCTAssertEqual(book.knowledge.model.events.count, before, "a weekly sample is never evidence")
        // Spent: the next sample does not repeat the set, nor offer one the course has not reached.
        for i in book.page!.exercises.indices { _ = book.sealBaseline(.init(pageID: book.page!.id, index: i), response: .init(started: book.now(), text: "x")) }
        XCTAssertFalse(book.openBaseline(), "both readings used; no third sample is invented")
        // -qa-listening-week style: a forced set regardless of progress.
        let fresh = try freshBook()
        XCTAssertTrue(fresh.openBaseline(listening: Listening.shared.weekly_samples[2]))
        XCTAssertEqual(fresh.page!.exercises.compactMap(\.baseline).filter { $0.kind == .listening }.map(\.id), Listening.shared.weekly_samples[2].items.map(\.id))
        // The task's new field is optional on reading.
        let old = try JSONDecoder().decode(BaselineTask.self, from: Data(#"{"id":"x","kind":"listening","prompt":"p","questions":[],"reference":[]}"#.utf8))
        XCTAssertNil(old.text)
    }

    func testTranscriptHighlightAndTutorDescription() throws {
        let corr = try item(.correction)
        let shown = Listening.highlighted(corr.ru!, span: corr.crucial_span, showStress: true)
        XCTAssertEqual(String(shown.characters), RussianText.unstressed(corr.ru!))
        // The span's underline is the red one; the stress marks' thin underline (`.single`) is not it.
        let underlined = shown.runs.filter { $0.swiftUI.underlineStyle.map { $0 != .single } ?? false }.map { String(shown[$0.range].characters) }.joined()
        XCTAssertEqual(underlined, RussianText.unstressed(corr.crucial_span))
        var ex = Exercise(kind: .listening); ex.listening = corr
        var r = ExerciseResult(); r.chosen = corr.answer; r.status = .correct
        let text = ListeningItem.describe(ex, r)
        XCTAssertTrue(text.contains(corr.ru!) && text.contains(corr.question!), text)
        XCTAssertEqual(ListeningItem.response(ex, r), corr.options?[corr.answer!])
        // The PDF names the card by its question.
        XCTAssertTrue(NotebookPDF.prompt(ex).contains(corr.question!))
    }
}
