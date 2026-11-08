import XCTest
@testable import Drussky
#if canImport(FoundationModels)
import FoundationModels
#endif

/// The on-device tutor: the router's decisions, the @Generable answers mapped onto the app's
/// contracts, and the benchmark harness (research/ONDEVICE_TUTOR.md), which runs only where
/// the model is available and skips otherwise.
@MainActor
final class OnDeviceTutorTests: XCTestCase {
    // MARK: Router

    private func conditions(onDevice: Bool, online: Bool, key: Bool) -> TutorConditions {
        TutorConditions(onDeviceAvailable: onDevice, online: online, hasKey: key)
    }

    func testOnlineWithKeyEveryTaskGoesToClaude() {
        for task in TutorTask.allCases {
            XCTAssertEqual(TutorRouter.route(task, conditions(onDevice: true, online: true, key: true)), .claude, "\(task)")
            XCTAssertEqual(TutorRouter.route(task, conditions(onDevice: false, online: true, key: true)), .claude, "\(task)")
        }
    }

    func testOfflineOnlyGlossesFallBackToTheDevice() {
        let offline = conditions(onDevice: true, online: false, key: true)
        XCTAssertEqual(TutorRouter.route(.gloss, offline), .onDevice)
        for task in [TutorTask.quickAnswer, .diaryCorrection, .lemmatize] {
            guard case .unavailable(let why) = TutorRouter.route(task, offline) else { return XCTFail("\(task) must wait for the network") }
            XCTAssertTrue(why.contains("offline"), why)
        }
    }

    func testNoKeyReadsLikeTheOldMessageExceptForGlosses() {
        let noKey = conditions(onDevice: true, online: true, key: false)
        XCTAssertEqual(TutorRouter.route(.gloss, noKey), .onDevice)
        XCTAssertEqual(TutorRouter.route(.quickAnswer, noKey), .unavailable(Tutor.Failure.noKey.errorDescription ?? ""))
        XCTAssertEqual(TutorRouter.route(.diaryCorrection, noKey), .unavailable(Tutor.Failure.noKey.errorDescription ?? ""))
    }

    func testNoDeviceModelAndNoClaudeIsUnavailableForEverything() {
        for task in TutorTask.allCases {
            if case .unavailable = TutorRouter.route(task, conditions(onDevice: false, online: false, key: true)) {} else { XCTFail("\(task)") }
            if case .unavailable = TutorRouter.route(task, conditions(onDevice: false, online: true, key: false)) {} else { XCTFail("\(task)") }
        }
    }

    func testBackendsCarryTheirProvenance() {
        XCTAssertEqual(TutorRouter.backend(for: .onDevice)?.provenance, .onDevice)
        XCTAssertEqual(TutorRouter.backend(for: .claude)?.provenance, .claude)
        XCTAssertNil(TutorRouter.backend(for: .unavailable("x")))
        XCTAssertEqual(TutorProvenance.onDevice.label, "on device")
    }

    func testTurnProvenanceIsOptionalInSavedNotes() throws {
        let old = try JSONDecoder().decode(Tutor.Turn.self, from: Data(#"{"role":"assistant","text":"hi"}"#.utf8))
        XCTAssertNil(old.provenance)
        let turn = Tutor.Turn(role: "assistant", text: "a", provenance: .onDevice)
        let round = try JSONDecoder().decode(Tutor.Turn.self, from: JSONEncoder().encode(turn))
        XCTAssertEqual(round.provenance, .onDevice)
    }

    func testLatinLemmasAreDropped() {
        XCTAssertEqual(OnDeviceBackend.cyrillic(" кни́га "), "кни́га")
        XCTAssertNil(OnDeviceBackend.cyrillic("kakdy"))
        XCTAssertNil(OnDeviceBackend.cyrillic("če"))
        XCTAssertNil(OnDeviceBackend.cyrillic(""))
        XCTAssertEqual(OnDeviceBackend.cyrillic("кто-то"), "кто-то")
    }

    // MARK: @Generable → contracts

    func testDiaryAnswerMapsOntoDiaryCorrection() throws {
        #if canImport(FoundationModels)
        guard #available(iOS 26.0, *) else { throw XCTSkip("no FoundationModels before iOS 26") }
        let answer = OnDeviceDiaryCorrection(
            corrected: "Я живу́ в Москве́.",
            edits: [.init(span: "Москва", replacement: "Москве́", category: .ending, why: "в + place → prepositional", en: "in Moscow"),
                    .init(span: "", replacement: "x", category: .other, why: "", en: ""),
                    .init(span: "живу", replacement: "живу", category: .spelling, why: "same", en: "")],
            praise: "Nice.",
            teach: [.init(title: "Where: в + -е", note: "Place after в takes -е: [[в Москве́|in Moscow]]."),
                    .init(title: "Two", note: ""), .init(title: "Three", note: "")])
        let json = OnDeviceBackend.contractJSON(answer)
        let c = try XCTUnwrap(DiaryTutor.parse(json, fallbackText: "Я живу в Москва.", at: Date(timeIntervalSince1970: 0)))
        XCTAssertEqual(c.corrected, "Я живу́ в Москве́.")
        XCTAssertEqual(c.edits.count, 1, "the empty span and the no-op edit are dropped by the shared parser")
        XCTAssertEqual(c.edits.first?.span, "Москва")
        XCTAssertEqual(c.edits.first?.category, .ending)
        XCTAssertEqual(c.edits.first?.en, "in Moscow")
        XCTAssertEqual(c.praise, "Nice.")
        XCTAssertEqual(c.teach.count, 2, "at most two teach items, as for Claude")
        XCTAssertEqual(c.teach.first?.title, "Where: в + -е")
        XCTAssertEqual(OnDeviceBackend.contractCategory(.wordChoice), "word-choice")
        XCTAssertEqual(OnDeviceBackend.contractCategory(.wordOrder), "word-order")
        XCTAssertEqual(DiaryEdit.Category(loose: OnDeviceBackend.contractCategory(.wordChoice)), .wordChoice)
        XCTAssertEqual(DiaryEdit.Category(loose: OnDeviceBackend.contractCategory(.agreement)), .agreement)
        #else
        throw XCTSkip("FoundationModels not in this SDK")
        #endif
    }

    func testLemmaTableMapsOntoGoalEntries() throws {
        #if canImport(FoundationModels)
        guard #available(iOS 26.0, *) else { throw XCTSkip("no FoundationModels before iOS 26") }
        let table = OnDeviceLemmaTable(rows: [.init(form: "льва", lemma: "лев", gloss: "lion"),
                                              .init(form: "телу", lemma: "те́ло", gloss: "body"),
                                              .init(form: "unasked", lemma: "x", gloss: "y")])
        let entries = GoalLemmaTable.entries(in: OnDeviceBackend.contractObject(table), asked: ["льва", "телу"])
        XCTAssertEqual(entries.count, 2)
        XCTAssertEqual(entries["льва"]?.lemma, "лев")
        XCTAssertEqual(entries["льва"]?.en, "lion")
        XCTAssertEqual(entries["телу"]?.lemma, "тело", "lemmas are keyed without stress")
        #else
        throw XCTSkip("FoundationModels not in this SDK")
        #endif
    }

    // MARK: Benchmark harness

    private struct Cases: Decodable {
        struct Gloss: Decodable { var id, word, context, reference: String }
        struct Diary: Decodable { var id, prompt, text, gold: String }
        struct Lemma: Decodable { var id, form: String; var gold: [String] }
        var glosses: [Gloss]; var diaries: [Diary]; var lemmas: [Lemma]
    }

    private func cases() throws -> Cases {
        let url = try XCTUnwrap(Bundle(for: Self.self).url(forResource: "cases", withExtension: "json"))
        return try JSONDecoder().decode(Cases.self, from: Data(contentsOf: url))
    }

    /// A slice of the research benchmark against the live model: glosses judged by whether the
    /// reference's first word appears in the answer (a floor, not the paper's Claude-judged
    /// figure), a few diary lines and one lemma batch mapped through the contracts.
    func testOnDeviceBenchmarkSlice() async throws {
        guard OnDeviceBackend.isAvailable else { throw XCTSkip("on-device model unavailable: \(OnDeviceBackend.unavailableReason ?? "")") }
        let all = try cases()
        let backend = OnDeviceBackend()

        // The iOS 26.2 simulator says "available" and then fails every generation (its safety model's
        // prompt template is missing); that is "unavailable" for the harness, not a red.
        do { _ = try await backend.gloss(word: "книга", context: "Это книга.") }
        catch { throw XCTSkip("the model is reported available but cannot generate here: \(error.localizedDescription)") }

        let glosses = Array(all.glosses.prefix(20))
        var hits = 0
        var seconds = 0.0
        for c in glosses {
            let t0 = Date()
            let g = try await backend.gloss(word: c.word, context: c.context)
            seconds += Date().timeIntervalSince(t0)
            XCTAssertEqual(g.provenance, .onDevice)
            XCTAssertFalse(g.en.isEmpty, c.id)
            let head = c.reference.split(whereSeparator: { !$0.isLetter }).first.map(String.init)?.lowercased() ?? ""
            if g.en.lowercased().contains(head) { hits += 1 }
            print("bench gloss \(c.word) → \(g.en) | \(g.lemma ?? "-") (ref: \(c.reference))")
        }
        print("bench gloss: \(hits)/\(glosses.count) contain the reference head word, \(seconds / Double(glosses.count))s mean")
        XCTAssertGreaterThanOrEqual(hits, glosses.count / 2, "the on-device gloss floor (measured 40/50 by the Claude judge)")

        for c in all.diaries.prefix(3) {
            let correction = try await backend.correctDiary(text: c.text, prompt: c.prompt,
                                                            briefing: "Letters met: all. Words they know: мама, папа, книга.", at: Date())
            XCTAssertEqual(correction.model, OnDeviceBackend.modelName)
            XCTAssertFalse(correction.corrected.isEmpty)
            print("bench diary \(c.id): \(correction.corrected) edits=\(correction.edits.map { "\($0.span)→\($0.replacement)" })")
        }

        let batch = Array(all.lemmas.prefix(10))
        do {
            let entries = try await backend.lemmatize(batch.map(\.form))
            let exact = batch.filter { c in entries[c.form].map { e in c.gold.contains(e.lemma.replacingOccurrences(of: "ё", with: "е")) } ?? false }.count
            print("bench lemma: \(exact)/\(batch.count) exact, \(entries.count) returned")
        } catch {
            // Seen in the research run: the guardrail refuses a plain list of Russian words.
            print("bench lemma: batch refused: \(error)")
        }
    }
}
